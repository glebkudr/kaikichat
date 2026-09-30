# V1 after the independent architecture review

**Mode as of September 14: a plan has been prepared; further execution was stopped by the user.**
The [detailed V1 decomposition](agentic_internet_v1_execution_plan/v1-plan-2026-09-14/README.md)
links the remaining scope, both reviews, and the final acceptance. All of the next steps below
remain technical backlog, not instructions to immediately launch code
or agents. The base intermediate commit is `993dece` (`start luna-mix pipeline`).

## Next checks after the current run

The machine matrix now records the accepted Native20 R10 for D05/E05. The cards
remain partial. A Full130 outcome is determined only by a completed
run with verification of actual imports, a cold pass over all 130 references,
unchanged inputs, and process cleanup; publishing 130 messages by itself
does not confirm their recovery.

After that, the user path of the first offline contact remains: currently
`AppCore::add_contact` atomically puts the Welcome into the local outbox, while the receiving
`receive` additionally requires a still-valid, unused invitation. History
recovery relies on an already established MLS conversation and does not deliver a missing
Welcome. A bounded route for storing and discovering the first Welcome is needed,
with an explicit expiry, recipient verification, and protection against re-creating
the contact; this must not be solved by disabling the invitation expiry check.

### Acceptance of the first offline contact (R05; D05/E05 remain open)

Code slice of September 13: `IssuedInvitation` stores expiry and consumed_by;
an invitation is valid for 7 days. `add_contact` atomically stores MLS state,
the contact, and the original Welcome in the direct outbox. `receive_verified_with_states`
accepts additional completion states only for application packets,
while Welcome separately consumes a still-valid invitation and verifies both MLS
identities. Public sender admission explicitly excludes Welcome. Therefore R05
cannot be closed by simply adding the message type to the queue: discovery
before the conversation exists and atomic completion of control receipt must also be defined.

| Required outcome | Proof in the next scenario |
| --- | --- |
| The recipient has not received the Welcome in advance | Bob creates an invitation and stops before `add_contact`; no warm-up, manual `receive`, or fixture-join |
| The initial object survives sender disappearance | Alice creates the contact through ordinary UI/CLI, pays for storage within an explicit budget, and sends messages; real data/index ACKs before Alice stops, with the control cost accounted separately |
| Discovery works without the conversation's MLS exporter | Cold Bob starts from his own original profile and invitation; the daemon itself finds and retrieves the encrypted control through the shared custody network; owner retrieval/import and a centralized inbox are absent |
| Join and progress do not diverge after a failure | A real SQL failure preserves the invitation, MLS snapshot, contact, and retrieval progress without a partial commit; after a restart, retrying the same object completes a single join |
| The active invitation constraints are preserved | A foreign recipient/author, a different invitation binding, expiry, and reuse do not create a contact; an exact retry of an already accepted Welcome is idempotent and does not advance the ratchet |
| After the join, ordinary paid history is available | Without Alice returning, Bob decrypts the original application messages; a cold retry creates no other contact, messages, or payment |

Reuse the existing `conversations.rs` checks for duplicate Welcome,
identity binding, and SQL rollback, extending them with the network control lifecycle. Separately,
agree in protocol/spec on the initial capability, its retention, and its link to the budget;
an established conversation and its exporter cannot be a prerequisite of this path.
Old invitations/profiles must have explicit compatible behavior without
attributing new network storage to them. These are implementation and test requirements,
not evidence of an accepted offline Welcome. The basis is R05 of the independent review;
a new parallel consensus or a separate hosted inbox is not required.

### Remaining offline/recovery acceptance

Next, mandatory are recovery beyond the available epochs and rejoin without
returning revoked rights; autonomous 10→7→10 with both clients offline;
attachments with chunk/resume/hash/TTL/disk-full. These items are not closed by the
`recovery_required` status or by a successful import of an established conversation's messages.
Groups, devices, independent operators, and all three platforms remain in
the full AR3–AR5 plan and the original 67/22/3 boundary.

The [platform IPC map](agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md)
records the real Unix dependencies of the daemon, CLI/MCP, private files, and the build.
U06/U07/E01/E24 remain partial; a Windows launch is not yet confirmed.

## Current stage — September 14

Diagnostic32: [the observer diff and the rig were accepted by a separate critic](../evidence/reviews/AR2-wallet-flow/diagnostic32-test-review.json).
[Targeted checks](../evidence/reviews/AR2-wallet-flow/diagnostic32-checks.json):
122 backend / 47 frontend, Clippy/fmt, and the release build passed before the run.
[Baseline32 ended with a failure](../evidence/reviews/AR2-wallet-flow/diagnostic32-native-r1.json)
of the first SQL gate (120 seconds): 32 originals published in 542 seconds,
23 leaves, 288 + 288 real losses, after trust — 48 reads and zero imports.
316 deferred attempts produced no import. Selection of originals 1 and 31 twice in each
case ended in a global-read-rate wait before enqueue. This does not prove the R14 cause.
Source/binary guard and cleanup passed. The diagnostic ordinal mapping was fixed
(zero-based numbering); 5 parser / 47 frontend pass, the partial trace was rechecked,
and the original failed report is preserved. The functional steps of the shared publisher,
read continuation, and staging merge remain open.

After an additional architect review, the [paid history lifecycle
plan](../Docs/V1_HISTORY_LIFECYCLE_R14.md) was accepted: shared batch/history
publication, read continuation between Work instances, and a single store for pending bodies.
The [offline cross-check](../evidence/reviews/AR2-wallet-flow/r14-architecture/recheck.json)
reproduces the structural measurements of R14: 117 leaves (106 single), 230 pages,
89 unique originals in the union of import and caches. At the time of the review, all 21 provided
code files matched the project. The cause of skipping 31 remains unknown.
The baseline is frozen; the unsuccessful trace is preserved and analyzed. The first
[shared collection component for a finished batch](../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_COLLECTION.md)
is implemented: short passes continue one group, and before the atomic commit the
current Core/Custody inputs are rechecked. The critic accepted the implementation and a separate legacy test;
the [check results](../evidence/reviews/AR2-wallet-flow/shared-collection-checks.json)
cover the collector. The [shared graph/pointer stages](../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_PUBLISHER.md)
are now also implemented:
different jobs continue one traversal, errors preserve retry, and the successor and the two conversations
do not mix ACK/pointer. Separate authorization and Core retirement of each job
are preserved. The [group placement priority](../evidence/reviews/AR2-wallet-flow/PLACEMENT_PRIORITY.md)
is implemented: at most 12 queued originals, a bounded no-progress window, preservation of
the other conversations' positions, and replacement after a pause. The critic accepted the implementation;
42 backend / 63 frontend, Clippy/fmt pass. The [collector under new
reservations](../evidence/reviews/AR2-wallet-flow/ARRIVAL_FENCES.md) was also verified:
four paid originals form one leaf between expensive quanta with new sends.
Core permits only monotonic growth of the existing book's counters, preserving
immutable bases, exact own rows, and the current authorization.
Independent ACCEPT; 80 unique backend / 63 frontend, Clippy/fmt pass.
Next — read continuation and unified staging, then a measurable improvement of
Diagnostic32 up to an unchanged Full130. Real 20 ms and native throughput have not been measured.
[Ready output before Work completion](../evidence/reviews/AR2-wallet-flow/READY_OUTPUT.md)
is now applied through the former checks; after the deadline a new request is no
longer queued. Independent ACCEPT, 60 backend /63 frontend /5 parser, Clippy/fmt.
This is part of step 2; the unfinished root proof still requires porting.
The [Core range API](../evidence/reviews/AR2-wallet-flow/RANGE_CORE.md) now stores the
holder position and the fully accepted bodies in one transaction, with cold restart,
SQL rollback, exact-head fences, and compatibility with the former body cache.
Six new tests and the implementation were accepted by the critic; the network receiver still needs
to be switched to this API. [Holder errors](../evidence/reviews/AR2-wallet-flow/NO_LEGACY_ON_FAILURE.md)
no longer create legacy retries; six policy tests pass, and the critic accepted the code.
Deadlines, limits, and the V1 boundary are preserved.

A short test confirmed a separate sender observation defect: a temporarily empty
history on a root change or cold restart caused `checkpoint_rejected`.
Node now defers such an incomplete observation while keeping the strict Core checks.
The independent critic accepted six tests; the former code failed in two cases.
[Fix checks](../evidence/reviews/AR2-wallet-flow/sender-observation-checks.json):
26 backend / 47 frontend, Clippy/fmt pass. Native throughput has not been measured yet,
and no new release build was run. Next — a short trace of the existing read of the
next missing original; a new Full130 only after a measurable improvement.
The [brief current status](../IMPLEMENTATION_STATUS.md) replaces the accumulation of
history in the root summary; the old records are fully preserved separately.


[Full130 R14](../evidence/reviews/AR2-wallet-flow/history-range-native-r14.json)
ended at recovery: all 130 originals published, 390
signatures verified, 1300 data/1300 index receipts and 13000 location ACKs received. After
a real loss of 1170+1170 copies, the no-trust refusal and the first SQL failure with
an exact rollback passed. During the next stage the earliest original expired.
The post-stop snapshot confirms the import of exactly 1..30; original 31 is in neither
prefetch nor deferred. This does not prove the cause of the individual delays.

All 956 inputs and five signed artifacts are unchanged over 3646 seconds; cleanup
is clean. Deadlines, limits, and criteria were not changed. The final SQL failure, the full import
of 130, and a cold full traversal were not reached. The new diagnostics selected nothing after the
provider failure: all location ACKs were received. The R13 failure was not reproduced,
but its cause has not been proven fixed either. Next — reproducing
tests for read scheduling and the temporary reset of sender observations on a history
root change. Full130 and all of V1 remain open.

[Full130 R13](../evidence/reviews/AR2-wallet-flow/history-range-native-r13.json)
ended with a publication failure of the seventh batch within the former 600 seconds. 96
originals published; the next 16 have 160 data/160 index receipts and 1580 of 1600
location ACKs. Messages 107 and 111 have ten unconfirmed locations on one
and the same provider (position 9); the connection to it is present in the sender snapshot.
The cause of the delay is not yet proven; a reproducing test and more precise
sender/provider diagnostics after the failure are needed. All 956 inputs and five artifacts
are unchanged over 2375.4 seconds, teardown is clean. Signature verification of the full range,
copy loss, and recipient recovery were not reached. The node hash matches the
build; the CLI post-check was not reached, so its hash is missing from the report.

[Native20 R10](../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r10.json)
passed fully: 20 ordinary paid originals, 60 verified signatures,
a real loss of 180+180 copies, the no-trust refusal, both SQL failures with an exact rollback,
partial import 1..19, full 1..20, and cold confirmation without new imports.
The first SQL failure was reached after 21 requests, the partial stage after 59.
All 956 inputs and five signed binaries are unchanged over 708.4s; teardown is clean.
Full130 R13 ran on the same release build after independent
acceptance of the rig; its failure is described above. The success of 20 messages
does not close 130, an independent testnet, or the full volume of 67 cards / 22 scenarios / 3 OSes.

After the shared request limit is exhausted, the recipient now keeps the current
attempt and continues a bounded pass over the available local data.
The unaccepted root and ancestry remain pending; limits and atomicity are preserved.
Five tests were accepted by the independent critic before implementation; 118 backend / 51 frontend,
Clippy/fmt pass on 397 unchanged inputs:
[scheduling checks](../evidence/reviews/AR2-wallet-flow/history-admission-yield-checks.json).
The ordinary release build and strict signature verification passed on 922 unchanged inputs.

The ordinary send queue merges ready originals into pages of 1–12 references.
Before each inclusion, the active permission and its
paid indexes/confirmations are rechecked. Collection takes one 500 ms interval; the additional
checks yield the main loop under the former 20 ms budget. The order of confirming
child pages before the parent, root, and pointer is preserved. Independently
verified tests and the implementation pass 71 Node/2 Core/51 frontend, Clippy/fmt:
[merge checks](../evidence/reviews/AR2-wallet-flow/history-batch-scheduling-checks.json).
Core previously passed 96 related history tests, including 130 originals.

The previous [Native20 R9](../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r9.json)
confirmed the sending of 20 originals, 60 signatures, and a real loss of 180+180 copies.
The first SQL failure was not reached within the former 120s: 48 requests, 25 bulk, 12 path
requests, 7 deferrals, 0 imports, and 0 storage errors. All 955 inputs and 5 binaries
are unchanged; the processes and temporary profiles were cleaned up.

The snapshot after a regular recipient stop shows: original 1 is eighth
in the signed history; originals 1 and 2 are already in the cache. In total 10 prefetched and 7 deferred
originals, the exact import list is empty; 6, 11, and 16 are absent from both caches.
This is the post-stop state, not an exact deadline snapshot. The presence of ciphertext
by itself does not permit import without confirmation of the current history root.
This failure defined the subsequent scheduling regressions under the former limits of
24 requests total / 12 per peer per 60s; the result of the fix is Native20 R10 above.
R8 with the first exact SQL rollback and R6 with the full Native20 are preserved as historical
results. Full V1, 67 cards, 22 scenarios, and three OSes remain open.

Automated tests use isolated temporary keys and do not touch the login
Keychain. This was verified on real application restarts:
[test key storage](../evidence/reviews/AR2-wallet-flow/E2E_SECRETS.md).

The Core recovery status is implemented and verified: 71 backend/35 frontend, Clippy/fmt. The shared owner projection and signed agent reader are bound to root/epoch/expiry and the actually committed imports; 7 new tests were accepted by the independent critic. [Checks](../evidence/reviews/AR2-wallet-flow/history-recovery-core-checks.json). Worker, CLI/MCP, and UI are now connected: 30 backend/45 frontend, Clippy, the frontend build, and headless visual pass. The [paid native scenario](../evidence/reviews/AR2-wallet-flow/history-recovery-native-r1.json) passed on 867 unchanged inputs: six signatures, a real loss of 18+18 copies without the sender, SQL failure 0/2, partial import 1/2, full 2/2, and cold 2/2 through CLI/owner pages. Native GUI recovery, recovery/rejoin, full130, and 67/22/3 remain open.

## Started-payment update — September 12

[Started payment after a checkpoint change](../evidence/reviews/AR2-wallet-flow/PENDING_POSTAGE_RENEWAL.md) passes the full focused native GREEN R3: two genuine successors, the former tickets/QC, release of pending slots, and cold recovery after data/index copy loss without the sender. All 833 inputs unchanged, 6 signatures verified, teardown clean. The atomic update of the pending context and a separate check of payment time/storage admission, shared by full and compact proofs, were fixed. The latest targeted follow-up: 37 backend/14 frontend, Clippy/fmt. Full130 release R6 ended after 128 stored originals and 130 accepted CLI sends: the correct over-budget refusal passed, but the full message snapshot comparison included mutable background delivery statuses. All 834 inputs unchanged, teardown clean. The corrected test oracle was accepted by the critic and passed a short release CLI run BO-R3: 6 signatures, a loss of 18+18 copies, both SQL failures, and cold recovery without the sender; all 837 inputs unchanged. Full130 release R7 ended on 838 unchanged inputs: all 130 originals published, 390 signatures verified, 1300 data/1300 index receipts and 13000 location ACKs stored. Before the first copy loss, a test SQL helper failed; a short regression reproduced the two-retained-objects limit and passes after its removal (retain 11 for data/index, exact survivors/cold reads; 1 backend/14 frontend, Clippy/fmt). Teardown clean; recovery of 130 is not yet verified. Full recovery of 130 and the 67/22/3 scope remain open.

Assignment of September 10, 2026: update the plans per the review and continue implementation.
The [review of snapshot eea229c](reviews/2026-09-10-independent/review.md) is preserved without
changes. The [machine map](agentic_internet_v1_execution_plan/architecture-followup.json)
contains all 24 findings with their original evidence and five continuation stages.
The `AR-Rxx` prefix distinguishes review findings from the `Rxx` requirements of the original backlog.

## Ordinary graph sender — September 12

The [end-to-end test of 130 paid messages](../evidence/reviews/AR2-wallet-flow/HISTORY_RANGE.md)
has been written; the former rig acceptance was replaced by a
[REVISE on feasibility](../evidence/reviews/AR2-wallet-flow/history-range-fixture-review.md).
Debug R2 did not complete the first packet
of 16 within 600 seconds; 823 input files unchanged, the processes cleaned up. The same data,
deadlines, and limits in release R3 produced the same timeout failure on 823 unchanged inputs.
[Repeat lookup is reduced](../evidence/reviews/AR2-wallet-flow/ROUTING_HINTS.md):
only missing positions are checked, and a previously found address speeds up the new
proof request. The full native test passed on both networks: 416 verified
positions, fresh proofs, role revocation, and the former negative/cache checks.
31 backend /31 frontend and Clippy/fmt passed. R4 completed 32 messages on 823
unchanged inputs, then was stopped: snapshot 1800 is mathematically insufficient
for admitting the last two. The corrected rig with 3600 snapshots and genuine
subsequent heads was accepted by the independent critic.
The [short head-change test](../evidence/reviews/AR2-wallet-flow/WALLET_RENEWAL.md)
R1 ended in failure on 825 unchanged inputs: the new head and authority were obtained
through peers, but the sender policy remained on the old state. The transition from
wallet refresh to the current sending authority is implemented: 54 backend /31 frontend,
Core/node Clippy and fmt passed. Native R2 passed on 826 unchanged inputs: a genuine
peer successor, the former tickets and page ACKs after a cold restart, and recovery
after a real copy loss without the sender. Full130 [release R5 ended with an error](../evidence/reviews/AR2-wallet-flow/history-range-native-r5.json)
by the original deadline of the fifth batch: 64 stored originals, three peer successors,
826 unchanged inputs, and a clean teardown. After the third successor, eight
unfinished finalizations occupied all client slots. The next regression
test and fix is [renewal of an already started request](../evidence/reviews/AR2-wallet-flow/PENDING_POSTAGE_RENEWAL.md). Message TTLs, quotas, and recovery checks are preserved.
The range is not yet accepted.

[Graph publication](../evidence/reviews/AR2-wallet-flow/GRAPH_SENDER.md) is connected
to ordinary paid sending: all ten child confirmations precede
the parent, then the root and the pointer. Traversal is bounded by a reference stack and one
page; subtree skipping relies on a completed Core checkpoint.
30 targeted backend /31 frontend and Clippy/fmt pass. Clean native C1:
822 unchanged inputs, two genuine paid CLI originals, five pages,
50 ordered ACKs, SQL/cold retry, and recovery after a real
copy loss without the sender.

Next — >128 simultaneously live paid originals in an ordinary native flow
after fixing the renewal of an unfinished payment. The standalone sender oracle already
[passes native R1](../evidence/reviews/AR2-wallet-flow/standalone-sender-graph-native-r1.json): four SQL faults, cold retirement, remote graph reads, and 826 unchanged inputs. Two messages do not
substitute for this range; the full 67/22/3 scope remains open.
This order replaces the further steps of the historical slices below.

## Ordinary graph recipient — September 12

The [network receiver](../evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md) is connected
to ordinary automatic synchronization: paid typed pages, history continuation
verification, the child-page holders' own addresses, and persisted attempts.
Runtime recovers 130 genuine messages after restarts; SQL INSERT/
UPDATE failures preserve the former checkpoint, and expired subtrees do not require
downloading. 30 backend /31 frontend, Clippy/fmt, and a paid CLI scenario of
two originals on 816 unchanged inputs passed. The end-to-end paid graph >128 is not closed by this.

The next stage is the child ACK order before parent/root/pointer for the ordinary sender,
then >128 simultaneously live paid messages without the sender. This order
replaces next-work in the historical records below; the 67/22/3 scope is preserved.

## Graph traversal cursor — September 12

The [Core reference cursor](../evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md) persists
the bounded attempt before I/O and survives a cold restart: 128 originals inside
an unchanged v1 page plus two v2 pages yield 130 references. Expired
subtrees are skipped only by a signed path. SQL rollback,
a real MLS epoch, exact pointer/root before a retry, and refusal of a foreign signed path were verified.
An attempt does not imply an import or history completeness. 105 backend /31 frontend,
Clippy/fmt passed on 811 unchanged inputs; the full Core import of 130 originals is preserved.

Next we connect typed paid root/path reads and the cursor to the ordinary receiver, then
the child ACK order before root/pointer. Native was not repeated here: the former C8
remains a historical v1 gate. Full >128 live paid recovery and the 67/22/3 scope
remain mandatory.

## Current graph integration — September 12

[Core checkpoint and the shared publication queue](../evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md)
store the exact root/member/pointer atomically with retirement; SQL/cold retry
and a real MLS epoch change were verified. Indexes and history use one scheduler with a yield
after expensive preparation, the former limits, backoff, and paid ACK checks.
117 backend /31 frontend, Clippy/fmt, and the ordinary paid CLI native C8 passed.
The original fixture/disk-full/IPC failures are preserved; C5/C7 are marked diagnostic.
A universal time bound for a single synchronous operation is not proven here.

The ordinary v2 graph, child ACK order, and >128 live paid native gate require
further integration. The full 67/22/3 scope and product recovery remain.

## Current history recovery — September 12

The [persistable recipient cursor](../evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md)
fixes the repeated start from the first pending messages. After 16 real MLS gaps,
a full Runtime restart reaches the 17th original, then recovers all 17.
The cursor records the attempt; import, MLS, and ACKs change only through the former commit.
A skip releases the entire old fetch plan and is accounted for separately as deferred.

72 backend /31 frontend, Clippy/fmt, and a paid CLI loss/recovery passed:
two originals, six QC signatures, SQL faults and cold ACKs, retirement, a loss of
18 data/18 index copies, owner work/retrieval calls = 0. Every candidate
recorded 802 unchanged inputs. The old native failure due to an unpublished
early original is preserved; the cursor does not recover missing ciphertext.
Next — ordinary publication and v2 graph traversal with child ACKs before root/pointer,
acceptance of >128 simultaneously live paid originals, and explicit product recovery.
The full 67/22/3 scope is preserved.

## Decision and boundaries

We continue with a targeted lifecycle rework. We keep P2P/libp2p, OpenMLS,
the transactional Core/SQLCipher, the capability broker, public signed postage, and the ready
consensus engine. We keep the shared issuer-global spent namespace, strict formats,
MLS/message/outbox/nonce atomicity, and issuing ACK/receipt only after commit.

This is a change of architectural order, not a product reduction. Still
mandatory are **67 cards, 22 E2E, and macOS arm64 / Windows x86_64 / Linux x86_64**
from [release-scope](agentic_internet_v1_execution_plan/release-scope.json).
R10, offline history, backup/recovery, groups, CLI/MCP, and transport economics
remain mandatory. Jobs/orders/reviews/A2A already belong to V2: further
development stops, active UI/tools are isolated from ordinary V1, and readers and
regressions are kept. Network operator discovery and repair are not V2 jobs.

The narrow preview proposed in the review — with reduced provider diversity, subsidy,
royalty/settlement, or platform count — remains a proposal: there is no explicit user
decision to exclude these requirements. An additional alignment
point is not needed for the work described below within the current scope.

## New delivery order

A clarification of current execution dated September 11: accepted is the **full declared range
of two originals from paid books with disjoint original data/index
sets**, recovered through the ordinary path without internal work commands. Additional preparation is allowed
only for an observed obstacle to this scenario. Then whole user
capabilities are closed, with updates to the
[compact 67 cards /22 E2E matrix](agentic_internet_v1_execution_plan/CAPABILITY_EVIDENCE.md).
The matrix links the verified parts and the remaining conditions; it does not turn
historical GREEN into acceptance of a whole card or of the current release.

| Stage | Outcome and changes | Acceptance |
|---|---|---|
| AR1. Long paid conversation | Durable stages/terminal retirement, a ready-job queue, an indexed spent set, bounded authenticated snapshots, handover and refresh authority. We keep the old spent keys across epochs. | E20/E21 and parts of E01/E05: significantly more than 128 spends by **the same issuer**, several epochs, concurrent same-ticket, crash before/after commit/QC, partition, lease outage. Separately exceed 128 admission jobs without passing this test off as the spend gate. |
| AR2. Ordinary human and agent entry | Wallet/sponsored book UI, messenger CLI and skill, a shared daemon read model. Split legacy verify/prove, remove the prover from the default V1 build; small neutral IPC/funding types. | E01/E11/E14: a new profile, book, UI, and CLI independent of MCP, restart, revoke queued runtime, SQL failure. Legacy historical retrieval remains verifiable. |
| AR3. History after sender disappearance | A network paid index, verified data-holder locations, books/epochs linkage, range completeness, durable first Welcome, attachments, data/index repair by one scheduler/store. | E05–E07: disjoint data rosters, several books, index/data node loss, cold restart, corrupt/gap pages; 10→7→10 with both clients off, real ciphertext verification. |
| AR4. Groups and device recovery | One complete group vertical: encrypted ordered control, availability before finalize, valid/no-op application, epoch-aware catch-up; revoke/rejoin and encrypted backup/import. | E08–E10/E23: concurrent membership, partition/heal, a removed member, offline beyond three epochs, loss of the last device, anti-rollback grants/spent/fences. |
| AR5. Independent testnet and three OSes | Windows IPC/ACL, clean platform builds/installers, operators/bootstrap/relay/checkpoint outside the company, onboarding/settlement, and diagnostics. | E02–E04/E17–E19/E22/E24–E26 and a shared single-revision gate: vendor-off cold join, installers, real provider flows, upgrade/rollback/disk-full. |

These stages replace the former "one more custody library first" as the continuation
priority. The historical C0–C6 and card IDs are preserved for traceability.
Windows/packaging, UX, and operator independence can be prepared before full AR1;
this is not a reason to postpone them to the last stage. The parked live 64-validator/R24 gate
remains parked with its own evidence and does not block the current module.

## Ordinary recovery of two disjoint books accepted

On the real chain, 24 books were paid before the beacon; among the 60 active bonded
operators, a pair with originally disjoint data/index R10 was selected. The ordinary
sender passes a cold restart and a retention change 3600→900; the older live original
remains the anchor. After sender disappearance, real data/index loss,
missing trust, two SQL-fault stages, partial progress, and cold/cache-loss retry,
the recipient recovers both exact messages without duplicates. Internal
work/retrieval calls and prover invocations — zero.

The run exposed two concrete network obstacles: an unchanged first set
of 32 candidates and 64 ordinary connections filled with duplicates after a cold restart.
The fixes reuse the bounded job cache and the existing connection limiter;
quotas and deadlines are preserved. Two native gates, including concurrent sends of one
book, and targeted checks pass on the same sources: **94 backend /21 frontend, Clippy/fmt, 614 unchanged inputs**.
Early failures, including the absence of a suitable pair in the final book set,
are preserved. [Evidence](../evidence/reviews/AR3-history-books/README.md).

A whole user-facing send flow is in progress: wallet/price/TTL/budget, a shared
status for UI/CLI/MCP, and retirement with history preservation. The daemon itself obtains
bounded RPC proofs; Core issues the exact transaction/ERC-681 request only after
the key and purchase conditions are atomically stored. The first native gate passed a real
payment, a cold wallet, an RPC refusal and retry, send setup, and recovery of
two messages after sender disappearance and data/index losses. The repeated
gate also passed with independent decoding and ERC-681 payment.
The UI for selecting the book, price, validity period, separate TTL, and limits is already connected
through the owner-only Tauri bridge. Passed: **56 Core wallet /19 L2 /2 Tauri bridge
/46 targeted frontend tests**, Clippy, and the UI build. Selection of the registry/finalizer files
is available through the trust panel and was verified visually. The next native candidate passed
obtaining the public committee/issuer proof from a connected peer, saving the client
configuration, and a cold restart without manual publish/configure commands, then two
ordinary sends and recovery after losses. 49 Core committee/lifecycle tests
and the client fence/scheduler checks passed. The separate CLI now uses the signing client
shared with MCP and scoped credentials; the GUI shows the exact command,
and the binary is part of the signed macOS debug build. Passed: 3 CLI process /3 Core
metadata /13 MCP /7 index /20 targeted frontend tests. The real paid CLI gate
passed two sends over the public NetworkID, an over-budget refusal, a retry without duplication,
and a full loss/recovery with six QC signatures and 561 unchanged input files.
A separate hidden WKWebView gate executed the GUI-shown CLI command from the bundle,
verified shared retry/delivery and the revocation of both clients via the UI; the
network/trust scenarios and a 1051-message history also passed, 40 desktop inputs unchanged.
The shared status and successful retirement passed a separate native gate below.
The shipped skill was verified in the current bundle; a separate Codex host carried out
a conversation via the CLI, read the entire reply, and confirmed processing.
[Host evidence](../evidence/reviews/AR2-wallet-flow/skill-host/README.md).
Native paid UI onboarding now passes on two empty profiles: real
contacts/trust, exact payment via Anvil, a cold wallet, RPC retry, budget, two
sends from the composer, and separate storage/ACK statuses. Both originals were recovered
after sender/data/index loss and SQL failures; six independent QC signatures,
754 unchanged inputs, and five unchanged bundle binaries.
[GUI evidence](../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md).
Automated tests use temporary keys without Keychain password prompts; the ordinary build
keeps the system Keychain. Full E11 and the long lifecycle remain open.
This does not close AR2.
[Contract and evidence](../evidence/reviews/AR2-wallet-flow/TEST_CONTRACT.md).
This is not acceptance of arbitrary history: real MLS epochs/Welcome/gaps,
autonomous R10, and the remaining V1 conditions remain mandatory.

## Historical acceptance: ordinary terminal history manifest

The [ordinary network integration](../spec/custody-history-automatic-v1.md) is accepted:
the sender builds the manifest from durable originals and real paid indexes, obtains
ten exact ACKs, and publishes the linked pointer; the recipient downloads each
reference through its own indexes and atomically stores message/MLS/dedup/progress.
The active-job roster intersection was removed. Three native gates passed on one binary:
sender with four SQL-fault stages/cold/all-ten remote audit, recipient with real
index/data loss, missing trust, two SQL failures, partial progress, and
cold dedup, plus legacy direct retrieval/copies/inspection. The pointer wait
was fixed: heavy sender checks are not repeated every 500 ms while a network job
awaits confirmation. Deadlines and requirements are preserved. The independent critic accepted
the tests before production and both later test harness corrections.
Result: **76 backend /21 frontend, Clippy/fmt, 613 unchanged inputs**; the backend
includes six existing process mailbox regressions. This is one funded book;
genuine disjoint books, MLS epochs/Welcome, retirement, lifecycle, and R10 are still
mandatory. [Results](../evidence/reviews/AR3-history-automatic/README.md) ·
[Continuation](../evidence/reviews/AR3-history-automatic/NEXT.md).
Below are the historical prerequisites of this change, not remaining API work.

The cryptographic [manifest format](../spec/custody-history-manifest-v1.md)
is implemented and verified: up to 128 exact descriptor references, up to four different
index candidates per record, the signature of the current MLS direction, and revision
rollback protection. The anchor is chosen with the maximum retention period: a short new
lease must not cut off access to older messages. The existing
protocol limits are preserved. The independent critic accepted seven new tests before implementation;
73 backend/21 frontend, Clippy/fmt, and 593 unchanged inputs passed.
[Results](../evidence/reviews/AR3-history-manifest/).

This crypto step was a prerequisite of the storage and Core checkpoints accepted below.
Locator binding, atomic import by reference, and server-side manifest transport
were also accepted below. Next we connect them to ordinary automatic publication and
retrieval, replacing index intersection.
Genuine multi-book/epoch recovery, completeness, first Welcome,
retirement, and R10 remain open. The previously accepted network runtime is `31ab80d`;
this step claims no new network acceptance.

Local [manifest storage in the paid anchor index](../spec/index-history-storage-v1.md)
is also accepted: the original funding/QC/receipt are preserved, the bytes count against the shared quota,
SQL failures do not return unconfirmed data, and new revisions cannot delete or
replace a still-live reference. A cold restart, reading after the end of
admission with public trust and no wallet, capability/byte bounds, and SQL corruption were verified.
Five tests were accepted by the independent critic before implementation; 78 backend/21
frontend, Clippy/fmt, and 595 unchanged inputs passed. This is local acceptance;
Core state was accepted in the next step, and the network path still requires integration.
[Results](../evidence/reviews/AR3-history-store/).

[Core manifest state](../spec/custody-history-core-v1.md) now stores the exact
outgoing bytes and the incoming checkpoint before issuing routes, including anchor changes.
A cold retry does not change the signature; the expiry of a short reference does not break a retry with
a live anchor. Genuine rollback/equivocation/issued_at rollback and deletion of a live
reference are rejected. An SQL failure does not change the old record; message/MLS/outbox and the fetch
cursor remain as before. Seven tests received an independent REVISE → ACCEPT before
production; 91 backend/21 frontend, Clippy/fmt, and 597 unchanged inputs passed.
This is local acceptance; the locator and atomic import by manifest reference were accepted
in the following steps, and network transport and real multi-book recovery are still ahead.
[Results](../evidence/reviews/AR3-history-core/).

The [exact locator binding](../spec/mailbox-history-locator-v1.md) is implemented in
crypto/Core: the full manifest and anchor descriptor hashes, epoch/revision/lifetimes,
preservation of the old v1 encoding and the former outer record size. Core
publishes only its stored manifest and rejects downgrade/rollback and
replacement of a still-downloadable version. The cold check binds the stored locator to
the signed records or the checkpoint hash. The independent critic accepted seven
tests after REVISE; 107 backend/21 frontend, Clippy/fmt, and 602 unchanged
inputs passed. The ordinary runtime does not use the new manifest yet and keeps index
intersection; network multi-book and completeness are not yet accepted.
Additionally, six existing process mailbox tests passed on real
nodes with the same inputs: cold sender-absent lookup/cache loss, hostile DHT, and
background work limits. This is a compatibility check of the old path.
[Results](../evidence/reviews/AR3-history-pointer/README.md).

[Import by individual references](../spec/custody-history-import-v1.md) now atomically
stores the original message, MLS/dedup, and the operation mark. Downloading a late
message does not hide an early gap; concurrently prepared distinct downloads
are merged with the current SQL state. A retry of an already received original does not
advance MLS, and an exact retry does not rewrite SQL. A new revision, even with the former
anchor, blocks the old tokens; the cold check refuses a wrong real
message ID/direction/conversation or descriptor commitment. An unavailable manifest
and expired undownloaded references remain explicit. Eight tests received an independent
REVISE → ACCEPT before production and a compile RED on missing APIs. Targeted groups
passed: 57 backend/21 frontend, Clippy/fmt, 604 unchanged inputs. The unchanged
crypto/paid-store suites were not rerun. This is Core acceptance; new network
acceptance, removing index intersection, and closing AR-R03 are not claimed.
[Results](../evidence/reviews/AR3-history-import/README.md) ·
[Next step](../evidence/reviews/AR3-history-import/NEXT.md).

[Server-side manifest exchange](../spec/paid-history-network-v1.md) is accepted on the real
custody Noise protocol: the write confirmation contains operation/hash, and the read
response contains the exact original paid anchor and the signed manifest. The full portable
payload fits the request limit before the read clock is committed. A query by operation also finds
a nonfirst anchor; this was separately verified in paid-store. The native gate passed with
genuine payment/QC, an index restart, the sender daemon off, SQL put/update/read
failures, exact original proofs, and refusal of an old revision.
Signature preparation uses a test helper of the genuinely stopped Core;
it does not count as ordinary automatic publication. Before production there were ACCEPT
and compile/runtime RED. A test error with five requests at a helper limit of four
was fixed without changing production, after a separate ACCEPT. Result: 44 backend /
21 frontend, Clippy/fmt, and 607 unchanged inputs matching the native gate.
The ordinary sender/recipient paths still require integration; AR-R03 remains open.
[Results](../evidence/reviews/AR3-history-network/README.md) ·
[Next step](../evidence/reviews/AR3-history-network/NEXT.md).

## Rework contracts

The [durable outgoing history layer](../spec/custody-history-outgoing-v1.md) is accepted:
Core reads the current live original envelopes independently of jobs/outbox and keeps the
CAS revision after anchor expiry. The shared paid carrier stores one manifest and
the list of exact index ACKs. A new revision clears the ACKs; genuine
paid peer/position, hash, current trust, and retention period are verified. SQL errors, cold
corruption, and the exact quota do not create false progress. After a test REVISE there was
an ACCEPT before production and a compilation RED. Result: **60 backend /21 frontend,
Clippy/fmt, 610 unchanged inputs**. This is a local prerequisite: the ordinary network
sender/recipient paths are not yet connected, and roster intersection and AR-R03 remain.
[Results](../evidence/reviews/AR3-history-outgoing/README.md).

Per the user's clarification of September 11, during implementation we run targeted
backend/frontend test groups; the full suite — only at the end of the whole plan.
Previously completed checks keep their actual coverage.

- The separation of the default verifier and the explicit legacy prover is verified (part of AR-R11 within AR2).
  The clean target already verifies the two original paid receipts under the frozen image ID without
  building the guest/kernel. A separate test of the real desktop package was accepted by an independent
  critic; its RED confirmed the former unconditional prover launch. The debug package without the
  prover already passed verification of the old receipts and of verifier availability after restart.
  The release package also passed without guest/kernel generation, with the same verifier and
  correct availability after restart. All 13 legacy proof/process tests,
  871 Rust tests, 59 frontend tests, 19 model tests, and Clippy/fmt/TS/Vite passed.
  All 569 build and test input checksums were preserved. Native public types,
  complete historical retrieval, and the remaining AR2 entries remain open.
  [Contract and results](../evidence/reviews/AR2-verifier-build/).
- The two 128 limits are independent: the lifetime spend log and active sender jobs. Simply
  raising the constants closes neither the lifecycle nor the authority change.
- The [finalized prefix index](../spec/finalized-history-index-v1.md) is prepared:
  SQL records by sequence/operation, the original QC as the hash chain root, atomic
  transfer pages of seven records, and a prohibition of negative membership answers
  during an incomplete transfer. The index is connected to the current P-256 runtime: the
  full bounded tail up to the confirmed anchor, the current Core fence, and the receipt are verified.
  The [genuine funded gate](../evidence/reviews/AR1-runtime-lifetime/) confirmed 144
  spends of one issuer/log/committee, recovery without peers after two SQL
  failures, candidate retirement, and the old QC after a real authority expiry.
  The following [native gate](../evidence/reviews/AR1-spend-recovery/) confirmed
  the recovery of the original SpendRecord by a lagging validator after retirement
  of the receipts on the other nodes: without owner resubmit or a new quorum, with an SQL failure,
  role revocation, and rejection of an altered QC from a selected peer. Network
  recovery still requires active credentials in the same epoch.
  The [checkpoint refresh](../evidence/reviews/AR1-checkpoint-refresh/) now
  preserves the full queue of 16 unfinished receipts: the authenticity of the former
  receipt is verified in the new context, and the replacement is first written to SQL.
  The native gate confirmed 18 spends/216 signatures in the former journal, an SQL failure,
  a cold restart, operation after the old lease, and reading confirmation of a spend made
  before the new checkpoint took effect, with the validator role disabled. The upper bound of historical authority
  is preserved; live authority and the prohibition of an empty start of a new epoch are not weakened.
  The [terminal journal record](../evidence/reviews/AR1-terminal-history/) now
  forbids child records both in the confirmed prefix and in the consensus tail.
  The rule is persisted in SQL before the signer starts; its own closing QC is required,
  and proof via a descendant is not accepted. Four genuine engines were verified
  with cold recovery, 131 records, and transfer in pages of seven records. This is a generic
  finalizer gate. The following [paid native gate](../evidence/reviews/AR1-epoch-closing/)
  already confirmed the closing of a real postage journal: Core verifies the successor on
  the same checkpoint without replacing the old roster; one owner candidate is propagated
  between the selected daemons and receives its own QC. A lagging node recovers the
  original record after an SQL failure, role revocation, and a cold restart from the only
  available source, preserving the unfinished paid application. 24
  signatures, the old SpendRecords, the prohibition of new spends and of another successor were verified.
  The [original SpendRecord transfer layer](../evidence/reviews/AR1-spent-history-import/)
  now stores the pages and readiness in one SQL transaction and verifies the exact
  match of the history record and the entire chain of previous epochs. Seven tests cover
  130 paid spends, 1→2→3, an epoch skip, a cold transfer, an SQL failure, and the loss of
  an old confirmation; this is a verification of the evidence API, not of a network successor launch.
  The [successor policy and public chain](../spec/postage/successor-spending-v1.md)
  now admit application-spends in epochs 2 and 3 only with a complete history
  and active credentials. Old postage stamps from both past epochs are rejected,
  and an exact retry preserves the first QC/time. A separate client admission check verifies
  all the selected closings up to epoch 1 without validator history; an incomplete handover
  does not become a voting right. Ten new tests also verified corruption of an ancestor
  QC, write errors, a cold start, owner/fence, and credential expiry.
  These results concern the application API; a separate native integration is described below.
  The case of [incomplete old history for overlapping rosters](../evidence/reviews/AR1-overlap-history/) was also fixed:
  a validator backfills the missing tail before closing, preserving the old rows,
  the original QCs, and the verification time. Four new tests verify the write refusal at
  the old/new data boundary, cold resume, the prohibition of replacement with a signed chain,
  and the unavailability of negative membership before the transfer fully completes.
  A full cold transfer remains O(N). The [native integration](../evidence/reviews/AR1-native-handover/native-check.json)
  has now passed on real daemons: 133 spends, two closings, 1605 verified
  signatures, and three genuinely selected rosters (in this run, epochs 1→3→4).
  The incomplete old validator, a fresh member of the third roster, SQL rollback,
  role revocation, cold resume, the preserved unfinished application, and the public client lineage were verified.
  The original archive delivered all 130 old records with an expired checkpoint despite the
  failed renewal write and without an active signer. The owner API set no QC,
  history pages, or spent rows. A separate [configuration atomicity check](../evidence/reviews/AR1-native-handover/configuration-atomic-check.json)
  passed on a selected validator and an ordinary client: the configuration and cursor are now
  stored together, and a retry with the same revision works after an SQL failure and a crash.
  The former failed runs and the critic-accepted fixture fixes are preserved
  in the evidence. The latest timeout cause was in the test deadline: 33 pages
  require a third admission window at 16 requests per peer per 60 seconds. The fix
  does not change the protocol limit, the real clocks, or the maximum lease of 600 seconds.
  The [full regression](../evidence/reviews/AR1-native-handover/checks.json) passed:
  871 Rust tests, 59 frontend tests, 19 model tests, TypeScript/Vite, and
  workspace Clippy/fmt; all 582 source fingerprints were preserved. Closing after the old authority expires,
  automatic acquisition/renewal of authority by clients, and recovery of
  undelivered/expired pending inputs remain mandatory; AR-R01 is not closed.
- An additional [native hostile-history check](../evidence/reviews/AR1-hostile-history/)
  passed: three genuine spends, two closings, 69 independent signature verifications.
  Responses of an ordinary Noise peer with a wrong target epoch, an altered QC, and a mixed
  page do not change SQL and do not start a signer. Genuine responses after a crash
  restore the original records, QCs, and verification time, including cold reads.
  This change affects only tests and the peer carrier; production remains on `d95671b`.
  195 node tests, 59 frontend tests, the former native recovery carrier,
  node Clippy, fmt, and TS/Vite passed; 570 fingerprints were preserved. The full regression of 871
  refers to the previous production commit, not to a new run of all crates.
- The [bounded sender scheduler](../spec/postage/public-sender-scheduling-v1.md)
  passed on a real queue of 128 jobs with 127 paused: the permitted
  tail was prepared in 1.30 s, confirmed in 11.66 s after a cold reconnect,
  and the maximum owner RPC was 1.64 s. The same daemon passed an ordinary send of two
  messages with 20 genuine receipts, an SQL fault/restart, a forged offer,
  and offline/cold retrieval. The TCP/QUIC cache invalidation
  (329 positions) and the hostile history transfer (69 signatures, two closings) passed again.
  The implementation keeps the Core checks and the exact Noise connection, removes the repeated
  verification of an unchanged offer on every poll, and immediately serves the local
  consensus mailbox. Targeted checks: 46 Core tests, including 30 sender tests, and 96
  daemon tests plus node Clippy. Before the policy clarification, 59
  frontend tests, 19 model tests, fmt, and TS/Vite managed to complete; the general Rust run was stopped,
  and there is no full workspace pass for this version. [Results](../evidence/reviews/AR1-sender-fairness/checks.json).
  The retirement of successful jobs remains open: a durable index
  with verified storage locations is needed first so that evicting jobs does not lose old history.
- Retention of a prepared packet starts at preparation. Do not delete
  unprepared work by `createdAt + retentionSeconds`. The first AR1
  change was the durable retirement of **already prepared expired** packets, including
  cold restart and a write failure; exposed stamps are never returned to the balance.
  A shared timeout for the unprepared queue and the retirement of successful jobs require
  further separate transitions. The latter must keep the former history discoverable
  before a record is excluded from the working set.
- Reserved sponsorship, the allocated postage stamp, the final spend, receipts,
  index publication, and the recipient ACK are distinct facts. The final shared UI/CLI/MCP
  projection may show "delivered, stored 4/10"; a direct ACK does not require waiting for
  ten copies and does not prove durability. An expired job does not imply a refund.
- A logical index is mandatory for searching the whole history; it uses a bounded
  entitlement within the same paid operation, not a second postage stamp per message. A shared
  physical store/repair scheduler and dedup of identical proof bundles reduce
  repetition; hash-only references are allowed only when verifiable bytes are available.
- The [compact data-holder pointer verification](../spec/custody-holder-location-v1.md) is implemented:
  a signed descriptor replaces the ciphertext in the portable packet, and native funding,
  the original QC, the selected data operator, and the original/copy receipts are verified by the same
  historical verifier. A cold reader works without the sender's wallet after
  admission/binding/consent has ended, as long as retention is in effect. Substituted
  payments, participants, and semantically invalid consent with genuine signatures
  are rejected. The critic accepted four tests before production; the targeted gate passed
  59 backend and 21 frontend tests, Clippy/fmt, 575 unchanged fingerprints.
  [Evidence](../evidence/reviews/AR3-index-holder-locations/). This gate verifies the verifier.
  The following [location persistence layer](../spec/index-holder-storage-v1.md) has already
  been accepted separately: the original and backup receipts are stored in the existing index,
  the shared QC/funding is not duplicated, SQL failure/read-clock/retry are atomic, and a cold
  read requires installed Core trust. 17 targeted backend tests,
  21 frontend, Clippy/fmt passed; 577 fingerprints unchanged.
  [Persistence evidence](../evidence/reviews/AR3-index-location-storage/).
  The [index network handlers](../spec/paid-index-network-v1.md)
  and bounded location queries with recipient entitlements are now connected. A real native gate confirmed a
  separate index roster, original/copy claims, SQL failure/retry, and a cold read
  retrieving the original MLS message with the sender/primary off. One ticket
  pays for index/data/copy; two independently observed snapshots preserve
  identical proof bytes at different admissible observedAt values. The critic accepted the tests
  before the corresponding production changes. 27 targeted backend and 21 frontend,
  Clippy/fmt passed; 580 fingerprints unchanged, three QC signatures independently verified.
  [New evidence](../evidence/reviews/AR3-index-network/). This gate is driven by an
  owner/raw Noise peer; the automatic sender stages, latest pointer, and recipient
  discovery still require integration. Received bytes must
  be separately matched against the descriptor; history completeness and the retirement of successful jobs
  are not yet proven.
- The [automatic index publication](../spec/postage/public-sender-index-v1.md) is accepted:
  the ordinary sender stores ten signed confirmations of the selected index nodes
  and one hundred storage location ACKs next to the original ciphertext and the single payment. Batch
  publication is atomic on the index and in the outgoing journal. An SQL failure and a cold restart
  do not lose confirmed stages and do not allocate a new postage stamp. The protocol limits for
  streams, size, time, and admission are preserved; a repeat lookup skips only
  positions with genuinely stored confirmations. The cache of fully verified
  carriers is bounded; every access keeps the current Core trust/time and SQL checks.
  The background scheduler checks permission revocation on every tick.
  The current index native gate passed: 20 promises, 200 ACKs, both SQL failures,
  cold recovery of the sender/index, and no repeated index writes.
  On the same application source/binary, the ordinary owner/agent send with
  offline/cold retrieval, single index ingress compatibility, hostile
  history, and the full epoch transfer (133 spends, two closings, 1605 signatures) passed.
  Targeted checks: 75 backend/21 frontend, Clippy/fmt, 587 current fingerprints.
  After the native runs, only the handover evidence collection path from `full/` was fixed;
  the original successful exit, logs, and source/binary hashes were re-verified.
  [Evidence and former failed candidates](../evidence/reviews/AR3-index-sender/).
  At the sender-only stage, the Latest pointer used shared data holders. The next step at that time was fetching the
  index page, the verified locations, and the corresponding ciphertext with an atomic
  message/cursor import. Native acceptance of recipient discovery, books/epochs linkage, completeness,
  durable Welcome, successful retirement, and R10 remain mandatory.
- The [Core contract for retrieval through the index](../spec/custody-index-progress-v1.md) is accepted:
  one signed record is accepted without writing state; then only its
  exact ciphertext is imported together with that index peer's cursor. An expired
  read capability is not renewed: a subsequent fetch is bounded by the local token,
  the current MLS epoch, revision, retention, and the holder's separate network permission.
  Seven tests passed an independent REVISE → ACCEPT before production, then a genuine
  compilation RED. Real MLS transitions, SQL failure, and cold retry were verified.
  22 targeted backend/21 frontend, Clippy/fmt, and 587 current fingerprints passed.
  This was a local Core acceptance; the subsequent native acceptance is described below.
  [Evidence](../evidence/reviews/AR3-index-recipient/).

- The [automatic retrieval through the paid index](../spec/custody-index-recipient-v1.md) is accepted.
  The recipient verifies the paid index against the actual Noise peer, the original/copy holder
  claims, and the exact ciphertext, then atomically stores the original and the index bookmark.
  Verified active connections and addresses from the current signed locator come
  before the DHT lookup with an authenticated NodeRecord. Old data locators keep
  direct reading. The Latest pointer uses confirmed index endpoints with an ACK
  for all locations; the former restriction was moved to the intersection of index rosters.
  Four native gates passed on one source/binary: ordinary owner/agent,
  publication of 20 index promises/200 ACKs, legacy direct retrieval, and retrieval of two
  originals with the sender off after a genuine ciphertext loss. In the latter,
  all pointer endpoints are empty and the messages have different surviving holders; the absence of
  public trust, the SQL rollback of the two originals, cold restart, cache loss, and dedup were verified.
  Final targeted checks: 49 backend/21 frontend, Clippy/fmt, 590 fingerprints.
  This supersedes the former "recipient integration ahead" notes but does not close
  AR-R03: books/epochs linkage, final completeness, Welcome, retirement, and repair remain.
  [Final evidence](../evidence/reviews/AR3-index-recipient/README.md) ·
  [Next stage](../evidence/reviews/AR3-index-recipient/NEXT.md).

- Snapshot/spent handover preparation does not change the frozen legacy relation/image IDs.
  Historical obligations and the existing funds keep a compatible verifier and the
  shared spent set. `default-members` is insufficient for the ordinary `--workspace` gate.
- Heavy immutable verification can be moved to bounded workers; the current
  grant/authority/expiry/CAS is rechecked by the single Core writer at commit.
  Indexed SQL and incremental MLS persistence require crash/migration checks.

## Trust, recovery, and measurements

The shared status and retirement are now connected to ordinary sending: independent
recipient ACKs, funding, storage, discovery, and work are stored in the Core journal.
Successful completion and removal from the active queue are atomic; the original messages and
the signed history are preserved. A real native gate passed three SQL failures,
restarts, identical CLI/MCP/desktop projections, the next send, and retrieval of
both originals with the sender off after data/index loss. Six
QC signatures and 697 unchanged inputs were confirmed; the former failed runs are preserved.
[Evidence](../evidence/reviews/AR2-wallet-flow/status-native-candidate-3.json).
This closes the given verified scenario, but not the whole user capability:
full E11 with new contacts and host/NAT, cross-epoch history, and
long operation beyond individual 128-entry stores remain mandatory.

Continuation from September 12: local prepared envelopes were moved to separate
sequence rows with bounded pages; 258 originals verified. Outgoing paid
receipts are now also stored separately: 134 paid originals after restart,
indexed conflicts, atomic quotas, chunked cleanup, and a migration with seven
SQL fault points. 89 backend /31 frontend checks of this stage passed.
[Evidence](../evidence/reviews/AR2-wallet-flow/OUTGOING_ROWS.md).
Outgoing limits and maintenance are already connected to the daemon: 4096 originals /64 MiB,
one bounded GC attempt per second, SQL rollback, and backoff. The shared constructor
and the real Runtime::pump were verified in a signed network fixture (51 backend /31
frontend); the ordinary constructor keeps the fixed daemon network.
[Runtime evidence](../evidence/reviews/AR2-wallet-flow/RUNTIME_RETENTION.md).
Incoming ciphertexts now use the same row and transaction mechanism:
134 paid originals at the custodian, a separate quota, bounded pages
with explicit continuation, cleanup, and an atomic migration.
[Incoming evidence](../evidence/reviews/AR2-wallet-flow/INCOMING_ROWS.md):
92 backend /31 frontend checks, Clippy, and formatting passed.
The compact paid index was also moved to the shared row mechanism: a separate quota,
bounded pages, cleanup, and preservation of the old locations/history during migration.
[Index checks](../evidence/reviews/AR2-wallet-flow/INDEX_ROWS.md):
95 backend /31 frontend scenarios, Clippy, and formatting passed.
Operator quotas and maintenance are also connected: separate 4096 objects /64 MiB
for data, index, and outgoing, one bounded attempt per second each. A failure of one
store does not block the cleanup of the others; an empty profile receives no extra rows.
[Operator Runtime](../evidence/reviews/AR2-wallet-flow/OPERATOR_RETENTION.md):
96 backend /31 frontend scenarios, Clippy, and formatting passed.
Observations are now also stored separately per operation: their own
byte quota, original signatures/readAt, atomic writes together with the receipts,
and a fourth independent Runtime cleanup queue. Checks of 134 genuine
observations, the migration, and the four stores passed: 98 backend /31 frontend,
Clippy, formatting, and 59 unchanged focused inputs.
[Observations](../evidence/reviews/AR2-wallet-flow/OBSERVATION_ROWS.md).
The signed page format is now implemented: 257 leaves and 255 branches match
the independent Python oracle, and appending does not rewrite old pages.
The connectivity checks keep the live parts of the old root at their original positions,
reject signed substitutions, and explicitly distinguish expired subtrees. 28 targeted
crypto /31 frontend checks passed. [Pages](../evidence/reviews/AR2-wallet-flow/HISTORY_PAGES.md).
Core now stores the sender's pages, membership, and the current root in one
transaction with bounded immutable packets. Nine new scenarios passed:
130 originals plus the next append, cold snapshots/proofs, SQL rollback,
the exact v1 migration, and a real MLS epoch change. All 81 targeted backend /
31 frontend tests, Core/crypto Clippy, and formatting passed on 66 focused inputs.
[Core pages](../evidence/reviews/AR2-wallet-flow/CORE_HISTORY_PAGES.md).
[Paid page retention](../evidence/reviews/AR2-wallet-flow/PAID_HISTORY_PAGES.md)
now accepts exact signed bodies under a genuine paid anchor. The shared 32 KiB
limit accounts for the stored versions and the legacy manifest; the full JSON map counts against
the operator quota. 106 backend /31 frontend, Clippy/fmt, and 124 focused
inputs passed: cold reads, current trust, SQL rollback, quotas, and expiry. Confirming
one page does not prove the availability of the whole graph. Next, connect network
publication/traversal, root/pointer fences, and atomic recipient imports; the current
sender still publishes the old flat manifest. When paid space is insufficient,
publication stays pending; old live pages must not be deleted or extended.
Acceptance still requires an ordinary long paid send, sender/data/
index loss, and full recovery by the recipient; a component test does not replace it.

[Network pages](../evidence/reviews/AR2-wallet-flow/NETWORK_HISTORY_PAGES.md)
are now transferred through the existing paid-custody protocol: exact confirmation
after the SQL commit, a signed body with paid evidence on read, and explicit unavailable
and capacity responses. Six new TCP/Noise scenarios on public paid fixtures and
nine transport regressions passed; also 31 frontend, Clippy/fmt, and 133 focused
hashes. The active connection limits and 16 requests per peer are preserved. The next
step is a durable page/graph ACK at the sender, bounded traversal after restart,
root/pointer fences, and atomic per-reference imports. Ordinary workers are still v1;
this result does not update the native acceptance and does not close all of D02.

The [sender page confirmation journal](../evidence/reviews/AR2-wallet-flow/OUTGOING_HISTORY_PAGES.md)
now stores separate bodies and ACKs per exact page and paid position.
The old root does not lose confirmations after a new one; a known peer's response for a different
position is rejected. Cold trust, the shared version/legacy limit, the exact JSON
and ACK quota, SQL rollback, semantic corruption, and expiry were verified. 116 backend /
31 frontend, Clippy/fmt, and 137 focused inputs passed. The signature and allowance checks are shared
with the operator store. A stored ACK does not imply a new availability observation.

[Confirmations from the network](../evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_SENDER.md)
are now recorded through the ordinary check of request ID, peer, connection, time,
and relay policy. The shared bounded client sends old leaves and typed
leaf/branch/root; the current sender already uses the new exact-page journal.
A late response confirms only its own body, including after the next root has been prepared.
27 targeted backend /31 frontend, node all-target Clippy, and fmt passed.
An ordinary paid CLI scenario confirmed ten ACKs of the exact page after
restart, retirement, and recovery after a real loss of 18 data and 18 index
copies. A second native gate verified four SQL failure stages and a cold retry; both
passed on 790 unchanged inputs. This is acceptance of the listed native scenarios; the GUI and
all of V1 are not closed here.
[Core admission from the start of a conversation](../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md)
now preserves the MLS order on both contact-creation paths and in the shared
direct/custody application handler. Future generations receive no ACK and do not
change MLS, messages, import rows, or the cursor. The original arrival order
1..129,0 is preserved; after filling the gap and repeating requests, all
130 exact originals were recovered, including restart and a constant number of records per import.
The v2 profile stores the policy explicitly; old contacts remain legacy. A new root acceptance
or the application of an old token cannot declare an old unsecured epoch
secure: `HistoryRecoveryRequired` is returned without changing the former data.

141 targeted backend /31 frontend, Core/crypto/node Clippy, and fmt passed.
798 inputs are unchanged in every run. The previous
[R19 failure](../evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_IMPORT.md) and
[MLS prerequisite](../evidence/reviews/AR2-wallet-flow/MLS_CONTIGUOUS.md) are preserved.
The ordinary native CLI first failed the SQL import gate: an unpublished
service sentinel in the same MLS chain left a real gap. This run
remains a FAIL. The independent critic accepted a separate genuine MLS conversation for
the service payment; the repeated positive scenario verified two paid sends,
budget/retry, SQL faults, cold page ACKs, retirement, and recovery after the
loss of 18 data and 18 index copies. Six QC signatures were verified. Such a result
does not close the original missing-original gap and is not a new GUI acceptance.

Next — explicit network/product gap and legacy recovery outcomes, bounded
publication and resumable graph traversal, then the >128 live paid native gate.
Root/pointer and retirement may be confirmed only after the required child ACKs;
reaching the page bound does not mean full recovery. The workers still use the
v1 flat manifest. The first offline Welcome, control/epochs/rejoin, independent
repair, and the full 67-card/22-E2E/three-platform scope remain mandatory.

The V1 testnet uses explicitly pinned threshold checkpoint attestors: this is not
L2 light-client finality. A trust manifest, independent key administrators, and
lease/outage drills are needed; ten signatures/keys do not prove ten failure domains.
After the lease ends, new promises stop, but the active historical
paid retrieval must not depend on a new company checkpoint.

Runtime credentials with 0600/0700 permissions protect against other UIDs; an arbitrary agent
with unrestricted same-UID access is not in a sandbox. A supported V1 host
must keep this boundary; OS isolation for untrusted code is a separate
task. Ciphertext availability does not recover lost root/MLS secrets.
Backup/rejoin must explicitly show the recoverable range and the gaps.

Measure separately: local commit, direct ACK, QC, receipt 1/10, index publication,
offline page/decrypt, repair time, retries, CPU/event-loop stalls, wire bytes,
disk writes, and memory. The 128×500 ms = 64 s calculation is the planned traversal interval,
not a measured end-to-end latency. The local 164 ms of a single stage and the former native
runs are not WAN p95/p99 or evidence of the current revision's readiness.

For every new backend module: tests first, a separate backend-test-critic
without inherited context, ACCEPT before production, and backend and frontend checks
after implementation. The evidence records the source revision/hash, commands, results,
and limitations. Do not add up test counts from different revisions; keep the failed runs.
Acceptance of a small module does not close a card, a stage, or all of V1.

## Fast diagnostic loop for runs — September 19

An architectural decision on speeding up the native run cycle was accepted (analysis at
`d278f2abae35bf5e50848bee4ee03ae1efa48210`). The plan gained the task
[V1-C05](agentic_internet_v1_execution_plan/v1-plan-2026-09-14/00-tasks.md#v1-c05).

Two separate loops:

1. **Harness:** selection of the target diagnostic phase, a budget for the absence of useful
   progress (on the order of 90–120 s as a local default, not a new liveness bound),
   early stop with a full snapshot. For publication — page identity,
   revision, manifest hash, the ACK set, data/index receipts, capacity/error
   counter deltas; for late H11 — the exact inbox MessageIDs,
   recipient cursor/deferred, `custodySync`, the exact SQL fault event; the import of
   previous originals and the exact fault are verified as separate conditions.
   An early stop means "observation obtained" or "stall suspected",
   not a failure of the full scenario.
2. **Controlled time:** a "clocks + timers + events" seam with four loops —
   protocol time (`issued_at`, lease, expiry, signature checks), monotonic
   scheduler time (`due`, backoff, admission), chain time (block timestamp,
   proofs/checkpoints), and real execution time. In the simulation profile the first
   two run under one controller with a jump to the nearest event. The incomplete
   `_at` seams (`maintain_public_sender_at`, `created.elapsed()` in paid_custody,
   a separate `SystemTime` in the embedded finalizer) are brought to a consistent state.
   The reproducer is several logical nodes in one process with
   production transitions, controlled message delivery, and a real SQLCipher;
   Anvil is attached through a separate adapter (timestamp → block → result).

Rejected paths: faking the system clocks does not move `Instant`/timers and
desynchronizes expiry from backoff; `tokio::time::pause` does not cover
`std::Instant` and requires the `current_thread` runtime; scaling
`authority_lease`/`registry_snapshot_lease`/the message count removes the
128-bound check, the cold-traversal branch below 129, and the consistency of `Source` with
1800/3600 — diagnostic renewal is done with `maintain(force=True)` in the needed
phase. QEMU `icount` with `sleep=off` can technically jump to the next
timer, but that is a separate experiment, not the main path.

Acceptance discipline: the full unchanged A04/H11 remain the only
acceptance evidence for a fix candidate; the virtual elapsed time in
the evidence is separated from native elapsed. In parallel, extra pauses
after dependencies become ready are checked — event-driven wakeup without weakening
the mandatory backoff/admission constraints.
