# V1 evidence matrix — 67 cards / 22 scenarios

## Current stage — September 14

Diagnostic32: [observer diff and the rig were accepted by a separate critic](../../evidence/reviews/AR2-wallet-flow/diagnostic32-test-review.json).
[Targeted checks](../../evidence/reviews/AR2-wallet-flow/diagnostic32-checks.json):
122 backend / 47 frontend, Clippy/fmt and the release build passed before the run.
[Baseline32 ended with a failure](../../evidence/reviews/AR2-wallet-flow/diagnostic32-native-r1.json)
of the first SQL gate (120 seconds): 32 originals published in 542 seconds,
23 leaves, 288 + 288 real losses, after trust — 48 reads and zero imports.
316 deferred attempts produced no import. Selection of originals 1 and 31 ended
twice in each case with a global-read-rate wait before enqueue. This does not
prove the R14 cause. Source/binary guard and cleanup passed. The diagnostic
ordinal mapping (zero-based numbering) was fixed; 5 parser / 47 frontend pass,
the partial trace was re-verified, the original failed report is preserved. The
functional steps of the shared publisher, read continuation and unified staging
remain open.

After an additional architect review the [paid history lifecycle
plan](../../Docs/V1_HISTORY_LIFECYCLE_R14.md) was accepted: shared batch/history
publication, read continuation across Work and a single store of pending bodies.
The [offline reconciliation](../../evidence/reviews/AR2-wallet-flow/r14-architecture/recheck.json)
reproduces the R14 structural measurements: 117 leaves (106 single), 230 pages,
89 unique originals in the union of imports and caches. At review time all 21
provided files with code matched the project. The cause of skipping 31 remains
unknown. The baseline is frozen; the failed trace is preserved and analyzed.
The first [component of shared collection of a finished
batch](../../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_COLLECTION.md) is
implemented: short passes continue one group, and before the atomic commit the
current Core/Custody inputs are verified. The critic accepted the implementation
and a separate legacy test; the [check
results](../../evidence/reviews/AR2-wallet-flow/shared-collection-checks.json)
relate to the collector. Now the [shared graph/pointer
stages](../../evidence/reviews/AR2-wallet-flow/SHARED_HISTORY_PUBLISHER.md) are
also implemented: different jobs continue one traversal, errors keep the retry,
and successor and two conversations do not mix ACK/pointer. Separate
authorization and Core retirement of each job are preserved. The [group
placement priority](../../evidence/reviews/AR2-wallet-flow/PLACEMENT_PRIORITY.md)
is implemented: at most 12 queued originals, a bounded no-progress window,
preserved positions of other conversations and replacement after a pause. The
critic accepted the implementation; 42 backend / 63 frontend, Clippy/fmt pass.
The [collector under new
reservations](../../evidence/reviews/AR2-wallet-flow/ARRIVAL_FENCES.md) is
verified as well: four paid originals form one leaf between expensive quanta
with new sends. Core allows only monotonic growth of an existing book's
counters, preserving immutable bases, exact own rows and the current
authorization. Independent ACCEPT; 80 unique backend / 63 frontend, Clippy/fmt
pass. Next — read continuation and unified staging, then a measurable
improvement of Diagnostic32 up to an unchanged Full130. Real 20 ms and native
throughput are not measured. The [ready output before Work
completion](../../evidence/reviews/AR2-wallet-flow/READY_OUTPUT.md) now applies
through the previous checks; after the deadline a new request is not queued.
Independent ACCEPT, 60 backend /63 frontend /5 parser, Clippy/fmt. This is part
of step 2; the unfinished root proof still requires carry-over. The [Core range
API](../../evidence/reviews/AR2-wallet-flow/RANGE_CORE.md) now persists the
holder position and fully accepted bodies in one transaction, with cold
restart, SQL rollback, exact-head fences and compatibility with the previous
body cache. Six new tests and the implementation were accepted by the critic;
the network receiver still needs to switch to this API. [Holder
errors](../../evidence/reviews/AR2-wallet-flow/NO_LEGACY_ON_FAILURE.md) no
longer create a legacy retry; six policy tests pass, the critic accepted the
code. Deadlines, limits and the V1 boundary are preserved.

A short test confirmed a separate sender-observation defect: a temporarily
empty history on a root change or cold restart caused `checkpoint_rejected`.
Node now defers such an incomplete observation while keeping the strict Core
checks. The independent critic accepted six tests; the previous code failed in
two cases. [Fix checks](../../evidence/reviews/AR2-wallet-flow/sender-observation-checks.json):
26 backend / 47 frontend, Clippy/fmt pass. Native throughput is not yet
measured; no new release build was run. Next — a short trace of the existing
read of the next missing original; a new Full130 only after a measurable
improvement. The [brief current status](../../IMPLEMENTATION_STATUS.md)
replaces history accumulation in the root summary; the old records are fully
preserved separately.


[Full130 R14](../../evidence/reviews/AR2-wallet-flow/history-range-native-r14.json)
ended during recovery: all 130 originals published, 390 signatures verified,
1300 data/1300 index receipts and 13000 location ACKs received. After a real
loss of 1170+1170 copies, the no-trust refusal and the first SQL failure with
an exact rollback passed. During the next phase the earliest original expired.
The post-stop snapshot confirms imports of exactly 1..30; original 31 is in
neither prefetch nor deferred. This does not prove the cause of the individual
delays.

All 956 inputs and the five signed artifacts are unchanged over 3646 seconds;
cleanup is clean. Deadlines, limits and criteria were not changed. The last SQL
failure, the full import of 130 and a cold full traversal were not reached. The
new post-provider-failure diagnostics selected nothing: all location ACKs were
received. The R13 failure was not reproduced, but its cause is likewise not
provenly eliminated. Next — reproducing tests of read scheduling and temporary
reset of sender observations on a history root change. Full130 and all of V1
remain open.

[Full130 R13](../../evidence/reviews/AR2-wallet-flow/history-range-native-r13.json)
ended with the seventh batch failing publication within the previous 600
seconds. 96 originals published; the next 16 have 160 data/160 index receipts
and 1580 of 1600 location ACKs. Messages 107 and 111 lack confirmation of ten
locations on the same provider (position 9); the connection to it is present in
the sender's snapshot. The cause of the delay is not yet proven; a reproducing
test and more precise post-failure sender/provider diagnostics are needed. All
956 inputs and the five artifacts are unchanged over 2375.4 seconds; teardown
is clean. Full-range signature verification, copy loss and recipient recovery
were not reached. The node hash matches the build; the CLI post-check was not
reached, so its hash is absent from the report.

[Native20 R10](../../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r10.json)
passed completely: 20 ordinary paid originals, 60 verified signatures, a real
loss of 180+180 copies, the no-trust refusal, both SQL failures with exact
rollback, partial import 1..19, full 1..20 and cold confirmation without new
imports. The first SQL failure was reached after 21 requests, the partial phase
— after 59. All 956 inputs and the five signed binaries are unchanged over
708.4s; teardown is clean. Full130 R13 ran on the same release build after
independent acceptance of the rig; its failure is described above. The success
of 20 messages does not close 130, an independent testnet or the full scope of
67 cards / 22 scenarios / 3 OSes.

After exhausting the total request limit, the receiver now keeps the current
attempt and continues a bounded pass over available local data. The unaccepted
root and ancestry remain pending; limits and atomicity are preserved. Five
tests were accepted by the independent critic before implementation; 118
backend / 51 frontend, Clippy/fmt pass on 397 unchanged inputs:
[scheduling checks](../../evidence/reviews/AR2-wallet-flow/history-admission-yield-checks.json).
The ordinary release build and the strict signature check passed on 922
unchanged inputs.

The ordinary send queue groups finished originals into pages of 1–12
references. Before including each, the active permission and its paid
indexes/confirmations are re-verified. Collection takes one 500ms interval;
extra checks yield to the main loop within the previous 20ms budget. The order
of acknowledging child pages before the parent, root and pointer is preserved.
Independently verified tests and the implementation pass 71 Node/2 Core/51
frontend, Clippy/fmt:
[batching checks](../../evidence/reviews/AR2-wallet-flow/history-batch-scheduling-checks.json).
Core previously passed 96 related history tests, including 130 originals.

The previous [Native20 R9](../../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r9.json)
confirmed sending 20 originals, 60 signatures and a real loss of 180+180
copies. The first SQL failure was not reached within the previous 120s: 48
requests, 25 bulk, 12 path requests, 7 deferrals, 0 imports and 0 storage
errors. All 955 inputs and 5 binaries are unchanged; processes and temporary
profiles were cleaned up.

The snapshot after a regular receiver stop shows: original 1 is eighth in the
signed history; originals 1 and 2 are already in the cache. In total 10
prefetched and 7 deferred originals, the exact imports list is empty; 6, 11 and
16 are absent from both caches. This is the post-stop state, not an exact
snapshot of the deadline. The presence of ciphertext by itself does not allow
an import without confirmation of the current history root. This failure
defined the subsequent scheduling regressions under the previous limits of 24
requests total / 12 per peer per 60s; the result of the fix is Native20 R10
above. R8 with the first exact SQL rollback and R6 with the full Native20 are
preserved as historical results. The full V1, 67 cards, 22 scenarios and three
OSes remain open.

Automated tests use isolated temporary keys and do not touch the login
keychain. This is verified on real application restarts:
[test key storage](../../evidence/reviews/AR2-wallet-flow/E2E_SECRETS.md).

Core recovery status is implemented and verified: 71 backend/35 frontend,
Clippy/fmt. The shared owner projection and signed agent reader are bound to
root/epoch/expiry and the actually committed imports; 7 new tests were accepted
by the independent critic.
[Checks](../../evidence/reviews/AR2-wallet-flow/history-recovery-core-checks.json).
Worker, CLI/MCP and UI are wired: 30 backend/45 frontend, Clippy, frontend
build and headless visual pass. The [paid native
scenario](../../evidence/reviews/AR2-wallet-flow/history-recovery-native-r1.json)
passed on 867 unchanged inputs: six signatures, a real loss of 18+18 copies
without the sender, SQL failure 0/2, partial import 1/2, full 2/2 and cold 2/2
via CLI/owner pages. Native GUI recovery, recovery/rejoin, full130 and 67/22/3
remain open.

Current focused gate: [pending renewal GREEN R3](../../evidence/reviews/AR2-wallet-flow/pending-renewal-native-green-r3.json)
passed both successor/cold scenarios and sender-absent recovery: 6 signatures,
833 unchanged inputs, clean teardown. [Full130 release R6](../../evidence/reviews/AR2-wallet-flow/history-range-native-r6.json)
ended after 128 stored originals and 130 accepted sends: the correct
over-budget refusal passed, the comparison of the mutable message snapshot did
not. All 834 inputs are unchanged. The critic-accepted test fix passed the
[short release BO-R3](../../evidence/reviews/AR2-wallet-flow/budget-oracle-native-r3.json):
6 signatures, 837 unchanged inputs, physical copy loss and cold recovery.
[Full130 release R7](../../evidence/reviews/AR2-wallet-flow/history-range-native-r7.json)
ended after publishing 130 originals and verifying 390 signatures; all 838
inputs are unchanged. Before the first copy loss the test SQL helper failed
with an obsolete limit of two retained objects (10–11 needed); a separate
regression reproduced the failure and passes after removing the limit: 1
backend/14 frontend, Clippy/fmt. Teardown is clean; full-range recovery and all
of V1 are not yet accepted.

Snapshot of September 12, 2026; the exact source and binary inputs are linked
in the [acceptance of the two books](../../evidence/reviews/AR3-history-books/acceptance.json).
The full V1 is **not accepted**. The boundary is
[release-scope.json](release-scope.json); 17 cards and 4 V2 scenarios are
excluded only by the user's direct decision. Three OSes remain mandatory.

"Partial" — the named part is verified at the revision of the linked report.
"Not demonstrated" — there is no full product demonstration here; the presence
of components is not ruled out. A historical GREEN is not passed off as the
general current release gate. Full status is not derived from the number of
unit tests.

The full declared range of two originals from genuinely disjoint paid books is
accepted: ordinary send/recovery, cold sender, mixed retention, real node
losses, SQL rollback and cold dedup. Next a whole user send is closed —
wallet/price/TTL/budget, shared status and lifecycle. Additional preparation is
allowed only for an observable obstacle to the complete scenario.

[Core receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md)
now applies from conversation creation. The original 130-original gate passes
after arrivals 1..129,0, an explicit deferral and repeated requests. Legacy
contacts require recovery before the v2 root/token. 141 backend /31 frontend,
Clippy/fmt passed; 798 inputs are unchanged in every run. The old failing
reports are preserved.

The first native CLI replay with an unpublished early message in the same chain
remains FAIL. An independently verified fixture moves the service payment into
a separate MLS conversation; on this baseline two paid originals, budget/retry,
SQL/cold, page ACKs/retirement and recovery after an 18 data/18 index loss with
6 QC signatures pass. The original missing-original gap, the network
graph-over-128 and the GUI are not closed here.

[The durable receiver cursor](../../evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md)
now passes the limit of 16 attempts after a restart and restores all 17
originals in a real Core/Runtime scenario. 72 backend /31 frontend, Clippy/fmt
and a repeat paid CLI loss/recovery on 802 stable inputs passed. This is the v1
traversal; the ordinary v2 graph, >128 paid native and the full scope remain
open.

[The Core checkpoint and shared queue](../../evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md)
passed exact root/member/pointer, SQL/cold retry and a real MLS epoch
separation; indexes and history yield to the main loop after expensive
preparation. 117 backend /31 frontend, Clippy/fmt and the ordinary paid native
C8 passed. The ordinary v2 graph and the >128 live paid native gate are still
open.

[The graph reference cursor](../../evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md) passed
128 originals inside a v1 leaf +2 v2 leaves, bounded cold passes, signed
expired-subtree skips, exact pointer/root, SQL and a real MLS epoch change.
105 backend /31 frontend, Clippy/fmt pass on 811 unchanged inputs, including
the previous full Core import of 130 originals. An attempt does not mean
delivery or history completeness. The ordinary Node graph is not wired yet;
native was not repeated here.

[The ordinary graph receiver](../../evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md)
is wired: 130 genuine MLS originals, cold bounded attempts, paid reply
validation, child routes and SQL root rollback. 30 backend /31 frontend,
Clippy/fmt and the paid v1 CLI loss/recovery passed. Next the sender child ACK
order and the >128 live paid native graph; the earlier component next-work
entries above describe historical slices.

[The ordinary graph sender](../../evidence/reviews/AR2-wallet-flow/GRAPH_SENDER.md)
now passes the paid CLI gate: five pages /50 ACKs with the order children →
parents → root/pointer, SQL/cold retry and recovery of two originals without
the sender after a real copy loss. 30 backend /31 frontend, Clippy/fmt; clean
C1 on 822 unchanged inputs. Next >128 simultaneously live paid originals in
native recovery; V1 completeness is not claimed.

[Authority renewal from the ordinary wallet](../../evidence/reviews/AR2-wallet-flow/WALLET_RENEWAL.md)
passes a real peer successor, preservation of the original tickets/admission
evidence, exact page ACK rows after a restart and sender-absent loss/recovery:
native renewal R2 on 826 unchanged inputs; 54 backend /31 frontend, Clippy/fmt.
The full release [run of 130 originals R5](../../evidence/reviews/AR2-wallet-flow/history-range-native-r5.json)
ended with an error at the original deadline of the fifth batch: 64 stored
originals, three real successor heads, 826 unchanged inputs and clean teardown.
Eight unfinished finalizations took all client slots; the next step is
[renewal of a started request](../../evidence/reviews/AR2-wallet-flow/PENDING_POSTAGE_RENEWAL.md).
The focused [native RED R2](../../evidence/reviews/AR2-wallet-flow/pending-renewal-native-red-r2.json)
reproduces the hang of the first payment after a real successor and cold
restart: 827 unchanged inputs, clean teardown. Pending-context refresh passes
31 backend/14 frontend, Clippy/fmt. Native GREEN R1 on 829 unchanged inputs
passed the first phase; in the second the payment completed (pending0), but
sender storage did not finish. Separately, the previous QC is being checked
with the new storage admission. TTL/quotas are preserved; the full range is not
yet accepted. The standalone sender migration to graph received the
[critic's ACCEPT](../../evidence/reviews/AR2-wallet-flow/standalone-sender-graph-critic.json),
applied after R5 finished. [Native R1](../../evidence/reviews/AR2-wallet-flow/standalone-sender-graph-native-r1.json)
passes: six QC signatures, four SQL faults, cold retirement and remote graph
reads, 826 unchanged inputs and clean teardown. After removing the full-range
blocker the next product gate is the
[user-observable recovery status](../../evidence/reviews/AR2-wallet-flow/HISTORY_RECOVERY_PRODUCT.md).

## Cards

| ID / capability | Evidence and the verified part | What remains for full acceptance |
|---|---|---|
| [F01](../agentic_internet_v1_1_plan/tasks/F01.md) · Verifiable specification of wishes and prohibition of requirement loss | **Partial.** Scope 67/22 and the DAG are preserved. [plan](../../Docs/agentic_internet_v1_execution_plan/planning-validation.json) | Link every card to full executable acceptance |
| [F02](../agentic_internet_v1_1_plan/tasks/F02.md) · Canonical wire protocol and version compatibility | **Partial.** A strict signed CBOR envelope and vectors. [wire](../../evidence/reviews/F02-signed-document.md) | The whole V1 wire corpus and three-OS compatibility |
| [F03](../agentic_internet_v1_1_plan/tasks/F03.md) · Deterministic simulator of network, time and failures | **Partial.** Finalizer and component-failure model/tests. [finalizer](../../evidence/reviews/P01-engine.md) | A general reproducible replay of network product scenarios |
| [F04](../agentic_internet_v1_1_plan/tasks/F04.md) · Transactional state journal and reliable local queue | **Partial.** SQLCipher, atomic MLS/outbox/import, real SQL faults and bounded traversal of 257 rows after restart. [store](../../evidence/reviews/F04-profile-store.md), [history](../../evidence/reviews/AR3-history-automatic/README.md), [range](../../evidence/reviews/AR2-wallet-flow/STATE_RANGE.md); The v2 contact admission policy migrates atomically with MLS/contact/outbox and preserves legacy mode after SQL rollback and restart. [receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md) | Migrations, disk-full and rollback of the whole profile |
| [F05](../agentic_internet_v1_1_plan/tasks/F05.md) · Executable model of threats, committees and resource economics | **Partial.** An executable conditional model of quorum and fixed units. [risk](../../evidence/reviews/F05-committee-risk.md) | Link real testnet parameters to the full resource model |
| [F06](../agentic_internet_v1_1_plan/tasks/F06.md) · Independent test oracles and delivery pipeline | **Partial.** Independent funding/QC/Noise oracles and run hashes. [history](../../evidence/reviews/AR3-history-automatic/README.md), [handover](../../evidence/reviews/AR1-native-handover/README.md), [books](../../evidence/reviews/AR3-history-books/acceptance.json) | A single release pipeline for all suites and OSes |
| [I01](../agentic_internet_v1_1_plan/tasks/I01.md) · Local owner, permanent address and key storage | **Partial.** Persistent identity, keychain and cold desktop. [native](../../evidence/reviews/D05-paid-index-store/release.json) | Clean install and keystore on every OS |
| [I02](../agentic_internet_v1_1_plan/tasks/I02.md) · Delegation to devices, agents and runtimes with limits | **Partial.** Scoped runtime grants, limits and revoke. [broker](../../evidence/reviews/I02-agent-broker.md), [grants](../../evidence/reviews/U04-native-runtime-panel.md) | Device/organization delegation and the full budget path |
| [I03](../agentic_internet_v1_1_plan/tasks/I03.md) · Revocation, epoch change, new owner and recovery | **Partial.** Runtime revocation with a check before the action. [broker](../../evidence/reviews/I02-agent-broker.md) | Full device/owner epoch and recovery without rights returning |
| [I04](../agentic_internet_v1_1_plan/tasks/I04.md) · Contacts, invitations and safe first message | **Partial.** Invitations and the first established MLS connection. [native](../../evidence/reviews/D05-paid-index-store/release.json) | First message with an initially offline recipient |
| [I05](../agentic_internet_v1_1_plan/tasks/I05.md) · MLS client for direct chat and multiple devices | **Partial.** Real MLS and direct chats. [mls](../../evidence/reviews/I05-mls-adapter.md), [native](../../evidence/reviews/D05-paid-index-store/release.json); Core includes contiguous receive from conversation creation; old contacts keep legacy mode and require recovery before the v2 root/token. [receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md) | An agreed multi-device lifecycle and long offline |
| [I06](../agentic_internet_v1_1_plan/tasks/I06.md) · Encrypted backup, migration and rollback protection | **Not demonstrated.** The review recorded a backup/recovery gap. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Encrypted backup/import, current authorities and rollback protection |
| [N01](../agentic_internet_v1_1_plan/tasks/N01.md) · Authenticated P2P channel and backpressure | **Partial.** Noise/TCP/QUIC, bounded streams, traversal of >32 peers and cold reconnect under a connection limit. [resources](../../evidence/reviews/N03-processing-reservations/README.md), [history](../../evidence/reviews/AR3-history-automatic/README.md), [books](../../evidence/reviews/AR3-history-books/acceptance.json) | A general hostile-peer acceptance on the current revision |
| [N02](../agentic_internet_v1_1_plan/tasks/N02.md) · First run without a single bootstrap server | **Partial.** Bootstrap exchange and saved independent hints. [bootstrap](../../evidence/reviews/N02-bootstrap-exchange.md), [routing](../../evidence/reviews/N03-dht-roles/README.md) | Clean entry into an independent network with the company off |
| [N03](../agentic_internet_v1_1_plan/tasks/N03.md) · Partial Kademlia routing and secret mailbox rendezvous | **Partial.** DHT roles, private pointer and paid indexes. [routing](../../evidence/reviews/N03-dht-roles/README.md), [history](../../evidence/reviews/AR3-history-automatic/README.md) | The rendezvous/control lifecycle and index repair |
| [N04](../agentic_internet_v1_1_plan/tasks/N04.md) · NAT traversal and decentralized relays | **Partial.** Linux NAT/relay loss and recovery of a real conversation. [nat](../../evidence/reviews/N04-linux-nat.md) | The full hole-punch/relay matrix with release CLI and desktop |
| [N05](../agentic_internet_v1_1_plan/tasks/N05.md) · Verifiable node set and safe placement selection | **Partial.** Verified registry proofs and real pre-beacon assignments of two disjoint books. [registry](../../evidence/reviews/L02-authenticated-registry-proofs.md), [history](../../evidence/reviews/AR3-history-automatic/README.md), [books](../../evidence/reviews/AR3-history-books/acceptance.json) | Repair assignments and independent testnet operators |
| [N06](../agentic_internet_v1_1_plan/tasks/N06.md) · Operator mode: resources, quotas and signed offers | **Partial.** A real daemon, limits and operator bindings. [resources](../../evidence/reviews/N03-processing-reservations/README.md) | Full operator UX, durable repair and resource payment |
| [D01](../agentic_internet_v1_1_plan/tasks/D01.md) · Verifiable envelope and cheap admission protection | **Partial.** An immutable MLS envelope, paid admission and peer checks. [history](../../evidence/reviews/AR3-history-automatic/README.md) | Ordinary attachments and the control payload through the same admission |
| [D02](../agentic_internet_v1_1_plan/tasks/D02.md) · Storage, retrieval and delivery lifecycle | **Partial.** Automatic send/store/retrieve, cold retry and retirement; local pages of 258 envelopes; separate outgoing, incoming data/index rows for 134 paid originals with quotas, cleanup and SQL rollback; Runtime data/index/outgoing/observation quotas and independent bounded maintenance; 134 signed observations with cold freshness and atomic migration; the 257 signed-page format and live-prefix verification; Core sender batches, atomic v1 migration and epoch fences; paid page bodies with a shared allowance, cold trust and SQL/expiry; typed page put/read over TCP/Noise with exact paid responses, SQL fences and capacity; separate durable page ACKs in outgoing paid rows with peer/position binding, cold trust and a shared quota; matched network ACK via ordinary authorization and a new journal in the active sender; native CLI restart/retirement and a real loss of 18 data +18 index copies with recovery. [books](../../evidence/reviews/AR3-history-books/acceptance.json), [lifecycle](../../evidence/reviews/AR2-wallet-flow/status-native-candidate-3.json), [envelopes](../../evidence/reviews/AR2-wallet-flow/ENVELOPE_PAGES.md), [outgoing](../../evidence/reviews/AR2-wallet-flow/OUTGOING_ROWS.md), [Runtime](../../evidence/reviews/AR2-wallet-flow/RUNTIME_RETENTION.md), [incoming data](../../evidence/reviews/AR2-wallet-flow/INCOMING_ROWS.md), [incoming index](../../evidence/reviews/AR2-wallet-flow/INDEX_ROWS.md), [operator Runtime](../../evidence/reviews/AR2-wallet-flow/OPERATOR_RETENTION.md), [observations](../../evidence/reviews/AR2-wallet-flow/OBSERVATION_ROWS.md), [pages](../../evidence/reviews/AR2-wallet-flow/HISTORY_PAGES.md), [Core pages](../../evidence/reviews/AR2-wallet-flow/CORE_HISTORY_PAGES.md), [paid pages](../../evidence/reviews/AR2-wallet-flow/PAID_HISTORY_PAGES.md), [network pages](../../evidence/reviews/AR2-wallet-flow/NETWORK_HISTORY_PAGES.md), [sender page ACKs](../../evidence/reviews/AR2-wallet-flow/OUTGOING_HISTORY_PAGES.md), [sender network](../../evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_SENDER.md); Core restores all 130 originals after arrivals 1..129,0 without dropping MLS keys; native: two originals passes with an isolated setup sentinel. [receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md); The durable v1 cursor and Runtime passed 16 gaps → cold restart → original 17 → all 17; the paid two-original loss/recovery passes again. [cursor](../../evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md); The graph Core checkpoint is atomic with retirement; exact root/member/pointer, SQL/cold retry and a real MLS epoch change are verified. [checkpoint](../../evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md); The Core cursor counts 130 references in the wrapped v1 128 +2 v2 leaves and survives bounded cold passes; authenticated expiry skip, exact root/pointer, real epoch and SQL rollback are verified; attempts do not mean import or completeness. [graph cursor](../../evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md); The ordinary receiver is wired: 130 genuine MLS originals after cold bounded passes, paid typed replies, child routes, expiry skip and SQL root rollback; v1 paid native compatibility passes. [receiver](../../evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md); The ordinary sender is wired: five pages /50 child-before-parent ACKs, last-child SQL/cold retry, exact root/pointer and sender-absent recovery of two real paid CLI originals; clean C1 on 822 unchanged inputs. [sender](../../evidence/reviews/AR2-wallet-flow/GRAPH_SENDER.md); The ordinary peer successor and cold original evidence/ACK rows pass in native R2 on 826 inputs. [renewal](../../evidence/reviews/AR2-wallet-flow/wallet-renewal-native-r2.json) | >128 simultaneously live paid originals in ordinary native graph recovery; user-visible gap/legacy/rejoin outcomes and the first offline Welcome |
| [D03](../agentic_internet_v1_1_plan/tasks/D03.md) · Ten replicas, retention certificates and honest durability status | **Partial.** 10 original receipts, paid copies/indexes; the shared persisted UI/CLI/MCP status separates ACK, partial storage and expiry. [copies](../../evidence/reviews/D03-paid-custody-copies/README.md), [status](../../evidence/reviews/AR2-wallet-flow/status-native-candidate-3.json), [GUI](../../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md) | Automatic R10 recovery |
| [D04](../agentic_internet_v1_1_plan/tasks/D04.md) · Autonomous loss detection and repair without owners | **Partial.** The paid copy and inspection as components. [copies](../../evidence/reviews/D03-paid-custody-copies/README.md) | Detect the loss and restore R10 with both clients offline |
| [D05](../agentic_internet_v1_1_plan/tasks/D05.md) · Durable indexes, cursors and long-offline recovery | **Partial.** Both originals of the two disjoint funded books were recovered without the sender after node loss, SQL faults and cold retry. [history](../../evidence/reviews/AR3-history-automatic/README.md), [books](../../evidence/reviews/AR3-history-books/acceptance.json); Exact-root/leaf proofs, per-operation imports and cold gap retry restored all 130 Core originals. [receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md); The durable v1 cursor and Runtime passed 16 gaps → cold restart → original 17 → all 17; the paid two-original loss/recovery passes again. [cursor](../../evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md); The graph Core checkpoint is atomic with retirement; exact root/member/pointer, SQL/cold retry and a real MLS epoch change are verified. [checkpoint](../../evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md); The Core cursor counts 130 references in the wrapped v1 128 +2 v2 leaves and survives bounded cold passes; authenticated expiry skip, exact root/pointer, real epoch and SQL rollback are verified; attempts do not mean import or completeness. [graph cursor](../../evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md); The ordinary receiver is wired: 130 genuine MLS originals after cold bounded passes, paid typed replies, child routes, expiry skip and SQL root rollback; v1 paid native compatibility passes. [receiver](../../evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md); The ordinary sender is wired: five pages /50 child-before-parent ACKs, last-child SQL/cold retry, exact root/pointer and sender-absent recovery of two real paid CLI originals; clean C1 on 822 unchanged inputs. [sender](../../evidence/reviews/AR2-wallet-flow/GRAPH_SENDER.md); The ordinary peer successor and cold original evidence/ACK rows pass in native R2 on 826 inputs. [renewal](../../evidence/reviews/AR2-wallet-flow/wallet-renewal-native-r2.json) | >128 simultaneously live paid originals in ordinary native graph recovery; user-visible gap/legacy/rejoin outcomes and the first offline Welcome |
| [D06](../agentic_internet_v1_1_plan/tasks/D06.md) · Attachments, protected fanout, TTL and garbage collection | **Not demonstrated.** The chunks/TTL/GC requirements remain mandatory. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Full chunked attachment, resume, hash, TTL and disk-full |
| [G01](../agentic_internet_v1_1_plan/tasks/G01.md) · Group with roles, invitations and devices | **Partial.** MLS Add/Remove as library transitions. [mls](../../evidence/reviews/I05-mls-adapter.md) | Ordinary group creation, roles and invitations through the UI |
| [G02](../agentic_internet_v1_1_plan/tasks/G02.md) · Decentralized ordering of the private group journal | **Partial.** BFT engine and durable journals. [finalizer](../../evidence/reviews/P01-engine.md) | Encrypted group control with the payload available before finality |
| [G03](../agentic_internet_v1_1_plan/tasks/G03.md) · Competing MLS commits, removal and safe epoch change | **Partial.** Membership-change cryptography and consensus separately. [mls](../../evidence/reviews/I05-mls-adapter.md), [finalizer](../../evidence/reviews/P01-engine.md) | Concurrent commits, a single roster, deterministic reject/no-op |
| [G04](../agentic_internet_v1_1_plan/tasks/G04.md) · Long offline, control log recovery and safe rejoin | **Partial.** Ordinary message recovery components. [history](../../evidence/reviews/AR3-history-automatic/README.md) | Group control catch-up and safe rejoin beyond the window |
| [G05](../agentic_internet_v1_1_plan/tasks/G05.md) · Two group privacy profiles and paid fanout | **Not demonstrated.** Two privacy profiles are kept in scope. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Group paid fanout and verification of privacy boundaries |
| [G06](../agentic_internet_v1_1_plan/tasks/G06.md) · Organizational groups and delegated budget | **Partial.** Runtime constraints as a component. [broker](../../evidence/reviews/I02-agent-broker.md) | An organizational group and a delegated budget; job execution — V2 |
| [L01](../agentic_internet_v1_1_plan/tasks/L01.md) · Funded issuance of postage stamps on a single EVM L2 | **Partial.** Real paid books, proofs and the ticket reserve; buy/balance/budget through the native GUI and two sends. [funding](../../evidence/reviews/L01-funded-issuance.md), [GUI](../../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md) | Independent testnet operation and external-wallet compatibility |
| [L02](../agentic_internet_v1_1_plan/tasks/L02.md) · Stake registry, epochs, root membership and randomness binding | **Partial.** Bonded snapshot, membership, beacon and epochs. [registry](../../evidence/reviews/L02-authenticated-registry-proofs.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | Independent testnet operation of the registry |
| [L03](../agentic_internet_v1_1_plan/tasks/L03.md) · SubsidyVault: time, shared fund and grant entitlement | **Not demonstrated.** SubsidyVault remains mandatory. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | A real fund, cutoff and non-duplicating entitlement |
| [L04](../agentic_internet_v1_1_plan/tasks/L04.md) · Non-controlling organization share and economic immutability | **Not demonstrated.** The non-controlling royalty is kept in scope. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Conservation and payout under recipient revert/compromise |
| [L05](../agentic_internet_v1_1_plan/tasks/L05.md) · Operator payments and correct service-proof boundaries | **Partial.** Signed custody/copies proofs. [copies](../../evidence/reviews/D03-paid-custody-copies/README.md), [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Funded settlement to the operator without double payout |
| [L06](../agentic_internet_v1_1_plan/tasks/L06.md) · Working L2 adapter: finality, reorg and independent sources | **Partial.** Threshold checkpoints, history and hard leases. [checkpoint](../../evidence/reviews/L06-checkpoint-network.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | Automatic renewal, outage and independent trust administrators |
| [P01](../agentic_internet_v1_1_plan/tasks/P01.md) · Reusable BFT finalizer for bounded protocol journals | **Partial.** Real finality and committee change. [finalizer](../../evidence/reviews/P01-engine.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | Full group integration and a shared release gate |
| [P02](../agentic_internet_v1_1_plan/tasks/P02.md) · Public signed postage and legacy commitment compatibility | **Partial.** Public signed postage and ordinary wallet/price/TTL through the native GUI; the legacy verifier keeps old commitments. [GUI](../../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md), [legacy](../../evidence/reviews/AR2-verifier-build/README.md) | External-wallet compatibility and full verifier separation |
| [P03](../agentic_internet_v1_1_plan/tasks/P03.md) · Atomic spend and admission certificate without double-spend | **Partial.** Operation-bound spending and a preserved issuer spent set. [spend](../../evidence/reviews/P03-concurrent-spend/README.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | A single product acceptance together with settlement/renewal |
| [P04](../agentic_internet_v1_1_plan/tasks/P04.md) · Epoch survival, reconfiguration and eventual autonomy from L2 | **Partial.** Native multi-epoch original spent-history handover. [handover](../../evidence/reviews/AR1-native-handover/README.md) | Automatic authority renewal/closing and outage pending work |
| [P05](../agentic_internet_v1_1_plan/tasks/P05.md) · Resource pricing, reservation and shared repair budget | **Partial.** One ticket per original, a bounded sender, durable observations and atomic retirement without returning allocations. [sender](../../evidence/reviews/AR1-sender-fairness/checks.json), [lifecycle](../../evidence/reviews/AR2-wallet-flow/status-native-candidate-3.json) | A shared repair budget and long-lifecycle retained evidence |
| [P06](../agentic_internet_v1_1_plan/tasks/P06.md) · Attacks on stamps, privacy and solvency as a shared regression suite | **Partial.** Negative spend/QC/SQL/cold scenarios. [spend](../../evidence/reviews/P03-concurrent-spend/README.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | A full economic regression on a single release revision |
| [O01](../agentic_internet_v1_1_plan/tasks/O01.md) · Native Google OAuth with local JWT verification | **Not demonstrated.** Google OAuth remains mandatory. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | A real native PKCE/state/JWT flow and cancel/retry |
| [O02](../agentic_internet_v1_1_plan/tasks/O02.md) · Shared ExternalCredential and TrustProfile for external networks | **Partial.** Checkpoint trust setup; not ExternalCredential E2E. [checkpoint](../../evidence/reviews/L06-checkpoint-network.md) | Provider-independent credential/trust profile lifecycle |
| [O03](../agentic_internet_v1_1_plan/tasks/O03.md) · Restricted attestors: one organization issuer or k-of-n | **Not demonstrated.** The restricted authority issuer is kept in scope. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Organizational and k-of-n attestor flows |
| [O04](../agentic_internet_v1_1_plan/tasks/O04.md) · Provider-scoped grant: deduplication, campaigns and cross-provider limits | **Not demonstrated.** Provider-scoped grant is kept in scope. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Campaign, owner and cross-provider limit deduplication |
| [O05](../agentic_internet_v1_1_plan/tasks/O05.md) · Freshness, revocation and unlinking of external providers without address loss | **Not demonstrated.** Unlink must preserve the address and chat. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Freshness/revoke/unlink of a real credential |
| [O06](../agentic_internet_v1_1_plan/tasks/O06.md) · Free start under an explicitly accepted trust profile | **Not demonstrated.** Funded onboarding remains mandatory. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | An ordinary user receives a funded bounded grant |
| [O07](../agentic_internet_v1_1_plan/tasks/O07.md) · Organization issuer and site OAuth/OIDC gateway | **Not demonstrated.** Site OIDC and the organizational issuer remain V1. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | A real reference-site flow and issuer refusal |
| [O08](../agentic_internet_v1_1_plan/tasks/O08.md) · Telegram Login/OIDC with safe desktop handoff | **Not demonstrated.** The Telegram gateway remains V1. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | A real handoff, owner binding and the secret only on the gateway |
| [M01](../agentic_internet_v1_1_plan/tasks/M01.md) · Local MCP server with narrow permissions | **Partial.** The packaged stdio MCP and grant verification. [mcp](../../evidence/reviews/M06-packaged-mcp.md), [broker](../../evidence/reviews/I02-agent-broker.md) | Full current host conformance and supported OSes |
| [M02](../agentic_internet_v1_1_plan/tasks/M02.md) · Inbox/outbox, acknowledgement and wake-up of a running runtime | **Partial.** Durable inbox/poll/ack and restart in MCP. [inbox](../../evidence/reviews/M02-agent-inbox.md), [mcp](../../evidence/reviews/M06-packaged-mcp.md) | A shared delivery/durability read model and host lifecycle |
| [M05](../agentic_internet_v1_1_plan/tasks/M05.md) · Multiple runtimes: messaging leases, fencing and shared budget | **Partial.** Scoped runtimes and the incoming queue. [broker](../../evidence/reviews/I02-agent-broker.md), [inbox](../../evidence/reviews/M02-agent-inbox.md) | Multiple-runtime switching, leases and a shared messaging budget |
| [M06](../agentic_internet_v1_1_plan/tasks/M06.md) · Packaged CLI skill and MCP for messaging | **Partial.** The macOS bundle contains CLI/MCP/SKILL.md; the native UI emits exact commands and text. A separate Codex host performed send/retry/delivery/poll/ack. [host](../../evidence/reviews/AR2-wallet-flow/skill-host/README.md), [native](../../evidence/reviews/AR2-wallet-flow/status-skill-native-ui.json) | A new contact, host lifecycle and NAT |
| [U01](../agentic_internet_v1_1_plan/tasks/U01.md) · Tauri desktop: profile, contacts and direct chats | **Partial.** Profile, contacts, chat and 1051 messages with pagination; fresh native paid UI send/recovery. [native](../../evidence/reviews/AR2-wallet-flow/e2e-vault-native-ui.json), [GUI](../../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md) | A long paid history, other MLS epochs and platforms |
| [U02](../agentic_internet_v1_1_plan/tasks/U02.md) · Onboarding: own keys, Google, Telegram and organization trust | **Partial.** Checkpoint/registry trust UI, buy/balance/TTL and budget on empty profiles. [GUI](../../evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md) | Google/Telegram/organization onboarding |
| [U03](../agentic_internet_v1_1_plan/tasks/U03.md) · Full group UX, device management and privacy profile | **Not demonstrated.** Group UX remains mandatory. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Groups, devices, privacy and recovery through the UI |
| [U04](../agentic_internet_v1_1_plan/tasks/U04.md) · Panel of agents, permissions and limits | **Partial.** Scoped agent grants and revoke in the native panel. [grants](../../evidence/reviews/U04-native-runtime-panel.md) | A V1 messaging-only default and the full limit model |
| [U05](../agentic_internet_v1_1_plan/tasks/U05.md) · Everyday operation: attachments, search, notifications and recovery | **Partial.** A long history and network settings as components. [ui](../../evidence/reviews/U01-desktop-history/README.md) | Attachments, search, notifications, lock and backup through the UI |
| [U06](../agentic_internet_v1_1_plan/tasks/U06.md) · Installable builds, headless CLI and voluntary updates | **Partial.** The signed macOS debug bundle contains daemon/MCP/CLI/skill, verified in a hidden WKWebView. [native](../../evidence/reviews/AR2-wallet-flow/status-skill-native-ui.json) | Release installers for three OSes and voluntary updates |
| [U07](../agentic_internet_v1_1_plan/tasks/U07.md) · Protected Tauri webview → Rust → daemon boundary | **Partial.** Rust/daemon broker and a real native bridge. [native](../../evidence/reviews/D05-paid-index-store/release.json), [broker](../../evidence/reviews/I02-agent-broker.md) | Windows IPC/ACL and a shared production boundary gate |
| [X01](../agentic_internet_v1_1_plan/tasks/X01.md) · End-to-end transport chaos and company resource shutdown | **Partial.** NAT/relay loss and source-off paid retrieval separately. [nat](../../evidence/reviews/N04-linux-nat.md), [history](../../evidence/reviews/AR3-history-automatic/README.md) | A single chaos/company-off scenario with repair |
| [X02](../agentic_internet_v1_1_plan/tasks/X02.md) · End-to-end group and privacy security | **Partial.** Separate MLS and consensus evidence. [mls](../../evidence/reviews/I05-mls-adapter.md), [finalizer](../../evidence/reviews/P01-engine.md) | Full group privacy/recovery E08–E10 |
| [X03](../agentic_internet_v1_1_plan/tasks/X03.md) · End-to-end economic security and autonomy from L2 | **Partial.** Spend safety and multi-epoch lineage. [spend](../../evidence/reviews/P03-concurrent-spend/README.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | Full settlement/subsidy/outage E19–E22 |
| [X04](../agentic_internet_v1_1_plan/tasks/X04.md) · End-to-end Google/Telegram/site onboarding and limited trust | **Not demonstrated.** The requirements for independent provider flows are preserved. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Real Google/Telegram/site onboarding E17–E19 |
| [X05](../agentic_internet_v1_1_plan/tasks/X05.md) · Agent messaging and malicious content | **Partial.** A real CLI skill/host for a permitted contact; shared MCP/CLI retry/delivery/revoke. [host](../../evidence/reviews/AR2-wallet-flow/skill-host/README.md), [native](../../evidence/reviews/AR2-wallet-flow/status-skill-native-ui.json) | A new contact, host/NAT and malicious content E11/E14 |
| [X06](../agentic_internet_v1_1_plan/tasks/X06.md) · Full acceptance of 67 cards /22 E2E /three OSes | **Partial.** Scope and separate historical acceptance snapshots. [plan](../../Docs/agentic_internet_v1_execution_plan/planning-validation.json), [native](../../evidence/reviews/D05-paid-index-store/release.json) | All 67 cards, 22 E2E and 3 OSes on a shared revision |

## End-to-end scenarios

| ID / scenario | Evidence and the verified part | What remains for full acceptance |
|---|---|---|
| E01 · Clean install and identity retention | **Partial.** Identity and restart in macOS native. [native](../../evidence/reviews/D05-paid-index-store/release.json) | Clean installers/keystore for all OSes |
| E02 · Bootstrap without the company | **Partial.** Local independent hints and cache. [bootstrap](../../evidence/reviews/N02-bootstrap-exchange.md), [routing](../../evidence/reviews/N03-dht-roles/README.md) | Company-off cold join in an independent testnet |
| E03 · NAT and relay failover | **Partial.** Restrictive Linux NAT and relay failover. [nat](../../evidence/reviews/N04-linux-nat.md) | Release CLI/desktop and the whole hole-punch matrix |
| E04 · Malicious peer and backpressure | **Partial.** Hostile Noise, quotas and bound requests. [resources](../../evidence/reviews/N03-processing-reservations/README.md), [history](../../evidence/reviews/AR3-history-automatic/README.md) | A general source-bound hostile-peer acceptance |
| E05 · Offline, crash and dedup | **Partial.** The full declared range of two books: disjoint original data/index rosters, sender absent, SQL rollback and cold dedup. [history](../../evidence/reviews/AR3-history-automatic/README.md), [books](../../evidence/reviews/AR3-history-books/acceptance.json); Core: 130 originals after deferred arrivals; native: two originals after an 18 data/18 index loss on a separate setup MLS conversation. [receive admission](../../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md); The durable v1 cursor and Runtime passed 16 gaps → cold restart → original 17 → all 17; the paid two-original loss/recovery passes again. [cursor](../../evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md); The graph Core checkpoint is atomic with retirement; exact root/member/pointer, SQL/cold retry and a real MLS epoch change are verified. [checkpoint](../../evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md); The Core cursor counts 130 references in the wrapped v1 128 +2 v2 leaves and survives bounded cold passes; authenticated expiry skip, exact root/pointer, real epoch and SQL rollback are verified; attempts do not mean import or completeness. [graph cursor](../../evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md); The ordinary receiver is wired: 130 genuine MLS originals after cold bounded passes, paid typed replies, child routes, expiry skip and SQL root rollback; v1 paid native compatibility passes. [receiver](../../evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md) | Sender child ACK before root/pointer and >128 live paid native graph recovery; user-visible gap/legacy/rejoin outcomes and the first offline Welcome |
| E06 · Ten replicas and repair without clients | **Partial.** R10 originals and paid copies. [copies](../../evidence/reviews/D03-paid-custody-copies/README.md), [history](../../evidence/reviews/AR3-history-automatic/README.md) | Autonomous repair with both clients offline |
| E07 · Indexes, chunks, TTL and disk-full | **Partial.** Independent paid indexes and actual index loss. [history](../../evidence/reviews/AR3-history-automatic/README.md) | Chunks/resume/hash/TTL/disk-full in full |
| E08 · Group and member removal | **Partial.** MLS Add/Remove component. [mls](../../evidence/reviews/I05-mls-adapter.md) | UI group and old/new secret denials |
| E09 · Competing MLS commits and partition | **Partial.** MLS and BFT verified separately. [mls](../../evidence/reviews/I05-mls-adapter.md), [finalizer](../../evidence/reviews/P01-engine.md) | Concurrent encrypted group control and partition |
| E10 · Offline rejoin and device recovery | **Partial.** Message catch-up and MLS components. [history](../../evidence/reviews/AR3-history-automatic/README.md), [mls](../../evidence/reviews/I05-mls-adapter.md) | Device recovery/control-log/rejoin beyond the window |
| E11 · CLI skill and MCP: messaging, lifecycle, permissions and NAT | **Partial.** Packaged CLI/MCP/skill, a real Codex host: send/retry/delivery, the full reply, poll limit correction, ack; UI revoke of both clients. [host](../../evidence/reviews/AR2-wallet-flow/skill-host/README.md), [native](../../evidence/reviews/AR2-wallet-flow/status-skill-native-ui.json) | A new contact and the full host/NAT lifecycle |
| E14 · Prompt injection, XSS and revoke | **Partial.** Runtime grant and revoke before the action. [broker](../../evidence/reviews/I02-agent-broker.md), [grants](../../evidence/reviews/U04-native-runtime-panel.md) | A single malicious content/XSS/modified approval scenario |
| E17 · Google native live OAuth | **Not demonstrated.** The mandatory live Google flow is preserved. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Real OAuth/retry/cancel/unlink |
| E18 · Telegram gateway and site OIDC | **Not demonstrated.** Telegram/site issuers remain V1. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Real flows, hostile handoff and issuer-off |
| E19 · Funded subsidy and deduplication | **Not demonstrated.** Funded subsidy remains V1. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Fund/cutoff and deduplication across device/wallet/attestor |
| E20 · Competing spend and handover | **Partial.** Competitive spending, native handover and cold recovery. [spend](../../evidence/reviews/P03-concurrent-spend/README.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | The full refund/payout lifecycle on a shared revision |
| E21 · L2 reorg, roots and outage | **Partial.** Signed history and leases. [checkpoint](../../evidence/reviews/L06-checkpoint-network.md), [handover](../../evidence/reviews/AR1-native-handover/README.md) | An integral reorg/outage/renewal + old retrieval |
| E22 · Royalty and absence of control rights | **Not demonstrated.** Non-controlling royalty remains V1. [review](../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) | Conservation, revert and the absence of control rights |
| E23 · Everyday UX and local data | **Partial.** History, SQLCipher and restart as components. [ui](../../evidence/reviews/U01-desktop-history/README.md), [store](../../evidence/reviews/F04-profile-store.md) | Search/attachments/notifications/lock/backup/disk-full |
| E24 · Native platforms and production smoke | **Partial.** Historical macOS native/release gate. [native](../../evidence/reviews/D05-paid-index-store/release.json) | Current native installers/smoke on three OSes |
| E25 · Compatibility and economic extensions | **Partial.** Strict wire and legacy postage compatibility. [wire](../../evidence/reviews/F02-signed-document.md), [legacy](../../evidence/reviews/AR2-verifier-build/README.md) | Two compatible release clients and all V1 adapters |
| E26 · Company infrastructure shutdown | **Partial.** Source-off retrieval and separate relay failover. [nat](../../evidence/reviews/N04-linux-nat.md), [history](../../evidence/reviews/AR3-history-automatic/README.md) | Company/gateway/CDN/revenue-off independent network in full |

The full scenario program: [TEST_AND_E2E_PLAN.md](TEST_AND_E2E_PLAN.md). Machine-readable rows and source SHA256: [capability-evidence.json](capability-evidence.json).

## Completion order

1. Finish the user send: wallet/price/TTL/budget, a shared status for UI/CLI/MCP and retirement with history preserved. Every step is tied to an observable obstacle of the complete scenario.
2. Close long offline: first contact/Welcome, MLS epochs/gaps and autonomous R10 repair.
3. Bring groups/recovery, everyday UX, provider onboarding and transport settlement to completion with whole scenarios.
4. Run the independent testnet/company-off and the shared release gate on macOS arm64, Windows x86_64, Linux x86_64. The full suite — at the end of the plan.
