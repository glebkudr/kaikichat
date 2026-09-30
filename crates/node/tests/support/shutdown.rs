use super::*;

fn connected(node: &Node, peer: &str) -> bool {
    node.call("node_info", json!({}))["peerConnections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["peerId"] == peer)
}
fn wait_bounded(description: &str, seconds: u64, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let done = ready();
        assert!(Instant::now() < deadline, "{description}");
        if done {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
fn conversation_messages<'a>(snapshot: &'a Value, group: &str) -> &'a Vec<Value> {
    snapshot["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == group)
        .unwrap()["messages"]
        .as_array()
        .unwrap()
}
fn stop_cleanly(node: &mut Node) {
    let pid = node.child.as_ref().unwrap().id();
    assert!(
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .unwrap()
            .success()
    );
    wait_bounded("SIGTERM did not exit within five seconds", 5, || {
        if let Some(status) = node.child.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success(), "graceful shutdown failed: {status}");
            true
        } else {
            false
        }
    });
    node.child = None;
}
fn exchange_bounded(sender: &Node, receiver: &Node, group: &str, operation: &str) -> String {
    let text = format!("Durable message {operation}");
    let sent = sender.send(group, &text, operation);
    wait_bounded(
        "MLS message/receipt did not recover after graceful restart",
        8,
        || {
            conversation_messages(&sender.snapshot(), group)
                .iter()
                .any(|m| m["id"] == sent["id"] && m["delivery"]["phase"] == "delivered")
        },
    );
    let snapshot = receiver.snapshot();
    let received = conversation_messages(&snapshot, group)
        .iter()
        .filter(|m| m["id"] == sent["id"])
        .collect::<Vec<_>>();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0]["text"], text);
    assert_eq!(received[0]["own"], false);
    sent["id"].as_str().unwrap().to_owned()
}
fn graceful_restart(transport: &str) {
    let tcp = "/ip4/127.0.0.1/tcp/0";
    let quic = "/ip4/127.0.0.1/udp/0/quic-v1";
    let endpoints = [tcp, quic];
    let mut server = Node::start(&endpoints[..if transport == "tcp" { 1 } else { 2 }]);
    let clients = [
        Node::start(&[tcp]),
        Node::start(&[if transport == "tcp" { tcp } else { quic }]),
        Node::start(&[if transport == "tcp" { tcp } else { quic }]),
    ];
    let groups = clients
        .iter()
        .enumerate()
        .map(|(i, client)| {
            let tcp_route = transport == "tcp" || i == 0;
            let address = server
                .listeners
                .iter()
                .find(|a| a.contains("/tcp/") == tcp_route)
                .unwrap();
            connect(
                client,
                &server,
                Some(vec![format!("{address}/p2p/{}", server.peer)]),
            )
        })
        .collect::<Vec<_>>();
    let listeners = server.listeners.clone();
    let mut expected = vec![Vec::<String>::new(); clients.len()];
    for cycle in 0..3 {
        for (i, client) in clients.iter().enumerate() {
            expected[i].push(exchange_bounded(
                client,
                &server,
                &groups[i],
                &format!("before-{cycle}-{i}"),
            ));
            assert!(connected(client, &server.peer));
            assert!(connected(&server, &client.peer));
            let info = client.call("node_info", json!({}));
            assert!(info["peerConnections"].as_array().unwrap().iter().all(|c| {
                let route = c["remoteAddress"].as_str().unwrap();
                !c["relayed"].as_bool().unwrap()
                    && if transport == "quic" && i > 0 {
                        route.contains("/quic-v1")
                    } else {
                        route.contains("/tcp/")
                    }
            }));
        }
        stop_cleanly(&mut server); // No SIGKILL fallback may turn this assertion green.
        wait_bounded(
            "peer retained a stale connection after successful graceful exit",
            2,
            || {
                clients
                    .iter()
                    .all(|client| !connected(client, &server.peer))
            },
        );
        server.launch(); // Existing helper asserts identical persisted transport identity.
        assert_eq!(
            server.listeners, listeners,
            "restart did not reuse the same endpoint"
        );
        for (i, client) in clients.iter().enumerate() {
            expected[i].push(exchange_bounded(
                client,
                &server,
                &groups[i],
                &format!("after-{cycle}-{i}"),
            ));
            assert!(connected(client, &server.peer));
            assert!(connected(&server, &client.peer));
            for node in [client, &server] {
                let snapshot = node.snapshot();
                let mut actual = conversation_messages(&snapshot, &groups[i])
                    .iter()
                    .map(|m| m["id"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>();
                actual.sort();
                let mut want = expected[i].clone();
                want.sort();
                assert_eq!(
                    actual, want,
                    "restart lost or duplicated historical messages"
                );
            }
        }
    }
}
#[test]
fn graceful_tcp_shutdown_retires_peer_connection_and_restarts_same_mls_identity() {
    graceful_restart("tcp");
}
#[test]
fn graceful_quic_shutdown_retires_peer_connection_and_restarts_same_mls_identity() {
    graceful_restart("quic");
}

fn bootstrap_restart(endpoint: &str) {
    let mut provider = Node::start(&[endpoint]);
    let route = format!("{}/p2p/{}", provider.listeners[0], provider.peer);
    // Model a client without a verified public address. Its unavailable independent
    // AutoNAT service cannot dial the bootstrap provider and mask reconnection.
    let mut nat_service = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
    let nat_route = format!("{}/p2p/{}", nat_service.listeners[0], nat_service.peer);
    nat_service.kill();
    let client = Node::start_with_args(
        &[endpoint],
        vec![
            "--bootstrap".into(),
            route,
            "--autonat-peer".into(),
            nat_route,
            "--autonat-probe-seconds".into(),
            "900".into(),
        ],
    );
    let verified = |node: &Node, peer: &str| {
        let info = node.call("node_info", json!({}));
        assert!(info["bootstrap"]["inFlight"].as_u64().unwrap() <= 4);
        info["bootstrap"]["verifiedPeers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["peerId"] == peer)
            && connected(node, peer)
    };
    wait_bounded("initial bootstrap did not authenticate", 8, || {
        verified(&client, &provider.peer) && verified(&provider, &client.peer)
    });
    let authenticated_root = |peer: &str| {
        let info = client.call("node_info", json!({}));
        info["bootstrap"]["verifiedPeers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peerId"] == peer)
            .unwrap()["rootId"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let provider_root = authenticated_root(&provider.peer);
    assert_eq!(
        client.call("node_info", json!({}))["advertisedAddresses"],
        json!([])
    );
    let listeners = provider.listeners.clone();
    for _ in 0..2 {
        stop_cleanly(&mut provider);
        wait_bounded("closed bootstrap peer was retained", 2, || {
            !connected(&client, &provider.peer) && !verified(&client, &provider.peer)
        });
        assert_eq!(
            client.call("node_info", json!({}))["bootstrap"]["state"],
            "bootstrap-needed"
        );
        provider.launch();
        assert_eq!(provider.listeners, listeners);
        // No invitation, message, explicit reconnect or settings refresh may trigger this.
        wait_bounded(
            "bootstrap did not reconnect within eight seconds",
            8,
            || verified(&client, &provider.peer) && verified(&provider, &client.peer),
        );
        assert_eq!(authenticated_root(&provider.peer), provider_root);
        for node in [&client, &provider] {
            assert!(
                node.snapshot()["conversations"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let info = node.call("node_info", json!({}));
            assert_eq!(info["bootstrap"]["state"], "connected");
            assert!(info["peerConnections"].as_array().unwrap().iter().all(|p| {
                !p["relayed"].as_bool().unwrap()
                    && p["remoteAddress"].as_str().unwrap().contains(
                        if endpoint.contains("/tcp/") {
                            "/tcp/"
                        } else {
                            "/quic-v1"
                        },
                    )
            }));
        }
        assert_eq!(
            client.call("node_info", json!({}))["advertisedAddresses"],
            json!([])
        );
    }
    let group = connect(&client, &provider, None);
    exchange_bounded(&client, &provider, &group, "bootstrap-restored-message");
}

#[test]
fn bootstrap_tcp_reconnects_after_verified_provider_restart_without_message_trigger() {
    bootstrap_restart("/ip4/127.0.0.1/tcp/0");
}

#[test]
fn bootstrap_quic_reconnects_after_verified_provider_restart_without_message_trigger() {
    bootstrap_restart("/ip4/127.0.0.1/udp/0/quic-v1");
}
