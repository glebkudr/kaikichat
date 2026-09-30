# Paid history lifecycle after the R14 review

**Execution is stopped per the user's latest instruction: a detailed plan
only.** The continuation is broken down into
[V1-H01–H11](agentic_internet_v1_execution_plan/v1-plan-2026-09-14/01-tasks.md);
full V1, including the remaining modules and acceptance, is in the
[completion plan](agentic_internet_v1_execution_plan/v1-plan-2026-09-14/README.md).

Date: 2026-09-14. Status: shared batch collection, graph/pointer stages, and
group placement priority are implemented. The slow collector's arrival fences
and the Core range API were accepted at the component level; ordinary Node
integration, the unified pending-body, and full native acceptance remain
open. The basis is the
[provided architect review](../evidence/reviews/AR2-wallet-flow/r14-architecture/architect-review.md),
[reproduced measurements](../evidence/reviews/AR2-wallet-flow/r14-architecture/structural-measurements.json),
and a [cross-check against local data and sources](../evidence/reviews/AR2-wallet-flow/r14-architecture/recheck.json).
The review's recommendations are separated here from confirmed results.

## Confirmed

The architect's script, checked before running, reproduced all of its
structural measurements on the saved local R14. This is a repetition of the
author's algorithm, not a new independent cryptographic verification and not
a native run. At the time of the review, all 21 provided files with code
matched the project. They describe the existing implementation; sources were
not replaced automatically.

| Observation | Consequence for the next work |
| --- | --- |
| 117 leaves for 130 originals: 106 × 1, 9 × 2, 2 × 3 | Fix the actual merging of already queued sends |
| The current graph contains 230 pages; the full representation of the cache of its non-root pages is 857593 bytes against a limit of 1048576 | Enlarging the cache is not justified by this calculation; it is a representation estimate, not a measured peak |
| Prefetch/deferred overlap on 8 operations; prefetch contains already-imported 1..24 | Eliminate re-storing and re-accepting already imported bodies |
| The two caches hold 83 unique originals, 89 together with the import | Queue size sums must not be passed off as history coverage |
| Publication took 2407 of the original retention's 3600 seconds | Measure the total publication and recovery budget |
| In `advance_custody_sync` the deadline is checked before applying a ready `index_output` | A quantum-boundary regression is needed; an earlier partial ciphertext save does not imply applying the whole result |
| On ReceiveGap deferred rotates the queue and persists it without an MLS change | Retry must depend on substantive changes; an SQL fault requires a separate retry |

Original 31 is the thirtieth graph reference, not the array element with
index 30. Surviving data/index holders were reconciled by the exact operation
in `remaining` after removals. The graph ordinal defines the proof traversal,
not the MLS import order. Original 31 expires 261 seconds later than the
first original; the earliest-expiry gate firing does not prove its own expiry
at that moment.

## What remains unknown

R14 does not show the full request path of 31 and does not establish the
cause of its absence. Deferred rotation does not explain the body's absence
in both caches. 117 prepared root versions do not mean 117 fully published
roots; the existing graph reuses live subtrees. Location ACK can arrive in
batches, so 13000 confirmations must not be counted as 13000 RPC. The 343
seconds of status RPC waits is caller-side time, not proven eliminable CPU
cost. The cause of the R13 publication failure remains open.

## Implementation order

### 0. Record the baseline behavior

The pre-instrumentation snapshot is saved: 961 input files, the archive, and
hashes are in `output/ar2-wallet-flow/diagnostic32-baseline`.
[A separate critic accepted](../evidence/reviews/AR2-wallet-flow/diagnostic32-test-review.json)
Diagnostic32, the oracle, and the observational emit-sites. Target checks
pass: 122 backend, 47 frontend, 3 parser tests, Node all-targets Clippy, and
fmt. These results do not yet confirm a native trace and the recovery of 32.

The diagnostic example includes JSON with time since process start and
process/Work bindings. The existing Job keeps only a tracing span to link a
real response to the request; Core results, deadlines, fallback, and quota
are saved. Events show staging, import, an unapplied response on deadline,
deferred retry, and admission wait reasons. The regular entrypoint subscriber
is not included. Diagnostic formatting and copying are protected against
teardown interruption. The source/execution guard reconciles the sources and
the actual node/CLI together with their release-origin even when the scenario
fails. Native32 explicitly includes the existing full cold-graph check; the
former Native20/Full130 conditions are preserved.

The `output/hrt32-r1` baseline used separate node/CLI copies from
`output/history-read-trace-bin`; the manifest, source archives, and binary
provenance are saved in `output/ar2-wallet-flow/diagnostic32-baseline`. The
run ended with a
[first SQL gate failure](../evidence/reviews/AR2-wallet-flow/diagnostic32-native-r1.json).
All 32 originals were published in 542 seconds: 96 signatures, 320 data /
320 index copies, and 3200 location ACK. 288 + 288 copies were actually
removed. 23 leaves survived: 16 single, five double, two triple. The
recipient, after establishing trust, made 48 reads; within 120 seconds it did
not reach the first import SQL transaction. Counters show zero imports and
storageErrors, 316 deferred attempts without import. For originals 1 and 31,
both reference selections in each case ended in a global-read-rate wait and
an admission yield before enqueue. This is a Diagnostic32 observation, not an
established cause of the Full130 R14 and not proof of the body's absence
from the cache: a bulk response may contain other operations.

The guard confirmed the immutability of sources and binaries throughout the
run; cleanup completed without errors, no processes or temporary profiles
remained. Afterward, a diagnostic mapping bug was fixed: the ordinal in Node
is zero-based. The original failed report is preserved; the
[reproducible analysis](../evidence/reviews/AR2-wallet-flow/analyze-diagnostic32.py)
re-checks the entire partial trace and explicitly leaves nativePassed=false.
A full successful trace still requires import; partial mode does not disable
the linkage, process, Work, time, and event-order checks. Five parser tests
and a repeat 47 frontend pass. These are baseline results before the step
1–3 behavior changes; the current result of the first component is described
below.

One shared-path trace links Work, root/index/epoch, operation, graph ordinal,
peer, request kind, and monotonic time. Needed: the reference → enqueue →
response → durable staging → import/rollback transitions, admission wait
reasons, fallback, and the count of new/retried bodies. Record events for the
other operations consuming the shared budget too. Do not add diagnostic RPC;
do not include plaintext, capabilities, or secret keys in the trace.

### 1. A single coordination of history publication

In `public_sender.rs`, `public_sender_history.rs`, `custody_history_sender.rs`,
reuse the existing Core batch preparation and paid publication queue. The
history graph and pointer are coordinated for
`(conversation, index_id, epoch)`. Payment, custody obligations, and the
delivery ledger remain individual.

A bounded group of already waiting sends first advances data/index placement,
then ready members land in a shared leaf. The limit of 12 and the pass time
bound are kept; group collection may continue between passes. Do not wait for
messages not yet created. A stuck send must not hold ready ones indefinitely;
use the existing attempt/error bounds. A single send keeps advancing
immediately. Root and pointer still complete only after canonical durable
ACK.

The target structure for two regular batches of 16 is at most 4 leaves; for
the whole 130 scenario the benchmark is 17. These numbers are so far a
verifiable hypothesis, not an achieved result. The spend, graph, and shared
transport formats are preserved.

The batch-collection regression uses `public_sender_batch_tests` and real
paid receipts from `custody_history_paid_batch_tests::Fixture`. The former
test with expensive optional admission confirmed only exit from a single
pass and produced two small leaves. The new expectation: a short pass keeps
the accumulated group, and subsequent calls from different jobs of the same
conversation continue it. A signed-graph traversal must confirm all source
operations exactly once and no more than two leaves for 16 simultaneously
ready originals. Scheduler counters and readiness hints are not by themselves
such proof.

Separately, verify suspension/revocation of credentials after inclusion in
the waiting group, a stuck placement of one member, a single send, and a real
SQL error before commit with a cold retry. Root preparation does not complete
any delivery ledger. With the shared publisher, each job still passes its own
current-credential check and `record_public_sender_progress`; Core reconciles
its membership, exact root, and pointer before removing the job from the
active queue. Network ACK and duration remain a separate native check.

Shared collection of an already-ready group is implemented:
[boundaries and evidence](../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_COLLECTION.md).
The first variant, with a repeat full verification of all members before
commit, was rejected by the critic; a RED control of real full calls over the
whole slow pass was added. Core and CustodyStore now emit separate opaque
verification results for unchanged inputs. The final stretch checks the
current bytes, queue, grant/trust, time, and original/receipt expiry. A
changed member returns to a bounded full verification. The implementation and
the legacy regression received an independent `FINAL ACCEPT`; run results are
kept in [checks](../evidence/reviews/AR2-wallet-flow/shared-collection-checks.json).

Graph/pointer stages were moved from separate Work into a shared map keyed by
the exact history key:
[implementation and checks](../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_PUBLISHER.md).
A caller change continues the traversal; a new root does not inherit old
ACK/pointer. An SQL failure keeps the retry; a cold restart creates a new
attempt with the same signed sequence. A separate Core observation of each
job is preserved.

[Bounded-group placement priority](../evidence/reviews/AR2-wallet-flow/PLACEMENT_PRIORITY.md)
is implemented in ordinary maintenance. Up to 12 already queued members get
priority within their queue positions; other conversations keep their places.
Ready ones wait for full group placement or the expiry of the former
5-second no-progress window. New arrivals do not extend it; the singleton
keeps the usual 500 ms. Tests verify real QC/receipts, first leaf 4, leaves
12+1, a stalled member, late recovery, and replacement on owner pause. An
independent critic accepted the code; 42 backend / 63 frontend, Clippy/fmt
pass.

[The collector under new reservations](../evidence/reviews/AR2-wallet-flow/ARRIVAL_FENCES.md)
is now verified by a separate expensive scenario: four paid originals form a
single leaf with real new send/prepare between quanta. Core preserves the
exact immutable BOOK/POLICY inputs and hashes INTENTS/own rows, but allows
monotonic growth of allocations/reserved within the verified book and current
sponsorship. Rollback, exceeding limits, changes to funding/config/authority,
and pause are rejected. The critic accepted the tests and the code; 80 unique
backend / 63 frontend, Clippy/fmt pass. One publisher test failure was a
legacy route conflict in the fixture; after moving to the existing versioned
records, all four publisher scenarios pass. The original unsuccessful log is
preserved.

Reservation growth in an existing book was verified. Continuous replacement
of INTENTS, funding, or policy authority requires a new full visit; liveness
under such changes is not claimed. The full proof path and cooperative clock
counters do not measure 20 ms or native throughput. The next step is read
continuation.

### 2. Read continuation survives the quantum

Keep the Work bounds of 120 seconds / 16 visits as scheduling limits. Before
releasing a finished quantum, apply the ready result through the existing
checks with current time, credentials, and root/epoch. No new network request
is started beyond the quantum boundary.

Premature discard is
[fixed](../evidence/reviews/AR2-wallet-flow/READY_OUTPUT.md): a ready
response is applied before the remaining deadline check. Four new tests cover
the durable branch cache, failure after a pointer/expiry change, and SQL
rollback with a cold retry of the exact original; 56 former receiver tests,
63 frontend, and five parser tests pass; Clippy/fmt are clean. The critic
accepted the tests and the code. This is a post-paid output boundary, not a
successful funded Noise response. An incomplete root proof is not yet
persisted in Flow memory; the next component prepares Core to persist a range
cursor.

The [Core range API](../evidence/reviews/AR2-wallet-flow/RANGE_CORE.md) now
atomically persists bodies and the holder-local position for the exact
current head. Partial cache fill does not advance the cursor; body UPDATE and
marker INSERT were verified with separate SQL faults. Cold restart, pointer
change under the same root, a skipped reference below the cursor, a 129th
holder without eviction, and the old body format are covered by six new
tests. The critic accepted the tests and the code. The former body cache
keeps 128 bodies / 4 MiB; metadata are separately bounded at 128 holders /
64 KiB. The ordinary Node receiver still calls the old cache API, so network
Work continuation is not proven by this component.

The target Core cluster finished with 64 PASS, including reversed-130 in
1302 seconds; together with 33 former Node checks — 97 backend / 73 frontend
/ 5 parser, Clippy/fmt pass. The new Node range runtime tests of the next
integration are not part of these results.

[Unjustified legacy retry](../evidence/reviews/AR2-wallet-flow/NO_LEGACY_ON_FAILURE.md)
after capacity/unavailable/rejected/transport_failure was removed. A
completed attempt moves to the next holder; the legacy non-history locator
still reads a single body. RED 3 PASS / 3 FAIL, GREEN 6 PASS; real Noise
rejection, legacy, and paid prefix/exact controls are preserved. The critic
accepted the change. Detection of old history holders by an explicit marker
was not added.

In the existing recovery cursor/cache, keep a bounded continuation for the
exact head/root and source. The shared range-read accepts `after_sequence`
and returns a verified `next_sequence`. The cursor advances only after
durable acceptance of all bodies it accounts for: the current cache API
allows partial acceptance, so one successful call is not enough for such an
advance.

Holder-local `complete` does not close the history. An exact skipped
reference can be read by address even below the saved cursor, including
after repair. Congestion, capacity, transport failure, and a generic
`Rejected` are not proof of an old protocol. The legacy reader is kept with a
separate compatibility justification. A root/epoch change is checked at the
apply boundary.

The next integration in Node must keep the verified `next_sequence` and
`complete` in the paid-page verifier result, the exact head in Pending, and
choose `after_sequence` via Core at the real request boundary. Prefix and
Obligation are replaced by a single range-read. A successful full staging up
to the chosen original continues the same holder from the new position;
capacity without full staging must not trigger an endless retry of the same
page. Real paid Noise tests are needed, with Work re-creation and a cold
Core, head changes between enqueue/apply, targeted repair below the cursor,
and unchanged admission/body/proof budgets.

### 3. A single store of bodies awaiting import

Merge the existing prefetch/deferred per operation with a binding to
index/epoch; a body is stored once. Proofs reuse the verified metadata for
the exact obligation and re-run the mutable checks at commit. Imported
operations do not return to staging. Changing the durable format includes
compatible reading of both old states and a lossless cold restart. Keep the
former combined budget of bodies and bytes until a separate justification to
change it; do not replace the two existing limits with one smaller one.

The gap check is repeated after a change in the essential MLS/root state or
the body. The order is the authenticated sender stream and epochs, not a
single sequence across all group members. Collection of further bodies and
proofs continues across a gap. An SQL fault allows a retry without a prior
MLS change. The message, MLS, import record, pending-body removal, and
recovery progress remain one `receive_verified_with_states` transaction.

## Acceptance

For every backend change, tests first, a separate backend-test-critic with
no inherited context and its final acceptance; then production code. Target
regressions: a ready response at the 120-second boundary, the pending stream,
current expiry/root/epoch, full staging without losing cursor/body, no
gap-rotation on unchanged state, SQL rollback/retry, cold migration, a batch
with a delayed member, and a single send. Backend/frontend checks run through
build-storage; the whole suite at the end of the plan.

Baseline and candidate Diagnostic32 use identical tracing and shared
ordinary CLI/Core paths. Mandatory: real 288+288 losses, no sender,
trust-refusal, both exact SQL fault/rollback stages, partial/full and cold
cycle. TTL, deadlines, and allowances are preserved. Verify the absence of
lost ready results, a prefix restart from scratch only due to a Work change,
SQL rotation without MLS progress, and pending duplicates.

The architect's proposal to cut admitted recovery reads at least in half is
an engineering efficiency criterion for the hypothesis, not a new product
promise. Compare it against full correctness and the entire scenario
duration. If the structure has tightened without a notable reduction in
reads, first find the remaining cost in the trace. The success of 32 does not
confirm the time budget of 130.

Then the unchanged Full130: 390 signatures, all receipts/location ACK, real
1170+1170 losses, trust refusal, first rollback, partial 129, last rollback,
full130, and a cold full graph up to the original earliest expiry. Actual
execution hashes and clean cleanup are required. R10 remains the control.

## Availability boundary and full V1

In the current fixture the surviving index is chosen among four advertised
routes. This is a valid test case, but not proof of discovery under any nine
out of ten losses. A broader guarantee needs a verifiable path for
discovering all admissible survivors, for example via the existing placement;
the current Full130 keeps its conditions. This gap does not explain the R14
failure.

After eliminating repeated work, the whole cycle must fit within 3600
seconds. If the mandatory work still does not fit, an explicit product
decision on resources/volume/availability conditions is required. In tests
the time limit is not extended. None of the steps automatically closes the 67
tasks / 22 E2E / three OSes. All of AR1–AR5 and the remaining full user
scenarios stay in the plan.
