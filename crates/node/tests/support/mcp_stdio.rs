use super::*;
use std::io::{BufRead, BufReader};
use std::process::ChildStdin;
use std::sync::mpsc::{self, Receiver};
const MODERN: &str = "2026-07-28";
const LEGACY: &str = "2025-11-25";
const EXPECTED_TOOLS: [&str; 4] = ["inbox.poll", "inbox.ack", "messages.send", "delivery.get"];
#[path = "mcp_delivery.rs"]
mod delivery;
#[path = "mcp_errors.rs"]
mod failures;

pub(super) fn credentials(
    node: &Node,
    key: &ed25519_dalek::SigningKey,
    grant: &agentic_core::RuntimeGrant,
) -> PathBuf {
    let path = node.root.path().join(format!(
        "runtime-{}.json",
        hex::encode(key.verifying_key().to_bytes())
    ));
    let value = json!({"version":1,"ipc":node.socket(),"domain":agentic_node::NETWORK_DOMAIN,"grantId":grant.grant_id,"ownershipEpoch":0,"signingSeed":hex::encode(key.to_bytes())});
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    path
}
struct Mcp {
    child: Child,
    input: Option<ChildStdin>,
    responses: Receiver<Result<Value, String>>,
    reader: Option<thread::JoinHandle<()>>,
    log: PathBuf,
    modern: bool,
    next_id: u64,
}
impl Drop for Mcp {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
impl Mcp {
    fn spawn(path: &PathBuf, modern: bool) -> Self {
        let log = path.with_extension("mcp.log");
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentic-mcp"))
            .arg("--credentials")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log)
                    .unwrap(),
            )
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (sent, responses) = mpsc::channel();
        let credential: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let secrets = [
            KEY.to_owned(),
            TOKEN.to_owned(),
            credential["signingSeed"].as_str().unwrap().to_owned(),
        ];
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let response = line.map_err(|e| e.to_string()).and_then(|line| {
                    if secrets.iter().any(|secret| line.contains(secret)) {
                        return Err("Secret appeared on MCP stdout".to_owned());
                    }
                    serde_json::from_str(&line).map_err(|e| e.to_string())
                });
                if sent.send(response).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            responses,
            reader: Some(reader),
            log,
            modern,
            next_id: 1,
        }
    }
    fn start(path: &PathBuf, modern: bool) -> Self {
        let mut client = Self::spawn(path, modern);
        if modern {
            let discovered = client.request("server/discover", json!({}));
            assert_eq!(discovered["result"]["resultType"], "complete");
            assert!(
                discovered["result"]["supportedVersions"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(MODERN))
            );
            assert_eq!(
                discovered["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
                "agentic-internet"
            );
        } else {
            let initialized=client.request("initialize",json!({"protocolVersion":LEGACY,"capabilities":{},"clientInfo":{"name":"real-stdio-test","version":"1"}}));
            assert_eq!(initialized["result"]["protocolVersion"], LEGACY);
            assert!(initialized["result"].get("resultType").is_none());
            client.write(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        client
    }
    fn write(&mut self, value: Value) {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{value}").unwrap();
        input.flush().unwrap();
    }
    fn send(&mut self, method: &str, mut params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        if self.modern {
            params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"real-stdio-test","version":"1"}});
        }
        self.write(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        id
    }
    fn response(&self, id: u64) -> Value {
        self.response_ignoring(id, None)
    }
    fn response_ignoring(&self, id: u64, cancelled: Option<u64>) -> Value {
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let value = self
                .responses
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|e| {
                    panic!(
                        "MCP response unavailable: {e}; stderr={}",
                        fs::read_to_string(&self.log).unwrap()
                    )
                })
                .unwrap();
            if value.get("id").is_some() {
                if cancelled.is_some_and(|cancelled| value["id"] == cancelled) {
                    continue;
                }
                assert_eq!(value["id"], id);
                return value;
            }
        }
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send(method, params);
        self.response(id)
    }
    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name":name,"arguments":arguments}))
    }
    fn success(&mut self, name: &str, arguments: Value) -> Value {
        let value = self.tool(name, arguments);
        assert!(value.get("error").is_none(), "{value}");
        assert_eq!(value["result"]["isError"], false, "{value}");
        if self.modern {
            assert_eq!(value["result"]["resultType"], "complete");
        } else {
            assert!(value["result"].get("resultType").is_none());
        }
        value["result"]["structuredContent"].clone()
    }
    fn audit_remaining_stdout(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
        for response in self.responses.try_iter() {
            response.unwrap();
        }
    }
    fn close_cleanly(&mut self) {
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "MCP process did not exit after stdin EOF"
            );
            thread::sleep(Duration::from_millis(20));
        }
        self.audit_remaining_stdout();
    }
}
fn poll_input(group: &str, op: &str) -> Value {
    json!({"conversationId":group,"operationId":op,"limit":10,"maxBytes":4096,"leaseSeconds":30})
}

fn runtime_context(mcp: &mut Mcp) -> Value {
    let response = mcp.request("resources/read", json!({"uri":"agentic://runtime"}));
    assert!(response.get("error").is_none(), "{response}");
    let result = &response["result"];
    assert_eq!(result["contents"].as_array().unwrap().len(), 1);
    assert_eq!(result["contents"][0]["uri"], "agentic://runtime");
    assert_eq!(result["contents"][0]["mimeType"], "application/json");
    if mcp.modern {
        assert_eq!(result["resultType"], "complete");
        assert_eq!(result["ttlMs"], 0);
        assert_eq!(result["cacheScope"], "private");
    } else {
        assert!(result.get("resultType").is_none());
    }
    let text = result["contents"][0]["text"].as_str().unwrap();
    assert!(text.len() <= 16 * 1024);
    serde_json::from_str(text).unwrap()
}

#[test]
fn mcp_resource_discovers_only_allowed_dialogs_and_live_permissions_for_modern_and_legacy_clients()
{
    for modern in [true, false] {
        let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
        let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
        let c = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
        let group = connect(&a, &b, None);
        let private = connect(&a, &c, None);
        b.send(&group, "Read via bounded inbox only", "resource-reply");
        c.send(&private, "Private unrelated message", "private-reply");
        a.wait(|v| {
            v["conversations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["messages"].as_array().unwrap().len() == 1)
        });
        let key = ed25519_dalek::SigningKey::from_bytes(&[97; 32]);
        let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
        let path = credentials(&a, &key, &grant);
        let owner = a.snapshot();
        let title = owner["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == group)
            .unwrap()["title"]
            .clone();
        let runtimes = a.call("list_runtimes", json!({}));
        let expiry = runtimes[0]["expiresAt"].clone();
        let mut mcp = Mcp::start(&path, modern);
        let catalog = mcp.request("resources/list", json!({}));
        let resources = catalog["result"]["resources"]
            .as_array()
            .expect("runtime resource catalog");
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["uri"], "agentic://runtime");
        assert_eq!(resources[0]["mimeType"], "application/json");
        let context = runtime_context(&mut mcp);
        assert_eq!(
            context,
            json!({"version":1,"networkId":owner["identity"]["networkId"],"grantId":hex::encode(grant.grant_id),"agentId":hex::encode([41;32]),"serviceId":hex::encode([42;32]),"principal":hex::encode(key.verifying_key().to_bytes()),"expiresAt":expiry,"maxDataBytes":4096,"actions":["read_inbox","send_message"],"conversations":[{"id":group,"title":title,"networkId":b.snapshot()["identity"]["networkId"]}]})
        );
        assert!(!context.to_string().contains(&private));
        assert!(
            !context
                .to_string()
                .contains(c.snapshot()["identity"]["networkId"].as_str().unwrap())
        );
        for uri in [
            "agentic://owner".to_owned(),
            format!("file://{}", path.display()),
        ] {
            let rejected = mcp.request("resources/read", json!({"uri":uri}));
            assert_eq!(rejected["error"]["code"], -32602);
            assert!(rejected.get("result").is_none());
        }
        // Agent chooses the ID from MCP, never from the owner API/test fixture.
        let discovered = context["conversations"][0]["id"].as_str().unwrap();
        let sent=mcp.success("messages.send",json!({"conversationId":discovered,"operationId":"discovered-send","text":"Found my permitted dialog through MCP"}));
        let received = b.wait(|v| messages(v).len() == 2);
        assert_eq!(messages(&received)[1]["id"], sent["id"]);
        assert_eq!(
            messages(&received)[1]["text"],
            "Found my permitted dialog through MCP"
        );
        let page = mcp.success("inbox.poll", poll_input(discovered, "discovered-page"));
        assert_eq!(page["items"].as_array().unwrap().len(), 1);
        assert_eq!(page["items"][0]["text"], "Read via bounded inbox only");
        mcp.success(
            "inbox.ack",
            json!({"conversationId":discovered,"leaseId":page["leaseId"]}),
        );
        a.call("revoke_runtime", json!({"grantId":grant.grant_id}));
        let denied = mcp.request("resources/read", json!({"uri":"agentic://runtime"}));
        assert_eq!(denied["error"]["code"], -32001);
        assert_eq!(denied["error"]["data"]["error"]["code"], "unauthorized");
        assert!(denied.get("result").is_none());
        assert!(!denied.to_string().contains(&group));
        let sender_key = ed25519_dalek::SigningKey::from_bytes(&[98; 32]);
        let sender_grant = super::grant(&a, &sender_key, &group, json!(["send_message"]));
        let sender_path = credentials(&a, &sender_key, &sender_grant);
        let mut sender = Mcp::start(&sender_path, modern);
        let sender_context = runtime_context(&mut sender);
        assert_eq!(sender_context["actions"], json!(["send_message"]));
        assert_eq!(
            sender_context["grantId"],
            hex::encode(sender_grant.grant_id)
        );
        assert_eq!(
            sender_context["principal"],
            hex::encode(sender_key.verifying_key().to_bytes())
        );
        assert_eq!(sender_context["conversations"], context["conversations"]);
        let sender_group = sender_context["conversations"][0]["id"].as_str().unwrap();
        let no_read = sender.tool("inbox.poll", poll_input(sender_group, "send-only-no-read"));
        assert_eq!(no_read["result"]["isError"], true);
        assert_eq!(
            no_read["result"]["structuredContent"]["error"]["code"],
            "unauthorized"
        );
        let sender_sent = sender.success("messages.send", json!({"conversationId":sender_group,"operationId":"send-only-discovery","text":"Send-only runtime discovered its recipient"}));
        let received = b.wait(|v| messages(v).len() == 3);
        assert_eq!(messages(&received)[2]["id"], sender_sent["id"]);
        assert_eq!(
            messages(&received)[2]["text"],
            "Send-only runtime discovered its recipient"
        );
        sender.close_cleanly();
        assert_eq!(messages(&c.snapshot()).len(), 1);
        mcp.close_cleanly();
    }
}

#[test]
fn modern_mcp_process_discovers_sends_real_message_and_resumes_inbox_after_client_restart() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[91; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
    let path = credentials(&a, &key, &grant);
    let mut mcp = Mcp::start(&path, true);
    let tools = mcp.request("tools/list", json!({}));
    let names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, EXPECTED_TOOLS);
    let send_tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "messages.send")
        .unwrap();
    assert_eq!(send_tool["inputSchema"]["additionalProperties"], false);
    assert!(
        send_tool["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("operationId"))
    );
    let args = json!({"conversationId":group,"text":"Sent through real MCP stdio","operationId":"mcp-send"});
    let sent = mcp.success("messages.send", args.clone());
    assert!(sent["id"].as_str().is_some());
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(
        messages(&received)[0]["text"],
        "Sent through real MCP stdio"
    );
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    let reply = b.send(&group, "Actual peer reply", "mcp-reply");
    a.wait(|v| messages(v).len() == 2);
    let page = mcp.success("inbox.poll", poll_input(&group, "mcp-page"));
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["id"], reply["id"]);
    assert_eq!(page["items"][0]["text"], "Actual peer reply");
    mcp.close_cleanly();
    drop(mcp);
    let mut mcp = Mcp::start(&path, true);
    assert_eq!(mcp.success("messages.send", args)["id"], sent["id"]);
    assert_eq!(
        mcp.success("inbox.poll", poll_input(&group, "mcp-page")),
        page
    );
    let ack = mcp.success(
        "inbox.ack",
        json!({"conversationId":group,"leaseId":page["leaseId"]}),
    );
    assert_eq!(ack["cursor"], page["cursor"]);
    let empty = mcp.success("inbox.poll", poll_input(&group, "next-page"));
    assert!(empty["items"].as_array().unwrap().is_empty());
    assert_eq!(messages(&a.snapshot()).len(), 2);
    a.call("revoke_runtime", json!({"grantId":grant.grant_id}));
    let denied = mcp.tool("inbox.poll", poll_input(&group, "revoked"));
    assert_eq!(denied["result"]["isError"], true);
    assert_eq!(
        denied["result"]["structuredContent"]["error"]["code"],
        "unauthorized"
    );
    mcp.close_cleanly();
    let log = fs::read_to_string(&mcp.log).unwrap();
    for secret in [
        KEY.to_owned(),
        TOKEN.to_owned(),
        hex::encode(key.to_bytes()),
    ] {
        assert!(!log.contains(&secret));
    }
}

#[test]
fn legacy_mcp_initialization_scopes_and_invalid_tools_do_not_create_messages() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[92; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox"]));
    let path = credentials(&a, &key, &grant);
    let reply = b.send(&group, "Legacy client input", "legacy-in");
    a.wait(|v| messages(v).len() == 1);
    let mut mcp = Mcp::start(&path, false);
    let tools = mcp.request("tools/list", json!({}));
    assert_eq!(
        tools["result"]["tools"].as_array().unwrap().len(),
        EXPECTED_TOOLS.len()
    );
    let denied = mcp.tool(
        "messages.send",
        json!({"conversationId":group,"text":"Scope escape","operationId":"legacy-send"}),
    );
    assert_eq!(denied["result"]["isError"], true);
    let unknown = mcp.tool("owner.export", json!({}));
    assert!(unknown.get("error").is_some() || unknown["result"]["isError"] == true);
    for invalid_args in [
        json!({"conversationId":group,"operationId":"invalid-limit","limit":0,"maxBytes":4096,"leaseSeconds":30}),
        json!({"conversationId":group,"operationId":"invalid-field","limit":10,"maxBytes":4096,"leaseSeconds":30,"principal":"owner"}),
    ] {
        let invalid = mcp.tool("inbox.poll", invalid_args);
        assert!(invalid.get("error").is_some() || invalid["result"]["isError"] == true);
    }
    let page = mcp.success("inbox.poll", poll_input(&group, "legacy-poll"));
    assert_eq!(page["items"][0]["id"], reply["id"]);
    mcp.success(
        "inbox.ack",
        json!({"conversationId":group,"leaseId":page["leaseId"]}),
    );
    assert_eq!(messages(&a.snapshot()).len(), 1);
    assert_eq!(messages(&b.snapshot()).len(), 1);
    mcp.close_cleanly();
}

#[test]
fn mcp_rejects_public_credentials_and_bounds_stdio_frames_without_owner_mutations() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[93; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
    let path = credentials(&a, &key, &grant);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let mut rejected = Mcp::spawn(&path, true);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = rejected.child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "public credential file did not fail startup"
        );
        thread::sleep(Duration::from_millis(20));
    }
    rejected.reader.take().unwrap().join().unwrap();
    assert!(
        rejected.responses.try_recv().is_err(),
        "startup errors must not pollute MCP stdout"
    );
    assert!(
        !fs::read_to_string(&rejected.log)
            .unwrap()
            .contains(&hex::encode(key.to_bytes()))
    );
    drop(rejected);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let mut mcp = Mcp::start(&path, true);
    let oversized = json!({"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientCapabilities":{}},"name":"messages.send","arguments":{"conversationId":group,"text":"x".repeat(1024*1024),"operationId":"oversize"}}});
    let _ = writeln!(mcp.input.as_mut().unwrap(), "{oversized}");
    let deadline = Instant::now() + Duration::from_secs(3);
    while mcp.child.try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "oversized stdio frame did not terminate transport"
        );
        thread::sleep(Duration::from_millis(20));
    }
    mcp.audit_remaining_stdout();
    assert!(messages(&a.snapshot()).is_empty());
    assert!(messages(&b.snapshot()).is_empty());
}

#[test]
fn mcp_cancellation_and_stdin_disconnect_close_the_pending_signed_ipc_request() {
    for (cancel_notification, resource) in
        [(true, false), (false, false), (true, true), (false, true)]
    {
        let root = tempfile::Builder::new()
            .prefix("ain-mcp-cancel-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let node = Node {
            root,
            child: None,
            listeners: vec![],
            peer: String::new(),
            arguments: vec![],
        };
        let listener = std::os::unix::net::UnixListener::bind(node.socket()).unwrap();
        listener.set_nonblocking(true).unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&[94; 32]);
        let grant = agentic_core::RuntimeGrant {
            grant_id: [9; 32],
            wire: vec![],
        };
        let path = credentials(&node, &key, &grant);
        let (arrived, observed) = mpsc::channel();
        let (closed, disconnected) = mpsc::channel();
        let reader = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "MCP did not open a signed IPC call"
                        );
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            // macOS accepts can inherit O_NONBLOCK from the listener.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let length = u32::from_be_bytes(length) as usize;
            assert!(length <= agentic_node::ipc::MAX_REQUEST);
            let mut bytes = vec![0; length];
            stream.read_exact(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                request.as_object().unwrap().len(),
                1,
                "agent IPC must contain only signed proof"
            );
            let wire = hex::decode(request["proof"].as_str().unwrap()).unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let verified = agentic_protocol::VerifiedDocument::decode(
                &wire,
                agentic_node::NETWORK_DOMAIN,
                now,
            )
            .unwrap();
            assert_eq!(verified.author(), &key.verifying_key().to_bytes());
            arrived.send(()).unwrap();
            let mut byte = [0];
            closed.send(stream.read(&mut byte)).unwrap();
        });
        let mut mcp = Mcp::start(&path, true);
        let id = if resource {
            mcp.send("resources/read", json!({"uri":"agentic://runtime"}))
        } else {
            mcp.send("tools/call",json!({"name":"messages.send","arguments":{"conversationId":"00".repeat(32),"text":"Cancelled before acceptance","operationId":"cancel-me"}}))
        };
        observed.recv_timeout(Duration::from_secs(3)).unwrap();
        if cancel_notification {
            mcp.write(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id,"reason":"client cancelled"}}));
        } else {
            mcp.input.take();
        }
        assert_eq!(
            disconnected
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            0,
            "pending IPC must close when the MCP call is abandoned"
        );
        reader.join().unwrap();
        if cancel_notification {
            let list_id = mcp.send("tools/list", json!({}));
            let listed = mcp.response_ignoring(list_id, Some(id));
            assert!(listed.get("error").is_none(), "{listed}");
            assert_eq!(
                listed["result"]["tools"].as_array().unwrap().len(),
                EXPECTED_TOOLS.len()
            );
            assert!(mcp.child.try_wait().unwrap().is_none());
        }
        mcp.close_cleanly();
    }
}

fn provisioning_input(group: &str, operation: &str) -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    json!({"operationId":operation,"name":"Desktop assistant","agentId":vec![75;32],"serviceId":vec![76;32],"conversationIds":[group],"actions":["read_inbox","send_message"],"expiresAt":now+3600,"maxDataBytes":4096})
}
fn configured_path(node: &Node, setup: &Value) -> PathBuf {
    let path = PathBuf::from(setup["credentialsPath"].as_str().unwrap());
    let grant_id: [u8; 32] = serde_json::from_value(setup["runtime"]["grantId"].clone()).unwrap();
    assert_eq!(
        path,
        fs::canonicalize(node.root.path())
            .unwrap()
            .join("runtimes")
            .join(format!("{}.json", hex::encode(grant_id)))
    );
    let configs = setup["mcpConfig"]["mcpServers"].as_object().unwrap();
    assert_eq!(configs.len(), 1);
    let config = configs.values().next().unwrap();
    assert_eq!(config["command"], env!("CARGO_BIN_EXE_agentic-mcp"));
    assert_eq!(config["args"], json!(["--credentials", path]));
    assert_eq!(config.as_object().unwrap().len(), 2);
    assert_eq!(
        setup["cliConfig"],
        json!({"command":env!("CARGO_BIN_EXE_agentic-cli"),"args":["--credentials",path]})
    );
    assert!(fs::symlink_metadata(&path).unwrap().is_file());
    assert_eq!(
        fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let credentials: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for secret in [KEY, TOKEN, credentials["signingSeed"].as_str().unwrap()] {
        assert!(!setup.to_string().contains(secret));
        assert!(!node.snapshot().to_string().contains(secret));
        for log in ["stderr.log", "stdout.log"] {
            assert!(
                !fs::read_to_string(node.root.path().join(log))
                    .unwrap()
                    .contains(secret)
            );
        }
    }
    path
}
#[test]
fn owner_provisioned_mcp_config_sends_to_peer_recovers_credentials_and_revokes_after_restart() {
    let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    assert_eq!(a.call("list_runtimes", json!({})), json!([]));
    let request = provisioning_input(&group, "desktop-setup");
    let supplied_key = ed25519_dalek::SigningKey::from_bytes(&[95; 32]);
    for (field, value) in [
        ("principal", json!(supplied_key.verifying_key().to_bytes())),
        ("signingSeed", json!(hex::encode(supplied_key.to_bytes()))),
        (
            "credentialsPath",
            json!(a.root.path().join("caller-runtime.json")),
        ),
        ("command", json!(env!("CARGO_BIN_EXE_agentic-mcp"))),
    ] {
        let mut invalid = request.clone();
        invalid[field] = value;
        assert!(
            rpc(&a.socket(), TOKEN, "provision_runtime", invalid)
                .unwrap()
                .get("error")
                .is_some()
        );
        assert_eq!(a.call("list_runtimes", json!({})), json!([]));
    }
    let setup = a.call("provision_runtime", request.clone());
    let path = configured_path(&a, &setup);
    assert_eq!(
        a.call("list_runtimes", json!({})),
        json!([setup["runtime"]])
    );
    let original = fs::read(&path).unwrap();
    let mut mcp = Mcp::start(&path, true);
    let args = json!({"conversationId":group,"text":"Configured from owner UI API","operationId":"desktop-config-send"});
    let sent = mcp.success("messages.send", args.clone());
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
    assert_eq!(messages(&received)[0]["text"], args["text"]);
    let peer = b.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let cli_send = [
        "messages",
        "send",
        "--to",
        peer.as_str(),
        "--operation-id",
        "desktop-config-send",
        "--text-stdin",
    ];
    let cli_retry = super::messaging_cli::cli(
        &path,
        &cli_send,
        args["text"].as_str().unwrap().as_bytes(),
        0,
    );
    assert_eq!(
        cli_retry["result"]["id"], sent["id"],
        "MCP and CLI share the same scoped operation identity"
    );
    mcp.close_cleanly();
    fs::remove_file(&path).unwrap();
    a.kill();
    a.launch();
    let restored = a.call("provision_runtime", request.clone());
    assert_eq!(restored, setup);
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut mcp = Mcp::start(&path, false);
    assert_eq!(mcp.success("messages.send", args)["id"], sent["id"]);
    assert_eq!(messages(&a.snapshot()).len(), 1);
    let credentials: Value = serde_json::from_slice(&original).unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(
        &hex::decode(credentials["signingSeed"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap(),
    );
    let grant = agentic_core::RuntimeGrant {
        grant_id: serde_json::from_value(setup["runtime"]["grantId"].clone()).unwrap(),
        wire: vec![],
    };
    for (i, method) in ["provision_runtime", "list_runtimes"]
        .into_iter()
        .enumerate()
    {
        let arguments = if method == "list_runtimes" {
            json!({})
        } else {
            request.clone()
        };
        let response = raw_ipc(
            &a.socket(),
            json!({"proof":hex::encode(proof(&key,&grant,method,arguments,i as u8+1))}),
        )
        .unwrap();
        assert_eq!(response["error"]["code"], "unauthorized");
    }
    a.call("revoke_runtime", json!({"grantId":grant.grant_id}));
    a.kill();
    a.launch();
    assert_eq!(a.call("list_runtimes", json!({}))[0]["status"], "revoked");
    assert_eq!(
        super::messaging_cli::cli(&path, &cli_send, b"Configured from owner UI API", 3)["error"]["code"],
        "unauthorized"
    );
    let denied = mcp.tool("inbox.poll", poll_input(&group, "revoked-provision"));
    assert_eq!(denied["result"]["isError"], true);
    assert!(
        rpc(&a.socket(), TOKEN, "provision_runtime", request)
            .unwrap()
            .get("error")
            .is_some()
    );
    assert_eq!(messages(&b.snapshot()).len(), 1);
    mcp.close_cleanly();
}
#[test]
fn failed_credentials_materialization_stays_visible_and_retries_same_grant_after_io_repair() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let request = provisioning_input(&group, "credentials-disk-retry");
    let directory = a.root.path().join("runtimes");
    fs::write(&directory, b"existing local file").unwrap();
    let failed = rpc(&a.socket(), TOKEN, "provision_runtime", request.clone()).unwrap();
    assert!(failed.get("error").is_some());
    assert_eq!(fs::read(&directory).unwrap(), b"existing local file");
    let registered = a.call("list_runtimes", json!({}));
    assert_eq!(registered.as_array().unwrap().len(), 1);
    assert_eq!(registered[0]["status"], "active");
    fs::remove_file(&directory).unwrap();
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        rpc(&a.socket(), TOKEN, "provision_runtime", request.clone())
            .unwrap()
            .get("error")
            .is_some()
    );
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(a.call("list_runtimes", json!({})), registered);
    fs::remove_dir(&directory).unwrap();
    let elsewhere = tempfile::TempDir::new_in("/tmp").unwrap();
    fs::set_permissions(elsewhere.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        fs::metadata(elsewhere.path()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    std::os::unix::fs::symlink(elsewhere.path(), &directory).unwrap();
    assert!(
        rpc(&a.socket(), TOKEN, "provision_runtime", request.clone())
            .unwrap()
            .get("error")
            .is_some()
    );
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
    fs::remove_file(&directory).unwrap();
    let setup = a.call("provision_runtime", request);
    assert_eq!(setup["runtime"], registered[0]);
    let path = configured_path(&a, &setup);
    let mut mcp = Mcp::start(&path, true);
    let sent=mcp.success("messages.send",json!({"conversationId":group,"text":"Recovered configuration","operationId":"recovered-config-send"}));
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["id"], sent["id"]);
    assert_eq!(messages(&received)[0]["text"], "Recovered configuration");
    mcp.close_cleanly();
}
