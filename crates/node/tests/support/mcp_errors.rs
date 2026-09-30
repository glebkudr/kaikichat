use super::*;

fn tool_failure(response: Value, code: &str, retryable: bool, details: Option<Value>) {
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isError"], true, "{response}");
    let failure = &response["result"]["structuredContent"]["error"];
    assert_eq!(failure["code"], code, "{response}");
    assert_eq!(failure["retryable"], retryable, "{response}");
    assert!(!failure["message"].as_str().unwrap().is_empty());
    match details {
        Some(details) => {
            assert_eq!(failure["details"], details);
            assert_eq!(failure.as_object().unwrap().len(), 4);
        }
        None => {
            assert!(failure.get("details").is_none());
            assert_eq!(failure.as_object().unwrap().len(), 3);
        }
    }
}

#[test]
fn mcp_actionable_inbox_failures_allow_corrected_page_retry_and_lease_recovery_without_message_loss()
 {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key1 = ed25519_dalek::SigningKey::from_bytes(&[101; 32]);
    let key2 = ed25519_dalek::SigningKey::from_bytes(&[102; 32]);
    let grant1 = grant(&a, &key1, &group, json!(["read_inbox"]));
    let grant2 = grant(&a, &key2, &group, json!(["read_inbox"]));
    let mut m1 = Mcp::start(&credentials(&a, &key1, &grant1), true);
    let mut m2 = Mcp::start(&credentials(&a, &key2, &grant2), true);
    let text = "x".repeat(160);
    assert_eq!(text.len(), 160);
    let first = b.send(&group, &text, "failure-first");
    a.wait(|v| messages(v).len() == 1);
    let mut input = poll_input(&group, "correct-page");
    input["maxBytes"] = json!(16);
    tool_failure(
        m1.tool("inbox.poll", input.clone()),
        "inbox_item_too_large",
        false,
        Some(json!({"requiredBytes":160})),
    );
    input["maxBytes"] = json!(256);
    let page = m1.success("inbox.poll", input.clone());
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["id"], first["id"]);
    assert_eq!(page["items"][0]["text"], text);
    let busy = poll_input(&group, "wait-for-lease");
    tool_failure(
        m2.tool("inbox.poll", busy.clone()),
        "inbox_busy",
        true,
        None,
    );
    let mut changed = input.clone();
    changed["limit"] = json!(1);
    tool_failure(
        m1.tool("inbox.poll", changed),
        "idempotency_conflict",
        false,
        None,
    );
    assert_eq!(m1.success("inbox.poll", input), page);
    m1.success(
        "inbox.ack",
        json!({"conversationId":group,"leaseId":page["leaseId"]}),
    );
    assert!(
        m2.success("inbox.poll", busy)["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let second = b.send(&group, "Lease recovery", "failure-second");
    a.wait(|v| messages(v).len() == 2);
    let mut expires = poll_input(&group, "expires");
    expires["leaseSeconds"] = json!(1);
    let expired = m1.success("inbox.poll", expires);
    assert_eq!(expired["items"][0]["id"], second["id"]);
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        < expired["expiresAt"].as_u64().unwrap()
    {
        assert!(Instant::now() < deadline, "lease expiry deadline");
        thread::sleep(Duration::from_millis(20));
    }
    tool_failure(
        m1.tool(
            "inbox.ack",
            json!({"conversationId":group,"leaseId":expired["leaseId"]}),
        ),
        "inbox_lease_expired",
        false,
        None,
    );
    let recovered = m2.success("inbox.poll", poll_input(&group, "recover-expired"));
    assert_eq!(recovered["items"], expired["items"]);
    m2.success(
        "inbox.ack",
        json!({"conversationId":group,"leaseId":recovered["leaseId"]}),
    );
    assert!(
        m2.success("inbox.poll", poll_input(&group, "after-recovery"))["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Unknown/disallowed scope and revoked authority cannot expose sizes or lease details.
    tool_failure(
        m2.tool("inbox.poll", poll_input(&"ff".repeat(32), "unknown-scope")),
        "unauthorized",
        false,
        None,
    );
    a.call("revoke_runtime", json!({"grantId":grant1.grant_id}));
    tool_failure(
        m1.tool("inbox.poll", poll_input(&group, "revoked-poll")),
        "unauthorized",
        false,
        None,
    );
    let denied = m1.request("resources/read", json!({"uri":"agentic://runtime"}));
    assert_eq!(denied["error"]["data"]["error"]["code"], "unauthorized");
    assert_eq!(denied["error"]["data"]["error"]["retryable"], false);
    assert!(!denied.to_string().contains(&group));
    assert_eq!(
        messages(&a.snapshot())
            .iter()
            .map(|m| m["id"].clone())
            .collect::<Vec<_>>(),
        vec![first["id"].clone(), second["id"].clone()]
    );
    assert_eq!(messages(&b.snapshot()).len(), 2);
    m1.close_cleanly();
    m2.close_cleanly();
}

#[test]
fn mcp_actionable_daemon_outage_and_send_conflict_preserve_exactly_once_retry_on_same_client() {
    let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[103; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
    let mut mcp = Mcp::start(&credentials(&a, &key, &grant), false);
    let original = json!({"conversationId":group,"text":"Committed before outage","operationId":"outage-original"});
    let first = mcp.success("messages.send", original.clone());
    b.wait(|v| messages(v).len() == 1);
    let mut conflict = original.clone();
    conflict["text"] = json!("Changed committed request");
    tool_failure(
        mcp.tool("messages.send", conflict),
        "idempotency_conflict",
        false,
        None,
    );
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    a.kill();
    tool_failure(
        mcp.tool("messages.send", original.clone()),
        "unavailable",
        true,
        None,
    );
    let unavailable = mcp.request("resources/read", json!({"uri":"agentic://runtime"}));
    assert_eq!(unavailable["error"]["data"]["error"]["code"], "unavailable");
    assert_eq!(unavailable["error"]["data"]["error"]["retryable"], true);
    assert!(
        !unavailable
            .to_string()
            .contains(&a.socket().to_string_lossy().to_string())
    );
    a.launch();
    assert_eq!(mcp.success("messages.send", original)["id"], first["id"]);
    let second=mcp.success("messages.send",json!({"conversationId":group,"text":"New work after recovery","operationId":"outage-next"}));
    let received = b.wait(|v| messages(v).len() == 2);
    assert_eq!(
        messages(&received)
            .iter()
            .map(|m| m["id"].clone())
            .collect::<Vec<_>>(),
        vec![first["id"].clone(), second["id"].clone()]
    );
    assert_eq!(messages(&received)[1]["text"], "New work after recovery");
    let invalid = raw_ipc(&a.socket(), json!({"proof":"deadbeef"})).unwrap();
    assert_eq!(invalid["error"]["code"], "unauthorized");
    assert_eq!(invalid["error"]["retryable"], false);
    assert_eq!(invalid["error"].as_object().unwrap().len(), 3);
    mcp.close_cleanly();
}

#[test]
fn mcp_actionable_storage_failure_is_sanitized_and_retry_commits_one_actual_peer_message() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[104; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
    let mut mcp = Mcp::start(&credentials(&a, &key, &grant), true);
    runtime_context(&mut mcp);
    a.wait(|_| a.call("node_info", json!({}))["pendingOutbox"] == 0);
    let db = rusqlite::Connection::open(a.root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{KEY}'\"; CREATE TRIGGER fail_agent_write BEFORE INSERT ON messages WHEN NEW.own=1 BEGIN SELECT RAISE(ABORT,'private path /very/private/profile.db write interrupted'); END;")).unwrap();
    let persisted = || {
        let states: Vec<(String, i64, Vec<u8>)> = db
            .prepare("SELECT namespace,revision,bytes FROM states ORDER BY namespace")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let counts:(i64,i64,i64)=db.query_row("SELECT (SELECT count(*) FROM messages),(SELECT count(*) FROM operations),(SELECT count(*) FROM outbox)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        (states, counts)
    };
    let before = persisted();
    let args = json!({"conversationId":group,"text":"Retry after storage repair","operationId":"storage-retry"});
    let failed = mcp.tool("messages.send", args.clone());
    assert!(!failed.to_string().contains("/very/private"));
    assert!(!failed.to_string().contains("write interrupted"));
    tool_failure(failed, "unavailable", true, None);
    assert_eq!(persisted(), before);
    let wire = proof(&key, &grant, "send_message", args.clone(), 1);
    let ipc_failed = raw_ipc(&a.socket(), json!({"proof":hex::encode(&wire)})).unwrap();
    assert_eq!(ipc_failed["error"]["code"], "unavailable");
    assert_eq!(ipc_failed["error"]["retryable"], true);
    assert_eq!(ipc_failed["error"].as_object().unwrap().len(), 3);
    assert!(!ipc_failed.to_string().contains("/very/private"));
    assert_eq!(persisted(), before);
    assert!(messages(&a.snapshot()).is_empty());
    assert!(messages(&b.snapshot()).is_empty());
    db.execute_batch("DROP TRIGGER fail_agent_write").unwrap();
    let sent = mcp.success("messages.send", args);
    let retried = raw_ipc(&a.socket(), json!({"proof":hex::encode(wire)})).unwrap();
    assert_eq!(retried["result"]["id"], sent["id"]);
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
    assert_eq!(messages(&received)[0]["text"], "Retry after storage repair");
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    assert_eq!(messages(&a.snapshot()).len(), 1);
    mcp.close_cleanly();
}
