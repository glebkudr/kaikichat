#![allow(clippy::expect_used)]
use super::nat_support::Reachability;
use libp2p::{Multiaddr, PeerId, swarm::ConnectionId};
use std::time::{Duration, Instant};
fn addr(value: &str) -> Multiaddr {
    value.parse().expect("valid test multiaddress")
}
fn state() -> Reachability<u64> {
    Reachability::new(
        PeerId::random(),
        Duration::from_secs(20),
        Duration::from_secs(6),
    )
}
const SERVICE: &str = "/ip4/198.18.0.2/tcp/4001";
const LOCAL: &str = "/ip4/192.168.1.4/tcp/4001";
const CALLBACK: &str = "/ip4/198.18.0.2/tcp/51003";
const PUBLIC: &str = "/ip4/198.18.0.9/tcp/4001";

#[test]
fn autonat_service_does_not_close_a_fresh_callback_before_remote_protocol_negotiation() {
    use super::nat_support::CallbackLeases;
    let now = Instant::now();
    let peer = PeerId::random();
    let callback = ConnectionId::new_unchecked(101);
    let ordinary = ConnectionId::new_unchecked(102);
    let mut leases = CallbackLeases::new(Duration::from_secs(8));
    leases.dial_started(peer, callback);
    leases.established(ordinary, now);
    leases.established(callback, now);
    assert!(
        leases.expired(now).is_empty(),
        "local establishment can precede remote multistream negotiation; closing now causes BrokenPipe"
    );
    assert!(leases.expired(now + Duration::from_millis(7999)).is_empty());
    // Abandoned callbacks still have a finite resource lifetime, without closing the peer's
    // ordinary control/relay connection that may share the same authenticated PeerID.
    assert_eq!(
        leases.expired(now + Duration::from_secs(8)),
        vec![(peer, callback)]
    );
    assert!(leases.expired(now + Duration::from_secs(9)).is_empty());
}

#[test]
fn autonat_callback_cleanup_tracks_exact_connections_after_client_close_failure_and_late_dial() {
    use super::nat_support::CallbackLeases;
    let now = Instant::now();
    let peer = PeerId::random();
    let healthy = ConnectionId::new_unchecked(111);
    let failed = ConnectionId::new_unchecked(112);
    let late = ConnectionId::new_unchecked(113);
    let mut leases = CallbackLeases::new(Duration::from_secs(8));
    leases.dial_started(peer, healthy);
    leases.dial_started(peer, failed);
    leases.dial_started(peer, late);
    leases.established(healthy, now);
    leases.closed(failed); // transport failure before an authenticated callback exists
    leases.closed(healthy); // client verified its fresh callback and closed it promptly
    assert!(leases.expired(now + Duration::from_secs(8)).is_empty());
    // The original probe has timed out, but an already-owned dial may still complete.
    let connected = now + Duration::from_secs(9);
    leases.established(late, connected);
    assert!(leases.expired(connected).is_empty());
    assert_eq!(
        leases.expired(connected + Duration::from_secs(8)),
        vec![(peer, late)]
    );
    assert!(
        leases
            .expired(connected + Duration::from_secs(9))
            .is_empty()
    );
}

#[test]
fn overlapping_autonat_callbacks_of_one_peer_expire_independently() {
    use super::nat_support::CallbackLeases;
    let now = Instant::now();
    let peer = PeerId::random();
    let first = ConnectionId::new_unchecked(121);
    let second = ConnectionId::new_unchecked(122);
    let failed = ConnectionId::new_unchecked(123);
    let mut leases = CallbackLeases::new(Duration::from_secs(8));
    leases.dial_started(peer, first);
    leases.established(first, now);
    leases.dial_started(peer, second);
    leases.established(second, now + Duration::from_secs(2));
    leases.dial_started(peer, failed);
    leases.closed(failed);
    assert!(leases.expired(now + Duration::from_secs(7)).is_empty());
    assert_eq!(
        leases.expired(now + Duration::from_secs(8)),
        vec![(peer, first)]
    );
    assert!(leases.expired(now + Duration::from_secs(9)).is_empty());
    assert_eq!(
        leases.expired(now + Duration::from_secs(10)),
        vec![(peer, second)]
    );
    assert!(leases.expired(now + Duration::from_secs(11)).is_empty());
}

#[test]
fn autonat_report_cannot_publish_address_without_fresh_matching_dial_back() {
    let now = Instant::now();
    let server = PeerId::random();
    let mut reach = state();
    assert_eq!(reach.status(), "unknown");
    reach.begin(1, server, addr(SERVICE), now);
    assert!(reach.succeeded(1, server, addr(PUBLIC), now).is_none());
    assert!(reach.public_address().is_none());
    reach.begin(2, server, addr(SERVICE), now);
    reach.inbound(
        PeerId::random(),
        ConnectionId::new_unchecked(11),
        addr(LOCAL),
        addr(CALLBACK),
        now,
    );
    assert!(reach.succeeded(2, server, addr(PUBLIC), now).is_none());
    reach.begin(3, server, addr(SERVICE), now);
    // Existing mapping to the service port is not an independent external reachability check.
    reach.inbound(
        server,
        ConnectionId::new_unchecked(12),
        addr(LOCAL),
        addr(SERVICE),
        now,
    );
    assert!(reach.succeeded(3, server, addr(PUBLIC), now).is_none());
    reach.begin(4, server, addr(SERVICE), now);
    reach.inbound(
        server,
        ConnectionId::new_unchecked(13),
        addr(LOCAL),
        addr(CALLBACK),
        now,
    );
    assert!(
        reach
            .succeeded(4, server, addr("/ip4/198.18.0.9/tcp/9000"), now)
            .is_none()
    );
    assert!(reach.public_address().is_none());
}

#[test]
fn autonat_verified_address_expires_and_distinguishes_closed_nat_from_unavailable_service() {
    let now = Instant::now();
    let server = PeerId::random();
    let mut reach = state();
    reach.begin(1, server, addr(SERVICE), now);
    let connection = ConnectionId::new_unchecked(20);
    reach.inbound(server, connection, addr(LOCAL), addr(CALLBACK), now);
    assert_eq!(
        reach.succeeded(1, server, addr(PUBLIC), now),
        Some(connection)
    );
    assert_eq!(reach.status(), "public");
    assert_eq!(reach.public_address(), Some(&addr(PUBLIC)));
    reach.expire(now + Duration::from_secs(21));
    assert_eq!(reach.status(), "unknown");
    assert!(reach.public_address().is_none());
    reach.begin(2, server, addr(SERVICE), now + Duration::from_secs(22));
    assert!(reach.failed(2, server, true));
    assert_eq!(reach.status(), "private");
    reach.begin(3, server, addr(SERVICE), now + Duration::from_secs(23));
    assert!(reach.failed(3, server, false));
    assert_eq!(reach.status(), "unknown");
    assert!(reach.public_address().is_none());
}

#[test]
fn autonat_superseded_probe_and_expired_witness_cannot_restore_an_old_public_route() {
    let now = Instant::now();
    let server = PeerId::random();
    let mut reach = state();
    reach.begin(1, server, addr(SERVICE), now);
    reach.inbound(
        server,
        ConnectionId::new_unchecked(31),
        addr(LOCAL),
        addr(CALLBACK),
        now,
    );
    reach.begin(2, server, addr(SERVICE), now + Duration::from_secs(1));
    // Check before processing old responses: those must not mask a carried-over witness.
    assert!(
        reach
            .succeeded(2, server, addr(PUBLIC), now + Duration::from_secs(1))
            .is_none()
    );
    assert!(
        reach
            .succeeded(1, server, addr(PUBLIC), now + Duration::from_secs(1))
            .is_none()
    );
    assert!(!reach.failed(1, server, true));
    reach.inbound(
        server,
        ConnectionId::new_unchecked(32),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_secs(2),
    );
    assert_eq!(
        reach.succeeded(2, server, addr(PUBLIC), now + Duration::from_secs(2)),
        Some(ConnectionId::new_unchecked(32))
    );
    reach.begin(3, server, addr(SERVICE), now + Duration::from_secs(3));
    reach.inbound(
        server,
        ConnectionId::new_unchecked(33),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_secs(4),
    );
    assert!(
        reach
            .succeeded(3, server, addr(PUBLIC), now + Duration::from_secs(10))
            .is_none()
    );
    // The old public lease may remain until expiry; an expired probe cannot renew it.
    reach.expire(now + Duration::from_secs(23));
    assert!(reach.public_address().is_none());
    reach.begin(4, server, addr(SERVICE), now + Duration::from_secs(24));
    reach.inbound(
        server,
        ConnectionId::new_unchecked(34),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_secs(25),
    );
    assert!(
        reach
            .succeeded(4, server, addr(PUBLIC), now + Duration::from_secs(25))
            .is_some()
    );
    reach.begin(5, server, addr(SERVICE), now + Duration::from_secs(26));
    assert!(reach.failed(5, server, true));
    assert_eq!(reach.status(), "private");
    assert!(reach.public_address().is_none());
}

#[test]
fn autonat_response_before_authenticated_callback_waits_for_witness_and_still_expires() {
    let now = Instant::now();
    let server = PeerId::random();
    let mut reach = state();
    reach.begin(1, server, addr(SERVICE), now);
    assert!(reach.succeeded(1, server, addr(PUBLIC), now).is_none());
    assert!(
        reach.public_address().is_none(),
        "bare server success cannot publish a route"
    );
    assert!(reach.finish(now + Duration::from_millis(2)).is_none());
    assert!(reach.public_address().is_none());
    assert_eq!(reach.status(), "unknown");
    reach.inbound(
        server,
        ConnectionId::new_unchecked(41),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_millis(4),
    );
    assert_eq!(
        reach.finish(now + Duration::from_millis(5)),
        Some(ConnectionId::new_unchecked(41))
    );
    assert_eq!(reach.status(), "public");
    assert_eq!(reach.public_address(), Some(&addr(PUBLIC)));
    reach.expire(now + Duration::from_secs(21));
    assert!(reach.public_address().is_none());
    reach.begin(2, server, addr(SERVICE), now + Duration::from_secs(22));
    assert!(
        reach
            .succeeded(2, server, addr(PUBLIC), now + Duration::from_secs(22))
            .is_none()
    );
    reach.expire(now + Duration::from_secs(29));
    reach.inbound(
        server,
        ConnectionId::new_unchecked(42),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_secs(29),
    );
    assert!(reach.finish(now + Duration::from_secs(29)).is_none());
    assert!(
        reach.public_address().is_none(),
        "late callback must not revive an expired success report"
    );
    reach.begin(3, server, addr(SERVICE), now + Duration::from_secs(30));
    assert!(
        reach
            .succeeded(3, server, addr(PUBLIC), now + Duration::from_secs(30))
            .is_none()
    );
    reach.begin(4, server, addr(SERVICE), now + Duration::from_secs(31));
    reach.inbound(
        server,
        ConnectionId::new_unchecked(43),
        addr(LOCAL),
        addr(CALLBACK),
        now + Duration::from_secs(31),
    );
    assert!(reach.finish(now + Duration::from_secs(31)).is_none());
    assert!(
        reach.public_address().is_none(),
        "new probe cannot inherit the superseded response"
    );
    assert_eq!(
        reach.succeeded(4, server, addr(PUBLIC), now + Duration::from_secs(31)),
        Some(ConnectionId::new_unchecked(43))
    );
}
