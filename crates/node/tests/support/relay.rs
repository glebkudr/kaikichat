use super::*;

const TCP: &str = "/ip4/127.0.0.1/tcp/0";

pub(super) fn server(capacity: usize) -> Node {
    server_on(capacity, TCP)
}
fn server_on(capacity: usize, transport: &str) -> Node {
    Node::start_with_args(
        &[transport],
        vec![
            "--relay-server".into(),
            "--relay-capacity".into(),
            capacity.to_string(),
        ],
    )
}
pub(super) fn client(relays: &[&Node]) -> Node {
    let mut arguments = vec!["--relay-only".into()];
    for relay in relays {
        arguments.extend([
            "--relay".into(),
            format!("{}/p2p/{}", relay.listeners[0], relay.peer),
        ]);
    }
    Node::start_with_args(&[TCP], arguments)
}
pub(super) fn info_until(node: &Node, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        let info = node.call("node_info", json!({}));
        if predicate(&info) {
            return info;
        }
        assert!(
            Instant::now() < deadline,
            "relay state did not converge: {info}"
        );
        thread::sleep(Duration::from_millis(40));
    }
}
pub(super) fn reservations(node: &Node, count: usize) -> Value {
    info_until(node, |v| {
        v["relayRoutes"]
            .as_array()
            .is_some_and(|a| a.len() == count)
    })
}
pub(super) fn assert_exchange(
    a: &Node,
    b: &Node,
    group: &str,
    text: &str,
    operation: &str,
    count: usize,
) {
    let accepted = a.send(group, text, operation);
    let received = b.wait(|v| messages(v).iter().any(|m| m["id"] == accepted["id"]));
    assert_eq!(messages(&received).len(), count);
    assert_eq!(messages(&received).last().unwrap()["text"], text);
    let sent = a.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == accepted["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(messages(&sent).len(), count);
    assert_eq!(a.send(group, text, operation)["id"], accepted["id"]);
    assert_eq!(messages(&b.snapshot()).len(), count);
}
fn peer_relay(node: &Node, peer: &str) -> String {
    let info = info_until(node, |v| {
        v["peerConnections"]
            .as_array()
            .is_some_and(|connections| connections.iter().any(|c| c["peerId"] == peer))
    });
    let connections = info["peerConnections"].as_array().unwrap();
    let matches: Vec<_> = connections.iter().filter(|c| c["peerId"] == peer).collect();
    assert!(
        matches.iter().all(|c| c["relayed"] == true),
        "direct peer connection escaped relay-only mode: {info}"
    );
    // Bootstrap recovery can open reciprocal circuits. A failover test must identify
    // the provider of EVERY live application circuit, including inbound endpoints.
    let providers: Vec<_> = matches
        .iter()
        .map(|connection| {
            let route = ["remoteAddress", "localAddress"]
                .iter()
                .filter_map(|field| connection[field].as_str())
                .find(|address| address.contains("/p2p-circuit"))
                .unwrap_or_else(|| panic!("relay circuit has no provider address: {info}"));
            relay_id(route)
        })
        .collect();
    assert!(
        providers.iter().all(|provider| provider == &providers[0]),
        "all active application circuits must use the provider being failed: {info}"
    );
    providers[0].clone()
}
fn relay_id(route: &str) -> String {
    let parts: Vec<_> = route.split('/').collect();
    let circuit = parts.iter().position(|s| *s == "p2p-circuit").unwrap();
    assert_eq!(parts[circuit - 2], "p2p");
    parts[circuit - 1].to_string()
}

#[test]
fn relay_ciphertext_delivery_fails_over_and_recovers_durable_queue_after_restart() {
    let mut r1 = server(16);
    let mut r2 = server_on(16, "/ip4/127.0.0.1/udp/0/quic-v1");
    let mut a = client(&[&r1, &r2]);
    let b = client(&[&r1, &r2]);
    reservations(&a, 2);
    let bob_routes = reservations(&b, 2)["relayRoutes"].clone();
    // Decode the actual signed invitation through a separate core, with no network or daemon state.
    a.profile("Alice");
    b.profile("Bob");
    let invite = b.call("create_invitation", json!({}));
    let root = TempDir::new().unwrap();
    let core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(root.path().join("reader.db"), &[73; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let advertised = core
        .invitation_addresses(invite.as_str().unwrap(), now)
        .unwrap();
    assert_eq!(json!(advertised), bob_routes);
    assert!(advertised.iter().all(|a| a.contains("/p2p-circuit/p2p/")));
    let conversation = a.call("add_contact", json!({"name":"Bob","invitation":invite}));
    let group = conversation["id"].as_str().unwrap();
    b.wait(|v| v["conversations"].as_array().unwrap().len() == 1);
    assert_exchange(&a, &b, group, "Private relay-only hello", "relay-hello", 1);
    assert_exchange(&b, &a, group, "Private relay-only reply", "relay-reply", 2);
    let active = peer_relay(&a, &b.peer);
    assert!(active == r1.peer || active == r2.peer);
    let (failed, survivor) = if active == r1.peer {
        (&mut r1, &mut r2)
    } else {
        (&mut r2, &mut r1)
    };
    assert!(
        failed.call("node_info", json!({}))["relayServer"]["acceptedCircuits"]
            .as_u64()
            .unwrap()
            > 0
    );
    failed.kill();
    reservations(&a, 1);
    reservations(&b, 1);
    assert_exchange(
        &a,
        &b,
        group,
        "Survives the active relay failing",
        "relay-failover",
        3,
    );
    assert_eq!(peer_relay(&a, &b.peer), survivor.peer);
    survivor.kill();
    reservations(&a, 0);
    reservations(&b, 0);
    let queued = a.send(group, "Durable after all relays disappear", "relay-offline");
    assert_eq!(queued["delivery"]["phase"], "queued");
    assert_eq!(messages(&b.snapshot()).len(), 3);
    a.kill();
    a.launch();
    let reopened = a.snapshot();
    assert_eq!(messages(&reopened).len(), 4);
    assert_eq!(messages(&reopened).last().unwrap()["id"], queued["id"]);
    assert_eq!(
        messages(&reopened).last().unwrap()["delivery"]["phase"],
        "queued"
    );
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 1);
    survivor.launch();
    reservations(&a, 1);
    reservations(&b, 1);
    let received = b.wait(|v| messages(v).iter().any(|m| m["id"] == queued["id"]));
    assert_eq!(messages(&received).len(), 4);
    assert_eq!(
        messages(&received).last().unwrap()["text"],
        "Durable after all relays disappear"
    );
    let delivered = a.wait(|v| {
        messages(v)
            .iter()
            .any(|m| m["id"] == queued["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert_eq!(messages(&delivered).len(), 4);
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    // Only after autonomous recovery do we retry the API operation.
    assert_eq!(
        a.send(group, "Durable after all relays disappear", "relay-offline")["id"],
        queued["id"]
    );
    assert_eq!(messages(&b.snapshot()).len(), 4);
    assert_eq!(peer_relay(&a, &b.peer), survivor.peer);
    // A relay is a transport intermediary, never an application conversation participant.
    assert!(
        survivor.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for relay in [&r1, &r2] {
        for filename in ["stderr.log", "stdout.log"] {
            let log = fs::read_to_string(relay.root.path().join(filename)).unwrap();
            for plaintext in [
                "Private relay-only hello",
                "Private relay-only reply",
                "Durable after all relays disappear",
                KEY,
                TOKEN,
            ] {
                assert!(
                    !log.contains(plaintext),
                    "relay leaked application plaintext or bootstrap"
                );
            }
        }
    }
}

#[test]
fn relay_capacity_denies_extra_reservation_without_evicting_live_conversations() {
    let relay = server(2);
    let a = client(&[&relay]);
    let mut b = client(&[&relay]);
    reservations(&a, 1);
    reservations(&b, 1);
    let group = connect(&a, &b, None);
    assert_exchange(
        &a,
        &b,
        &group,
        "Existing stream before capacity pressure",
        "capacity-before",
        1,
    );
    let c = client(&[&relay]);
    let bounded = info_until(&relay, |v| {
        v["relayServer"]["deniedReservations"]
            .as_u64()
            .is_some_and(|n| n > 0)
    });
    assert_eq!(bounded["relayServer"]["activeReservations"], 2);
    assert_eq!(c.call("node_info", json!({}))["relayRoutes"], json!([]));
    assert_exchange(
        &a,
        &b,
        &group,
        "Existing stream under capacity pressure",
        "capacity-after",
        2,
    );
    assert_eq!(peer_relay(&a, &b.peer), relay.peer);
    b.kill();
    // Capacity is reclaimed on disconnect; the previously denied client retries autonomously.
    reservations(&c, 1);
    let restored = info_until(&relay, |v| v["relayServer"]["activeReservations"] == 2);
    assert_eq!(restored["relayServer"]["capacity"], 2);
    c.profile("Carol");
    let invite = c.call("create_invitation", json!({}));
    let conversation = a.call("add_contact", json!({"name":"Carol","invitation":invite}));
    let id = conversation["id"].as_str().unwrap();
    c.wait(|v| v["conversations"].as_array().unwrap().len() == 1);
    let sent = a.send(
        id,
        "New participant after capacity returns",
        "capacity-recovered",
    );
    let received = c.wait(|v| messages(v).iter().any(|m| m["id"] == sent["id"]));
    assert_eq!(messages(&received).len(), 1);
    assert_eq!(
        messages(&received)[0]["text"],
        "New participant after capacity returns"
    );
    a.wait(|v| {
        v["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["id"] == id)
            .any(|c| {
                c["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|m| m["id"] == sent["id"] && m["delivery"]["phase"] == "delivered")
            })
    });
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
}

#[test]
fn relay_service_requires_opt_in_and_waiting_client_recovers_after_operator_enables_it() {
    let mut ordinary = Node::start(&[TCP]);
    let waiting = client(&[&ordinary]);
    let rejected = info_until(&waiting, |v| {
        v["failedReservations"].as_u64().is_some_and(|n| n > 0)
    });
    assert_eq!(rejected["relayRoutes"], json!([]));
    assert_eq!(
        ordinary.call("node_info", json!({}))["relayServer"]["enabled"],
        false
    );
    ordinary.kill();
    ordinary.arguments.push("--relay-server".into());
    ordinary.launch();
    reservations(&waiting, 1);
    let peer = client(&[&ordinary]);
    reservations(&peer, 1);
    let group = connect(&waiting, &peer, None);
    assert_exchange(
        &waiting,
        &peer,
        &group,
        "Operator enabled transport service",
        "relay-opt-in",
        1,
    );
    assert_eq!(peer_relay(&waiting, &peer.peer), ordinary.peer);
}

#[test]
fn relay_route_validation_rejects_malformed_or_mixed_destination_before_mutation() {
    let a = Node::start(&[TCP]);
    a.profile("Alice");
    let root = TempDir::new().unwrap();
    let mut bob = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(root.path().join("bob.db"), &[54; 32]).unwrap(),
        agentic_node::NETWORK_DOMAIN,
    )
    .unwrap();
    bob.create_profile("Bob").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let relay = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let dest = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let prefix = format!("/ip4/127.0.0.1/tcp/4100/p2p/{relay}/p2p-circuit");
    let valid = format!("{prefix}/p2p/{dest}");
    let before = a.snapshot();
    for addresses in [
        vec![format!("{prefix}")],
        vec![format!("/ip4/127.0.0.1/tcp/4100/p2p-circuit/p2p/{dest}")],
        vec![format!("{valid}/p2p-circuit/p2p/{dest}")],
        vec![format!(
            "/ip4/0.0.0.0/tcp/4100/p2p/{relay}/p2p-circuit/p2p/{dest}"
        )],
        vec![valid.clone(), format!("{prefix}/p2p/{relay}")],
    ] {
        let invitation = bob.create_invitation(now, addresses).unwrap();
        let result = rpc(
            &a.socket(),
            TOKEN,
            "add_contact",
            json!({"name":"Bob","invitation":invitation}),
        )
        .unwrap();
        assert!(
            result.get("error").is_some(),
            "invalid signed relay route accepted: {result}"
        );
        assert_eq!(a.snapshot(), before);
        assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    }
    // Control: the same signed invitation shape with a valid one-hop route is accepted.
    let invitation = bob.create_invitation(now, vec![valid]).unwrap();
    a.call("add_contact", json!({"name":"Bob","invitation":invitation}));
    assert_eq!(a.snapshot()["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 1);
}
