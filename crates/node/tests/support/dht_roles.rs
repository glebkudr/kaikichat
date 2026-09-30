use super::*;
use futures::StreamExt;
use libp2p::{StreamProtocol, SwarmBuilder, identity, kad, noise, swarm::SwarmEvent, yamux};

const TRANSPORTS: &[&str] = &["/ip4/127.0.0.1/tcp/0", "/ip4/127.0.0.1/udp/0/quic-v1"];

/// Independent stock Kad client; success counts come from actual decoded network replies.
/// A refused protocol must still have reached the authenticated target transport.
fn probe(node: &Node, index: usize, serving: bool) {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let key = identity::Keypair::generate_ed25519();
        let own = key.public().to_peer_id();
        let peer: libp2p::PeerId = node.peer.parse().unwrap();
        let address: libp2p::Multiaddr = format!("{}/p2p/{peer}", node.listeners[index])
            .parse()
            .unwrap();
        let mut config = kad::Config::new(StreamProtocol::new("/agentic-internet/kad/1"));
        config
            .set_query_timeout(Duration::from_secs(4))
            .set_periodic_bootstrap_interval(None)
            .set_kbucket_inserts(kad::BucketInserts::Manual);
        let mut routing =
            kad::Behaviour::with_config(own, kad::store::MemoryStore::new(own), config);
        routing.set_mode(Some(kad::Mode::Client));
        routing.add_address(&peer, address.clone());
        let mut swarm = SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(
                Default::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .unwrap()
            .with_quic()
            .with_behaviour(|_| routing)
            .unwrap()
            .build();
        swarm.dial(address).unwrap();
        tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
                    swarm.select_next_some().await
                {
                    assert_eq!(peer_id, peer);
                    break;
                }
            }
        })
        .await
        .unwrap();
        let target = identity::Keypair::generate_ed25519().public().to_peer_id();
        let id = swarm.behaviour_mut().get_closest_peers(target);
        let successes = tokio::time::timeout(Duration::from_secs(7), async {
            loop {
                if let SwarmEvent::Behaviour(kad::Event::OutboundQueryProgressed {
                    id: found,
                    stats,
                    step,
                    ..
                }) = swarm.select_next_some().await
                    && found == id
                    && step.last
                {
                    assert!(
                        stats.num_requests() > 0,
                        "a local empty-table result is not a network probe"
                    );
                    break stats.num_successes();
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(
            successes > 0,
            serving,
            "DHT replies on {}: {successes}",
            node.listeners[index]
        );
    });
}
fn both(node: &Node, serving: bool) {
    for index in 0..2 {
        probe(node, index, serving);
    }
    assert_eq!(
        node.call("node_info", json!({}))["connectionCapacity"]["ordinaryLimit"],
        if serving { 128 } else { 64 },
        "only the durably owner-enabled and policy-allowed DHT server gains headroom"
    );
}
fn settings(node: &Node) -> Value {
    node.call("network_settings", json!({}))
}
fn request(node: &Node, serving: bool) -> Value {
    let saved = settings(node);
    let mut preferences = saved["preferences"].clone();
    preferences["dhtServer"] = json!(serving);
    json!({"expectedRevision":saved["revision"],"preferences":preferences})
}
fn mode(node: &Node, expected: &str) {
    for routing in [
        node.call("node_info", json!({}))["routing"].clone(),
        settings(node)["status"]["routing"].clone(),
    ] {
        assert_eq!(routing["mode"], expected);
        assert_eq!(routing["enabled"], expected != "disabled");
        assert_eq!(routing["blockedByPolicy"], expected == "disabled");
    }
}

#[test]
fn dht_roles_confirmed_relay_address_never_automatically_promotes_a_client() {
    let node = Node::start_with_args(TRANSPORTS, vec!["--relay-server".into()]);
    // A real successful reservation returns the server's confirmed external addresses.
    // This exercises Runtime::NewListenAddr -> add_external_address, not an injected event.
    let client = relay::client(&[&node]);
    let confirmed = relay::reservations(&client, 1);
    assert!(
        confirmed["relayRoutes"][0]
            .as_str()
            .unwrap()
            .contains(&format!("/p2p/{}/p2p-circuit/", node.peer))
    );
    relay::info_until(&node, |v| v["relayServer"]["activeReservations"] == 1);
    both(&node, false);
    mode(&node, "client");
    for serving in [true, false] {
        node.call("configure_network", request(&node, serving));
        // Reconfiguration creates a fresh swarm; verify its confirmation path too.
        let fresh = relay::client(&[&node]);
        let routes = relay::reservations(&fresh, 1);
        assert!(
            routes["relayRoutes"][0]
                .as_str()
                .unwrap()
                .contains(&format!("/p2p/{}/p2p-circuit/", node.peer))
        );
        both(&node, serving);
        mode(&node, if serving { "server" } else { "client" });
    }
}

#[test]
fn dht_roles_default_client_and_owner_toggle_changes_real_tcp_quic_service_only_after_commit() {
    let mut node = Node::start(TRANSPORTS);
    // Behavioral RED precedes all new diagnostic/schema assertions.
    both(&node, false);
    mode(&node, "client");
    assert_eq!(settings(&node)["preferences"]["dhtServer"], false);
    node.profile("DHT owner");
    let original = node.snapshot();
    let enable = request(&node, true);
    let denied = rpc(
        &node.socket(),
        &"00".repeat(32),
        "configure_network",
        enable.clone(),
    )
    .unwrap();
    assert_eq!(denied["error"]["code"], "unauthorized");
    assert_eq!(settings(&node)["revision"], 0);
    both(&node, false);
    assert_eq!(
        node.call("configure_network", enable.clone())["revision"],
        1
    );
    both(&node, true);
    mode(&node, "server");
    assert_eq!(node.call("configure_network", enable)["revision"], 1);
    node.kill();
    node.launch();
    both(&node, true);
    mode(&node, "server");
    assert_eq!(node.snapshot()["identity"], original["identity"]);
    assert_eq!(node.snapshot()["conversations"], original["conversations"]);
    let disable = request(&node, false);
    let db = rusqlite::Connection::open(node.root.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{KEY}'")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_dht BEFORE UPDATE ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let failed = rpc(&node.socket(), TOKEN, "configure_network", disable.clone()).unwrap();
    assert_eq!(failed["error"]["code"], "unavailable");
    assert_eq!(settings(&node)["revision"], 1);
    both(&node, true);
    mode(&node, "server");
    db.execute_batch("DROP TRIGGER fail_dht;").unwrap();
    let mut stale = disable.clone();
    stale["expectedRevision"] = json!(0);
    assert_eq!(
        rpc(&node.socket(), TOKEN, "configure_network", stale).unwrap()["error"]["code"],
        "state_conflict"
    );
    both(&node, true);
    assert_eq!(
        node.call("configure_network", disable.clone())["revision"],
        2
    );
    both(&node, false);
    mode(&node, "client");
    assert_eq!(node.call("configure_network", disable)["revision"], 2);
    node.kill();
    node.arguments = vec!["--dht-server".into()];
    node.launch();
    both(&node, false);
    mode(&node, "client");
    assert_eq!(node.snapshot()["identity"], original["identity"]);
    assert_eq!(node.snapshot()["conversations"], original["conversations"]);
}

#[test]
fn dht_roles_explicit_cli_server_remains_independent_and_relay_policy_suppresses_it() {
    let node = Node::start_with_args(TRANSPORTS, vec!["--dht-server".into()]);
    both(&node, true);
    mode(&node, "server");
    let provider = relay::server(4);
    let mut change = request(&node, true);
    change["preferences"]["relayOnly"] = json!(true);
    change["preferences"]["relays"] =
        json!([format!("{}/p2p/{}", provider.listeners[0], provider.peer)]);
    node.call("configure_network", change);
    relay::reservations(&node, 1);
    both(&node, false);
    mode(&node, "disabled");
    let state = settings(&node);
    assert_eq!(state["preferences"]["dhtServer"], true);
    assert_eq!(state["status"]["routing"]["blockedByPolicy"], true);
    let target = identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    assert_eq!(
        rpc(
            &node.socket(),
            TOKEN,
            "lookup_peer",
            json!({"peerId":target})
        )
        .unwrap()["error"]["code"],
        "policy_blocked"
    );
    let mut restore = request(&node, true);
    restore["preferences"]["relayOnly"] = json!(false);
    node.call("configure_network", restore);
    both(&node, true);
    mode(&node, "server");
    assert_eq!(
        settings(&node)["status"]["routing"]["blockedByPolicy"],
        false
    );
}
