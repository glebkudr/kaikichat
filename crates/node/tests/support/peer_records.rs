use super::*;

#[test]
fn signed_route_refresh_survives_both_daemon_restarts_and_drains_original_queued_message() {
    let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let mut b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let group = connect(&a, &b, None);
    relay::assert_exchange(&a, &b, &group, "Original address works", "route-before", 1);
    let original = b.listeners.clone();
    let alice_identity = a.snapshot()["identity"].clone();
    let bob_identity = b.snapshot()["identity"].clone();
    b.kill();
    // Reserve the obsolete port so the new daemon cannot accidentally reuse it.
    let old_port = original[0]
        .rsplit('/')
        .next()
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let obsolete = std::net::TcpListener::bind(("127.0.0.1", old_port)).unwrap();
    b.listeners = vec!["/ip4/127.0.0.1/tcp/0".into()];
    b.launch();
    assert_ne!(b.listeners, original);
    assert_eq!(b.snapshot()["identity"], bob_identity);
    relay::assert_exchange(
        &b,
        &a,
        &group,
        "My signed return address changed",
        "route-announcement",
        2,
    );
    // Remove every existing authenticated connection and all in-memory route state.
    b.kill();
    a.kill();
    // Bootstrap can now initiate reverse connections. Model Bob losing only his disposable
    // discovery cache so such a connection cannot hide a broken restored Alice→Bob route.
    // Bob's confirmed contact, MLS state, identity and history remain intact.
    let mut store =
        agentic_store::ProfileStore::open(b.root.path().join("profile.db"), &[0x11; 32]).unwrap();
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
    drop(store);
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(a.root.path().join("profile.db"), &[0x11; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    let cached = core
        .cached_node_records(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap();
    let record = cached.iter().find(|r| r.peer_id == b.peer).unwrap();
    assert_eq!(
        record.addresses,
        vec![format!("{}/p2p/{}", b.listeners[0], b.peer)]
    );
    drop(core);
    a.launch();
    assert_eq!(a.snapshot()["identity"], alice_identity);
    let pending = a.send(
        &group,
        "Queued once, delivered through the restored new route",
        "route-after-restart",
    );
    assert_eq!(pending["delivery"]["phase"], "queued");
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 1);
    b.launch();
    let arrived = b.wait(|v| messages(v).iter().any(|m| m["id"] == pending["id"]));
    assert_eq!(messages(&arrived).len(), 3);
    assert_eq!(
        messages(&arrived).last().unwrap()["text"],
        "Queued once, delivered through the restored new route"
    );
    let acknowledged = a.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == pending["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(messages(&acknowledged).len(), 3);
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    assert_eq!(a.snapshot()["identity"], alice_identity);
    assert_eq!(b.snapshot()["identity"], bob_identity);
    let info = a.call("node_info", json!({}));
    let connection = info["peerConnections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["peerId"] == b.peer)
        .unwrap();
    assert!(
        connection["remoteAddress"]
            .as_str()
            .unwrap()
            .starts_with(&b.listeners[0])
    );
    drop(obsolete);
}
