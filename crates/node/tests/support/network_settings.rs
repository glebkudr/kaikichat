use super::*;
const TCP: &str = "/ip4/127.0.0.1/tcp/0";

fn route(provider: &Node) -> String {
    format!("{}/p2p/{}", provider.listeners[0], provider.peer)
}
fn preferences(provider: &Node, relay_only: bool, verify: bool) -> Value {
    json!({"relays":[route(provider)],"relayOnly":relay_only,"autoNatPeers":if verify {vec![route(provider)]} else {vec![]},"bootstrapPeers":[],"lanDiscovery":false,"dhtServer":false})
}
fn settings(node: &Node) -> Value {
    node.call("network_settings", json!({}))
}
fn configure(node: &Node, expected: u64, preferences: Value) -> Value {
    node.call(
        "configure_network",
        json!({"expectedRevision":expected,"preferences":preferences}),
    )
}
fn advertised(node: &Node) -> Vec<String> {
    let invite = node.call("create_invitation", json!({}));
    let root = TempDir::new().unwrap();
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(root.path().join("reader.db"), &[73; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    core.invitation_addresses(
        invite.as_str().unwrap(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}
fn core_settings(node: &Node) -> Value {
    let view = settings(node);
    json!({"revision":view["revision"],"preferences":view["preferences"]})
}
fn group_messages<'a>(value: &'a Value, group: &str) -> &'a [Value] {
    value["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == group)
        .and_then(|c| c["messages"].as_array())
        .map_or(&[], Vec::as_slice)
}
fn denied(node: &Node, request: Value, code: &str) {
    let reply = rpc(&node.socket(), TOKEN, "configure_network", request).unwrap();
    assert!(reply.get("result").is_none());
    assert_eq!(reply["error"]["code"], code, "{reply}");
}

#[test]
fn owner_network_settings_apply_real_relay_and_autonat_then_survive_daemon_restart() {
    let provider = Node::start_with_args(
        &[TCP],
        vec![
            "--relay-server".into(),
            "--autonat-server".into(),
            "--autonat-allow-local".into(),
        ],
    );
    let mut a = Node::start(&[TCP]);
    let b = Node::start(&[TCP]);
    let original = a.call("node_info", json!({}));
    assert_eq!(
        core_settings(&a),
        json!({"revision":0,"preferences":{"relays":[],"relayOnly":false,"autoNatPeers":[],"lanDiscovery":false,"dhtServer":false}})
    );
    let prefs = preferences(&provider, true, true);
    for node in [&a, &b] {
        let saved = configure(node, 0, prefs.clone());
        assert_eq!(saved["revision"], 1);
        assert_eq!(saved["preferences"], prefs);
        relay::reservations(node, 1);
        let verified = relay::info_until(node, |v| v["autoNat"]["status"] == "public");
        assert!(verified["autoNat"]["successfulProbes"].as_u64().unwrap() > 0);
        assert_eq!(verified["holePunch"]["enabled"], false);
        assert_eq!(verified["advertisedAddresses"], verified["relayRoutes"]);
    }
    let group = connect(&a, &b, None);
    assert_eq!(
        json!(advertised(&a)),
        a.call("node_info", json!({}))["relayRoutes"]
    );
    assert_eq!(
        json!(advertised(&b)),
        b.call("node_info", json!({}))["relayRoutes"]
    );
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Configured through owner API",
        "settings-live",
        1,
    );
    let info = a.call("node_info", json!({}));
    let application = info["peerConnections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["peerId"] == b.peer)
        .collect::<Vec<_>>();
    assert!(!application.is_empty());
    assert!(application.iter().all(|p| p["relayed"] == true));
    assert_eq!(info["listeners"], original["listeners"]);
    assert_eq!(info["peerId"], original["peerId"]);
    let identity = a.snapshot()["identity"].clone();
    a.kill();
    a.launch();
    assert_eq!(core_settings(&a), json!({"revision":1,"preferences":prefs}));
    relay::reservations(&a, 1);
    assert_eq!(a.snapshot()["identity"], identity);
    assert_eq!(
        a.call("node_info", json!({}))["listeners"],
        original["listeners"]
    );
    assert_eq!(configure(&a, 0, prefs.clone())["revision"], 1);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Same profile and settings after restart",
        "settings-restart",
        2,
    );
    relay::assert_exchange(
        &b,
        &a,
        &group,
        "Return route after restart",
        "settings-reply",
        3,
    );
    let reply = rpc(&a.socket(), &hex::encode([0x33;32]), "configure_network", json!({"expectedRevision":1,"preferences":{"relays":[],"relayOnly":false,"autoNatPeers":[]}})).unwrap();
    assert!(reply.get("result").is_none());
    assert_eq!(reply["error"]["code"], "unauthorized");
    assert_eq!(core_settings(&a), json!({"revision":1,"preferences":prefs}));
}

#[test]
fn invalid_stale_or_failed_setting_changes_leave_live_relay_and_history_usable() {
    let old = relay::server(16);
    let next = relay::server(16);
    let a = relay::client(&[&old]);
    let b = relay::client(&[&old]);
    relay::reservations(&a, 1);
    relay::reservations(&b, 1);
    let first = preferences(&old, true, false);
    configure(&a, 0, first.clone());
    relay::reservations(&a, 1);
    let group = connect(&a, &b, None);
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Working before settings failure",
        "settings-before-error",
        1,
    );
    let before = a.snapshot();
    let before_routes = a.call("node_info", json!({}))["relayRoutes"].clone();
    for bad in [
        json!({"relays":["/ip4/127.0.0.1/tcp/4001"],"relayOnly":true,"autoNatPeers":[]}),
        json!({"relays":[route(&old),route(&old)],"relayOnly":true,"autoNatPeers":[]}),
        json!({"relays":[],"relayOnly":true,"autoNatPeers":[]}),
        json!({"relays":[route(&old)],"relayOnly":false,"autoNatPeers":[format!("{}/p2p-circuit/p2p/{}",route(&next),a.peer)]}),
        json!({"relays":[route(&old)],"relayOnly":true,"autoNatPeers":[],"relayServer":true}),
    ] {
        denied(
            &a,
            json!({"expectedRevision":1,"preferences":bad}),
            "invalid_request",
        );
        assert_eq!(core_settings(&a), json!({"revision":1,"preferences":first}));
        assert_eq!(a.call("node_info", json!({}))["relayRoutes"], before_routes);
    }
    let db = rusqlite::Connection::open(a.root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{KEY}'\"; CREATE TRIGGER fail_settings BEFORE INSERT ON states BEGIN SELECT RAISE(ABORT,'disk full'); END; CREATE TRIGGER fail_settings_update BEFORE UPDATE ON states BEGIN SELECT RAISE(ABORT,'disk full'); END;")).unwrap();
    let changed = preferences(&next, true, false);
    let request = json!({"expectedRevision":1,"preferences":changed});
    denied(&a, request.clone(), "unavailable");
    assert_eq!(core_settings(&a), json!({"revision":1,"preferences":first}));
    assert_eq!(a.call("node_info", json!({}))["relayRoutes"], before_routes);
    assert_eq!(
        old.call("node_info", json!({}))["relayServer"]["activeReservations"],
        2
    );
    assert_eq!(
        next.call("node_info", json!({}))["relayServer"]["activeReservations"],
        0
    );
    assert_eq!(a.snapshot(), before);
    db.execute_batch("DROP TRIGGER fail_settings; DROP TRIGGER fail_settings_update;")
        .unwrap();
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Existing route survives failed preference write",
        "settings-write-recovered",
        2,
    );
    assert_eq!(a.call("configure_network", request.clone())["revision"], 2);
    relay::reservations(&a, 1);
    let current = relay::info_until(&a, |v| {
        v["relayRoutes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.as_str().unwrap().contains(&next.peer))
            && v["relayRoutes"].as_array().unwrap().len() == 1
    });
    relay::info_until(&old, |v| v["relayServer"]["activeReservations"] == 1);
    assert_eq!(json!(advertised(&a)), current["relayRoutes"]);
    assert_eq!(a.call("configure_network", request)["revision"], 2);
    denied(
        &a,
        json!({"expectedRevision":1,"preferences":first}),
        "state_conflict",
    );
    assert_eq!(
        core_settings(&a),
        json!({"revision":2,"preferences":changed})
    );
    relay::assert_exchange(
        &a,
        &b,
        &group,
        "Continues with new incoming relay provider",
        "settings-replaced",
        3,
    );
    // A fresh contact must actually reach A through its new incoming provider.
    let fresh = Node::start(&[TCP]);
    fresh.profile("Carol");
    let invite = a.call("create_invitation", json!({}));
    let contact = fresh.call("add_contact", json!({"name":"Alice","invitation":invite}));
    let incoming_group = contact["id"].as_str().unwrap();
    a.wait(|v| {
        v["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == incoming_group)
    });
    let incoming = fresh.send(
        incoming_group,
        "Fresh contact reaches the replacement relay",
        "settings-fresh-incoming",
    );
    let arrived = a.wait(|v| {
        group_messages(v, incoming_group)
            .iter()
            .any(|m| m["id"] == incoming["id"])
    });
    assert_eq!(group_messages(&arrived, incoming_group).len(), 1);
    assert_eq!(
        group_messages(&arrived, incoming_group)[0]["text"],
        "Fresh contact reaches the replacement relay"
    );
    let receipted = fresh.wait(|v| {
        group_messages(v, incoming_group)
            .iter()
            .any(|m| m["id"] == incoming["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(group_messages(&receipted, incoming_group).len(), 1);
    assert_eq!(group_messages(&a.snapshot(), &group).len(), 3);
    let connections = a.call("node_info", json!({}))["peerConnections"]
        .as_array()
        .unwrap()
        .clone();
    let via_new = connections
        .iter()
        .filter(|p| p["peerId"] == fresh.peer)
        .collect::<Vec<_>>();
    assert!(!via_new.is_empty());
    assert!(
        via_new
            .iter()
            .all(|p| p["relayed"] == true
                && p["localAddress"].as_str().unwrap().contains(&next.peer))
    );
    assert!(
        next.call("node_info", json!({}))["relayServer"]["acceptedCircuits"]
            .as_u64()
            .unwrap()
            > 0
    );
    // Carol advertises direct addresses, but the established authenticated relay circuit
    // is bidirectional. Replying must use it without falling back to a direct dial.
    let reply = a.send(
        incoming_group,
        "Reply through the existing relay circuit",
        "settings-fresh-reply",
    );
    let reply_received = fresh.wait(|v| {
        group_messages(v, incoming_group)
            .iter()
            .any(|m| m["id"] == reply["id"])
    });
    assert_eq!(group_messages(&reply_received, incoming_group).len(), 2);
    assert_eq!(
        group_messages(&reply_received, incoming_group)[1]["text"],
        "Reply through the existing relay circuit"
    );
    let reply_acked = a.wait(|v| {
        group_messages(v, incoming_group)
            .iter()
            .any(|m| m["id"] == reply["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(group_messages(&reply_acked, incoming_group).len(), 2);
    assert!(
        a.call("node_info", json!({}))["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["peerId"] == fresh.peer)
            .all(|p| p["relayed"] == true)
    );
}

#[test]
fn enabling_relay_only_closes_existing_direct_application_connection_and_preserves_queue() {
    let provider = relay::server(16);
    let a = Node::start(&[TCP]);
    let b = Node::start(&[TCP]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(&a, &b, &group, "Initial direct chat", "settings-direct", 1);
    let before = a.call("node_info", json!({}));
    assert!(
        before["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["peerId"] == b.peer && p["relayed"] == false)
    );
    configure(&a, 0, preferences(&provider, true, false));
    relay::reservations(&a, 1);
    relay::info_until(&a, |v| {
        !v["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["peerId"] == b.peer && p["relayed"] == false)
    });
    let pending = a.send(
        &group,
        "Wait until this contact has an allowed route",
        "settings-queued-policy",
    );
    assert_eq!(pending["delivery"]["phase"], "queued");
    thread::sleep(Duration::from_millis(800));
    assert_eq!(
        messages(&b.snapshot()).len(),
        1,
        "an existing direct connection must not bypass the changed route policy"
    );
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 1);
    configure(&a, 1, preferences(&provider, false, false));
    // Reopening direct routing must drain the original operation without another send command.
    let received = b.wait(|v| messages(v).iter().any(|m| m["id"] == pending["id"]));
    assert_eq!(messages(&received).len(), 2);
    assert_eq!(
        messages(&received).last().unwrap()["text"],
        "Wait until this contact has an allowed route"
    );
    a.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == pending["id"] && m["delivery"]["phase"] == "delivered")
    });
    let after = a.call("node_info", json!({}));
    assert_eq!(after["pendingOutbox"], 0);
    assert_eq!(after["peerId"], before["peerId"]);
    assert_eq!(after["listeners"], before["listeners"]);
    assert_eq!(a.snapshot()["identity"]["name"], "Alice");
}

/// What an operator names with `--public-address` is what the node tells
/// others, not the addresses its listener binds.
#[test]
fn public_addresses_given_at_start_are_what_the_node_advertises() {
    let node = Node::start_with_args(
        &[TCP],
        vec![
            "--public-address".into(),
            "/ip4/203.0.113.7/udp/4101/quic-v1".into(),
            "--public-address".into(),
            "/ip4/203.0.113.7/tcp/4101".into(),
        ],
    );
    let info = node.call("node_info", json!({}));
    assert_eq!(
        info["advertisedAddresses"],
        json!([
            format!("/ip4/203.0.113.7/udp/4101/quic-v1/p2p/{}", node.peer),
            format!("/ip4/203.0.113.7/tcp/4101/p2p/{}", node.peer),
        ]),
        "{info}"
    );
}
