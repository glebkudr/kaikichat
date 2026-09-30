use super::*;
use libp2p::identity;
const TCP: &str = "/ip4/127.0.0.1/tcp/0";
fn route(n: &Node) -> String {
    format!("{}/p2p/{}", n.listeners[0], n.peer)
}
fn verified(v: &Value, peer: &str) -> bool {
    v["bootstrap"]["verifiedPeers"]
        .as_array()
        .is_some_and(|p| p.iter().any(|p| p["peerId"] == peer))
}
fn routed(v: &Value, peer: &str) -> bool {
    v["routing"]["lookups"].as_array().is_some_and(|p| {
        p.iter()
            .any(|p| p["peerId"] == peer && p["state"] == "verified")
    })
}
fn seed(n: &Node) -> Node {
    Node::start_with_args(
        &[TCP],
        vec!["--bootstrap".into(), route(n), "--dht-server".into()],
    )
}

#[test]
fn routing_crosses_two_intermediaries_and_restores_a_moved_contacts_queued_message() {
    moved_recipient(false);
}
#[test]
fn routing_automatically_recovers_a_failed_durable_delivery_without_an_owner_lookup() {
    moved_recipient(true);
}
fn moved_recipient(automatic: bool) {
    let mut recipient = Node::start(&[TCP]);
    let mut sender = Node::start(&[TCP]);
    let group = connect(&sender, &recipient, None);
    relay::assert_exchange(
        &sender,
        &recipient,
        &group,
        "Before moving",
        "routing-before",
        1,
    );
    assert_eq!(
        sender.call("node_info", json!({}))["routing"]["lookups"],
        json!([]),
        "healthy delivery must not initiate DHT lookup"
    );
    let original = recipient.snapshot()["identity"].clone();
    recipient.kill();
    let old_port: u16 = recipient.listeners[0]
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let _obsolete = std::net::TcpListener::bind(("127.0.0.1", old_port)).unwrap();
    clear_cache(&recipient);
    let queued = sender.send(
        &group,
        "Delivered to the same recipient at its new address",
        "routing-after",
    );
    sender.kill();
    // Empty intermediate profiles know only their next hop; no contact or DB is copied.
    let mut c = Node::start_with_args(&[TCP], vec!["--dht-server".into()]);
    recipient.listeners = vec![TCP.into()];
    recipient.arguments = vec!["--bootstrap".into(), route(&c)];
    recipient.launch();
    relay::info_until(&c, |v| verified(v, &recipient.peer));
    let mut b = seed(&c);
    relay::info_until(&b, |v| verified(v, &c.peer));
    sender.arguments = vec!["--bootstrap".into(), route(&b)];
    sender.launch();
    relay::info_until(&sender, |v| verified(v, &b.peer));
    assert_eq!(sender.call("node_info", json!({}))["pendingOutbox"], 1);
    assert_eq!(messages(&recipient.snapshot()).len(), 1);
    if !automatic {
        sender.call("lookup_peer", json!({"peerId":recipient.peer}));
    }
    let info = relay::info_until(&sender, |v| routed(v, &recipient.peer));
    assert_eq!(info["routing"]["mode"], "client");
    assert_eq!(
        recipient.call("node_info", json!({}))["routing"]["mode"],
        "client"
    );
    for intermediary in [&b, &c] {
        assert_eq!(
            intermediary.call("node_info", json!({}))["routing"]["mode"],
            "server"
        );
    }
    let found = info["routing"]["lookups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["peerId"] == recipient.peer)
        .unwrap();
    assert_eq!(found["rootId"], original["networkId"]);
    assert!(found["requests"].as_u64().unwrap() >= 2);
    assert!(found["requests"].as_u64().unwrap() <= 32);
    recipient.wait(|v| messages(v).iter().any(|m| m["id"] == queued["id"]));
    sender.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == queued["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(messages(&recipient.snapshot()).len(), 2);
    assert_eq!(
        messages(&recipient.snapshot()).last().unwrap()["text"],
        "Delivered to the same recipient at its new address"
    );
    assert_eq!(sender.call("node_info", json!({}))["pendingOutbox"], 0);
    assert_eq!(recipient.snapshot()["identity"], original);
    assert!(c.snapshot()["conversations"].as_array().unwrap().is_empty());
    assert!(b.snapshot()["conversations"].as_array().unwrap().is_empty());
    sender.kill();
    // The authenticated new route survives restart without an explicit intermediate.
    b.kill();
    c.kill();
    recipient.kill();
    clear_cache(&recipient);
    let records = cached(&sender);
    assert_eq!(
        records
            .iter()
            .find(|r| r.peer_id == recipient.peer)
            .unwrap()
            .addresses,
        vec![route(&recipient)]
    );
    recipient.arguments.clear();
    recipient.launch();
    sender.arguments.clear();
    sender.launch();
    relay::assert_exchange(
        &sender,
        &recipient,
        &group,
        "After route cache restart",
        "routing-restart",
        3,
    );
}

#[test]
fn routing_uses_a_surviving_seed_and_does_not_turn_closest_peers_into_a_found_target() {
    let target = Node::start(&[TCP]);
    target.profile("Target");
    let live = seed(&target);
    relay::info_until(&live, |v| verified(v, &target.peer));
    let mut dead = Node::start_with_args(&[TCP], vec!["--dht-server".into()]);
    let a = Node::start_with_args(
        &[TCP],
        vec![
            "--bootstrap".into(),
            route(&dead),
            "--bootstrap".into(),
            route(&live),
        ],
    );
    a.profile("Searching owner");
    relay::info_until(&a, |v| verified(v, &dead.peer) && verified(v, &live.peer));
    dead.kill();
    a.call("lookup_peer", json!({"peerId": target.peer}));
    relay::info_until(&a, |v| routed(v, &target.peer));
    assert!(a.snapshot()["conversations"].as_array().unwrap().is_empty());
    let absent = identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    a.call("lookup_peer", json!({"peerId": absent}));
    let info = relay::info_until(&a, |v| {
        v["routing"]["lookups"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|r| r["peerId"] == absent && r["state"] == "not-found")
        })
    });
    assert!(!verified(&info, &absent));
    assert!(info["routing"]["peers"].as_u64().unwrap() <= 128);
    assert!(info["routing"]["inFlight"].as_u64().unwrap() <= 2);
    assert!(a.snapshot()["conversations"].as_array().unwrap().is_empty());
    let invitation = target.call("create_invitation", json!({}));
    let contact = a.call(
        "add_contact",
        json!({"name":"Target","invitation":invitation}),
    );
    target.wait(|s| {
        s["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == contact["id"])
    });
    relay::assert_exchange(
        &a,
        &target,
        contact["id"].as_str().unwrap(),
        "Lookup did not grant contact authority",
        "routing-consent",
        1,
    );
}

#[test]
fn routing_owner_requests_are_bounded_and_respect_relay_only_policy() {
    let a = Node::start(&[TCP]);
    let peer = identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    for request in [
        json!({"peerId":"bad"}),
        json!({"peerId":a.peer}),
        json!({"peerId":peer,"addresses":["/ip4/127.0.0.1/tcp/1"]}),
    ] {
        let r = rpc(&a.socket(), TOKEN, "lookup_peer", request).unwrap();
        assert_eq!(r["error"]["code"], "invalid_request");
    }
    let r = rpc(
        &a.socket(),
        &"00".repeat(32),
        "lookup_peer",
        json!({"peerId":peer}),
    )
    .unwrap();
    assert_eq!(r["error"]["code"], "unauthorized");
    assert!(r.get("result").is_none());
    assert_eq!(
        a.call("node_info", json!({}))["routing"]["lookups"],
        json!([])
    );
    let provider = relay::server(2);
    a.call("configure_network",json!({"expectedRevision":0,"preferences":{"relays":[route(&provider)],"relayOnly":true,"autoNatPeers":[],"bootstrapPeers":[],"lanDiscovery":false}}));
    relay::reservations(&a, 1);
    let r = rpc(&a.socket(), TOKEN, "lookup_peer", json!({"peerId":peer})).unwrap();
    assert_eq!(r["error"]["code"], "policy_blocked");
    let info = a.call("node_info", json!({}));
    assert_eq!(info["routing"]["enabled"], false);
    assert_eq!(info["routing"]["inFlight"], 0);
    assert_eq!(info["routing"]["peers"], 0);
    assert_eq!(info["routing"]["lookups"], json!([]));
}

fn clear_cache(n: &Node) {
    let mut store =
        agentic_store::ProfileStore::open(n.root.path().join("profile.db"), &[0x11; 32]).unwrap();
    let revision = store
        .state("network/peer-records")
        .unwrap()
        .map_or(0, |s| s.revision);
    store
        .commit_states(vec![agentic_store::StateChange {
            namespace: "network/peer-records".into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&json!({"version":1,"entries":{}})).unwrap(),
        }])
        .unwrap();
}
fn cached(n: &Node) -> Vec<agentic_core::NodeRecord> {
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(n.root.path().join("profile.db"), &[0x11; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    core.cached_node_records(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}
#[path = "routing_fixture.rs"]
mod fixture;
#[test]
fn routing_rejects_a_reachable_targets_bad_signature_then_recovers_after_a_valid_reply() {
    let farm = fixture::Farm::new(2);
    let mut a = Node::start_with_args(&[TCP], vec!["--bootstrap".into(), farm.seed()]);
    a.profile("Owner");
    relay::info_until(&a, |v| verified(v, &farm.seed_peer()));
    let before = a.snapshot();
    a.kill();
    let baseline = cached(&a);
    a.launch();
    farm.invalid(true);
    a.call("lookup_peer", json!({"peerId":farm.target()}));
    relay::info_until(&a, |v| {
        v["routing"]["lookups"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|r| r["peerId"] == farm.target() && r["state"] == "authentication-failed")
        })
    });
    assert!(
        farm.target_bootstrap_requests() > 0,
        "target was actually reached, not merely absent"
    );
    assert!(!verified(&a.call("node_info", json!({})), &farm.target()));
    assert_eq!(a.snapshot()["conversations"], before["conversations"]);
    a.kill();
    assert_eq!(
        cached(&a)
            .iter()
            .map(|r| r.peer_id.clone())
            .collect::<Vec<_>>(),
        baseline
            .iter()
            .map(|r| r.peer_id.clone())
            .collect::<Vec<_>>()
    );
    farm.invalid(false);
    a.launch();
    a.call("lookup_peer", json!({"peerId":farm.target()}));
    relay::info_until(&a, |v| routed(v, &farm.target()));
    assert_eq!(a.snapshot()["identity"], before["identity"]);
    assert_eq!(a.snapshot()["conversations"], before["conversations"]);
    a.kill();
    assert!(cached(&a).iter().any(|r| r.peer_id == farm.target()));
}
#[test]
fn routing_stops_a_real_referral_chain_at_the_request_budget_and_releases_the_slot() {
    let farm = fixture::Farm::new(40);
    let a = Node::start_with_args(&[TCP], vec!["--bootstrap".into(), farm.seed()]);
    relay::info_until(&a, |v| verified(v, &farm.seed_peer()));
    let before = farm.find_requests();
    a.call("lookup_peer", json!({"peerId":farm.target()}));
    let info = relay::info_until(&a, |v| {
        v["routing"]["lookups"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|r| r["peerId"] == farm.target() && r["state"] == "budget-exhausted")
        })
    });
    let row = info["routing"]["lookups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["peerId"] == farm.target())
        .unwrap();
    assert_eq!(row["requests"], 32);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        farm.find_requests() - before,
        32,
        "observe admitted requests on the independent servers; a 33rd request is forbidden"
    );
    assert_eq!(a.call("node_info", json!({}))["routing"]["inFlight"], 0);
    a.call("lookup_peer", json!({"peerId":farm.seed_peer()}));
    relay::info_until(&a, |v| routed(v, &farm.seed_peer()));
}
