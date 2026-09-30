//! Scheduling regressions for real client fan-in; the unchanged daemon gate checks proofs.
#![allow(clippy::unwrap_used)]
#[path = "../src/finalizer_discovery.rs"]
mod discovery;
use discovery::{Candidate, Discovery};
use libp2p::{PeerId, identity};
use std::collections::{BTreeMap, BTreeSet};

fn keys() -> [Vec<u8>; 3] {
    [
        "029ff21ef1bf7a7cb660936ee53367cbdcf05024eacb9f84039617f10c9b5b8cf5",
        "03164ff60a23de578188a29c6ba734db557d3470dbc59b12d26823eed89de673dd",
        "039fc57f49fa281efe849e2f3db70c7266e4ec64ca9980942ccc415f85cd7014b3",
    ]
    .map(|key| hex::decode(key).unwrap())
}
fn peers(count: usize) -> Vec<PeerId> {
    let mut peers = (0..count)
        .map(|_| identity::Keypair::generate_ed25519().public().to_peer_id())
        .collect::<Vec<_>>();
    peers.sort();
    peers
}
struct Outcome {
    found_at: Option<u64>,
    attempts: Vec<(u64, PeerId, Vec<u8>)>,
}
fn run(
    peers: &[PeerId],
    preferred: &BTreeSet<PeerId>,
    real_operator: PeerId,
    held: Option<PeerId>,
    duration: u64,
) -> Outcome {
    let keys = keys();
    let all = peers
        .iter()
        .flat_map(|peer| keys.iter().map(move |key| ([1; 32], 1, key.clone(), *peer)))
        .collect::<Vec<Candidate>>();
    let mut scheduler = Discovery::default();
    let mut last_peer = BTreeMap::<PeerId, u64>::new();
    let mut last_global = 8_000;
    let mut result = Outcome {
        found_at: None,
        attempts: vec![],
    };
    for tick in 0..=duration / 100 {
        let now = 10_000 + tick * 100;
        if now - last_global < 2_000 {
            continue;
        }
        let mut candidates = all.clone();
        candidates.rotate_left(tick as usize % all.len());
        if tick % 2 == 0 {
            candidates.reverse();
        }
        candidates.extend(std::iter::repeat_n(all[0].clone(), 8));
        if preferred.is_empty() {
            scheduler.order(&mut candidates);
        } else {
            scheduler.order_preferred(&mut candidates, preferred);
        }
        let Some(candidate) = candidates.into_iter().find(|c| {
            Some(c.3) != held && now - last_peer.get(&c.3).copied().unwrap_or(0) >= 6_000
        }) else {
            continue;
        };
        assert!(now - last_global >= 2_000);
        assert!(now - last_peer.get(&candidate.3).copied().unwrap_or(0) >= 6_000);
        scheduler.sent(&candidate);
        result
            .attempts
            .push((now - 10_000, candidate.3, candidate.2.clone()));
        last_peer.insert(candidate.3, now);
        last_global = now;
        // Only this actual operator owns the last selected key. All other replies
        // are unavailable; an owner-supplied dial hint never establishes a route.
        if candidate.3 == real_operator && candidate.2 == keys[2] {
            result.found_at.get_or_insert(now - 10_000);
        }
    }
    assert!(result.attempts.len() <= duration as usize / 2_000 + 1);
    result
}

#[test]
fn selected_dial_hint_is_found_amid_thirteen_ordinary_clients_before_ingress_deadline() {
    let peers = peers(14);
    let selected = peers[13];
    let result = run(&peers, &BTreeSet::from([selected]), selected, None, 60_000);
    assert!(
        result.found_at.is_some_and(|time| time <= 20_000),
        "selected peer starved: {:?}",
        result.attempts
    );
    assert!(
        result.attempts.iter().any(|(_, peer, _)| *peer != selected),
        "dial hints monopolized discovery"
    );
}

#[test]
fn wrong_bootstrap_hint_cannot_starve_an_unhinted_operator_under_continuous_load() {
    let peers = peers(14);
    let ordinary_bootstrap = peers[0];
    let selected = peers[13];
    let result = run(
        &peers,
        &BTreeSet::from([ordinary_bootstrap]),
        selected,
        None,
        180_000,
    );
    assert!(
        result.found_at.is_some(),
        "unhinted selected peer starved: {:?}",
        result.attempts
    );
    let observed = result
        .attempts
        .iter()
        .map(|(_, peer, key)| (*peer, key.clone()))
        .collect::<BTreeSet<_>>();
    for peer in &peers[1..] {
        for key in keys() {
            assert!(
                observed.contains(&(*peer, key)),
                "ordinary dial hint starved a fallback peer/key"
            );
        }
    }
}

#[test]
fn pending_preferred_peer_does_not_hold_back_other_connected_operators() {
    let peers = peers(4);
    let selected = peers[3];
    let result = run(
        &peers,
        &BTreeSet::from([peers[0]]),
        selected,
        Some(peers[0]),
        35_000,
    );
    assert!(
        result.found_at.is_some(),
        "pending dial hint stalled fallback discovery"
    );
    assert!(result.attempts.iter().all(|(_, peer, _)| *peer != peers[0]));
}
