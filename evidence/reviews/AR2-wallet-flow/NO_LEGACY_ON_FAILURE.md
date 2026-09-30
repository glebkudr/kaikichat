# A holder error does not select the legacy protocol

Date: 2026-09-14. Part of step 2 of [R14](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).

Previously `capacity`, `unavailable`, `rejected` and `transport_failure` after
Prefix/Obligation caused one more request to the same holder via the legacy Read.
These reasons do not confirm the old protocol. Now the processing of a finished
attempt moves to the next holder and keeps the mode chosen by the source's
history. When holders are exhausted, no new legacy retry appears.

Locators without a history commitment still use the old single-body read. A
successful prefix truncation keeps the existing exact bulk retry;
that is a different branch, not error handling. Network admission, proof-bytes
limits, stream count, deadlines, paid-response verification and SQL handling are
unchanged. This fix adds no new way to detect old history holders.

The three existing fallback tests were changed before the production code and
accepted by a separate critic. RED: three PASS controls and three FAILs on the
former same-holder retry. The updated tests check the transition, typed requests
and queue exhaustion. A real `Rejected` comes from a relay-only server over
TCP/Noise; the other failure classes are fed into the existing routing helper as
error events. Real successful paid prefix/exact and non-history legacy reads
remain. This is not a measurement of network failure frequency or native
throughput.

Results and source hashes: [checks](no-legacy-checks.json),
[independent review](no-legacy-test-review.json). The Core range API is checked
[separately](RANGE_CORE.md); the network receiver still needs to be switched to
durable continuation instead of per-Work Prefix/Obligation. V1 and the native
gates remain open.
