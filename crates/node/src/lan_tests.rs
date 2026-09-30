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
    for port in 4002..4010 {
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
        4
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
