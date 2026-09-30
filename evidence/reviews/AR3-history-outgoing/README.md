# Durable outgoing manifest prerequisite acceptance

Core now inventories current live originals independently of transient jobs and
delivery ACK/outbox state, and exposes the retained outgoing history CAS revision
after anchor expiry. The outgoing paid ledger stores one exact manifest and
sorted local index ACK set alongside its original payment. Successors clear ACKs;
stale hashes, wrong paid peers, missing index promises and expired/untrusted
anchors cannot advance progress. [Contract](../../../spec/custody-history-outgoing-v1.md).

[The critic](critic.md) required two test revisions and then ACCEPTed before
production. [Baseline-1](baseline-1.json) and [baseline-2](baseline-2.json) preserve
compilation RED for absent APIs. [Candidate-1](candidate-1.json) passed Core and
frontend but failed paid-store/clippy on an internal method visibility error.
The production visibility correction yielded [candidate-2](candidate-2.json):
**60 backend /21 frontend**, changed-library Clippy and format checks, with
**610 unchanged source inputs**. Final counts are 21 Core history and 39 paid-index
tests; seven new tests exercise the new prerequisite. Earlier runs are not summed.
Candidate-1's frontend count field was zero because the initial report parser only
understood Rust output; its preserved log passed 21. The runner now parses Vitest.

Tests use real MLS and encrypted SQLite plus genuine paid data/index/QC fixtures.
They cover delivery then cold inventory, directional/epoch isolation, expiry CAS,
historical public trust without a wallet, exact no-write retries, data-only refusal,
successor ACK invalidation, SQL stage/ACK/read/update failure, precise quota and
corrupt cold signature/ACK metadata. Raw logs remain in managed `output/`; reports
retain their hashes and frozen input maps.

No native scheduling path changed and no new native run is claimed. The ordinary
runtime still intersects index rosters. Next: integrate sender manifest put/ACK and
pointer-bound recipient traversal together; then genuinely funded disjoint-book
recovery, Welcome/control, successful job retirement and autonomous R10. AR-R03 and
full V1 remain open; 67 cards /22 E2E /three platforms remain required. No push.
