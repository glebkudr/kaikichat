use super::bootstrap::RecordPeer;
use super::*;
const TCP: &str = "/ip4/127.0.0.1/tcp/0";
fn prefs(addresses: Vec<String>) -> Value {
    json!({"relays":[],"relayOnly":false,"autoNatPeers":[],"bootstrapPeers":addresses,"lanDiscovery":false,"dhtServer":false})
}
fn settings(node: &Node) -> Value {
    node.call("network_settings", json!({}))
}
fn configure(node: &Node, revision: u64, preferences: Value) -> Value {
    node.call(
        "configure_network",
        json!({"expectedRevision":revision,"preferences":preferences}),
    )
}
fn ready(node: &Node, peer: &str) -> Value {
    relay::info_until(node, |v| {
        v["bootstrap"]["verifiedPeers"]
            .as_array()
            .is_some_and(|peers| peers.iter().any(|p| p["peerId"] == peer))
    })
}
fn app_state(node: &Node) -> Value {
    let v = node.snapshot();
    json!({"identity":v["identity"],"conversations":v["conversations"]})
}

#[test]
fn owner_bootstrap_settings_restore_without_cache_override_cli_hints_and_preserve_pending_message()
{
    let mut a = Node::start(&[TCP]);
    let mut b = Node::start(&[TCP]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Before discovery settings",
        "settings-bootstrap-before",
        1,
    );
    b.kill();
    let pending = a.send(
        &group,
        "Same operation after settings and restart",
        "settings-bootstrap-pending",
    );
    assert_eq!(pending["delivery"]["phase"], "queued");
    let before = app_state(&a);
    let initial = settings(&a);
    let provider = RecordPeer::start(6);
    let wanted = prefs(vec![provider.address.clone()]);
    let saved = configure(&a, 0, wanted.clone());
    assert_eq!(saved["revision"], 1);
    assert_eq!(saved["preferences"], wanted);
    ready(&a, &provider.peer);
    let live = settings(&a);
    let count = provider.requests.lock().unwrap().len();
    assert_eq!(configure(&a, 0, wanted.clone())["revision"], 1);
    assert_eq!(
        provider.requests.lock().unwrap().len(),
        count,
        "lost-response retry must not rebuild the swarm and issue another handshake"
    );
    assert_eq!(
        settings(&a)["status"]["bootstrap"]["verifiedPeers"],
        live["status"]["bootstrap"]["verifiedPeers"]
    );
    assert_eq!(app_state(&a), before);
    a.kill();
    // Lose only disposable discovery cache. The independent raw peer never dials back,
    // so reopening now needs the saved preference, not a learned/reverse connection.
    let mut store =
        agentic_store::ProfileStore::open(a.root.path().join("profile.db"), &[0x11; 32]).unwrap();
    let revision = store
        .state("network/peer-records")
        .unwrap()
        .unwrap()
        .revision;
    store
        .commit_states(vec![agentic_store::StateChange {
            namespace: "network/peer-records".into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&json!({"version":1,"entries":{}})).unwrap(),
        }])
        .unwrap();
    drop(store);
    let sentinel = RecordPeer::start(6);
    a.arguments = vec!["--bootstrap".into(), sentinel.address.clone()];
    a.launch();
    let restored = settings(&a);
    assert_eq!(restored["revision"], 1);
    assert_eq!(restored["preferences"], wanted);
    assert_eq!(restored["status"]["peerId"], initial["status"]["peerId"]);
    assert_eq!(
        restored["status"]["listeners"],
        initial["status"]["listeners"]
    );
    ready(&a, &provider.peer);
    assert!(provider.requests.lock().unwrap().len() > count);
    assert_eq!(app_state(&a), before);
    b.launch();
    b.wait(|v| messages(v).iter().any(|m| m["id"] == pending["id"]));
    a.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == pending["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(messages(&a.snapshot()).len(), 2);
    assert_eq!(messages(&b.snapshot()).len(), 2);
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    assert_eq!(
        messages(&b.snapshot()).last().unwrap()["text"],
        "Same operation after settings and restart"
    );
    assert!(
        sentinel.requests.lock().unwrap().is_empty(),
        "saved hints replace CLI hints; a reachable CLI sentinel must never be contacted"
    );
}

#[test]
fn rejected_bootstrap_settings_cannot_dial_new_hint_or_interrupt_working_chat() {
    let a = Node::start(&[TCP]);
    let b = Node::start(&[TCP]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Working before failed save",
        "bootstrap-config-before",
        1,
    );
    let before = app_state(&a);
    let initial = settings(&a);
    let old = RecordPeer::start(6);
    let next = RecordPeer::start(6);
    let db = rusqlite::Connection::open(a.root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{KEY}'\"; CREATE TRIGGER fail_bootstrap_insert BEFORE INSERT ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'disk full'); END;")).unwrap();
    let request = json!({"expectedRevision":0,"preferences":prefs(vec![old.address.clone()])});
    let failure = rpc(&a.socket(), TOKEN, "configure_network", request.clone()).unwrap();
    assert_eq!(failure["error"]["code"], "unavailable");
    assert_eq!(settings(&a)["preferences"], initial["preferences"]);
    assert_eq!(settings(&a)["revision"], 0);
    assert!(old.requests.lock().unwrap().is_empty());
    assert_eq!(app_state(&a), before);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Chat works with failed initial settings write",
        "bootstrap-config-insert-failure",
        2,
    );
    assert!(
        old.requests.lock().unwrap().is_empty(),
        "failed INSERT must not queue a background discovery dial"
    );
    let before = app_state(&a);
    db.execute_batch("DROP TRIGGER fail_bootstrap_insert;")
        .unwrap();
    let first = configure(&a, 0, prefs(vec![old.address.clone()]));
    ready(&a, &old.peer);
    ready(&a, &b.peer);
    let source = settings(&a);
    for addresses in [
        vec![next.address.clone(), next.address.clone()],
        vec![next.address.clone(); 5],
        vec![format!("{}/p2p/{}", a.listeners[0], a.peer)],
        vec!["/ip4/127.0.0.1/tcp/4001".into()],
        vec![format!(
            "/dns4/untrusted.invalid/tcp/4001/p2p/{}",
            next.peer
        )],
    ] {
        let response = rpc(
            &a.socket(),
            TOKEN,
            "configure_network",
            json!({"expectedRevision":1,"preferences":prefs(addresses)}),
        )
        .unwrap();
        assert_eq!(response["error"]["code"], "invalid_request");
        assert_eq!(settings(&a)["revision"], 1);
        assert_eq!(settings(&a)["preferences"], first["preferences"]);
        assert_eq!(
            settings(&a)["status"]["bootstrap"]["verifiedPeers"],
            source["status"]["bootstrap"]["verifiedPeers"]
        );
    }
    let request = json!({"expectedRevision":1,"preferences":prefs(vec![next.address.clone()])});
    let stale = rpc(
        &a.socket(),
        TOKEN,
        "configure_network",
        json!({"expectedRevision":0,"preferences":prefs(vec![next.address.clone()])}),
    )
    .unwrap();
    assert_eq!(stale["error"]["code"], "state_conflict");
    db.execute_batch("CREATE TRIGGER fail_bootstrap_update BEFORE UPDATE ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let failure = rpc(&a.socket(), TOKEN, "configure_network", request.clone()).unwrap();
    assert_eq!(failure["error"]["code"], "unavailable");
    assert!(next.requests.lock().unwrap().is_empty());
    assert_eq!(settings(&a)["preferences"], first["preferences"]);
    assert_eq!(
        settings(&a)["status"]["bootstrap"]["verifiedPeers"],
        source["status"]["bootstrap"]["verifiedPeers"]
    );
    assert_eq!(app_state(&a), before);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Chat still works while settings writes fail",
        "bootstrap-config-disk-full",
        3,
    );
    assert!(
        next.requests.lock().unwrap().is_empty(),
        "failed UPDATE must not queue a background discovery dial"
    );
    db.execute_batch("DROP TRIGGER fail_bootstrap_update;")
        .unwrap();
    let saved = a.call("configure_network", request);
    assert_eq!(saved["revision"], 2);
    ready(&a, &next.peer);
    assert!(!next.requests.lock().unwrap().is_empty());
    assert_eq!(messages(&a.snapshot()).len(), 3);
    assert_eq!(messages(&b.snapshot()).len(), 3);
}

#[test]
fn owner_live_bootstrap_configuration_obeys_relay_only_during_actual_chat() {
    let relay = relay::server(8);
    let forbidden = RecordPeer::start(6);
    let a = Node::start(&[TCP]);
    let b = relay::client(&[&relay]);
    relay::reservations(&b, 1);
    let relay_route = format!("{}/p2p/{}", relay.listeners[0], relay.peer);
    let configured = json!({"relays":[relay_route],"relayOnly":true,"lanDiscovery":true,"dhtServer":false,"autoNatPeers":[],"bootstrapPeers":[forbidden.address]});
    let saved = configure(&a, 0, configured.clone());
    assert_eq!(saved["preferences"], configured);
    relay::reservations(&a, 1);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Relay policy applies after owner configuration",
        "bootstrap-owner-relay-policy",
        1,
    );
    assert!(forbidden.requests.lock().unwrap().is_empty());
    let info = a.call("node_info", json!({}));
    assert_eq!(info["lanDiscovery"]["enabled"], true);
    assert_eq!(info["lanDiscovery"]["active"], false);
    assert_eq!(info["lanDiscovery"]["blockedByPolicy"], true);
    assert_eq!(info["lanDiscovery"]["peers"], json!([]));
    assert!(info["bootstrap"]["policyBlockedHints"].as_u64().unwrap() >= 1);
    let connections: Vec<_> = info["peerConnections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["peerId"] == b.peer)
        .collect();
    assert!(!connections.is_empty());
    assert!(connections.iter().all(|c| c["relayed"] == true));
    assert!(
        !info["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["peerId"] == forbidden.peer)
    );
}
