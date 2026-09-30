//! Admission to the mailbox protocol before a request's bytes are read
//! (Docs/V1_DISCOVERY_2026_09_27.md, part 1): units and peers with an
//! accepted pass get in under their principal's limits, everyone else only
//! through the small path that shows a pass.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::runtime::clock::Virtual;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const WALL: u64 = 20_000 * 86_400 + 1_000;
const BOOK: [u8; 32] = [0xb0; 32];

fn peer() -> PeerId {
    PeerId::random()
}

fn address(n: u8) -> Option<IpAddr> {
    Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, n)))
}

fn gate() -> Gate {
    let gate = Gate::new();
    gate.set_enabled(true);
    gate
}

fn on_probation(admission: io::Result<Admission>) -> bool {
    matches!(admission, Ok(Admission::Probation))
}

/// Admissions of `peer` in a row, until the first refusal.
fn admitted(gate: &Gate, peer: PeerId, from: Option<IpAddr>, tries: usize) -> usize {
    (0..tries)
        .take_while(|_| gate.admit(peer, from).is_ok())
        .count()
}

#[test]
fn a_disabled_gate_lets_every_peer_in() {
    let _clock = Virtual::install(WALL);
    let gate = Gate::new();
    for _ in 0..100 {
        assert!(matches!(
            gate.admit(peer(), address(1)),
            Ok(Admission::Open)
        ));
    }
    assert_eq!(gate.principal(&peer()), None);
}

#[test]
fn a_peer_without_a_credential_only_gets_the_small_path_to_show_one() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let stranger = peer();
    assert!(on_probation(gate.admit(stranger, address(1))));
    assert!(on_probation(gate.admit(stranger, address(1))));
    assert!(
        gate.admit(stranger, address(1)).is_err(),
        "two a second per peer"
    );
    // Other peers behind the same address share its eight a second.
    let neighbours: usize = (0..10)
        .map(|_| admitted(&gate, peer(), address(1), 1))
        .sum();
    assert_eq!(neighbours, 6);
    // All addresses together get sixty-four a second.
    let crowd: usize = (10..100)
        .map(|n| admitted(&gate, peer(), address(n), 1))
        .sum();
    assert_eq!(crowd, 64 - 8);
    clock.advance(Duration::from_secs(1));
    assert!(on_probation(gate.admit(stranger, address(1))));
    assert_eq!(gate.principal(&stranger), None);
}

#[test]
fn local_peers_are_limited_per_peer_not_per_address() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let local = [
        Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        Some(IpAddr::V6(Ipv6Addr::LOCALHOST)),
    ];
    for from in local {
        let each: usize = (0..20).map(|_| admitted(&gate, peer(), from, 2)).sum();
        assert_eq!(each, 40);
        clock.advance(Duration::from_secs(1));
    }
}

#[test]
fn a_peer_that_showed_a_bad_credential_and_its_address_wait_five_minutes() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let cheat = peer();
    assert!(on_probation(gate.admit(cheat, address(7))));
    gate.penalize(&cheat);
    clock.advance(Duration::from_secs(1));
    assert!(gate.admit(cheat, address(7)).is_err());
    assert!(gate.admit(peer(), address(7)).is_err(), "same address");
    assert!(on_probation(gate.admit(peer(), address(8))));
    clock.advance(Duration::from_secs(298));
    assert!(gate.admit(cheat, address(7)).is_err());
    clock.advance(Duration::from_secs(1));
    assert!(on_probation(gate.admit(cheat, address(7))));
    // A local peer is penalized alone, not everyone on its machine.
    let local = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let (bad, good) = (peer(), peer());
    gate.admit(bad, local).unwrap();
    gate.penalize(&bad);
    assert!(gate.admit(bad, local).is_err());
    assert!(on_probation(gate.admit(good, local)));
}

#[test]
fn all_peers_of_one_book_share_thirty_requests_a_second() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let peers: Vec<PeerId> = (0..1_000).map(|_| peer()).collect();
    for (n, p) in peers.iter().enumerate() {
        gate.accept_book(*p, BOOK, WALL + 3_600);
        assert_eq!(gate.principal(p), Some(Principal::Book(BOOK)), "peer {n}");
    }
    let total: usize = peers
        .iter()
        .enumerate()
        .map(|(n, p)| admitted(&gate, *p, address((n % 200) as u8), 5))
        .sum();
    assert_eq!(total, 30);
    assert!(gate.admit(peers[0], address(0)).is_err());
    clock.advance(Duration::from_secs(1));
    assert!(matches!(
        gate.admit(peers[999], address(9)),
        Ok(Admission::Book(BOOK))
    ));
}

/// Fairness looks at the books that asked in the previous second, refused
/// requests included, so the order books ask in within a second does not
/// matter; a second without any request makes the node quiet again.
#[test]
fn a_busy_node_shares_its_rate_evenly_between_books() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let books: Vec<([u8; 32], PeerId)> = (0..20u8)
        .map(|n| {
            let p = peer();
            gate.accept_book(p, [n; 32], WALL + 3_600);
            ([n; 32], p)
        })
        .collect();
    // A second with twenty books asking as much as they may...
    for (_, p) in &books {
        admitted(&gate, *p, address(1), 30);
    }
    clock.advance(Duration::from_secs(1));
    // ...then each gets an even share of 256 a second, and the node's
    // total stays within it.
    let shares: Vec<usize> = books
        .iter()
        .map(|(_, p)| admitted(&gate, *p, address(1), 30))
        .collect();
    assert!(shares.iter().all(|s| *s == 256 / 20), "{shares:?}");
    // A quiet node lets one book use its full thirty again.
    clock.advance(Duration::from_secs(2));
    assert_eq!(admitted(&gate, books[0].1, address(1), 40), 30);
}

#[test]
fn units_are_let_in_without_a_book_limit() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let (listed, introduced) = (peer(), peer());
    gate.set_units(BTreeSet::from([listed]));
    gate.accept_unit(introduced, WALL + 3_600);
    for unit in [listed, introduced] {
        assert_eq!(gate.principal(&unit), Some(Principal::Unit));
        for _ in 0..100 {
            assert!(matches!(gate.admit(unit, address(1)), Ok(Admission::Unit)));
        }
    }
    // A unit the directory no longer lists is a stranger again; one that
    // introduced itself stays a unit until its introduction ends.
    gate.set_units(BTreeSet::new());
    assert_eq!(gate.principal(&listed), None);
    assert_eq!(gate.principal(&introduced), Some(Principal::Unit));
    clock.advance(Duration::from_secs(3_600));
    assert_eq!(gate.principal(&introduced), None);
}

#[test]
fn a_flood_of_bad_credentials_does_not_keep_out_book_owners_or_units() {
    let _clock = Virtual::install(WALL);
    let gate = gate();
    let (owner, unit) = (peer(), peer());
    gate.accept_book(owner, BOOK, WALL + 3_600);
    gate.set_units(BTreeSet::from([unit]));
    // A cheat behind the owner's address gets it penalized...
    let cheat = peer();
    assert!(on_probation(gate.admit(cheat, address(1))));
    gate.penalize(&cheat);
    assert!(gate.admit(peer(), address(1)).is_err());
    // ...and the whole small path is used up from many addresses.
    for n in 2..100 {
        admitted(&gate, peer(), address(n), 2);
    }
    assert!(gate.admit(peer(), address(200)).is_err());
    assert!(matches!(
        gate.admit(owner, address(1)),
        Ok(Admission::Book(BOOK))
    ));
    assert!(matches!(gate.admit(unit, address(1)), Ok(Admission::Unit)));
}

#[test]
fn a_pass_lasts_until_its_end() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let p = peer();
    gate.accept_book(p, BOOK, WALL + 10);
    clock.advance(Duration::from_secs(9));
    assert_eq!(gate.principal(&p), Some(Principal::Book(BOOK)));
    clock.advance(Duration::from_secs(1));
    assert_eq!(gate.principal(&p), None);
    assert!(on_probation(gate.admit(p, address(1))));
}

#[test]
fn an_unknown_book_is_read_from_the_chain_once_a_minute_per_peer_and_address() {
    let clock = Virtual::install(WALL);
    let gate = gate();
    let (p, neighbour, elsewhere) = (peer(), peer(), peer());
    gate.admit(p, address(3)).unwrap();
    gate.admit(neighbour, address(3)).unwrap();
    gate.admit(elsewhere, address(4)).unwrap();
    assert!(gate.may_read_book(&p));
    assert!(!gate.may_read_book(&p));
    assert!(!gate.may_read_book(&neighbour), "same address");
    assert!(gate.may_read_book(&elsewhere));
    clock.advance(Duration::from_secs(60));
    assert!(gate.may_read_book(&neighbour));
}
