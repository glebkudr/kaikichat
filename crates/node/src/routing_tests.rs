#![allow(clippy::unwrap_used)]
use super::routing_support::GuardedRouting;
use super::*;
use libp2p::{
    core::Endpoint,
    swarm::{ConnectionId, NetworkBehaviour},
};
fn peer() -> PeerId {
    identity::Keypair::generate_ed25519().public().to_peer_id()
}
fn address(peer: PeerId, port: u16) -> Multiaddr {
    format!("/ip4/127.0.0.1/tcp/{port}/p2p/{peer}")
        .parse()
        .unwrap()
}
#[tokio::test]
async fn routing_retains_a_partial_table_without_an_implicit_application_address_book() {
    let own = peer();
    let mut routing = GuardedRouting::new(own, false);
    let mut seen = vec![];
    let first = peer();
    routing.remember(first, vec![address(first, 2000)]);
    for _ in 0..2048 {
        let remote = peer();
        seen.push(remote);
        routing.remember(remote, (2000..2010).map(|p| address(remote, p)).collect());
    }
    let info = routing.info();
    assert_eq!(info["peers"], 128);
    assert!(info["addresses"].as_u64().unwrap() <= 512);
    for remote in seen {
        assert!(routing.addresses(remote).len() <= 4);
    }
    assert_eq!(routing.addresses(first), vec![address(first, 2000)]);
    // An unrelated application dial may not inherit DHT referrals or even routing-table hints.
    let extra = routing
        .handle_pending_outbound_connection(
            ConnectionId::new_unchecked(900),
            Some(first),
            &[],
            Endpoint::Dialer,
        )
        .unwrap();
    assert!(extra.is_empty());
    // Updating a retained peer replaces stale endpoints even at capacity.
    routing.remember(first, vec![address(first, 3000)]);
    assert_eq!(routing.addresses(first), vec![address(first, 3000)]);
}

#[tokio::test]
async fn routing_info_lists_known_peers_and_only_confirmed_kad_servers() {
    let own = peer();
    let mut routing = GuardedRouting::new(own, false);
    let known_server = peer();
    let identified_server = peer();
    let client = peer();
    let unidentified = peer();
    routing.remember(known_server, vec![address(known_server, 2000)]);
    routing.remember(client, vec![address(client, 2001)]);
    routing.remember(unidentified, vec![address(unidentified, 2002)]);
    routing.observe_protocols(
        known_server,
        &[StreamProtocol::new("/agentic-internet/kad/1")],
    );
    routing.observe_protocols(
        identified_server,
        &[StreamProtocol::new("/agentic-internet/kad/1")],
    );
    routing.observe_protocols(client, &[]);

    let info = routing.info();
    let known = info["knownPeers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|peer| peer.as_str().unwrap().to_owned())
        .collect::<HashSet<_>>();
    assert_eq!(
        known,
        HashSet::from([
            known_server.to_string(),
            client.to_string(),
            unidentified.to_string()
        ])
    );
    let serving = info["servingPeers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|peer| peer.as_str().unwrap().to_owned())
        .collect::<HashSet<_>>();
    assert_eq!(
        serving,
        HashSet::from([known_server.to_string(), identified_server.to_string()])
    );
}

#[tokio::test]
async fn routing_limits_concurrent_searches_and_rejects_unsupported_routes() {
    let own = peer();
    let mut routing = GuardedRouting::new(own, false);
    let a = peer();
    let b = peer();
    let c = peer();
    routing.remember(own, vec![address(own, 2000)]);
    routing.remember(
        a,
        vec![
            address(b, 2000),
            format!("/dns4/untrusted.invalid/tcp/2000/p2p/{a}")
                .parse()
                .unwrap(),
            format!("/ip4/0.0.0.0/tcp/2000/p2p/{a}").parse().unwrap(),
        ],
    );
    assert_eq!(routing.info()["peers"], 0);
    routing.remember(a, vec![address(a, 2000)]);
    assert!(routing.start(a).is_ok());
    assert!(
        routing.start(a).is_ok(),
        "exact active retry must reuse the search"
    );
    assert_eq!(routing.info()["inFlight"], 1);
    assert!(routing.start(b).is_ok());
    assert!(routing.start(c).is_err());
    assert!(routing.start(own).is_err());
    assert_eq!(routing.info()["inFlight"], 2);
}

#[tokio::test]
async fn failed_delivery_searches_cannot_start_a_lookup_per_queued_message() {
    let mut routing = GuardedRouting::new(peer(), false);
    let seed = peer();
    routing.remember(seed, vec![address(seed, 2000)]);
    let now = Instant::now();
    assert!(routing.start_automatically(peer(), now));
    for _ in 0..1000 {
        assert!(!routing.start_automatically(peer(), now));
    }
    assert_eq!(routing.info()["inFlight"], 1);
    assert!(!routing.start_automatically(peer(), now + Duration::from_millis(4999)));
    assert!(routing.start_automatically(peer(), now + Duration::from_secs(5)));
    assert!(!routing.start_automatically(peer(), now + Duration::from_secs(10)));
    assert_eq!(routing.info()["inFlight"], 2);
}

#[tokio::test]
async fn an_unresolved_recipient_has_a_cooldown_after_its_lookup_slot_is_freed() {
    let mut routing = GuardedRouting::new(peer(), false);
    let target = peer();
    let now = Instant::now();
    assert!(routing.start_automatically(target, now));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    // An empty real Kad table completes the lookup with no route and releases its slot.
    for _ in 0..16 {
        let _ = NetworkBehaviour::poll(&mut routing, &mut cx);
    }
    assert_eq!(routing.info()["inFlight"], 0);
    assert_eq!(routing.info()["lookups"][0]["state"], "not-found");
    assert!(!routing.start_automatically(target, now + Duration::from_millis(59999)));
    assert_eq!(routing.info()["inFlight"], 0);
    assert!(routing.start_automatically(target, now + Duration::from_secs(60)));
    assert_eq!(routing.info()["inFlight"], 1);
}
