//! Real admission and retirement through the production owner-mode network builder.
//! The 64-daemon TCP/QUIC gate covers actual discovery under this server workload.
use super::*;

fn network(server: bool, relay_only: bool) -> Swarm<Network> {
    let directory = tempfile::TempDir::new().unwrap();
    let template = test_support::runtime(directory.path());
    build_network(
        template.transport_key.clone(),
        relay_only,
        false,
        server,
        &template.relay_server,
        &template.nat,
        &template.access_gate,
    )
    .unwrap()
}

#[tokio::test]
async fn owner_selected_dht_server_admits_clients_and_seed_links_with_a_finite_ordinary_bound() {
    for (server, relay_only, capacity) in [(false, false, 64), (true, true, 64), (true, false, 128)]
    {
        let mut swarm = network(server, relay_only);
        let limits = &mut swarm.behaviour_mut().limits;
        let ordinary = (0..capacity)
            .map(|i| {
                let p = peer();
                (p, connect(limits, p, i).unwrap())
            })
            .collect::<Vec<_>>();
        assert!(connect(limits, peer(), capacity).is_err());
        assert_eq!(limits.info()["ordinaryEstablished"], capacity);
        assert_eq!(limits.info()["ordinaryLimit"], capacity);
        assert_eq!(limits.info()["reservedEstablished"], 0);
        assert!(
            limits.info()["reservedPeers"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(retire(limits).is_empty());
        let (p, id) = ordinary[0];
        closed(limits, p, id);
        connect(limits, peer(), capacity + 1).unwrap();
        assert_eq!(limits.info()["ordinaryEstablished"], capacity);
        assert!(connect(limits, peer(), capacity + 2).is_err());
    }
}

#[tokio::test]
async fn dht_server_headroom_preserves_selected_absolute_and_per_peer_limits_and_revocation() {
    let mut swarm = network(true, false);
    let limits = &mut swarm.behaviour_mut().limits;
    let ordinary = (0..128)
        .map(|i| {
            let p = peer();
            (p, connect(limits, p, i).unwrap())
        })
        .collect::<Vec<_>>();
    let permit = Arc::new(AtomicBool::new(true));
    let selected = (0..64).map(|_| peer()).collect::<Vec<_>>();
    limits
        .replace(
            selected
                .iter()
                .map(|p| grant(*p, &permit, now().unwrap()))
                .collect(),
        )
        .unwrap();
    let mut sockets = Vec::new();
    for (i, p) in selected[..32].iter().enumerate() {
        sockets.push((*p, connect(limits, *p, 200 + 2 * i).unwrap()));
        sockets.push((
            *p,
            connect_with_sibling(limits, *p, 201 + 2 * i, 1).unwrap(),
        ));
        assert!(
            connect(limits, *p, 1000 + i).is_err(),
            "server mode bypassed the per-peer limit"
        );
    }
    assert_eq!(limits.info()["established"], 192);
    assert_eq!(limits.info()["reservedEstablished"], 64);
    assert!(
        connect(limits, selected[32], 2000).is_err(),
        "server mode bypassed the absolute bound"
    );
    assert!(retire(limits).is_empty());
    permit.store(false, Ordering::Release);
    let retired = retire(limits);
    assert_eq!(retired.len(), 64);
    assert!(retired.iter().all(|socket| sockets.contains(socket)));
    assert!(retired.iter().all(|socket| !ordinary.contains(socket)));
    for (p, id) in retired {
        closed(limits, p, id);
    }
    assert_eq!(limits.info()["ordinaryEstablished"], 128);
    assert_eq!(limits.info()["reservedEstablished"], 0);
    assert!(connect(limits, peer(), 3000).is_err());
}
