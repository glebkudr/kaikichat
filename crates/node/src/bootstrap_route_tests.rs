//! A changed signed route reaches already verified bootstrap peers promptly.
//! A network (re)configuration clears the listener set and schedules the
//! explicit bootstrap peers at once, so the first exchange can carry a record
//! with no route. Waiting for the ordinary 30 s refresh then leaves the peer
//! unable to route to this node (A04/H10: DHT seeds answered exact lookups for
//! restarted custodians only ~30 s later). Real swarms and exchanges run
//! unchanged; only the installed clock advances.
#![allow(clippy::unwrap_used)]
use super::*;
use futures::FutureExt;
use tempfile::TempDir;

fn drain(node: &mut Runtime) -> bool {
    let mut progressed = false;
    while let Some(event) = node.swarm.select_next_some().now_or_never() {
        node.event(event);
        progressed = true;
    }
    progressed
}

/// Quarter-second managed steps of ordinary maintenance for both nodes; real
/// localhost IO gets a short real-time grace inside each step.
async fn run(nodes: [&mut Runtime; 2], clock: &clock::Virtual, seconds: u64) {
    let [a, b] = nodes;
    for _ in 0..seconds * 4 {
        a.pump();
        b.pump();
        for _ in 0..5 {
            let progressed = drain(a) | drain(b);
            if !progressed {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
        clock.advance(Duration::from_millis(250));
    }
}

fn routable(node: &Runtime, peer: PeerId) -> bool {
    node.routing_info()["knownPeers"]
        .as_array()
        .is_some_and(|peers| peers.iter().any(|p| p == &json!(peer.to_string())))
}

/// Inbound exchanges the peer has answered in its current admission window.
fn answered(node: &Runtime) -> u32 {
    node.discovery.admission.snapshot().0
}

#[tokio::test(flavor = "current_thread")]
async fn a_route_bound_after_the_first_exchange_reaches_the_bootstrap_peer_promptly() {
    let node_dir = TempDir::new().unwrap();
    let seed_dir = TempDir::new().unwrap();
    let mut node = test_support::runtime(node_dir.path());
    let mut seed = test_support::runtime(seed_dir.path());
    node.core.create_profile("Node").unwrap();
    seed.core.create_profile("Seed").unwrap();
    let clock = clock::Virtual::install(now().unwrap());
    test_support::connect(&mut node, &mut seed, false).await;
    let node_peer = *node.swarm.local_peer_id();
    let seed_peer = *seed.swarm.local_peer_id();
    let seed_route = seed
        .swarm
        .listeners()
        .next()
        .unwrap()
        .clone()
        .with(Protocol::P2p(seed_peer))
        .to_string();
    // The node was just (re)configured with the seed as its bootstrap peer
    // and has not yet learned its own listener: its first record has no route.
    // A second explicit hint never answers; it stays in failure backoff.
    let unreachable = identity::Keypair::generate_ed25519().public().to_peer_id();
    let dead_route = format!("/ip4/127.0.0.1/tcp/1/p2p/{unreachable}");
    node.discovery = Discovery::new(&[seed_route, dead_route], node_peer).unwrap();
    assert!(node.advertised().is_empty());
    run([&mut node, &mut seed], &clock, 2).await;
    assert_eq!(answered(&seed), 1, "the first exchange ran");
    assert!(
        !routable(&seed, node_peer),
        "a record without a route gives the seed nothing to answer lookups with"
    );
    // Without a route change the ordinary refresh interval stands: no extra
    // exchange within 25 s.
    run([&mut node, &mut seed], &clock, 25).await;
    assert_eq!(answered(&seed), 1);
    assert!(!routable(&seed, node_peer));
    // The node's listener binds. The verified seed must learn the new signed
    // route within a couple of seconds, not at the next 30 s refresh. The
    // listener is inserted directly, as the NewListenAddr handler records it:
    // the contract is about any change of the advertised route.
    let failed = node.discovery.failed;
    node.listeners
        .insert(format!("/ip4/127.0.0.1/tcp/4711/p2p/{node_peer}"));
    run([&mut node, &mut seed], &clock, 2).await;
    assert!(
        routable(&seed, node_peer),
        "a changed route must reach the verified bootstrap peer promptly"
    );
    assert_eq!(answered(&seed), 2, "exactly one announcing exchange");
    assert_eq!(
        node.discovery.failed, failed,
        "an unverified peer in failure backoff is not dialed early"
    );
}

/// A peer moved to new addresses while the node's cache still holds its old
/// signed record, listing eight routes nobody answers at any more — the
/// testnet's holders on 2026-09-29, after they stopped listening on the
/// host's docker bridges: a holder restarted with its peers' old records
/// cached never reached one of them again. The route given with
/// `--bootstrap` (or by the network preset) is still dialed, and the node
/// joins.
#[tokio::test(flavor = "current_thread")]
async fn a_cached_record_of_eight_dead_routes_does_not_hide_the_bootstrap_route() {
    let node_dir = TempDir::new().unwrap();
    let seed_dir = TempDir::new().unwrap();
    let mut node = test_support::runtime(node_dir.path());
    let mut seed = test_support::runtime(seed_dir.path());
    node.core.create_profile("Node").unwrap();
    seed.core.create_profile("Seed").unwrap();
    let clock = clock::Virtual::install(now().unwrap());
    let node_peer = *node.swarm.local_peer_id();
    let seed_peer = *seed.swarm.local_peer_id();
    let time = now().unwrap();
    let dead: Vec<String> = (1..=8)
        .map(|port| format!("/ip4/127.0.0.1/tcp/{port}/p2p/{seed_peer}"))
        .collect();
    let old = seed
        .core
        .publish_node_record(&seed_peer.to_string(), dead, time)
        .unwrap();
    assert!(
        node.core
            .remember_node_record(&old, &seed_peer.to_string(), time)
            .unwrap()
    );
    seed.swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let route = loop {
        let event = seed.swarm.select_next_some().await;
        let address = match &event {
            SwarmEvent::NewListenAddr { address, .. } => Some(address.clone()),
            _ => None,
        };
        seed.event(event);
        if let Some(address) = address {
            break address.with(Protocol::P2p(seed_peer)).to_string();
        }
    };
    node.discovery = Discovery::new(&[route], node_peer).unwrap();
    run([&mut node, &mut seed], &clock, 10).await;
    assert_eq!(
        node.bootstrap_info()["state"],
        "connected",
        "{}",
        node.bootstrap_info()
    );
    assert!(answered(&seed) >= 1, "the exchange reached the seed");
    // The cached record was merged, not ignored.
    assert_eq!(node.bootstrap_info()["cachedHints"], 1);
}
