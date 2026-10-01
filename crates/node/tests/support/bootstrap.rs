use super::*;
use futures::StreamExt;
use libp2p::{
    StreamProtocol, SwarmBuilder, identity, noise,
    request_response::{Config, Event, Message, ProtocolSupport, cbor},
    swarm::SwarmEvent,
    yamux,
};
use std::sync::{Arc, Mutex};

const TCP: &str = "/ip4/127.0.0.1/tcp/0";
fn route(node: &Node) -> String {
    format!("{}/p2p/{}", node.listeners[0], node.peer)
}
fn verified(info: &Value, peer: &str) -> bool {
    info["bootstrap"]["verifiedPeers"]
        .as_array()
        .is_some_and(|peers| peers.iter().any(|p| p["peerId"] == peer))
}
fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn bootstrap_falls_back_from_dead_hint_without_creating_a_trusted_contact() {
    let mut dead = Node::start(&[TCP]);
    let missing = route(&dead);
    dead.kill();
    let live = Node::start(&[TCP]);
    live.profile("Independent peer");
    let a = Node::start_with_args(
        &[TCP],
        vec![
            "--bootstrap".into(),
            missing,
            "--bootstrap".into(),
            route(&live),
        ],
    );
    a.profile("Alice");
    let info = relay::info_until(&a, |v| verified(v, &live.peer));
    assert_eq!(info["bootstrap"]["state"], "connected");
    assert_eq!(
        info["bootstrap"]["networkDomain"],
        hex::encode(agentic_node::NETWORK_DOMAIN)
    );
    let peers = info["bootstrap"]["verifiedPeers"].as_array().unwrap();
    let peer = peers.iter().find(|p| p["peerId"] == live.peer).unwrap();
    assert_eq!(peer["rootId"], live.snapshot()["identity"]["networkId"]);
    assert!(!verified(&info, &dead.peer));
    assert!(a.snapshot()["conversations"].as_array().unwrap().is_empty());
    assert!(
        live.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // A transport hint grants no chat authority; only this separately signed invitation does.
    let invite = live.call("create_invitation", json!({}));
    let contact = a.call(
        "add_contact",
        json!({"name":"Independent peer","invitation":invite}),
    );
    let group = contact["id"].as_str().unwrap();
    live.wait(|s| {
        s["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == group)
    });
    relay::assert_exchange(
        &a,
        &live,
        group,
        "Reached through an independent IP hint",
        "bootstrap-message",
        1,
    );
}

#[test]
fn bootstrap_reports_missing_hints_and_recovers_when_an_unreachable_peer_returns() {
    let fresh = Node::start(&[TCP]);
    let info = fresh.call("node_info", json!({}));
    assert_eq!(info["bootstrap"]["state"], "bootstrap-needed");
    assert_eq!(info["bootstrap"]["verifiedPeers"], json!([]));
    assert!(!info["bootstrap"]["action"].as_str().unwrap().is_empty());
    let mut provider = Node::start(&[TCP]);
    let hint = route(&provider);
    provider.kill();
    let a = Node::start_with_args(&[TCP], vec!["--bootstrap".into(), hint]);
    let failed = relay::info_until(&a, |v| {
        v["bootstrap"]["failedAttempts"]
            .as_u64()
            .is_some_and(|n| n > 0)
            && v["bootstrap"]["state"] == "bootstrap-needed"
    });
    assert_eq!(failed["bootstrap"]["verifiedPeers"], json!([]));
    let own = failed["peerId"].clone();
    provider.launch();
    relay::info_until(&a, |v| verified(v, &provider.peer));
    assert_eq!(a.call("node_info", json!({}))["peerId"], own);
    provider.kill();
    let down = relay::info_until(&a, |v| v["bootstrap"]["state"] == "bootstrap-needed");
    assert_eq!(
        down["bootstrap"]["verifiedPeers"],
        json!([]),
        "a cached record is not a live network connection"
    );
    provider.launch();
    relay::info_until(&a, |v| verified(v, &provider.peer));
}

// Independent wire DTO and live libp2p peer. It never dials a node or imports a contact cache.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Exchange {
    #[serde(with = "serde_bytes")]
    node_record: Vec<u8>,
}
pub(super) struct RecordPeer {
    pub(super) address: String,
    pub(super) peer: String,
    pub(super) requests: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for RecordPeer {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
impl RecordPeer {
    pub(super) fn start(mode: u8) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let (ready, address) = std::sync::mpsc::channel();
        let requests2 = requests.clone();
        let thread = thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async move {
                let root = TempDir::new().unwrap();
                let core = agentic_core::AppCore::new(agentic_store::ProfileStore::open(root.path().join("valid.db"), &[84;32]).unwrap(), agentic_node::NETWORK_DOMAIN).unwrap();
                let foreign = agentic_core::AppCore::new(agentic_store::ProfileStore::open(root.path().join("foreign.db"), &[85;32]).unwrap(), [71;32]).unwrap();
                let key = identity::Keypair::generate_ed25519();
                let peer = key.public().to_peer_id();
                let wrong = identity::Keypair::generate_ed25519().public().to_peer_id().to_string();
                let mut swarm = peer_swarm(key);
                swarm.listen_on(TCP.parse().unwrap()).unwrap();
                let address = loop {
                    if let SwarmEvent::NewListenAddr {address, ..} = swarm.select_next_some().await {
                        break format!("{address}/p2p/{peer}");
                    }
                };
                ready.send((address.clone(), peer.to_string())).unwrap();
                tokio::pin!(stopped);
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        event = swarm.select_next_some() => {
                            if let SwarmEvent::Behaviour(Event::Message { message: Message::Request {request, channel, ..}, ..}) = event {
                                requests2.lock().unwrap().push(request.node_record);
                                let now = timestamp();
                                let node_record = match mode {
                                    0 => foreign.create_node_record(&peer.to_string(), vec![address.clone()], now).unwrap(),
                                    1 => { let mut wire = core.create_node_record(&peer.to_string(), vec![address.clone()], now).unwrap(); *wire.last_mut().unwrap() ^= 1; wire },
                                    2 => core.create_node_record(&wrong, vec![], now).unwrap(),
                                    3 => core.create_node_record(&peer.to_string(), vec![address.clone()], now-86401).unwrap(),
                                    4 => core.create_node_record(&peer.to_string(), vec![format!("/ip4/127.0.0.1/tcp/4444/p2p/{wrong}")], now).unwrap(),
                                    5 => core.create_node_record(&peer.to_string(), vec![format!("/dns4/untrusted.invalid/tcp/4444/p2p/{peer}")], now).unwrap(),
                                    _ => core.create_node_record(&peer.to_string(), vec![address.clone()], now).unwrap(),
                                };
                                swarm.behaviour_mut().send_response(channel, Exchange {node_record}).unwrap();
                            }
                        }
                    }
                }
            });
        });
        let (address, peer) = address.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            address,
            peer,
            requests,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
}

#[test]
fn bootstrap_rejects_invalid_records_exchanges_only_own_record_and_redials_from_encrypted_cache() {
    let mut a = Node::start(&[TCP]);
    let mut b = Node::start(&[TCP]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Private history stays on this device",
        "private-bootstrap",
        1,
    );
    let before = a.snapshot();
    a.kill();
    b.kill();
    let baseline = cache_state(&a);
    // Each mode gets a new transport peer, connection and daemon. A previous rejection
    // cannot satisfy the next mode, and disk state is checked before any valid response.
    for mode in 0..6 {
        let raw = RecordPeer::start(mode);
        a.arguments = vec!["--bootstrap".into(), raw.address.clone()];
        a.launch();
        let info = relay::info_until(&a, |v| {
            v["bootstrap"]["rejectedRecords"]
                .as_u64()
                .is_some_and(|n| n > 0)
        });
        assert!(!raw.requests.lock().unwrap().is_empty());
        assert!(!verified(&info, &raw.peer), "invalid mode {mode}");
        assert_eq!(
            info["bootstrap"]["networkDomain"],
            hex::encode(agentic_node::NETWORK_DOMAIN)
        );
        assert_eq!(a.snapshot()["identity"], before["identity"]);
        assert_eq!(a.snapshot()["conversations"], before["conversations"]);
        a.kill();
        assert_eq!(
            cache_state(&a),
            baseline,
            "invalid mode {mode} mutated disk cache"
        );
    }
    let raw = RecordPeer::start(6);
    a.arguments = vec!["--bootstrap".into(), raw.address.clone()];
    a.launch();
    relay::info_until(&a, |v| verified(v, &raw.peer));
    a.kill();
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(a.root.path().join("profile.db"), &[0x11; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    let cached = core.cached_node_records(timestamp()).unwrap();
    assert_eq!(cached.iter().filter(|r| r.peer_id == raw.peer).count(), 1);
    for wire in raw.requests.lock().unwrap().iter() {
        let record = core.verify_node_record(wire, &a.peer, timestamp()).unwrap();
        assert_eq!(
            agentic_protocol::network_id(&record.author),
            before["identity"]["networkId"]
        );
        assert_eq!(record.addresses, vec![route(&a)]);
        assert!(wire.len() <= 4096);
    }
    drop(core);
    let requests = raw.requests.lock().unwrap().len();
    a.arguments.clear();
    a.launch();
    let info = relay::info_until(&a, |v| verified(v, &raw.peer));
    assert!(info["bootstrap"]["cachedHints"].as_u64().unwrap() >= 1);
    assert!(
        raw.requests.lock().unwrap().len() > requests,
        "the independent peer never initiates connections; the node must dial its durable cache"
    );
    assert_eq!(a.snapshot()["identity"], before["identity"]);
    assert_eq!(a.snapshot()["conversations"], before["conversations"]);
}

#[test]
fn bootstrap_hints_cannot_bypass_relay_only_policy() {
    let provider = relay::server(4);
    let forbidden = RecordPeer::start(6);
    let b = relay::client(&[&provider]);
    relay::reservations(&b, 1);
    let circuit = b.call("node_info", json!({}))["relayRoutes"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let a = Node::start_with_args(
        &[TCP],
        vec![
            "--relay-only".into(),
            "--relay".into(),
            route(&provider),
            "--bootstrap".into(),
            forbidden.address.clone(),
            "--bootstrap".into(),
            circuit,
        ],
    );
    relay::reservations(&a, 1);
    let info = relay::info_until(&a, |v| verified(v, &b.peer));
    assert_eq!(info["bootstrap"]["policyBlockedHints"], 1);
    let connections: Vec<_> = info["peerConnections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["peerId"] == b.peer)
        .collect();
    assert!(!connections.is_empty());
    assert!(connections.iter().all(|p| p["relayed"] == true));
    assert!(
        !info["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["peerId"] == forbidden.peer)
    );
    assert!(
        forbidden.requests.lock().unwrap().is_empty(),
        "successful circuit discovery must never probe the forbidden direct endpoint"
    );
}

fn cache_wires(node: &Node) -> Vec<Vec<u8>> {
    assert!(node.child.is_none());
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(node.root.path().join("profile.db"), &[0x11; 32])
            .unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    let mut wires: Vec<_> = core
        .cached_node_records(timestamp())
        .unwrap()
        .into_iter()
        .map(|r| r.wire)
        .collect();
    wires.sort();
    wires
}

#[test]
fn bootstrap_rejects_a_fifth_explicit_hint_before_starting_network_service() {
    let root = TempDir::new().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kaiki-agentic-node"));
    cmd.args(["serve", "--secrets-stdin", "--profile"])
        .arg(root.path().join("profile.db"))
        .arg("--ipc")
        .arg(root.path().join("node.sock"));
    for port in 1..=5 {
        let peer = identity::Keypair::generate_ed25519().public().to_peer_id();
        cmd.arg("--bootstrap")
            .arg(format!("/ip4/127.0.0.1/tcp/{port}/p2p/{peer}"));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "{}",
        json!({"masterKey":KEY,"ownerToken":TOKEN})
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("fifth hint did not fail startup");
        }
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("at most4 bootstrap"),
        "unexpected rejection: {error}"
    );
    assert!(!root.path().join("node.sock").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_inbound_invalid_routes_cannot_mutate_cache_and_valid_request_returns_only_self_record()
 {
    let mut a = Node::start(&[TCP]);
    let b = Node::start(&[TCP]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Existing private conversation",
        "bootstrap-inbound",
        1,
    );
    let before = a.snapshot();
    a.kill();
    let baseline = cache_state(&a);
    a.launch();
    let root = TempDir::new().unwrap();
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(root.path().join("raw.db"), &[91; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    let key = identity::Keypair::generate_ed25519();
    let own = key.public().to_peer_id();
    let mut swarm = peer_swarm(key.clone());
    let wrong_route = core
        .create_node_record(&own.to_string(), vec![route(&b)], timestamp())
        .unwrap();
    let valid = core
        .create_node_record(&own.to_string(), vec![], timestamp())
        .unwrap();
    let mut corrupt = valid.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    for wire in [wrong_route, corrupt] {
        let response = exchange(&mut swarm, &a, wire).await;
        assert!(response.is_err() || response.unwrap().node_record.is_empty());
        assert!(
            a.call("node_info", json!({}))["bootstrap"]["rejectedRecords"]
                .as_u64()
                .unwrap()
                > 0,
            "the actual inbound record must reach and fail verification"
        );
        assert_eq!(a.snapshot()["conversations"], before["conversations"]);
        a.kill();
        assert_eq!(cache_state(&a), baseline);
        a.launch();
        swarm = peer_swarm(key.clone());
    }
    let response = exchange(&mut swarm, &a, valid.clone()).await.unwrap();
    let record = core
        .verify_node_record(&response.node_record, &a.peer, timestamp())
        .unwrap();
    assert_eq!(
        agentic_protocol::network_id(&record.author),
        before["identity"]["networkId"]
    );
    assert_eq!(record.addresses, vec![route(&a)]);
    for _ in 0..7 {
        let response = exchange(&mut swarm, &a, valid.clone()).await.unwrap();
        assert!(!response.node_record.is_empty());
    }
    let throttled = exchange(&mut swarm, &a, valid).await;
    assert!(
        throttled.is_err() || throttled.unwrap().node_record.is_empty(),
        "the ninth inbound verification from one peer must be denied"
    );
    assert_eq!(a.snapshot()["conversations"], before["conversations"]);
    a.kill();
    assert_eq!(cache_wires(&a).len(), 2);
}

async fn exchange(
    swarm: &mut libp2p::Swarm<cbor::Behaviour<Exchange, Exchange>>,
    node: &Node,
    wire: Vec<u8>,
) -> Result<Exchange, String> {
    let peer = node.peer.parse().unwrap();
    let address: libp2p::Multiaddr = route(node).parse().unwrap();
    let _ = swarm.dial(
        libp2p::swarm::dial_opts::DialOpts::peer_id(peer)
            .allocate_new_port()
            .addresses(vec![address.clone()])
            .build(),
    );
    let id = swarm.behaviour_mut().send_request_with_addresses(
        &peer,
        Exchange { node_record: wire },
        vec![address],
    );
    tokio::time::timeout(Duration::from_secs(10), async {
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
    .expect("raw bootstrap request timed out")
}

fn peer_swarm(key: identity::Keypair) -> libp2p::Swarm<cbor::Behaviour<Exchange, Exchange>> {
    SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| {
            cbor::Behaviour::<Exchange, Exchange>::new(
                [(
                    StreamProtocol::new("/agentic-internet/bootstrap/1"),
                    ProtocolSupport::Full,
                )],
                Config::default().with_request_timeout(Duration::from_secs(5)),
            )
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build()
}

fn cache_state(node: &Node) -> Option<(u64, Vec<u8>)> {
    assert!(node.child.is_none());
    let store = agentic_store::ProfileStore::open(node.root.path().join("profile.db"), &[0x11; 32])
        .unwrap();
    store
        .state("network/peer-records")
        .unwrap()
        .map(|state| (state.revision, state.bytes.clone()))
}
