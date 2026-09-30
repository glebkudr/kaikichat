#![allow(clippy::unwrap_used)]
use super::bootstrap_support::{Admission, Hint, Schedule};
use super::*;

fn hint(port: u16) -> Hint {
    let peer = identity::Keypair::generate_ed25519().public().to_peer_id();
    Hint {
        peer,
        addresses: vec![
            format!("/ip4/127.0.0.1/tcp/{port}/p2p/{peer}")
                .parse()
                .unwrap(),
        ],
        root: None,
    }
}

#[test]
fn bootstrap_scheduler_bounds_merged_sources_and_keeps_slots_until_each_request_finishes() {
    let now = Instant::now();
    let explicit: Vec<_> = (1..=4).map(hint).collect();
    let mut queue = Schedule::new(explicit.clone(), now);
    // Duplicate source hints must not schedule parallel requests to the same PeerID.
    let cached: Vec<_> = explicit.iter().cloned().chain((5..=80).map(hint)).collect();
    queue.replace_cached(cached.clone(), now);
    assert!(queue.len() <= 68);
    let first = queue.ready(now, false);
    assert_eq!(first.len(), 4);
    assert!(
        explicit
            .iter()
            .all(|e| first.iter().any(|h| h.peer == e.peer)),
        "explicit fallback sources must survive cache pressure"
    );
    assert!(
        queue.ready(now + Duration::from_secs(30), false).is_empty(),
        "unanswered requests still own their four slots"
    );
    queue.replace_cached(cached, now + Duration::from_secs(30));
    assert!(
        queue.ready(now + Duration::from_secs(30), false).is_empty(),
        "a refresh cannot forget in-flight work"
    );
    queue.finished(first[0].peer, false, now + Duration::from_secs(30));
    let next = queue.ready(now + Duration::from_secs(30), false);
    assert_eq!(next.len(), 1);
    assert!(first.iter().all(|h| h.peer != next[0].peer));
    for h in &first[1..] {
        queue.finished(h.peer, true, now + Duration::from_secs(30));
    }
    let next_three = queue.ready(now + Duration::from_secs(30), false);
    assert_eq!(next_three.len(), 3);
    let mut active: HashSet<_> = next_three.iter().map(|h| h.peer).collect();
    assert!(active.insert(next[0].peer));
    assert_eq!(active.len(), 4);
}

#[test]
fn bootstrap_scheduler_retries_after_backoff_without_spinning_or_starving_other_hints() {
    let now = Instant::now();
    let only = hint(9000);
    let mut queue = Schedule::new(vec![only.clone()], now);
    assert_eq!(queue.ready(now, false).len(), 1);
    queue.finished(only.peer, false, now);
    assert!(
        queue
            .ready(now + Duration::from_millis(499), false)
            .is_empty()
    );
    assert_eq!(
        queue.ready(now + Duration::from_millis(500), false).len(),
        1
    );
    queue.finished(only.peer, false, now + Duration::from_millis(500));
    queue.replace_cached(vec![only.clone()], now + Duration::from_millis(600));
    assert!(
        queue
            .ready(now + Duration::from_millis(1499), false)
            .is_empty(),
        "refresh cannot reset retry delay"
    );
    assert_eq!(
        queue.ready(now + Duration::from_millis(1500), false).len(),
        1
    );
    queue.finished(only.peer, true, now + Duration::from_millis(1500));
    assert!(
        queue.ready(now + Duration::from_secs(2), false).is_empty(),
        "success must not cause a request loop"
    );
    assert_eq!(
        queue.ready(now + Duration::from_secs(62), false).len(),
        1,
        "successful records are refreshed within a finite interval"
    );
    let mut time = now + Duration::from_secs(62);
    for _ in 0..12 {
        queue.finished(only.peer, false, time);
        assert!(
            queue
                .ready(time + Duration::from_millis(499), false)
                .is_empty()
        );
        time += Duration::from_secs(30);
        assert_eq!(
            queue.ready(time, false).len(),
            1,
            "backoff must remain capped at30seconds"
        );
    }
}

#[test]
fn bootstrap_admission_limits_one_abusive_peer_and_total_work_then_restores_capacity() {
    let now = Instant::now();
    let mut gate = Admission::<64, 8>::new(now);
    let peers: Vec<_> = (1..=9).map(hint).map(|h| h.peer).collect();
    for _ in 0..8 {
        assert!(gate.allow(peers[0], now));
    }
    for _ in 0..20 {
        assert!(!gate.allow(peers[0], now));
    }
    for peer in &peers[1..8] {
        for _ in 0..8 {
            assert!(
                gate.allow(*peer, now),
                "one abusive peer cannot consume other peers' allowance"
            );
        }
    }
    assert!(
        !gate.allow(peers[8], now),
        "the65th admitted verification exceeds global work budget"
    );
    assert!(!gate.allow(peers[8], now + Duration::from_secs(59)));
    assert!(gate.allow(peers[8], now + Duration::from_secs(60)));
    assert!(gate.allow(peers[0], now + Duration::from_secs(60)));
}

#[test]
fn admission_reports_only_the_five_busiest_requesters() {
    let now = Instant::now();
    let mut gate = Admission::<64, 16>::new(now);
    let peers = (1..=6).map(hint).map(|hint| hint.peer).collect::<Vec<_>>();
    for (peer, count) in peers.iter().zip(1..=6) {
        for _ in 0..count {
            assert!(gate.allow(*peer, now));
        }
    }

    let top = gate.top_peers(5);
    assert_eq!(top.len(), 5);
    assert_eq!(
        top.iter().map(|(_, count)| *count).collect::<Vec<_>>(),
        [6, 5, 4, 3, 2]
    );
    assert_eq!(top[0].0, peers[5]);
    assert!(!top.iter().any(|(peer, _)| *peer == peers[0]));
}

#[test]
fn bootstrap_disconnect_retries_healthy_peer_without_bypassing_backoff_or_active_slots() {
    let start = Instant::now();
    let first = hint(9401);
    let other = hint(9402);
    let mut queue = Schedule::new(vec![first.clone(), other.clone()], start);
    assert_eq!(queue.ready(start, false).len(), 2);
    queue.finished(first.peer, true, start);
    queue.finished(other.peer, true, start);
    let down = start + Duration::from_secs(1);
    queue.disconnected(first.peer, down);
    queue.disconnected(first.peer, down + Duration::from_millis(100));
    queue.disconnected(hint(9403).peer, down);
    assert_eq!(
        queue.len(),
        2,
        "disconnect cannot invent a discovery source"
    );
    assert!(
        queue
            .ready(down + Duration::from_millis(499), false)
            .is_empty()
    );
    let retry_at = down + Duration::from_millis(500);
    let retry = queue.ready(retry_at, false);
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].peer, first.peer);
    // A close while the request is active cannot release its slot or create a duplicate.
    queue.disconnected(first.peer, retry_at + Duration::from_millis(100));
    assert!(
        queue
            .ready(retry_at + Duration::from_millis(200), false)
            .is_empty()
    );
    let failed = retry_at + Duration::from_millis(300);
    queue.finished(first.peer, false, failed);
    queue.disconnected(first.peer, failed + Duration::from_millis(10));
    queue.replace_cached(vec![first.clone()], failed + Duration::from_millis(20));
    assert!(
        queue
            .ready(failed + Duration::from_millis(999), false)
            .is_empty()
    );
    let second = queue.ready(failed + Duration::from_secs(1), false);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].peer, first.peer);
    queue.finished(first.peer, true, failed + Duration::from_secs(1));
    assert!(
        queue
            .ready(start + Duration::from_secs(29), false)
            .is_empty()
    );
    let refresh = queue.ready(start + Duration::from_secs(30), false);
    assert_eq!(refresh.len(), 1);
    assert_eq!(
        refresh[0].peer, other.peer,
        "unaffected healthy peer keeps normal refresh"
    );

    let crowded: Vec<_> = (9501..=9505).map(hint).collect();
    let mut slots = Schedule::new(crowded[..4].to_vec(), start);
    slots.replace_cached(vec![crowded[4].clone()], start);
    let active = slots.ready(start, false);
    assert_eq!(active.len(), 4);
    assert!(active.iter().all(|p| p.peer != crowded[4].peer));
    slots.disconnected(active[0].peer, start + Duration::from_millis(100));
    assert!(
        slots
            .ready(start + Duration::from_secs(1), false)
            .is_empty(),
        "disconnect must not free an active slot for the fifth ready peer"
    );
    slots.finished(active[0].peer, false, start + Duration::from_secs(1));
    let admitted = slots.ready(start + Duration::from_secs(1), false);
    assert_eq!(admitted.len(), 1, "only completion releases one slot");
    assert_eq!(admitted[0].peer, crowded[4].peer);
    assert!(
        slots
            .ready(start + Duration::from_secs(2), false)
            .is_empty()
    );
}
