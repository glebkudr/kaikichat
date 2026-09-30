#![allow(clippy::unwrap_used)]
use super::lan_support::LanHints;
use super::*;

fn lan_peer() -> PeerId {
    identity::Keypair::generate_ed25519().public().to_peer_id()
}
fn address(peer: PeerId, port: u16) -> Multiaddr {
    format!("/ip4/192.168.20.15/tcp/{port}/p2p/{peer}")
        .parse()
        .unwrap()
}

#[test]
fn lan_untrusted_hints_are_bounded_expire_and_cannot_claim_root_authority() {
    let now = Instant::now();
    let own = lan_peer();
    let mut hints = LanHints::new(own);
    let peers: Vec<_> = (0..40).map(|_| lan_peer()).collect();
    for peer in &peers {
        hints.observe(*peer, address(*peer, 4001), now);
    }
    let first = hints.hints(now);
    assert_eq!(first.len(), 32);
    assert!(first.iter().all(|hint| hint.root.is_none()));
    let retained = first[0].peer;
    for port in 4002..4020 {
        hints.observe(retained, address(retained, port), now);
    }
    assert_eq!(
        hints
            .hints(now)
            .iter()
            .find(|h| h.peer == retained)
            .unwrap()
            .addresses
            .len(),
        8
    );
    hints.observe(
        retained,
        address(retained, 4001),
        now + Duration::from_secs(45),
    );
    let later = hints.hints(now + Duration::from_secs(61));
    assert_eq!(
        later.len(),
        1,
        "unrefreshed advertisements cannot occupy capacity forever"
    );
    assert_eq!(later[0].peer, retained);
    assert_eq!(later[0].addresses, vec![address(retained, 4001)]);
    let replacement = lan_peer();
    hints.observe(
        replacement,
        address(replacement, 5000),
        now + Duration::from_secs(61),
    );
    assert_eq!(hints.hints(now + Duration::from_secs(61)).len(), 2);
    assert!(hints.hints(now + Duration::from_secs(122)).is_empty());
}

#[test]
fn lan_advertisements_reject_self_mismatched_peer_and_unsupported_routes() {
    let now = Instant::now();
    let own = lan_peer();
    let other = lan_peer();
    let mut hints = LanHints::new(own);
    for (peer, route) in [
        (own, address(own, 4001)),
        (other, address(own, 4001)),
        (
            other,
            format!("/dns4/peer.example/tcp/4001/p2p/{other}")
                .parse()
                .unwrap(),
        ),
        (
            other,
            format!("/ip4/0.0.0.0/tcp/4001/p2p/{other}")
                .parse()
                .unwrap(),
        ),
        (
            other,
            format!("/ip4/127.0.0.1/tcp/4001/p2p/{other}")
                .parse()
                .unwrap(),
        ),
        (
            other,
            format!("/ip4/224.0.0.251/udp/5353/quic-v1/p2p/{other}")
                .parse()
                .unwrap(),
        ),
        (
            other,
            format!("/ip4/192.168.20.15/tcp/4001/p2p/{own}/p2p-circuit/p2p/{other}")
                .parse()
                .unwrap(),
        ),
    ] {
        hints.observe(peer, route, now);
        assert!(hints.hints(now).is_empty());
    }
    hints.observe(other, address(other, 4001), now);
    assert_eq!(hints.hints(now)[0].addresses, vec![address(other, 4001)]);
}

#[test]
fn signed_cache_root_survives_lan_address_refresh_without_duplicate_bootstrap_work() {
    use super::bootstrap_support::{Hint, Schedule};
    let now = Instant::now();
    let peer = lan_peer();
    let signed = Hint {
        peer,
        addresses: vec![address(peer, 4001)],
        root: Some([91; 32]),
    };
    let unsigned = Hint {
        peer,
        addresses: vec![address(peer, 5001)],
        root: None,
    };
    let mut queue = Schedule::new(vec![], now);
    queue.replace_sources(vec![signed.clone()], vec![unsigned.clone()], now);
    let request = queue.ready(now, false);
    assert_eq!(request.len(), 1);
    assert_eq!(
        request[0].root,
        Some([91; 32]),
        "mDNS cannot erase the expected signed root"
    );
    assert!(
        request[0].addresses.contains(&address(peer, 5001)),
        "fresh LAN address must recover a moved peer"
    );
    assert!(request[0].addresses.contains(&address(peer, 4001)));
    queue.replace_sources(vec![signed], vec![unsigned], now + Duration::from_secs(1));
    assert!(queue.ready(now + Duration::from_secs(1), false).is_empty());
    queue.finished(peer, false, now + Duration::from_secs(1));
    assert!(
        queue
            .ready(now + Duration::from_millis(1499), false)
            .is_empty()
    );
    assert_eq!(
        queue.ready(now + Duration::from_millis(1500), false).len(),
        1
    );
    queue.finished(peer, false, now + Duration::from_millis(1500));
    assert!(
        queue.ready(now + Duration::from_secs(4), true).is_empty(),
        "LAN addresses cannot bypass relay-only"
    );
}

/// A contact's signed record lists the eight routes it had before it
/// restarted or moved (a host with bridges lists eight). Offline, the LAN
/// addresses it answers mDNS from are the only way to reach it: they are
/// dialed first (two at a time), and the full record neither crowds them out
/// nor loses its own routes to unsigned hints.
#[test]
fn a_fresh_lan_address_reaches_a_moved_peer_whose_signed_record_is_full() {
    use super::bootstrap_support::{Hint, Schedule};
    let now = Instant::now();
    let peer = lan_peer();
    let left: Vec<_> = (4001..4009).map(|port| address(peer, port)).collect();
    let fresh: Vec<_> = (5001..5005).map(|port| address(peer, port)).collect();
    let signed = Hint {
        peer,
        addresses: left.clone(),
        root: Some([91; 32]),
    };
    let unsigned = Hint {
        peer,
        addresses: fresh.clone(),
        root: None,
    };
    let mut queue = Schedule::new(vec![], now);
    queue.replace_sources(vec![signed.clone()], vec![unsigned.clone()], now);
    let request = queue.ready(now, false);
    assert_eq!(request.len(), 1);
    assert_eq!(
        request[0].root,
        Some([91; 32]),
        "mDNS cannot erase the expected signed root"
    );
    let expected: Vec<_> = fresh.iter().chain(&left[..4]).cloned().collect();
    assert_eq!(
        request[0].addresses, expected,
        "fresh LAN addresses first, then the signed routes"
    );
    // The next refresh, after the attempt failed, dials them first again.
    queue.finished(peer, false, now);
    let later = now + Duration::from_secs(5);
    queue.replace_sources(vec![signed], vec![unsigned], later);
    let retry = queue.ready(later, false);
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].addresses, expected);
}

fn tcp(ip: &str, peer: PeerId) -> Multiaddr {
    format!("/ip4/{ip}/tcp/4001/p2p/{peer}").parse().unwrap()
}
fn quic(ip: &str, peer: PeerId) -> Multiaddr {
    format!("/ip4/{ip}/udp/4001/quic-v1/p2p/{peer}")
        .parse()
        .unwrap()
}
fn interfaces(hints: &mut LanHints, networks: &[&str]) {
    for network in networks {
        hints.interface(&if_watch::IfEvent::Up(network.parse().unwrap()));
    }
}
fn routes_of(hints: &[bootstrap_support::Hint], peer: PeerId) -> Vec<Multiaddr> {
    hints
        .iter()
        .find(|hint| hint.peer == peer)
        .map(|hint| hint.addresses.clone())
        .unwrap_or_default()
}
fn sorted(mut routes: Vec<Multiaddr>) -> Vec<Multiaddr> {
    routes.sort();
    routes
}
/// The Mac the failure was seen on: its LAN, and OrbStack bridges, three of
/// which hold their subnet's network address and refuse connections
/// (`EADDRNOTAVAIL`).
const THIS_MAC: [&str; 6] = [
    "127.0.0.1/8",
    "192.168.10.41/24",
    "192.168.139.3/23",
    "192.168.215.0/24",
    "172.16.42.0/24",
    "172.31.250.0/24",
];

/// libp2p mDNS answers from every interface and gives every advertised route
/// the answer's source address, so on a Mac with bridges a peer is first
/// heard on bridge routes (chain 4 of the LAN evidence: Alice's first four
/// routes of Bob were all on `.0` bridges and Bob was never reached). A route
/// on a subnet's network address is never kept, a peer on another Mac is
/// dialed on the LAN first, and another profile on this Mac keeps its
/// usable local routes.
#[test]
fn mdns_routes_on_a_host_with_bridges_keep_the_usable_ones_first() {
    let now = Instant::now();
    let mut hints = LanHints::new(lan_peer());
    interfaces(&mut hints, &THIS_MAC);
    // Bob runs on this Mac; Dave on another Mac of the LAN with the same
    // OrbStack bridges (OrbStack uses the same subnets on every Mac).
    let bob = lan_peer();
    let dave = lan_peer();
    for ip in [
        "172.31.250.0",
        "172.16.42.0",
        "192.168.215.0",
        "192.168.139.3",
    ] {
        for peer in [bob, dave] {
            hints.observe(peer, tcp(ip, peer), now);
            hints.observe(peer, quic(ip, peer), now);
        }
    }
    for (peer, ip) in [(bob, "192.168.10.41"), (dave, "192.168.10.52")] {
        hints.observe(peer, tcp(ip, peer), now);
        hints.observe(peer, quic(ip, peer), now);
    }
    let found = hints.hints(now);
    let (bob_routes, dave_routes) = (routes_of(&found, bob), routes_of(&found, dave));
    assert_eq!(
        sorted(bob_routes),
        sorted(vec![
            tcp("192.168.139.3", bob),
            quic("192.168.139.3", bob),
            tcp("192.168.10.41", bob),
            quic("192.168.10.41", bob),
        ]),
        "only this Mac's routes that accept connections"
    );
    assert!(
        dave_routes.iter().all(|route| {
            !matches!(route.iter().next(), Some(Protocol::Ip4(ip))
                if ["172.31.250.0", "172.16.42.0", "192.168.215.0"].contains(&ip.to_string().as_str()))
        }),
        "no route on a subnet's network address: {dave_routes:?}"
    );
    assert_eq!(
        sorted(dave_routes[..2].to_vec()),
        sorted(vec![
            tcp("192.168.10.52", dave),
            quic("192.168.10.52", dave)
        ]),
        "the LAN routes first; 192.168.139.3 is this Mac's own bridge"
    );
}

/// A Mac without bridges hears a peer from all of that peer's bridges and
/// virtual machines before its LAN address. The kept routes stay bounded,
/// the LAN route is kept anyway and goes into the first pair dialed, also
/// ahead of the routes of the peer's full signed record.
#[test]
fn a_peer_heard_from_many_foreign_bridges_is_dialed_on_the_lan_first() {
    use super::bootstrap_support::{Hint, Schedule};
    let now = Instant::now();
    let mut hints = LanHints::new(lan_peer());
    interfaces(&mut hints, &["127.0.0.1/8", "192.168.10.41/24"]);
    let erin = lan_peer();
    for ip in [
        "172.31.250.0",
        "172.16.42.0",
        "192.168.215.0",
        "192.168.139.3",
        "10.211.55.2",
        "192.168.64.1",
    ] {
        hints.observe(erin, tcp(ip, erin), now);
        hints.observe(erin, quic(ip, erin), now);
    }
    let lan = sorted(vec![
        tcp("192.168.10.60", erin),
        quic("192.168.10.60", erin),
    ]);
    for route in &lan {
        hints.observe(erin, route.clone(), now);
    }
    let found = hints.hints(now);
    let routes = routes_of(&found, erin);
    assert!(routes.len() <= 8, "bounded: {routes:?}");
    assert_eq!(sorted(routes[..2].to_vec()), lan);
    // Erin's signed record from before lists eight routes she has left.
    let signed = Hint {
        peer: erin,
        addresses: (4001..4009).map(|port| address(erin, port)).collect(),
        root: Some([7; 32]),
    };
    let mut queue = Schedule::new(vec![], now);
    queue.replace_sources(vec![signed], found, now);
    let request = queue.ready(now, false);
    assert_eq!(request.len(), 1);
    assert_eq!(request[0].root, Some([7; 32]));
    assert_eq!(
        sorted(request[0].addresses[..2].to_vec()),
        lan,
        "routes are dialed two at a time: the LAN route is in the first pair"
    );
}

/// When this host's interfaces cannot be read, nothing tells a bridge route
/// from a LAN route. Routes that refused a dial then go behind the untried
/// ones, and a route heard again can replace them, so a peer is never locked
/// onto the routes heard first.
#[test]
fn routes_that_refused_a_dial_give_way_to_untried_ones() {
    use libp2p::{
        TransportError,
        swarm::{ConnectionId, DialError, DialFailure, FromSwarm},
    };
    let now = Instant::now();
    let mut hints = LanHints::new(lan_peer());
    let bob = lan_peer();
    let bridges: Vec<_> = [
        "172.31.250.0",
        "172.16.42.0",
        "192.168.215.0",
        "10.211.55.0",
    ]
    .iter()
    .flat_map(|ip| [tcp(ip, bob), quic(ip, bob)])
    .collect();
    let lan = sorted(vec![tcp("192.168.10.41", bob), quic("192.168.10.41", bob)]);
    let heard: Vec<_> = bridges.iter().chain(&lan).cloned().collect();
    for route in &heard {
        hints.observe(bob, route.clone(), now);
    }
    let kept = routes_of(&hints.hints(now), bob);
    assert_eq!(
        sorted(kept.clone()),
        sorted(bridges.clone()),
        "bounded to the first eight"
    );
    let refuse = |hints: &mut LanHints, routes: &[Multiaddr]| {
        let error = DialError::Transport(
            routes
                .iter()
                .map(|route| {
                    (
                        route.clone(),
                        TransportError::Other(std::io::Error::from(
                            std::io::ErrorKind::AddrNotAvailable,
                        )),
                    )
                })
                .collect(),
        );
        hints.on_swarm_event(&FromSwarm::DialFailure(DialFailure {
            peer_id: Some(bob),
            error: &error,
            connection_id: ConnectionId::new_unchecked(1),
        }));
    };
    // An exchange with a contact whose signed record is kept dials four LAN
    // routes first; all refuse.
    refuse(&mut hints, &kept[..4]);
    let rotated = routes_of(&hints.hints(now), bob);
    assert_eq!(sorted(rotated[..4].to_vec()), sorted(kept[4..].to_vec()));
    refuse(&mut hints, &rotated[..4]);
    // The next mDNS answers, ten seconds later, bring the LAN routes back.
    let later = now + Duration::from_secs(10);
    for route in &heard {
        hints.observe(bob, route.clone(), later);
    }
    let routes = routes_of(&hints.hints(later), bob);
    assert!(routes.len() <= 8, "bounded: {routes:?}");
    assert_eq!(sorted(routes[..2].to_vec()), lan);
}
