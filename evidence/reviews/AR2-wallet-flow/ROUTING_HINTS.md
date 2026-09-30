# Custody discovery optimization — verification in progress

The ordinary sender now requests only data positions without a durable current
receipt, using the same selector already used for indexes. New custody jobs can
start a position's existing bounded candidate window at a recently verified
address. The hint comes from the existing bounded result cache, with matching
checkpoint/domain/epoch and current binding/connection. Every new offer still
requires a fresh network response and the unchanged paid-placement verifier.
After a failed hint, the scan wraps through every candidate exactly once.
Admission quotas, deadlines, paid draws and storage allowances are unchanged.

The independent context-free backend test critic first required fresh-proof and
live-role withdrawal coverage, then returned FINAL ACCEPT before production
changes. Accepted `tests/evm/custody_resolution.py` SHA256:
`972baf4b8146d0e7a0efd6931f9326121d1a0e8f9cc02ae04692f7734ef4b72a`.

- RED failed on the expected warm-lookup assertion: the old code could not
  resolve all 14 positions with exactly 14 requests to 16 connected candidates.
- G1 did not compile because an optional capacity precheck accessed a private
  field. That precheck was removed; the existing put admission remains.
- [G2](routing-hints-native-g2.json) passed warm lookup with 14 actual provider
  responses, withdrawal with 29 queries while the original binding and
  connection remained live, and restoration of all 14 positions on both chains.
  Chain31337 completed the entire existing gate. Chain31338 later failed its
  resolution after returning from relay-only mode: 13/14 positions, 128 queries.
  Cleanup succeeded. This is not a full native pass; failure-only diagnostics
  are being added to distinguish transport churn from existing admission quotas.
- [G3](routing-hints-native-g3.json) failed its first healthy connected lookup
  before the hint assertions: 13/14 proofs, 119 queries. Custody proof admission
  counters were zero at every provider. The client's 50 cumulative transport
  failures also include the earlier deliberate held-stream failures, so they
  do not identify the particular missing response's cause.

The critic accepted a diagnostic-only delta, then a bounded positive-lookup retry
contract consistent with the specification's disposable partial jobs. The concrete
candidate allows at most two ordinary jobs in one 25-second deadline only for
healthy setup/reconnect/head transitions. Every partial uses the original exact
oracle; a single final result must contain all 14 proofs, without merging offers.
All strict hint/withdrawal/negative/cache assertions remain. Attempts and counter
deltas are retained. FINAL ACCEPT candidate SHA256:
`8f4e86a9201f9b15a2f50dea279a82f6dd587dc06acba52476db9cacf4e37156`.
The exact accepted candidate was applied only after native R4 stopped and its
823 inputs were verified unchanged. [Full native G4](routing-hints-native-g4.json)
passed in isolation: both chains, 416 verified positions, warm14 and withdrawal29
on each chain, all old negative/expiry/head/cache gates and empty cleanup errors.
All 16 healthy lookups completed in one attempt in this run. G2/G3 remain recorded;
the passing run does not identify their precise transient transport failure.

31 targeted backend tests, 31 frontend tests, all-target node Clippy and fmt
passed after production changes; [check manifest](routing-hints-checks.json).
The [separate graph fault gate](graph-sender-routing-r1.json) also passed on
823 unchanged inputs: two originals, six independently verified finality
signatures, five graph pages/50 child-before-parent ACKs, SQL faults/cold retry
and sender-absent data/index loss recovery. Cleanup succeeded. The corrected
native 130-original range remains required.
