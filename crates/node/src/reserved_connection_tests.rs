//! Real NetworkBehaviour admission/lifecycle methods. The daemon gate additionally
//! exercises 64 independent Noise/QUIC peers and Core-derived selected authority.
#![allow(clippy::unwrap_used)]
use super::reserved_connections::{Limits, Reservation};
use super::*;
use libp2p::{
    core::{ConnectedPoint, Endpoint},
    swarm::{
        CloseConnection, ConnectionDenied, DialError, FromSwarm, ListenError, ToSwarm,
        behaviour::{ConnectionClosed, ConnectionEstablished, DialFailure, ListenFailure},
    },
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

#[path = "dht_server_capacity_tests.rs"]
mod dht_capacity;

fn peer() -> PeerId {
    identity::Keypair::generate_ed25519().public().to_peer_id()
}
fn address() -> Multiaddr {
    "/ip4/127.0.0.1/tcp/43123".parse().unwrap()
}
fn endpoint() -> ConnectedPoint {
    ConnectedPoint::Listener {
        local_addr: address(),
        send_back_addr: address(),
    }
}
fn grant(peer: PeerId, permit: &Arc<AtomicBool>, time: u64) -> Reservation {
    Reservation::new(peer, permit.clone(), time, time + 60).unwrap()
}
fn connect(
    limits: &mut Limits,
    peer: PeerId,
    number: usize,
) -> std::result::Result<ConnectionId, ConnectionDenied> {
    connect_with_sibling(limits, peer, number, 0)
}
fn connect_with_sibling(
    limits: &mut Limits,
    peer: PeerId,
    number: usize,
    other_established: usize,
) -> std::result::Result<ConnectionId, ConnectionDenied> {
    let id = ConnectionId::new_unchecked(number);
    limits.handle_pending_inbound_connection(id, &address(), &address())?;
    if let Err(error) =
        limits.handle_established_inbound_connection(id, peer, &address(), &address())
    {
        limits.on_swarm_event(FromSwarm::ListenFailure(ListenFailure {
            local_addr: &address(),
            send_back_addr: &address(),
            error: &ListenError::Aborted,
            connection_id: id,
            peer_id: Some(peer),
        }));
        return Err(error);
    }
    limits.on_swarm_event(FromSwarm::ConnectionEstablished(ConnectionEstablished {
        peer_id: peer,
        connection_id: id,
        endpoint: &endpoint(),
        failed_addresses: &[],
        other_established,
    }));
    Ok(id)
}
fn closed(limits: &mut Limits, peer: PeerId, id: ConnectionId) {
    closed_with_sibling(limits, peer, id, 0);
}
fn closed_with_sibling(
    limits: &mut Limits,
    peer: PeerId,
    id: ConnectionId,
    remaining_established: usize,
) {
    limits.on_swarm_event(FromSwarm::ConnectionClosed(ConnectionClosed {
        peer_id: peer,
        connection_id: id,
        endpoint: &endpoint(),
        cause: None,
        remaining_established,
    }));
}
fn retire(limits: &mut Limits) -> Vec<(PeerId, ConnectionId)> {
    let waker = futures::task::noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut result = Vec::new();
    for _ in 0..=192 {
        match limits.poll(&mut context) {
            Poll::Pending => return result,
            Poll::Ready(ToSwarm::CloseConnection {
                peer_id,
                connection: CloseConnection::One(id),
            }) => {
                assert!(
                    !result.iter().any(|(_, previous)| *previous == id),
                    "duplicate exact close"
                );
                result.push((peer_id, id));
            }
            _ => panic!("admission guard must only retire exact excess connections"),
        }
    }
    panic!("unbounded or repeatedly emitted retirement work");
}

#[test]
fn saturated_ordinary_duplicates_release_slots_without_losing_peers_or_selected_connections() {
    let mut limits = Limits::new(2);
    let ordinary = (0..60)
        .map(|i| {
            let p = peer();
            (p, connect(&mut limits, p, i).unwrap())
        })
        .collect::<Vec<_>>();
    let selected = peer();
    let permit = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![grant(selected, &permit, now().unwrap())])
        .unwrap();
    let selected_connections = [
        connect(&mut limits, selected, 100).unwrap(),
        connect_with_sibling(&mut limits, selected, 101, 1).unwrap(),
    ];
    let mut duplicates = Vec::new();
    for (i, (p, _)) in ordinary.iter().take(3).enumerate() {
        duplicates.push((
            *p,
            connect_with_sibling(&mut limits, *p, 110 + i, 1).unwrap(),
        ));
    }
    assert!(
        retire(&mut limits).is_empty(),
        "spare capacity must allow normal reconnect overlap"
    );
    duplicates.push((
        ordinary[3].0,
        connect_with_sibling(&mut limits, ordinary[3].0, 113, 1).unwrap(),
    ));
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    let fresh = peer();
    assert!(connect(&mut limits, fresh, 200).is_err());
    let retiring = retire(&mut limits);
    assert!(
        !retiring.is_empty(),
        "duplicate sockets at the full ordinary limit starve distinct peers"
    );
    for (p, id) in &retiring {
        assert_ne!(
            *p, selected,
            "ordinary pressure retired a selected connection"
        );
        assert!(!selected_connections.contains(id));
        let siblings = ordinary
            .iter()
            .chain(duplicates.iter())
            .filter(|(peer, _)| peer == p)
            .collect::<Vec<_>>();
        assert_eq!(siblings.len(), 2);
        assert_eq!(
            retiring.iter().filter(|(peer, _)| peer == p).count(),
            1,
            "the last connection to a peer was retired"
        );
        assert!(siblings.iter().any(|(_, connection)| connection == id));
    }
    assert!(
        retire(&mut limits).is_empty(),
        "pending closes were duplicated or removed surviving peers"
    );
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    assert!(
        connect(&mut limits, fresh, 201).is_err(),
        "requested close released capacity before its actual event"
    );
    for (p, id) in retiring {
        closed_with_sibling(&mut limits, p, id, 1);
    }
    connect(&mut limits, fresh, 202).unwrap();
    assert_eq!(limits.info()["reservedEstablished"], 2);
    assert!(limits.info()["ordinaryEstablished"].as_u64().unwrap() <= 64);
}

#[test]
fn full_ordinary_class_still_accepts_selected_peers_without_bypassing_absolute_or_peer_limits() {
    let mut limits = Limits::new(2);
    let ordinary = (0..64)
        .map(|i| {
            let p = peer();
            (p, connect(&mut limits, p, i).unwrap())
        })
        .collect::<Vec<_>>();
    assert!(connect(&mut limits, peer(), 64).is_err());
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
    let mut reserved = Vec::new();
    for (i, p) in selected.iter().enumerate() {
        reserved.push((*p, connect(&mut limits, *p, 100 + 2 * i).unwrap()));
        reserved.push((
            *p,
            connect_with_sibling(&mut limits, *p, 101 + 2 * i, 1).unwrap(),
        ));
        assert!(
            connect(&mut limits, *p, 1000 + i).is_err(),
            "selected status bypassed two-per-peer bound"
        );
    }
    assert_eq!(limits.info()["established"], 192);
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    assert_eq!(limits.info()["reservedEstablished"], 128);
    assert!(
        retire(&mut limits).is_empty(),
        "ordinary connections were evicted to make selected capacity"
    );
    // A valid rotation can name a fresh peer while its predecessor's two sockets
    // are still open. Only the absolute guard can deny this 193rd connection.
    let fresh = peer();
    let mut next = selected[..63]
        .iter()
        .map(|p| grant(*p, &permit, now().unwrap()))
        .collect::<Vec<_>>();
    next.push(grant(fresh, &permit, now().unwrap()));
    limits.replace(next).unwrap();
    assert!(
        connect(&mut limits, fresh, 3000).is_err(),
        "global limit bypassed during rotation"
    );
    let retiring = retire(&mut limits);
    assert_eq!(
        retiring.iter().copied().collect::<HashSet<_>>(),
        reserved[126..].iter().copied().collect()
    );
    assert!(retiring.iter().all(|p| !ordinary.contains(p)));
    assert!(
        connect(&mut limits, fresh, 3001).is_err(),
        "pending close prematurely released physical capacity"
    );
    for (index, (p, id)) in retiring.into_iter().enumerate() {
        closed_with_sibling(&mut limits, p, id, 1 - index);
    }
    connect(&mut limits, fresh, 3002).unwrap();
    connect_with_sibling(&mut limits, fresh, 3003, 1).unwrap();
    assert_eq!(limits.info()["established"], 192);
}

#[test]
fn revoked_grants_retire_only_excess_demoted_sockets_and_capacity_returns_after_real_close_events()
{
    let mut limits = Limits::new(2);
    let mut ordinary = Vec::new();
    for i in 0..64 {
        let p = peer();
        ordinary.push((p, connect(&mut limits, p, i).unwrap()));
    }
    let permit = Arc::new(AtomicBool::new(true));
    let p = peer();
    limits
        .replace(vec![grant(p, &permit, now().unwrap())])
        .unwrap();
    let reserved = [
        connect(&mut limits, p, 100).unwrap(),
        connect_with_sibling(&mut limits, p, 101, 1).unwrap(),
    ];
    permit.store(false, Ordering::Release); // Same shared signal as the Core service fence.
    let retiring = retire(&mut limits);
    assert_eq!(
        retiring.iter().map(|(_, id)| *id).collect::<HashSet<_>>(),
        reserved.into_iter().collect()
    );
    assert!(retiring.iter().all(|(id, _)| *id == p));
    assert!(connect(&mut limits, peer(), 102).is_err());
    assert!(
        retire(&mut limits).is_empty(),
        "already requested close was re-emitted"
    );
    for (index, (p, id)) in retiring.into_iter().enumerate() {
        closed_with_sibling(&mut limits, p, id, 1 - index);
    }
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    assert_eq!(limits.info()["reservedEstablished"], 0);
    let (old, id) = ordinary.pop().unwrap();
    closed(&mut limits, old, id);
    connect(&mut limits, peer(), 103).unwrap();
    let renewed = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![grant(p, &renewed, now().unwrap())])
        .unwrap();
    connect(&mut limits, p, 104).unwrap();
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    assert_eq!(limits.info()["reservedEstablished"], 1);
}

#[test]
fn selected_handshakes_share_hard_bounds_but_ordinary_pending_dials_cannot_fill_their_reserve() {
    let mut limits = Limits::new(2);
    let selected = (0..33).map(|_| peer()).collect::<Vec<_>>();
    let permit = Arc::new(AtomicBool::new(true));
    limits
        .replace(
            selected
                .iter()
                .map(|p| grant(*p, &permit, now().unwrap()))
                .collect(),
        )
        .unwrap();
    let ordinary = (0..18).map(|_| peer()).collect::<Vec<_>>();
    for (i, ordinary_peer) in ordinary.iter().enumerate().take(16) {
        limits
            .handle_pending_outbound_connection(
                ConnectionId::new_unchecked(i),
                Some(*ordinary_peer),
                &[address()],
                Endpoint::Dialer,
            )
            .unwrap();
    }
    assert!(
        limits
            .handle_pending_outbound_connection(
                ConnectionId::new_unchecked(16),
                None,
                &[address()],
                Endpoint::Dialer
            )
            .is_err()
    );
    for i in 100..132 {
        limits
            .handle_pending_outbound_connection(
                ConnectionId::new_unchecked(i),
                Some(selected[i - 100]),
                &[address()],
                Endpoint::Dialer,
            )
            .unwrap();
    }
    assert!(
        limits
            .handle_pending_outbound_connection(
                ConnectionId::new_unchecked(133),
                Some(selected[32]),
                &[address()],
                Endpoint::Dialer
            )
            .is_err()
    );
    limits.on_swarm_event(FromSwarm::DialFailure(DialFailure {
        peer_id: Some(ordinary[0]),
        error: &DialError::Aborted,
        connection_id: ConnectionId::new_unchecked(0),
    }));
    limits
        .handle_pending_outbound_connection(
            ConnectionId::new_unchecked(17),
            Some(ordinary[16]),
            &[address()],
            Endpoint::Dialer,
        )
        .unwrap();
    assert!(
        limits
            .handle_pending_outbound_connection(
                ConnectionId::new_unchecked(18),
                Some(ordinary[17]),
                &[address()],
                Endpoint::Dialer
            )
            .is_err()
    );
    limits.on_swarm_event(FromSwarm::DialFailure(DialFailure {
        peer_id: Some(selected[0]),
        error: &DialError::Aborted,
        connection_id: ConnectionId::new_unchecked(100),
    }));
    limits
        .handle_pending_outbound_connection(
            ConnectionId::new_unchecked(134),
            Some(selected[32]),
            &[address()],
            Endpoint::Dialer,
        )
        .unwrap();
    // An inbound identity is unknown until authentication: it gets no pre-auth bypass.
    for i in 200..232 {
        limits
            .handle_pending_inbound_connection(
                ConnectionId::new_unchecked(i),
                &address(),
                &address(),
            )
            .unwrap();
    }
    assert!(
        limits
            .handle_pending_inbound_connection(
                ConnectionId::new_unchecked(233),
                &address(),
                &address()
            )
            .is_err()
    );
    limits.on_swarm_event(FromSwarm::ListenFailure(ListenFailure {
        local_addr: &address(),
        send_back_addr: &address(),
        error: &ListenError::Aborted,
        connection_id: ConnectionId::new_unchecked(200),
        peer_id: None,
    }));
    limits
        .handle_pending_inbound_connection(ConnectionId::new_unchecked(234), &address(), &address())
        .unwrap();
}

#[test]
fn reservation_catalog_is_finite_and_invalid_updates_preserve_the_previous_live_grant() {
    let permit = Arc::new(AtomicBool::new(true));
    let time = now().unwrap();
    let p = peer();
    assert!(Reservation::new(p, permit.clone(), time, time).is_err());
    assert!(Reservation::new(p, permit.clone(), time, time + 61).is_err());
    let mut limits = Limits::new(1);
    limits.replace(vec![grant(p, &permit, time)]).unwrap();
    let overflow = (0..65).map(|_| grant(peer(), &permit, time)).collect();
    assert!(limits.replace(overflow).is_err());
    for i in 0..64 {
        let n = peer();
        connect(&mut limits, n, i).unwrap();
    }
    connect(&mut limits, p, 100).unwrap();
    assert!(
        connect(&mut limits, p, 101).is_err(),
        "relay's one-per-peer policy was bypassed"
    );
    assert_eq!(limits.info()["reservedPeers"], json!([p.to_string()]));
}

#[tokio::test]
async fn signed_lease_deadline_expires_independently_of_live_role_and_a_fresh_lease_restores_capacity()
 {
    let permit = Arc::new(AtomicBool::new(true));
    let p = peer();
    let time = now().unwrap();
    let mut limits = Limits::new(2);
    for i in 0..64 {
        let n = peer();
        connect(&mut limits, n, i).unwrap();
    }
    limits
        .replace(vec![
            Reservation::new(p, permit.clone(), time, time + 2).unwrap(),
        ])
        .unwrap();
    let id = connect(&mut limits, p, 100).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while now().unwrap() < time + 2 {
        assert!(
            Instant::now() < deadline,
            "wall clock did not reach the lease deadline"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(permit.load(Ordering::Acquire));
    assert_eq!(retire(&mut limits), vec![(p, id)]);
    assert!(connect(&mut limits, p, 101).is_err());
    closed(&mut limits, p, id);
    limits
        .replace(vec![grant(p, &permit, now().unwrap())])
        .unwrap();
    connect(&mut limits, p, 102).unwrap();
    assert_eq!(limits.info()["reservedEstablished"], 1);
}

#[test]
fn a_peer_shared_by_two_live_local_authorities_keeps_its_reserve_until_both_are_gone() {
    let mut limits = Limits::new(2);
    let p = peer();
    let time = now().unwrap();
    let a = Arc::new(AtomicBool::new(true));
    let b = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![grant(p, &a, time), grant(p, &b, time)])
        .unwrap();
    for i in 0..64 {
        let n = peer();
        connect(&mut limits, n, i).unwrap();
    }
    let id = connect(&mut limits, p, 100).unwrap();
    a.store(false, Ordering::Release);
    assert!(retire(&mut limits).is_empty());
    assert_eq!(limits.info()["reservedEstablished"], 1);
    b.store(false, Ordering::Release);
    assert_eq!(retire(&mut limits), vec![(p, id)]);
    closed(&mut limits, p, id);
    assert_eq!(limits.info()["established"], 64);
}

#[test]
fn partial_demotion_keeps_older_sockets_and_future_observation_cannot_reserve() {
    let mut limits = Limits::new(2);
    let permit = Arc::new(AtomicBool::new(true));
    let time = now().unwrap();
    for i in 0..62 {
        let p = peer();
        connect(&mut limits, p, i).unwrap();
    }
    let peers = (0..6).map(|_| peer()).collect::<Vec<_>>();
    limits
        .replace(peers.iter().map(|p| grant(*p, &permit, time)).collect())
        .unwrap();
    let ids = peers
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, connect(&mut limits, *p, 100 + i).unwrap()))
        .collect::<Vec<_>>();
    permit.store(false, Ordering::Release);
    assert_eq!(
        retire(&mut limits),
        ids[2..].iter().rev().copied().collect::<Vec<_>>()
    );
    assert_eq!(limits.info()["established"], 68);
    for (p, id) in &ids[2..] {
        closed(&mut limits, *p, *id);
    }
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    let fresh = Arc::new(AtomicBool::new(true));
    let p = peer();
    limits
        .replace(vec![grant(p, &fresh, now().unwrap() + 30)])
        .unwrap();
    assert!(
        connect(&mut limits, p, 200).is_err(),
        "time earlier than observation accepted a reserve"
    );
    limits
        .replace(vec![grant(p, &fresh, now().unwrap())])
        .unwrap();
    connect(&mut limits, p, 201).unwrap();
    assert_eq!(limits.info()["reservedEstablished"], 1);
}

#[test]
fn client_reserve_needs_current_public_authority_and_an_unfinished_request_even_when_sources_share_a_peer()
 {
    let mut limits = Limits::new(2);
    for i in 0..64 {
        let p = peer();
        connect(&mut limits, p, i).unwrap();
    }
    let p = peer();
    let time = now().unwrap();
    let local = Arc::new(AtomicBool::new(true));
    let authority = Arc::new(AtomicBool::new(true));
    let first = Arc::new(AtomicBool::new(true));
    let second = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![
            grant(p, &local, time),
            Reservation::client(p, authority.clone(), first.clone(), time, time + 60).unwrap(),
            Reservation::client(p, authority.clone(), second.clone(), time, time + 60).unwrap(),
        ])
        .unwrap();
    let id = connect(&mut limits, p, 100).unwrap();
    local.store(false, Ordering::Release);
    first.store(false, Ordering::Release);
    assert!(retire(&mut limits).is_empty());
    assert_eq!(limits.info()["reservedEstablished"], 1);
    second.store(false, Ordering::Release);
    assert_eq!(retire(&mut limits), vec![(p, id)]);
    assert!(
        authority.load(Ordering::Acquire),
        "completion must not fabricate public-authority revocation"
    );
    closed(&mut limits, p, id);
    assert!(
        connect(&mut limits, p, 101).is_err(),
        "completed request kept capacity"
    );
    let fresh = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![
            Reservation::client(p, authority.clone(), fresh.clone(), time, time + 60).unwrap(),
        ])
        .unwrap();
    let next = connect(&mut limits, p, 102).unwrap();
    authority.store(false, Ordering::Release);
    assert_eq!(retire(&mut limits), vec![(p, next)]);
    assert!(fresh.load(Ordering::Acquire));
    closed(&mut limits, p, next);
    assert!(
        connect(&mut limits, p, 103).is_err(),
        "live request bypassed revoked public authority"
    );
    authority.store(true, Ordering::Release);
    fresh.store(false, Ordering::Release);
    assert!(
        connect(&mut limits, p, 104).is_err(),
        "renewed authority revived a completed request"
    );
    let renewed_time = now().unwrap();
    limits
        .replace(vec![
            Reservation::client(
                p,
                authority.clone(),
                Arc::new(AtomicBool::new(true)),
                renewed_time,
                renewed_time + 60,
            )
            .unwrap(),
        ])
        .unwrap();
    connect(&mut limits, p, 105).unwrap();
    assert_eq!(limits.info()["ordinaryEstablished"], 64);
    assert_eq!(limits.info()["reservedEstablished"], 1);
}

#[path = "network_budget_tests.rs"]
mod processing;

#[path = "processing_codec_tests.rs"]
mod codec_lifetime;
