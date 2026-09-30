//! Separate CLI processes exercise the real scoped daemon without MCP or owner secrets.
use super::mcp_stdio::credentials;
use super::*;

pub(super) fn cli(path: &PathBuf, args: &[&str], input: &[u8], code: i32) -> Value {
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("stdout.json");
    let errors = directory.path().join("stderr.txt");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentic-cli"))
        .arg("--credentials")
        .arg(path)
        .args(args)
        // The CLI must use the daemon directly, not launch MCP or another executable.
        .env("PATH", directory.path())
        .stdin(Stdio::piped())
        .stdout(fs::File::create(&output).unwrap())
        .stderr(fs::File::create(&errors).unwrap())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let until = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("messaging CLI did not terminate");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let stdout = fs::read_to_string(output).unwrap();
    let stderr = fs::read_to_string(errors).unwrap();
    let saved: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for secret in [KEY, TOKEN, saved["signingSeed"].as_str().unwrap()] {
        assert!(
            !stdout.contains(secret) && !stderr.contains(secret),
            "CLI leaked credentials"
        );
    }
    assert_eq!(
        status.code(),
        Some(code),
        "stdout={stdout}; stderr={stderr}"
    );
    assert!(
        stderr.is_empty(),
        "machine command wrote unstructured stderr"
    );
    let value: Value = serde_json::from_str(&stdout).expect("exactly one JSON result on stdout");
    assert_eq!(value.as_object().unwrap().len(), 1);
    if code != 0 {
        assert!(value.get("result").is_none());
        let error = value.get("error").expect("structured error envelope");
        assert!(!error["message"].as_str().unwrap().is_empty());
        assert_eq!(error["retryable"], code == 4);
    }
    value
}
fn ok(path: &PathBuf, args: &[&str], input: &[u8]) -> Value {
    let value = cli(path, args, input, 0);
    assert_eq!(value.as_object().unwrap().len(), 1);
    value["result"].clone()
}
#[test]
fn messaging_cli_public_ids_send_reply_lease_ack_and_cold_retry_without_mcp() {
    let mut alice = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let mut bob = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&alice, &bob, None);
    let own = alice.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let peer = bob.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let key = ed25519_dalek::SigningKey::from_bytes(&[121; 32]);
    let permission = grant(&alice, &key, &group, json!(["read_inbox", "send_message"]));
    let path = credentials(&alice, &key, &permission);
    let context = ok(&path, &["context"], b"");
    assert_eq!(context["networkId"], own);
    assert_eq!(context["conversations"][0]["networkId"], peer);
    assert_eq!(context["conversations"].as_array().unwrap().len(), 1);
    assert!(context.get("messages").is_none());
    bob.kill();
    let args = [
        "messages",
        "send",
        "--to",
        peer.as_str(),
        "--operation-id",
        "cli-first",
        "--text-stdin",
    ];
    let text = "  Hi from a separate CLI.\nReply in this conversation.";
    // A here-document's final line break is not part of the message (the
    // rest is kept as written); the retry below without it is the same
    // message.
    let sent = ok(&path, &args, format!("{text}\n").as_bytes());
    assert_eq!(sent["text"], text);
    assert_eq!(sent["delivery"]["phase"], "queued");
    let get = [
        "delivery",
        "get",
        "--to",
        peer.as_str(),
        "--operation-id",
        "cli-first",
    ];
    let pending = ok(&path, &get, b"");
    assert_eq!(pending["messageId"], sent["id"]);
    assert_eq!(pending["delivery"]["phase"], "queued");
    alice.kill();
    alice.launch();
    assert_eq!(ok(&path, &args, text.as_bytes())["id"], sent["id"]);
    assert_eq!(ok(&path, &get, b""), pending);
    bob.launch();
    let received = bob.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["text"], text);
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
    alice.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    assert_eq!(ok(&path, &get, b"")["delivery"]["phase"], "delivered");
    let reply = bob.send(&group, "A reply from a regular client", "cli-peer-reply");
    alice.wait(|v| messages(v).len() == 2);
    let poll = [
        "inbox",
        "poll",
        "--from",
        peer.as_str(),
        "--operation-id",
        "cli-page",
        "--limit",
        "10",
        "--max-bytes",
        "4096",
        "--lease-seconds",
        "60",
    ];
    let page = ok(&path, &poll, b"");
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["id"], reply["id"]);
    assert_eq!(page["items"][0]["text"], "A reply from a regular client");
    alice.kill();
    alice.launch();
    assert_eq!(ok(&path, &poll, b""), page);
    let next_poll = [
        "inbox",
        "poll",
        "--from",
        peer.as_str(),
        "--operation-id",
        "cli-next",
    ];
    assert_eq!(
        cli(&path, &next_poll, b"", 4)["error"]["code"],
        "inbox_busy"
    );
    let ack = [
        "inbox",
        "ack",
        "--from",
        peer.as_str(),
        "--lease-id",
        page["leaseId"].as_str().unwrap(),
    ];
    assert_eq!(ok(&path, &ack, b"")["cursor"], page["cursor"]);
    assert_eq!(ok(&path, &ack, b"")["cursor"], page["cursor"]);
    let next = ok(&path, &next_poll, b"");
    assert!(next["items"].as_array().unwrap().is_empty());
    assert_eq!(messages(&alice.snapshot()).len(), 2);
    assert_eq!(messages(&bob.snapshot()).len(), 2);
}
#[test]
fn messaging_cli_denied_recipient_foreign_operation_read_only_and_revoke_do_not_expand_scope() {
    let alice = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let bob = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let carol = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    carol.profile("Carol outside grant");
    let outside = carol.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let group = connect(&alice, &bob, None);
    let peer = bob.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let key = ed25519_dalek::SigningKey::from_bytes(&[122; 32]);
    let permission = grant(&alice, &key, &group, json!(["read_inbox", "send_message"]));
    let path = credentials(&alice, &key, &permission);
    let other_key = ed25519_dalek::SigningKey::from_bytes(&[123; 32]);
    let other_grant = grant(&alice, &other_key, &group, json!(["read_inbox"]));
    let readonly = credentials(&alice, &other_key, &other_grant);
    let send = [
        "messages",
        "send",
        "--to",
        peer.as_str(),
        "--operation-id",
        "cli-scope",
        "--text-stdin",
    ];
    assert_eq!(
        cli(&readonly, &send, b"Not allowed", 3)["error"]["code"],
        "unauthorized"
    );
    assert_eq!(
        cli(
            &path,
            &[
                "messages",
                "send",
                "--to",
                &outside,
                "--operation-id",
                "cli-denied",
                "--text-stdin"
            ],
            b"Outside grant",
            3
        )["error"]["code"],
        "unauthorized"
    );
    assert!(messages(&alice.snapshot()).is_empty());
    let sent = ok(&path, &send, b"Allowed");
    let foreign_key = ed25519_dalek::SigningKey::from_bytes(&[124; 32]);
    let foreign_grant = grant(&alice, &foreign_key, &group, json!(["send_message"]));
    let foreign = credentials(&alice, &foreign_key, &foreign_grant);
    let get = [
        "delivery",
        "get",
        "--to",
        peer.as_str(),
        "--operation-id",
        "cli-scope",
    ];
    let denied = cli(&foreign, &get, b"", 3);
    assert_eq!(denied["error"]["code"], "unauthorized");
    assert!(!denied.to_string().contains(sent["id"].as_str().unwrap()));
    assert_eq!(
        cli(&path, &send, b"Changed body", 3)["error"]["code"],
        "idempotency_conflict"
    );
    alice.call("revoke_runtime", json!({"grantId":permission.grant_id}));
    assert_eq!(cli(&path, &get, b"", 3)["error"]["code"], "unauthorized");
    assert_eq!(
        cli(&path, &send, b"Allowed", 3)["error"]["code"],
        "unauthorized"
    );
    assert_eq!(messages(&alice.snapshot()).len(), 1);
}
#[test]
fn messaging_cli_bounds_input_rejects_owner_commands_and_reports_recoverable_daemon_loss() {
    let mut alice = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let bob = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&alice, &bob, None);
    let peer = bob.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let key = ed25519_dalek::SigningKey::from_bytes(&[125; 32]);
    let permission = grant(&alice, &key, &group, json!(["send_message"]));
    let path = credentials(&alice, &key, &permission);
    let send = [
        "messages",
        "send",
        "--to",
        peer.as_str(),
        "--operation-id",
        "cli-invalid",
        "--text-stdin",
    ];
    for input in [vec![b'x'; 12001], vec![0xff], vec![]] {
        assert_eq!(
            cli(&path, &send, &input, 2)["error"]["code"],
            "invalid_request"
        );
    }
    for args in [
        vec!["wallet", "configure"],
        vec!["call", "create_identity"],
        vec!["messages", "send", "--to", peer.as_str(), "--text-stdin"],
        vec!["inbox", "poll", "--from", peer.as_str()],
    ] {
        assert_eq!(
            cli(&path, &args, b"", 2)["error"]["code"],
            "invalid_request"
        );
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        cli(&path, &["context"], b"", 2)["error"]["code"],
        "invalid_credentials"
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(messages(&alice.snapshot()).is_empty());
    alice.kill();
    let lost = cli(&path, &send, b"Retry after restart", 4);
    assert_eq!(lost["error"]["code"], "unavailable");
    assert_eq!(lost["error"]["retryable"], true);
    alice.launch();
    let sent = ok(&path, &send, b"Retry after restart");
    let received = bob.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
}
