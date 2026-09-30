use super::*;

#[test]
fn event_wakeup_fires_once_per_lapsed_deadline() {
    let now = Instant::now();
    let past = now - Duration::from_secs(1);
    // A lapsed, unserviced deadline earns an out-of-tick pump.
    assert!(event_wakeup_due(Some(past), None, now));
    // The boundary is inclusive: a deadline lapsing exactly now still fires.
    assert!(event_wakeup_due(Some(now), None, now));
    // The same instant already serviced does not refire: lanes legitimately
    // hold a lapsed deadline while work is in flight or blocked, and refiring
    // after every event starved the loop (A04 freeze).
    assert!(!event_wakeup_due(Some(past), Some(past), now));
    // A different lapsed instant is a newly armed deadline and fires again;
    // real re-arms always move forward, so the next due instant is later.
    assert!(event_wakeup_due(
        Some(past + Duration::from_millis(500)),
        Some(past),
        now
    ));
    // Future deadlines and empty lanes never earn an early pump, and a stale
    // serviced record after lanes go quiet cannot fire by itself.
    assert!(!event_wakeup_due(
        Some(now + Duration::from_secs(1)),
        None,
        now
    ));
    assert!(!event_wakeup_due(None, None, now));
    assert!(!event_wakeup_due(None, Some(past), now));
}
