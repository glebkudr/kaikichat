//! Shared processing budgets. Actual codecs and authenticated connections are exercised
//! by the TCP/QUIC daemon gate; these tests isolate resource lifetime and weighted limits.
use super::*;

const LARGE: usize = 16 * 1024 * 1024 + 1024; // Existing public postage response bound.

#[test]
fn ordinary_readers_cannot_consume_selected_capacity_and_a_peer_cannot_consume_all_of_it() {
    let mut limits = Limits::new(2);
    let budget = limits.processing();
    let signal = Arc::new(AtomicBool::new(true));
    let selected = [peer(), peer(), peer(), peer()];
    let time = now().unwrap();
    limits
        .replace(selected.iter().map(|p| grant(*p, &signal, time)).collect())
        .unwrap();
    let mut ordinary = (0..16)
        .map(|_| budget.acquire(peer(), 8192).unwrap())
        .collect::<Vec<_>>();
    assert!(budget.acquire(peer(), 8192).is_err());
    let mut service = Vec::new();
    for p in &selected[..3] {
        for _ in 0..16 {
            service.push(budget.acquire(*p, 34816).unwrap());
        }
        assert!(
            budget.acquire(*p, 34816).is_err(),
            "one selected peer monopolized the reserve"
        );
    }
    assert!(
        budget.acquire(selected[3], 1).is_err(),
        "global selected capacity was bypassed by a fresh peer"
    );
    assert_eq!(budget.info()["active"], 64);
    assert_eq!(budget.info()["ordinaryActive"], 16);
    assert_eq!(budget.info()["selectedActive"], 48);
    assert!(budget.acquire(selected[0], 1).is_err());
    ordinary.pop();
    let replacement = budget.acquire(peer(), 8192).unwrap();
    assert_eq!(budget.info()["active"], 64);
    drop(replacement);
    service.pop();
    let replacement = budget.acquire(selected[2], 34816).unwrap();
    assert_eq!(budget.info()["selectedActive"], 48);
    drop(replacement);
    drop(service);
    drop(ordinary);
    assert_eq!(budget.info()["active"], 0);
    assert_eq!(budget.info()["ordinaryBytes"], 0);
    assert_eq!(budget.info()["selectedBytes"], 0);
    assert_eq!(budget.info()["peers"], json!([]));
}

#[test]
fn large_receipt_responses_obey_shared_byte_limits_even_when_reader_slots_remain() {
    let mut limits = Limits::new(2);
    let budget = limits.processing();
    let signal = Arc::new(AtomicBool::new(true));
    let selected = [peer(), peer(), peer()];
    let time = now().unwrap();
    limits
        .replace(selected.iter().map(|p| grant(*p, &signal, time)).collect())
        .unwrap();
    let mut ordinary = (0..4)
        .map(|_| budget.acquire(peer(), LARGE).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(budget.info()["ordinaryActive"], 4);
    assert!(
        budget.acquire(peer(), 1).is_err(),
        "free reader slots bypassed the byte ceiling"
    );
    let first = budget.acquire(selected[0], LARGE).unwrap();
    let second = budget.acquire(selected[0], LARGE).unwrap();
    assert!(
        budget.acquire(selected[0], 1).is_err(),
        "one peer exceeded its shared byte allowance"
    );
    let third = budget.acquire(selected[1], LARGE).unwrap();
    let fourth = budget.acquire(selected[1], LARGE).unwrap();
    assert!(budget.acquire(selected[2], 1).is_err());
    assert_eq!(budget.info()["selectedBytes"], 4 * LARGE);
    ordinary.pop();
    let small = budget.acquire(peer(), 34816).unwrap();
    assert_eq!(budget.info()["ordinaryBytes"], 3 * LARGE + 34816);
    drop((first, second, third, fourth, small, ordinary));
    assert!(budget.acquire(peer(), 2 * LARGE + 1).is_err());
    assert!(budget.acquire(peer(), 0).is_err());
    assert_eq!(budget.info()["active"], 0);
}

#[test]
fn processing_clones_share_capacity_across_protocols_and_connections_without_peer_bypass() {
    let limits = Limits::new(2);
    let frames = limits.processing();
    let bootstrap = limits.processing();
    let p = peer();
    let a = frames.acquire(p, 34816).unwrap();
    let b = bootstrap.acquire(p, 8192).unwrap();
    let c = frames.acquire(p, 34816).unwrap();
    let d = bootstrap.acquire(p, 8192).unwrap();
    assert!(frames.acquire(p, 1).is_err());
    assert!(bootstrap.acquire(p, 1).is_err());
    assert_eq!(frames.info()["active"], bootstrap.info()["active"]);
    assert_eq!(frames.info()["ordinaryBytes"], 2 * (34816 + 8192));
    drop((b, c));
    let e = bootstrap.acquire(p, 8192).unwrap();
    assert_eq!(frames.info()["active"], 3);
    drop((a, d, e));
    assert_eq!(bootstrap.info()["active"], 0);
}

#[test]
fn revocation_invalidates_inflight_service_io_without_freeing_its_physical_allocation_or_reviving_it()
 {
    let mut limits = Limits::new(2);
    let budget = limits.processing();
    let p = peer();
    let authority = Arc::new(AtomicBool::new(true));
    let first = Arc::new(AtomicBool::new(true));
    let second = Arc::new(AtomicBool::new(true));
    let time = now().unwrap();
    limits
        .replace(vec![
            Reservation::client(p, authority.clone(), first.clone(), time, time + 60).unwrap(),
            Reservation::client(p, authority.clone(), second.clone(), time, time + 60).unwrap(),
        ])
        .unwrap();
    let ordinary = (0..16)
        .map(|_| budget.acquire(peer(), 8192).unwrap())
        .collect::<Vec<_>>();
    let held = budget.acquire(p, 34816).unwrap();
    assert!(held.is_live());
    first.store(false, Ordering::Release);
    assert!(held.is_live(), "a second live source was discarded");
    second.store(false, Ordering::Release);
    assert!(!held.is_live());
    assert!(budget.acquire(p, 34816).is_err());
    assert_eq!(
        budget.info()["selectedActive"],
        1,
        "revocation released still-owned memory"
    );
    let fresh = Arc::new(AtomicBool::new(true));
    limits
        .replace(vec![
            Reservation::client(p, authority.clone(), fresh.clone(), time, time + 60).unwrap(),
        ])
        .unwrap();
    assert!(
        !held.is_live(),
        "new request revived an old in-flight token"
    );
    let next = budget.acquire(p, 34816).unwrap();
    assert!(next.is_live());
    authority.store(false, Ordering::Release);
    assert!(!next.is_live());
    assert!(budget.acquire(p, 34816).is_err());
    drop((held, next, ordinary));
    assert_eq!(budget.info()["active"], 0);
    let fallback = budget.acquire(p, 8192).unwrap();
    assert_eq!(budget.info()["ordinaryActive"], 1);
    drop(fallback);
}

#[test]
fn fast_completion_does_not_refill_rate_allowances_and_selected_progress_has_an_independent_allowance()
 {
    let mut limits = Limits::new(2);
    let budget = limits.processing();
    let ordinary = (0..4).map(|_| peer()).collect::<Vec<_>>();
    let selected = (0..5).map(|_| peer()).collect::<Vec<_>>();
    let signal = Arc::new(AtomicBool::new(true));
    let time = now().unwrap();
    limits
        .replace(selected.iter().map(|p| grant(*p, &signal, time)).collect())
        .unwrap();
    for p in &ordinary {
        for _ in 0..64 {
            drop(budget.acquire(*p, 8192).unwrap());
        }
        assert!(budget.acquire(*p, 8192).is_err());
    }
    assert!(budget.acquire(peer(), 8192).is_err());
    for p in &selected[..4] {
        for _ in 0..256 {
            drop(budget.acquire(*p, 34816).unwrap());
        }
        assert!(budget.acquire(*p, 34816).is_err());
    }
    assert!(
        budget.acquire(selected[4], 1).is_err(),
        "global selected rate was bypassed by a fresh peer"
    );
    assert_eq!(budget.info()["active"], 0);
    assert_eq!(budget.info()["startedOrdinary"], 256);
    assert_eq!(budget.info()["startedSelected"], 1024);
    std::thread::sleep(Duration::from_millis(1100));
    drop(budget.acquire(ordinary[0], 8192).unwrap());
    drop(budget.acquire(selected[0], 34816).unwrap());
    assert_eq!(budget.info()["startedOrdinary"], 257);
    assert_eq!(budget.info()["startedSelected"], 1025);
}

#[test]
fn original_signed_deadline_stops_a_held_reader_even_if_its_authority_signal_remains_live() {
    let mut limits = Limits::new(2);
    let budget = limits.processing();
    let signal = Arc::new(AtomicBool::new(true));
    let p = peer();
    let time = now().unwrap();
    limits
        .replace(vec![
            Reservation::new(p, signal.clone(), time, time + 2).unwrap(),
        ])
        .unwrap();
    let held = budget.acquire(p, 8192).unwrap();
    assert!(held.is_live());
    while now().unwrap() < time + 2 {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(signal.load(Ordering::Acquire));
    assert!(!held.is_live());
    assert_eq!(budget.info()["selectedActive"], 1);
    drop(held);
    let ordinary = budget.acquire(p, 8192).unwrap();
    assert_eq!(budget.info()["selectedActive"], 0);
    assert_eq!(budget.info()["ordinaryActive"], 1);
    drop(ordinary);
}
