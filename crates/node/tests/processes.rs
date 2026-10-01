#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;
const TOKEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
#[path = "support/bootstrap.rs"]
mod bootstrap;
#[path = "support/bootstrap_settings.rs"]
mod bootstrap_settings;
#[path = "support/dht_roles.rs"]
mod dht_roles;
#[path = "support/mcp_stdio.rs"]
mod mcp_stdio;
#[path = "support/network_settings.rs"]
mod network_settings;
#[path = "support/owner_cli.rs"]
mod owner_cli;
#[path = "support/peer_records.rs"]
mod peer_records;
#[path = "support/relay.rs"]
mod relay;
#[path = "support/relay_resources.rs"]
mod relay_resources;
#[path = "support/routing.rs"]
mod routing;
#[path = "support/shutdown.rs"]
mod shutdown;
#[path = "support/swarm_native.rs"]
mod swarm_native;
struct Node {
    root: TempDir,
    child: Option<Child>,
    listeners: Vec<String>,
    peer: String,
    arguments: Vec<String>,
}
impl Drop for Node {
    fn drop(&mut self) {
        self.kill();
    }
}
impl Node {
    fn start(transports: &[&str]) -> Self {
        Self::start_with_args(transports, vec![])
    }
    fn start_with_args(transports: &[&str], arguments: Vec<String>) -> Self {
        let root = tempfile::Builder::new()
            .prefix("ain-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut node = Self {
            root,
            child: None,
            listeners: transports.iter().map(|s| s.to_string()).collect(),
            peer: String::new(),
            arguments,
        };
        node.launch();
        node
    }
    fn socket(&self) -> PathBuf {
        self.root.path().join("node.sock")
    }
    fn launch(&mut self) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kaiki-agentic-node"));
        cmd.arg("serve")
            .arg("--profile")
            .arg(self.root.path().join("profile.db"))
            .arg("--ipc")
            .arg(self.socket())
            .arg("--secrets-stdin");
        for addr in &self.listeners {
            cmd.arg("--listen").arg(addr);
        }
        cmd.args(&self.arguments);
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.path().join("stderr.log"))
            .unwrap();
        let stdout = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.path().join("stdout.log"))
            .unwrap();
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(log)
            .spawn()
            .unwrap();
        writeln!(
            child.stdin.take().unwrap(),
            "{}",
            json!({"masterKey":KEY,"ownerToken":TOKEN})
        )
        .unwrap();
        self.child = Some(child);
        // A wildcard address binds a listener on every interface, as the
        // owner's daemon does: ready once each transport listens, and kept
        // as given for the next start.
        let wildcard = self
            .listeners
            .iter()
            .any(|l| l.starts_with("/ip4/0.0.0.0/"));
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                panic!(
                    "node exited {status}: {}",
                    fs::read_to_string(self.root.path().join("stderr.log")).unwrap()
                );
            }
            if let Ok(response) = rpc(&self.socket(), TOKEN, "node_info", json!({}))
                && let Some(info) = response.get("result")
            {
                let addrs = info["listeners"].as_array().unwrap();
                let bound = if wildcard {
                    self.listeners.iter().all(|spec| {
                        let quic = spec.contains("/quic-v1");
                        addrs
                            .iter()
                            .any(|a| a.as_str().unwrap().contains("/quic-v1") == quic)
                    })
                } else {
                    addrs.len() == self.listeners.len()
                };
                if bound {
                    let peer = info["peerId"].as_str().unwrap().to_owned();
                    if !self.peer.is_empty() {
                        assert_eq!(peer, self.peer, "transport identity changed after restart");
                    }
                    for addr in addrs {
                        assert!(
                            addr.as_str().unwrap().ends_with(&format!("/p2p/{peer}")),
                            "listener lacks own peer ID"
                        );
                    }
                    self.peer = peer;
                    if !wildcard {
                        self.listeners = addrs
                            .iter()
                            .map(|a| {
                                a.as_str()
                                    .unwrap()
                                    .split("/p2p/")
                                    .next()
                                    .unwrap()
                                    .to_string()
                            })
                            .collect();
                    }
                    break;
                }
            }
            assert!(Instant::now() < deadline, "node not ready");
            thread::sleep(Duration::from_millis(30));
        }
    }
    fn kill(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
    fn call(&self, method: &str, request: Value) -> Value {
        let v = rpc(&self.socket(), TOKEN, method, request).unwrap();
        assert!(v.get("error").is_none(), "{method}: {v}");
        v["result"].clone()
    }
    fn snapshot(&self) -> Value {
        self.call("snapshot", json!({}))
    }
    fn wait(&self, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let v = self.snapshot();
            if predicate(&v) {
                return v;
            }
            assert!(
                Instant::now() < deadline,
                "state did not converge: {v}; node info: {}",
                self.call("node_info", json!({}))
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn profile(&self, name: &str) {
        let v = self.call("create_identity", json!({"name":name}));
        assert_eq!(v["name"], name);
    }
    fn send(&self, group: &str, text: &str, op: &str) -> Value {
        self.call(
            "send_message",
            json!({"conversationId":group,"text":text,"operationId":op}),
        )
    }
}
fn rpc(socket: &PathBuf, token: &str, method: &str, request: Value) -> std::io::Result<Value> {
    raw_ipc(
        socket,
        json!({"token":token,"method":method,"request":request}),
    )
}
fn raw_ipc(socket: &PathBuf, payload: Value) -> std::io::Result<Value> {
    let mut s = UnixStream::connect(socket)?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    let wire = serde_json::to_vec(&payload)?;
    s.write_all(&(wire.len() as u32).to_be_bytes())?;
    s.write_all(&wire)?;
    let mut size = [0; 4];
    s.read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    assert!(n <= 16 * 1024 * 1024, "unbounded IPC response");
    let mut bytes = vec![0; n];
    s.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn connect(a: &Node, b: &Node, addresses: Option<Vec<String>>) -> String {
    a.profile("Alice");
    b.profile("Bob");
    let invite = b.call("create_invitation", json!({"addresses":addresses}));
    let c = a.call("add_contact", json!({"name":"Bob","invitation":invite}));
    let id = c["id"].as_str().unwrap().to_owned();
    b.wait(|v| {
        v["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id)
    });
    id
}
fn messages(v: &Value) -> &Vec<Value> {
    v["conversations"][0]["messages"].as_array().unwrap()
}
fn grant(
    n: &Node,
    key: &ed25519_dalek::SigningKey,
    group: &str,
    actions: Value,
) -> agentic_core::RuntimeGrant {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    serde_json::from_value(n.call("grant_runtime", json!({
        "name":"Local assistant", "principal":key.verifying_key().to_bytes(), "agentId":([41;32]), "serviceId":([42;32]),
        "conversationIds":[group], "actions":actions, "expiresAt":now+3600, "maxDataBytes":4096
    }))).unwrap()
}
fn proof(
    key: &ed25519_dalek::SigningKey,
    grant: &agentic_core::RuntimeGrant,
    method: &str,
    request: Value,
    nonce: u8,
) -> Vec<u8> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let call = agentic_core::AgentCall {
        grant_id: grant.grant_id,
        method: method.into(),
        request,
        nonce: [nonce; 32],
    };
    agentic_protocol::SignedDocument::sign(
        call.draft(agentic_node::NETWORK_DOMAIN, 0, now).unwrap(),
        key,
    )
    .unwrap()
    .to_wire()
}
#[tokio::test]
async fn signed_agent_ipc_drives_real_offline_delivery_retry_reply_and_durable_revocation() {
    let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let mut b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[61; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox", "send_message"]));
    b.kill();
    let request = json!({"conversationId":group,"text":"From an authenticated runtime","operationId":"agent-op"});
    let sent = agentic_node::ipc::call_agent(
        &a.socket(),
        &proof(&key, &grant, "send_message", request.clone(), 1),
    )
    .await
    .unwrap();
    assert!(sent.get("error").is_none(), "{sent}");
    assert_eq!(sent["result"]["delivery"]["phase"], "queued");
    a.kill();
    a.launch();
    let retry_wire = proof(&key, &grant, "send_message", request.clone(), 2);
    let retry = agentic_node::ipc::call_agent(&a.socket(), &retry_wire)
        .await
        .unwrap();
    assert_eq!(retry["result"]["id"], sent["result"]["id"]);
    let replay = agentic_node::ipc::call_agent(&a.socket(), &retry_wire)
        .await
        .unwrap();
    assert_eq!(replay["error"]["code"], "unauthorized");
    assert_eq!(messages(&a.snapshot()).len(), 1);
    b.launch();
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["id"], sent["result"]["id"]);
    assert_eq!(
        messages(&received)[0]["text"],
        "From an authenticated runtime"
    );
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    b.send(&group, "Reply to runtime", "reply");
    a.wait(|v| messages(v).len() == 2);
    let inbox =
        agentic_node::ipc::call_agent(&a.socket(), &proof(&key, &grant, "snapshot", json!({}), 3))
            .await
            .unwrap();
    assert!(inbox["result"].get("identity").is_none());
    assert_eq!(inbox["result"]["agentId"], hex::encode([41; 32]));
    assert!(
        inbox["result"]["conversations"][0]
            .get("messages")
            .is_none()
    );
    let page = agentic_node::ipc::call_agent(&a.socket(), &proof(&key, &grant, "inbox_poll", json!({"conversationId":group,"operationId":"reply-page","limit":10,"maxBytes":4096,"leaseSeconds":30}),30)).await.unwrap();
    assert_eq!(page["result"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["result"]["items"][0]["text"], "Reply to runtime");
    a.call("revoke_runtime", json!({"grantId":grant.grant_id}));
    a.kill();
    a.launch();
    let denied = agentic_node::ipc::call_agent(
        &a.socket(),
        &proof(&key, &grant, "send_message", request, 4),
    )
    .await
    .unwrap();
    assert_eq!(denied["error"]["code"], "unauthorized");
    assert_eq!(messages(&a.snapshot()).len(), 2);
    a.send(&group, "Owner remains authorized", "owner-after-revoke");
    let after = b.wait(|v| messages(v).len() == 3);
    assert_eq!(messages(&after)[2]["text"], "Owner remains authorized");
    for name in ["stderr.log", "stdout.log"] {
        let log = fs::read_to_string(a.root.path().join(name)).unwrap();
        assert!(!log.contains(TOKEN));
        assert!(!log.contains(KEY));
        assert!(!log.contains(&hex::encode(key.to_bytes())));
    }
}
#[test]
fn agent_ipc_cannot_select_owner_branch_forge_runtime_or_expand_registered_scope() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    let key = ed25519_dalek::SigningKey::from_bytes(&[62; 32]);
    let grant = grant(&a, &key, &group, json!(["read_inbox"]));
    let read_wire = proof(&key, &grant, "snapshot", json!({}), 1);
    a.wait(|_| a.call("node_info", json!({}))["pendingOutbox"] == 0);
    for extra in [
        json!({"token":TOKEN}),
        json!({"method":"create_identity"}),
        json!({"request":{"name":"Intruder"}}),
        json!({"principal":"owner"}),
        json!({"token":TOKEN,"method":"send_message","request":{"conversationId":group,"text":"Mixed authority escalation","operationId":"mixed-owner-command"}}),
    ] {
        let mut mixed = json!({"proof":hex::encode(&read_wire)});
        mixed
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let result = raw_ipc(&a.socket(), mixed);
        assert!(
            result.is_err() || result.unwrap().get("error").is_some(),
            "mixed authentication must fail closed"
        );
        assert!(messages(&a.snapshot()).is_empty());
        assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    }
    let other = ed25519_dalek::SigningKey::from_bytes(&[63; 32]);
    let forged = raw_ipc(
        &a.socket(),
        json!({"proof":hex::encode(proof(&other,&grant,"snapshot",json!({}),2))}),
    )
    .unwrap();
    assert_eq!(forged["error"]["code"], "unauthorized");
    for (i, method) in [
        "grant_runtime",
        "revoke_runtime",
        "create_identity",
        "node_info",
        "send_message",
    ]
    .iter()
    .enumerate()
    {
        let request = if *method == "send_message" {
            json!({"conversationId":group,"text":"Not permitted","operationId":"forbidden"})
        } else {
            json!({})
        };
        let result = raw_ipc(
            &a.socket(),
            json!({"proof":hex::encode(proof(&key,&grant,method,request,10+i as u8))}),
        )
        .unwrap();
        assert_eq!(
            result["error"]["code"], "unauthorized",
            "{method}: {result}"
        );
    }
    for malformed in ["zz".into(), "00".repeat(65537)] {
        let rejected = raw_ipc(&a.socket(), json!({"proof":malformed}));
        assert!(rejected.is_err() || rejected.unwrap().get("error").is_some());
    }
    let valid = raw_ipc(&a.socket(), json!({"proof":hex::encode(&read_wire)})).unwrap();
    assert_eq!(valid["result"]["conversations"][0]["id"], group);
    let replay = raw_ipc(&a.socket(), json!({"proof":hex::encode(read_wire)})).unwrap();
    assert_eq!(replay["error"]["code"], "unauthorized");
    assert_eq!(a.snapshot()["identity"]["name"], "Alice");
    assert!(messages(&a.snapshot()).is_empty());
    assert!(messages(&b.snapshot()).is_empty());
}
fn exchange(transport: &str) {
    let addr = if transport == "tcp" {
        "/ip4/127.0.0.1/tcp/0"
    } else {
        "/ip4/127.0.0.1/udp/0/quic-v1"
    };
    let a = Node::start(&[addr]);
    let b = Node::start(&[addr]);
    let group = connect(&a, &b, None);
    let first = a.send(&group, "Hi from a separate process", "a-1");
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&received)[0]["text"], "Hi from a separate process");
    assert_eq!(messages(&received)[0]["id"], first["id"]);
    assert_eq!(messages(&received)[0]["own"], false);
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    b.send(&group, "A reply over the saved route", "b-1");
    let received = a.wait(|v| messages(v).len() == 2);
    assert_eq!(
        messages(&received)[1]["text"],
        "A reply over the saved route"
    );
    assert_eq!(messages(&received)[0]["delivery"]["replicas"], 0);
    assert!(received["network"]["connectedPeers"].as_u64().unwrap() > 0);
    assert_eq!(
        received["network"]["state"], "online",
        "shared desktop DTO contract"
    );
    let info = a.call("node_info", json!({}));
    assert!(
        info["transportsUsed"]
            .as_array()
            .unwrap()
            .contains(&json!(transport)),
        "actual route missing: {info}"
    );
    assert!(!info.to_string().contains(KEY));
    assert!(!info.to_string().contains(TOKEN));
}
#[test]
fn separate_processes_exchange_mls_over_tcp_noise() {
    exchange("tcp");
}
#[test]
fn separate_processes_exchange_mls_over_quic() {
    exchange("quic");
}
#[test]
fn offline_queue_survives_sigkill_of_both_nodes_and_operation_retry_is_exactly_once() {
    let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let mut b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    a.kill();
    let queued = b.send(&group, "Saved before the crash", "durable-op");
    assert_eq!(queued["delivery"]["phase"], "queued");
    b.kill();
    b.launch();
    let pending = b.snapshot();
    assert_eq!(messages(&pending).len(), 1);
    assert_eq!(messages(&pending)[0]["delivery"]["phase"], "queued");
    let retry = b.send(&group, "Saved before the crash", "durable-op");
    assert_eq!(retry["id"], queued["id"]);
    a.launch();
    let arrived = a.wait(|v| messages(v).len() == 1);
    assert_eq!(messages(&arrived)[0]["text"], "Saved before the crash");
    assert_eq!(messages(&arrived)[0]["id"], queued["id"]);
    b.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    b.send(&group, "A second packet after recovery", "after-restart");
    let all = a.wait(|v| messages(v).len() >= 2);
    assert_eq!(messages(&all).len(), 2);
    assert_eq!(messages(&all)[1]["text"], "A second packet after recovery");
    assert_eq!(
        messages(&all)
            .iter()
            .filter(|m| m["id"] == queued["id"])
            .count(),
        1
    );
}
#[test]
fn unreachable_quic_hint_falls_back_to_tcp_without_duplicate_welcome_or_message() {
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0", "/ip4/127.0.0.1/udp/0/quic-v1"]);
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let dead = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let bad = format!(
        "/ip4/127.0.0.1/udp/{}/quic-v1/p2p/{}",
        dead.local_addr().unwrap().port(),
        b.peer
    );
    let good = format!("{}/p2p/{}", b.listeners[0], b.peer);
    let group = connect(&a, &b, Some(vec![bad, good]));
    a.send(&group, "Fallback works", "fallback");
    let received = b.wait(|v| messages(v).len() == 1);
    assert_eq!(received["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(messages(&received)[0]["text"], "Fallback works");
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    assert_eq!(
        a.call("node_info", json!({}))["transportsUsed"],
        json!(["tcp"])
    );
}
#[test]
fn local_ipc_requires_owner_token_and_bounds_frames_without_stopping_daemon() {
    let n = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    assert_eq!(
        fs::metadata(n.socket()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let rejected = rpc(
        &n.socket(),
        "3333333333333333333333333333333333333333333333333333333333333333",
        "create_identity",
        json!({"name":"Intruder"}),
    )
    .unwrap();
    assert_eq!(rejected["error"]["code"], "unauthorized");
    assert!(n.snapshot()["identity"].is_null());
    let unknown = rpc(
        &n.socket(),
        TOKEN,
        "sign_arbitrary",
        json!({"bytes":[1,2,3]}),
    )
    .unwrap();
    assert_eq!(unknown["error"]["code"], "unknown_method");
    let mut oversized = UnixStream::connect(n.socket()).unwrap();
    oversized
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    oversized.write_all(&u32::MAX.to_be_bytes()).unwrap();
    let mut byte = [0];
    let read = oversized.read(&mut byte);
    assert!(
        matches!(read, Ok(0))
            || matches!(read,Err(ref e) if e.kind()==std::io::ErrorKind::ConnectionReset),
        "oversized request not closed: {read:?}"
    );
    let mut slow = UnixStream::connect(n.socket()).unwrap();
    slow.write_all(&[0, 0]).unwrap();
    n.profile("Actual owner");
    drop(slow);
    assert_eq!(n.snapshot()["identity"]["name"], "Actual owner");
    let logs = fs::read_to_string(n.root.path().join("stderr.log")).unwrap();
    assert!(!logs.contains(KEY));
    assert!(!logs.contains(TOKEN));
    let stdout = fs::read_to_string(n.root.path().join("stdout.log")).unwrap();
    assert!(!stdout.contains(KEY));
    assert!(!stdout.contains(TOKEN));
}

// This peer uses libp2p directly and independent RPC DTOs, not the production node client.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RawDelivery {
    #[serde(with = "serde_bytes")]
    node_record: Vec<u8>,
    #[serde(with = "serde_bytes")]
    envelope: Vec<u8>,
}
type RawBehaviour = libp2p::request_response::cbor::Behaviour<RawDelivery, RawDelivery>;
async fn raw_request(
    swarm: &mut libp2p::Swarm<RawBehaviour>,
    peer: libp2p::PeerId,
    address: libp2p::Multiaddr,
    request: RawDelivery,
) -> Result<RawDelivery, String> {
    use futures::StreamExt;
    use libp2p::{
        request_response::{Event, Message},
        swarm::SwarmEvent,
    };
    let id = swarm
        .behaviour_mut()
        .send_request_with_addresses(&peer, request, vec![address]);
    tokio::time::timeout(Duration::from_secs(12), async {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(Event::Message {
                    message:
                        Message::Response {
                            request_id,
                            response,
                        },
                    ..
                }) if request_id == id => return Ok(response),
                SwarmEvent::Behaviour(Event::OutboundFailure {
                    request_id, error, ..
                }) if request_id == id => return Err(error.to_string()),
                _ => {}
            }
        }
    })
    .await
    .expect("raw peer request timed out")
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_peer_binding_attacks_and_oversize_frames_cannot_consume_welcome() {
    use agentic_core::AppCore;
    use agentic_store::ProfileStore;
    use libp2p::{
        StreamProtocol, SwarmBuilder, identity, noise,
        request_response::{Config, ProtocolSupport, cbor},
        yamux,
    };
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    b.profile("Bob");
    let info = b.call("node_info", json!({}));
    let peer = info["peerId"].as_str().unwrap().parse().unwrap();
    let address: libp2p::Multiaddr = info["listeners"][0].as_str().unwrap().parse().unwrap();
    let invitation = b.call("create_invitation", json!({}));
    let root = TempDir::new().unwrap();
    let domain: [u8; 32] =
        hex::decode("ae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18")
            .unwrap()
            .try_into()
            .unwrap();
    let mut alice = AppCore::new(
        ProfileStore::open(root.path().join("alice.db"), &[0x33; 32]).unwrap(),
        domain,
    )
    .unwrap();
    alice.create_profile("Alice").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let group = alice
        .add_contact("Bob", invitation.as_str().unwrap(), now)
        .unwrap()
        .id;
    let welcome = alice.outbox(10).unwrap()[0].wire.clone();
    let key = identity::Keypair::generate_ed25519();
    let own = key.public().to_peer_id().to_string();
    let mut swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| {
            cbor::Behaviour::<RawDelivery, RawDelivery>::new(
                [(
                    StreamProtocol::new("/agentic-internet/delivery/1"),
                    ProtocolSupport::Full,
                )],
                Config::default().with_request_timeout(Duration::from_secs(6)),
            )
        })
        .unwrap()
        .build();
    let binding = alice.create_node_record(&own, vec![], now).unwrap();
    let mut damaged = binding.clone();
    *damaged.last_mut().unwrap() ^= 1;
    let wrong_id = identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let wrong_binding = alice.create_node_record(&wrong_id, vec![], now).unwrap();
    for record in [damaged, wrong_binding] {
        let response = raw_request(
            &mut swarm,
            peer,
            address.clone(),
            RawDelivery {
                node_record: record,
                envelope: welcome.clone(),
            },
        )
        .await
        .unwrap();
        assert!(
            response.envelope.is_empty(),
            "invalid binding got an acknowledgment"
        );
        assert!(b.snapshot()["conversations"].as_array().unwrap().is_empty());
    }
    let oversized = raw_request(
        &mut swarm,
        peer,
        address.clone(),
        RawDelivery {
            node_record: binding.clone(),
            envelope: vec![0; 140_000],
        },
    )
    .await;
    assert!(
        oversized.is_err(),
        "oversized RPC reached application handler instead of failing at transport frame limit"
    );
    assert!(b.snapshot()["conversations"].as_array().unwrap().is_empty());
    let accepted = raw_request(
        &mut swarm,
        peer,
        address.clone(),
        RawDelivery {
            node_record: binding.clone(),
            envelope: welcome.clone(),
        },
    )
    .await
    .unwrap();
    assert!(!accepted.envelope.is_empty());
    alice
        .receive_from(
            &accepted.envelope,
            &accepted.node_record,
            &peer.to_string(),
            now,
        )
        .unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    let duplicate = raw_request(
        &mut swarm,
        peer,
        address.clone(),
        RawDelivery {
            node_record: binding.clone(),
            envelope: welcome,
        },
    )
    .await
    .unwrap();
    assert!(!duplicate.envelope.is_empty());
    alice
        .receive_from(
            &duplicate.envelope,
            &duplicate.node_record,
            &peer.to_string(),
            now,
        )
        .unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    assert_eq!(b.snapshot()["conversations"].as_array().unwrap().len(), 1);
    alice
        .send_message(&group, "Healthy after rejected wire", "healthy", now)
        .unwrap();
    let response = raw_request(
        &mut swarm,
        peer,
        address,
        RawDelivery {
            node_record: binding,
            envelope: alice.outbox(10).unwrap()[0].wire.clone(),
        },
    )
    .await
    .unwrap();
    alice
        .receive_from(
            &response.envelope,
            &response.node_record,
            &peer.to_string(),
            now,
        )
        .unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    let received = b.snapshot();
    assert_eq!(messages(&received).len(), 1);
    assert_eq!(
        messages(&received)[0]["text"],
        "Healthy after rejected wire"
    );
}

#[test]
fn invitation_routes_are_validated_before_contact_and_outbox_mutation() {
    use agentic_core::AppCore;
    use agentic_store::ProfileStore;
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    a.profile("Alice");
    let root = TempDir::new().unwrap();
    let domain = hex::decode("ae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18")
        .unwrap()
        .try_into()
        .unwrap();
    let mut bob = AppCore::new(
        ProfileStore::open(root.path().join("bob.db"), &[0x33; 32]).unwrap(),
        domain,
    )
    .unwrap();
    bob.create_profile("Bob").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let p1 = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let p2 = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let before = a.snapshot();
    for routes in [
        vec!["not a multiaddr".into()],
        vec![
            format!("/ip4/127.0.0.1/tcp/4100/p2p/{p1}"),
            format!("/ip4/127.0.0.1/tcp/4200/p2p/{p2}"),
        ],
        vec!["/ip4/127.0.0.1/tcp/4200".into()],
    ] {
        let invitation = bob.create_invitation(now, routes).unwrap();
        let rejected = rpc(
            &a.socket(),
            TOKEN,
            "add_contact",
            json!({"name":"Bob","invitation":invitation}),
        )
        .unwrap();
        assert!(
            rejected.get("error").is_some(),
            "invalid routes were imported: {rejected}"
        );
        assert_eq!(a.snapshot(), before);
        assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    }
    let b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    b.profile("Bob");
    let invitation = b.call("create_invitation", json!({}));
    a.call("add_contact", json!({"name":"Bob","invitation":invitation}));
    b.wait(|v| v["conversations"].as_array().unwrap().len() == 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sender_retries_identical_wire_after_invalid_ack_and_lost_response() {
    use agentic_core::AppCore;
    use agentic_store::ProfileStore;
    use futures::StreamExt;
    use libp2p::{
        StreamProtocol, SwarmBuilder, identity, noise,
        request_response::{Config, Event, Message, ProtocolSupport, cbor},
        swarm::SwarmEvent,
        yamux,
    };
    let a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    a.profile("Alice");
    let key = identity::Keypair::generate_ed25519();
    let own = key.public().to_peer_id();
    let mut swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| {
            cbor::Behaviour::<RawDelivery, RawDelivery>::new(
                [(
                    StreamProtocol::new("/agentic-internet/delivery/1"),
                    ProtocolSupport::Full,
                )],
                Config::default().with_request_timeout(Duration::from_secs(6)),
            )
        })
        .unwrap()
        .build();
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
                break format!("{address}/p2p/{own}");
            }
        }
    })
    .await
    .unwrap();
    let root = TempDir::new().unwrap();
    let domain = hex::decode("ae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18")
        .unwrap()
        .try_into()
        .unwrap();
    let mut bob = AppCore::new(
        ProfileStore::open(root.path().join("bob.db"), &[0x33; 32]).unwrap(),
        domain,
    )
    .unwrap();
    bob.create_profile("Bob").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let binding = bob
        .create_node_record(&own.to_string(), vec![address.clone()], now)
        .unwrap();
    let invitation = bob.create_invitation(now, vec![address]).unwrap();
    let c = a.call("add_contact", json!({"name":"Bob","invitation":invitation}));
    let group = c["id"].as_str().unwrap();
    async fn incoming(
        swarm: &mut libp2p::Swarm<RawBehaviour>,
    ) -> (
        libp2p::PeerId,
        libp2p::request_response::InboundRequestId,
        RawDelivery,
        libp2p::request_response::ResponseChannel<RawDelivery>,
    ) {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let SwarmEvent::Behaviour(Event::Message {
                    peer,
                    message:
                        Message::Request {
                            request_id,
                            request,
                            channel,
                        },
                    ..
                }) = swarm.select_next_some().await
                {
                    break (peer, request_id, request, channel);
                }
            }
        })
        .await
        .expect("sender failed to retry queued work")
    }
    async fn sent(
        swarm: &mut libp2p::Swarm<RawBehaviour>,
        id: libp2p::request_response::InboundRequestId,
    ) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let SwarmEvent::Behaviour(Event::ResponseSent { request_id, .. }) =
                    swarm.select_next_some().await
                    && request_id == id
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
    }
    let (peer, id, welcome, channel) = incoming(&mut swarm).await;
    let reply = bob
        .receive_from(
            &welcome.envelope,
            &welcome.node_record,
            &peer.to_string(),
            now,
        )
        .unwrap()
        .reply
        .unwrap();
    swarm
        .behaviour_mut()
        .send_response(
            channel,
            RawDelivery {
                node_record: binding.clone(),
                envelope: reply,
            },
        )
        .unwrap();
    sent(&mut swarm, id).await;
    let original = a.send(group, "Receipt must be authentic", "ack-retry");
    let mut first_wire = None;
    for attempt in 0..3 {
        let (peer, id, request, channel) = incoming(&mut swarm).await;
        if let Some(wire) = &first_wire {
            assert_eq!(
                &request.envelope, wire,
                "runtime re-encrypted a retry instead of replaying durable wire"
            );
        } else {
            first_wire = Some(request.envelope.clone());
        }
        let mut reply = bob
            .receive_from(
                &request.envelope,
                &request.node_record,
                &peer.to_string(),
                now,
            )
            .unwrap()
            .reply
            .unwrap();
        assert_eq!(bob.snapshot().unwrap().conversations[0].messages.len(), 1);
        if attempt == 1 {
            drop(channel);
            continue;
        }
        if attempt == 0 {
            *reply.last_mut().unwrap() ^= 1;
        }
        swarm
            .behaviour_mut()
            .send_response(
                channel,
                RawDelivery {
                    node_record: binding.clone(),
                    envelope: reply,
                },
            )
            .unwrap();
        sent(&mut swarm, id).await;
        if attempt == 0 {
            assert_eq!(messages(&a.snapshot())[0]["delivery"]["phase"], "queued");
        }
    }
    a.wait(|v| messages(v)[0]["delivery"]["phase"] == "delivered");
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    assert_eq!(
        a.send(group, "Receipt must be authentic", "ack-retry")["id"],
        original["id"]
    );
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Receipt must be authentic"
    );
}

#[path = "support/messaging_cli.rs"]
mod messaging_cli;
