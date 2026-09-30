# Independent backend test critic

Reviewer `/root/index_holder_test_critic`, originally spawned without inherited
context, completed read-only reviews before production changes.

First verdict: **REVISE**. The byte-limit test attempted to move Core time backward
after full anchor verification; live reference preservation only checked omission;
same-direction wrong-anchor and capability-read missing-trust checks were absent.

Corrections: size failure and success use one time; a genuine signed newer
manifest changes the non-anchor descriptor commitment while retaining operation
and passing general crypto verification, then storage must reject it without SQL
changes; a genuine other anchor in the same direction is rejected; a valid read
capability with untrusted Core is rejected. Other-epoch capability coverage was
added. Final review: **ACCEPT**, no mandatory missing scenarios for this scope.

The suggested existing `custody_operator::resign_document` helper was then reused
instead of duplicate document signing. A separate follow-up confirmed **ACCEPT**
for that unchanged mutation/oracle. Final accepted test SHA256:

- `crates/postage-spend/tests/support/index_history.rs`: `1ae164ea7ca82b5aecb7e6c36cc2ab6ce22d892152afd03dcb1d3eb12701ad04`
- `crates/postage-spend/tests/paid_index.rs`: `a19ccbb1e492194ef89498bd050be615df4493211b45bbec64de205e89855add`

Actual current-input baseline 3 completed before production: 34 E0599 absent
storage APIs, no source changes. Earlier baselines are retained separately.
All are compilation RED, not executed behavior failures. No native, multi-book,
MLS, completeness or automatic delivery acceptance is implied by these reviews.
