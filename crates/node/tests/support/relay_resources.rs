use super::{relay::*, *};

fn short_server(reservation_seconds: u64, circuit_seconds: u64) -> Node {
    Node::start_with_args(
        &["/ip4/127.0.0.1/tcp/0"],
        vec![
            "--relay-server".into(),
            "--relay-capacity".into(),
            "2".into(),
            "--relay-reservation-seconds".into(),
            reservation_seconds.to_string(),
            "--relay-circuit-seconds".into(),
            circuit_seconds.to_string(),
        ],
    )
}
fn renewal_count(info: &Value, peer: &str) -> u64 {
    info["relayServer"]["reservationPeers"]
        .as_array()
        .and_then(|peers| peers.iter().find(|p| p["peerId"] == peer))
        .and_then(|p| p["renewals"].as_u64())
        .unwrap_or(0)
}

#[test]
fn relay_full_capacity_renews_each_live_reservation_without_admitting_waiting_peer() {
    let relay = short_server(4, 120);
    let a = client(&[&relay]);
    let b = client(&[&relay]);
    reservations(&a, 1);
    reservations(&b, 1);
    let group = connect(&a, &b, None);
    assert_exchange(
        &a,
        &b,
        &group,
        "Conversation before reservation renewal",
        "renew-before",
        1,
    );
    let waiting = client(&[&relay]);
    info_until(&relay, |v| {
        v["relayServer"]["deniedReservations"]
            .as_u64()
            .is_some_and(|n| n > 0)
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let info = relay.call("node_info", json!({}));
        assert_eq!(info["relayServer"]["activeReservations"], 2);
        for node in [&a, &b] {
            let state = node.call("node_info", json!({}));
            assert_eq!(
                state["relayRoutes"].as_array().unwrap().len(),
                1,
                "legitimate reservation was dropped: {state}"
            );
            assert_eq!(
                state["failedReservations"], 0,
                "renewal must not fail and reacquire a slot"
            );
        }
        assert_eq!(
            waiting.call("node_info", json!({}))["relayRoutes"],
            json!([])
        );
        if renewal_count(&info, &a.peer) >= 2 && renewal_count(&info, &b.peer) >= 2 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "each original peer must renew twice: {info}"
        );
        thread::sleep(Duration::from_millis(40));
    }
    assert_exchange(
        &b,
        &a,
        &group,
        "Both peers survived repeated full-capacity renewal",
        "renew-after",
        2,
    );
}

#[test]
fn relay_expired_reservation_frees_capacity_while_original_tcp_connection_remains_open() {
    let relay = short_server(4, 120);
    let a = client(&[&relay]);
    let b = client(&[&relay]);
    reservations(&a, 1);
    reservations(&b, 1);
    let group = connect(&a, &b, None);
    assert_exchange(
        &a,
        &b,
        &group,
        "Before paused peer misses renewal",
        "expiry-before",
        1,
    );
    // SIGSTOP prevents the actual client from renewing, while its TCP socket stays open.
    // Node::Drop always SIGKILLs and reaps even if an assertion fails before SIGCONT.
    let pid = b.child.as_ref().unwrap().id().to_string();
    assert!(
        Command::new("/bin/kill")
            .args(["-STOP", &pid])
            .status()
            .unwrap()
            .success()
    );
    let waiting = client(&[&relay]);
    info_until(&relay, |v| {
        v["relayServer"]["deniedReservations"]
            .as_u64()
            .is_some_and(|n| n > 0)
    });
    reservations(&waiting, 1);
    let expired = relay.call("node_info", json!({}));
    assert!(
        expired["peerConnections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["peerId"] == b.peer),
        "reservation expiry was replaced by connection closure: {expired}"
    );
    assert!(
        expired["relayServer"]["expiredReservations"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(expired["relayServer"]["activeReservations"], 2);
    assert!(
        expired["relayServer"]["reservationPeers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["peerId"] != b.peer)
    );
    assert_eq!(a.call("node_info", json!({}))["failedReservations"], 0);
    waiting.profile("Carol");
    let invitation = waiting.call("create_invitation", json!({}));
    let contact = a.call(
        "add_contact",
        json!({"name":"Carol","invitation":invitation}),
    );
    let group = contact["id"].as_str().unwrap();
    waiting.wait(|v| v["conversations"].as_array().unwrap().len() == 1);
    let sent = a.send(
        group,
        "Reclaimed expired reservation works",
        "expiry-reclaimed",
    );
    let received = waiting.wait(|v| messages(v).iter().any(|m| m["id"] == sent["id"]));
    assert_eq!(messages(&received).len(), 1);
    a.wait(|v| {
        v["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == group)
            .unwrap()["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == sent["id"] && m["delivery"]["phase"] == "delivered")
    });
    assert!(
        Command::new("/bin/kill")
            .args(["-CONT", &pid])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn relay_circuit_duration_expires_and_next_message_reconnects_without_duplicates() {
    let relay = short_server(300, 2);
    let a = client(&[&relay]);
    let b = client(&[&relay]);
    reservations(&a, 1);
    reservations(&b, 1);
    let group = connect(&a, &b, None);
    assert_exchange(
        &a,
        &b,
        &group,
        "Before finite circuit duration",
        "duration-before",
        1,
    );
    let initial = relay.call("node_info", json!({}))["relayServer"]["acceptedCircuits"]
        .as_u64()
        .unwrap();
    let expired = info_until(&relay, |v| {
        v["relayServer"]["timedOutCircuits"]
            .as_u64()
            .is_some_and(|n| n > 0)
    });
    assert_eq!(expired["relayServer"]["activeReservations"], 2);
    assert_exchange(
        &a,
        &b,
        &group,
        "After finite circuit duration",
        "duration-after",
        2,
    );
    assert!(
        relay.call("node_info", json!({}))["relayServer"]["acceptedCircuits"]
            .as_u64()
            .unwrap()
            > initial
    );
}

#[test]
fn relay_circuit_byte_limit_rolls_over_real_ciphertext_and_preserves_every_message() {
    let relay = server(2);
    let a = client(&[&relay]);
    let b = client(&[&relay]);
    reservations(&a, 1);
    reservations(&b, 1);
    let group = connect(&a, &b, None);
    let initial = relay.call("node_info", json!({}))["relayServer"]["acceptedCircuits"]
        .as_u64()
        .unwrap();
    let mut ids = std::collections::HashSet::new();
    for index in 0..28 {
        let text = format!("{index:02}:{}", "🦀".repeat(11980));
        assert_exchange(
            &a,
            &b,
            &group,
            &text,
            &format!("byte-cap-{index}"),
            index + 1,
        );
        let snapshot = a.snapshot();
        assert!(
            ids.insert(
                messages(&snapshot).last().unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            )
        );
    }
    let usage = relay.call("node_info", json!({}));
    assert!(
        usage["relayServer"]["byteLimitedCircuits"]
            .as_u64()
            .unwrap()
            >= 1,
        "1MiB circuit cap did not terminate actual ciphertext forwarding: {usage}"
    );
    assert!(usage["relayServer"]["acceptedCircuits"].as_u64().unwrap() > initial);
    assert_eq!(messages(&a.snapshot()).len(), 28);
    assert_eq!(messages(&b.snapshot()).len(), 28);
    assert_eq!(a.call("node_info", json!({}))["pendingOutbox"], 0);
    assert!(
        relay.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
