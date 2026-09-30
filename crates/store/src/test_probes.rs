//! Test-only observation of synchronous ProfileStore state reads on this thread.
//! Counts API reads, including failed/missing reads, not physical SQLite I/O.
//! Payloads, validation, storage and clocks are never changed by this observer.
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static STATE_READS: RefCell<Option<BTreeMap<String, usize>>> = const { RefCell::new(None) };
}

struct ResetObservation;

impl Drop for ResetObservation {
    fn drop(&mut self) {
        STATE_READS.with(|reads| *reads.borrow_mut() = None);
    }
}

/// Observe only the supplied synchronous call, excluding fixture setup/oracles.
/// Nested observations are rejected; unwinding also removes the observer.
pub fn observe_state_reads<T>(call: impl FnOnce() -> T) -> (T, BTreeMap<String, usize>) {
    STATE_READS.with(|reads| {
        let mut reads = reads.borrow_mut();
        assert!(reads.is_none(), "nested state-read observation");
        *reads = Some(BTreeMap::new());
    });
    let reset = ResetObservation;
    let value = call();
    let reads = STATE_READS.with(|reads| reads.borrow_mut().take().unwrap_or_default());
    drop(reset);
    (value, reads)
}

pub(super) fn state_read(namespace: &str) {
    STATE_READS.with(|reads| {
        if let Some(reads) = reads.borrow_mut().as_mut() {
            let count = reads.entry(namespace.to_owned()).or_default();
            *count = count.saturating_add(1);
        }
    });
}
