# Shared status and successful retirement — integrated

The common `Delivery.postage` projection is now read by CLI, MCP, desktop snapshot,
overview and paged history. Recipient ACK remains independent. The sender ledger
stores a small trusted-host observation and a separate bounded failure, preserving
confirmed counts and observation time across failures and restart. Signed expiry
remains visible without a live grant or wallet refresh. Incoming/non-sponsored
messages retain the old wire shape. The UI shows partial storage, explicit expiry
and historical observations, and refreshes loaded messages at expiry.

Successful R10 data, index/history and exact pointer publication completes the job
and removes its active ID in one transaction. No completion deletes originals,
outbox, allocations, cumulative sponsorship or signed history. Later publication
uses the retained original inventory. Completed diagnostic proof views are rebuilt
from canonical evidence; no heavy `Work.view` copy is persisted and completed work
is not re-enrolled. Full diagnostic proofs still require existing live authority
checks; the common product projection does not.

## Evidence

- Six new Core tests cover real ACK plus partial storage, idempotent observations
  and failures, all three SQL fault points, exact operation/history/pointer bounds,
  cold completion, scope/revoke, actual Unicode pagination and reopened active
  admission. The initial RED was missing APIs/types. Thirty-six sender cases pass.
- Twenty-six chat-shell tests and frontend build pass, including storage updates
  for already delivered old messages and expiry without a database event. Headless
  screenshots were visually inspected at 850×650 and 1280×720.
- [Native candidate 3](status-native-candidate-3.json) passes the complete paid
  CLI scenario with actual verified evidence, all three SQL failures and restarts,
  exact CLI/MCP/desktop equality, cold completed rows, second publication and both
  original messages recovered with sender absent after real data/index loss.
  Six independent QC signatures; 697 unchanged inputs; clean process teardown.
- [Final targeted regressions](status-checks.json): 60 backend and 35 frontend
  tests, Core/node all-target Clippy and formatting pass. These counts overlap the
  earlier suites and must not be added to them as independent coverage.
- [Sanitized observations](status-native-lifecycle-observations.json) preserve
  the public projections and SQL row digests. Full local trace is retained under
  `output/status-c3/trace.json`; it is not a public artifact.
- Candidates 1 and 2 remain recorded as failures: macOS socket path length, then
  an inherited wallet expectation after newly added restarts. The independently
  accepted correction refreshes the wallet through its normal RPC command and
  proves that doing so changes no completed projection or lifecycle row.

## Continue the whole capability

[130-original native gate](HISTORY_RANGE.md) is written; its earlier fixture
acceptance is superseded by [FINAL REVISE](history-range-fixture-review.md).
Debug R2 failed its first 16-original publication deadline on 823
unchanged inputs. Release R3 also failed that deadline on 823 unchanged inputs.
[Discovery optimization](ROUTING_HINTS.md) is implemented; fresh-proof and live-role
withdrawal cases and the full native G4 gate passed on both chains (416 positions).
R4 completed 32 originals, then was stopped on 823 unchanged inputs because
snapshot1800 cannot admit the last two originals under existing quotas. The
corrected initial snapshots and real successor-head fixture passed independent
test review. [The focused renewal gate](WALLET_RENEWAL.md) now passes on826
unchanged inputs after an ordinary wallet-to-sender proof refresh bridge, with
54 backend /31 frontend and Clippy/fmt passing. Full130 [release R5 failed](history-range-native-r5.json) at its original fifth-batch
deadline after64 stored originals and three peer successors, with826 unchanged
inputs and clean teardown. [Pending-request renewal](PENDING_POSTAGE_RENEWAL.md) now passes focused native GREEN R3 on833 unchanged inputs, including both actual successor/cold boundaries and sender-absent recovery. Full130 release R6 terminated after128 stored originals and130 accepted CLI sends: the correct budget refusal passed, but full mutable Message snapshot equality failed. All834 inputs stayed unchanged and teardown was clean. The independently accepted native oracle correction now passes full focused release BO-R3 on837 unchanged inputs, including exact budget refusal and cold sender-absent loss/recovery. Full130 release R7 terminated after130 stored originals and390 verified signatures on838 unchanged inputs. The first SQL loss helper refused before any loss/recovery; its obsolete keep-two limit is now reproduced and removed with accepted actual keep11 data/index regression, exact survivors and cold reads (1 backend/14 frontend, Clippy/fmt). Cleanup was clean; the full range remains open.
The separate routing graph regression passed on its earlier823 inputs. Keep the range open until full recovery,
live retention and all 130 cold cursor claims pass. No quota/deadline increase.

[Ordinary graph sender](GRAPH_SENDER.md) is now integrated and verified for two
paid CLI originals: five signed pages /50 child-before-parent ACKs, last-child
SQL/cold retry and exact sender-absent recovery after data/index loss. 30 backend
/31 frontend, Clippy/fmt and native C1 pass on 822 unchanged inputs. Next extend
the native range to >128 simultaneously live paid originals with the accepted in-flight renewal fix. The standalone sender graph migration now [passes native R1](standalone-sender-graph-native-r1.json), including all four SQL faults, cold retirement and remote graph reads on 826 unchanged inputs. Keep all existing quotas and full 67/22/3.
This is the current next-work order; component notes below are historical.

[Ordinary graph receiver](GRAPH_RECEIVER.md) now integrates paid typed responses,
root consistency, signed child routes and durable attempts. 130 genuine MLS
originals recover across cold Runtime passes. 30 backend /31 frontend, Clippy/fmt
and the ordinary two-paid-original native compatibility gate pass. Next implement
sender child ACK ordering and verify >128 live paid native graph recovery; this
supersedes the earlier component next-work notes below.


Fresh native paid GUI onboarding now passes: [GUI acceptance](GUI_ACCEPTANCE.md).
It joins the exact displayed purchase, external local-chain payment, cold wallet,
budget and two UI sends with actual R10/loss/recovery. Automated native profiles
use the isolated E2E vault and never access system Keychain. The shipped messaging skill
and its actual independent host flow now pass for an existing granted local
contact: [host evidence](skill-host/README.md). The current packaged
[native GUI gate](status-skill-native-ui.json) exposes the exact skill, executes
the displayed CLI command and checks shared MCP/CLI delivery and revocation.
Complete E11 still requires new-contact onboarding and isolated host/NAT evidence.
An existing granted contact and a localhost exchange do not establish those claims.

Prepared envelopes now have immutable sequence rows and bounded local pages;
the 258-original/legacy/SQL checks pass: [storage evidence](ENVELOPE_PAGES.md).
The [bounded Store range prerequisite](STATE_RANGE.md) now passes its 257-row
and cold-continuation checks. The [outgoing component](OUTGOING_ROWS.md) now retains
134 paid originals in separate rows, with indexed conflicts, occupied quota,
bounded expiry and atomic legacy migration. Its 89 backend /31 frontend checks
pass. [Runtime integration](RUNTIME_RETENTION.md) now sets the separate outgoing
quota and drives one bounded GC attempt per second, with SQL rollback/backoff and
actual pump wiring checked in the signed fixture network (51 backend /31 frontend).
[Incoming ciphertext](INCOMING_ROWS.md) now shares the row engine and passes
the 134-original, bounded scan/expiry and atomic legacy migration gate.
[Incoming index](INDEX_ROWS.md) also uses the shared engine for compact paid
anchors, locations/history, quotas, bounded pages and atomic legacy migration.
[Operator Runtime](OPERATOR_RETENTION.md) now configures independent data/index/
outgoing quotas and schedules one bounded batch per store with failure isolation.
[Inspection observations](OBSERVATION_ROWS.md) now have separate operation bodies,
byte quotas and a fourth independent Runtime expiry queue. All 98 affected
backend /31 frontend checks, Clippy and formatting pass on 59 focused inputs.
The [signed-page wire prerequisite](HISTORY_PAGES.md) now passes independent
257-page vectors, immutable append, cold consistency and expired-prefix checks:
28 targeted crypto /31 frontend tests. The ordinary sender still uses the
128-reference flat directory. [Core sender page persistence](CORE_HISTORY_PAGES.md)
now passes its nine new scenarios: bounded immutable batches, exact root/member
transactions, v1 migration, cold proofs and real epoch fences.
[Paid operator pages](PAID_HISTORY_PAGES.md) now charge all retained versions to
one finite anchor allowance plus exact global quota; 106 backend /31 frontend,
Clippy/fmt and 124 focused source hashes pass. This is a body ACK, not a graph
publication ACK. [Typed network ingress](NETWORK_HISTORY_PAGES.md) now passes
six real TCP/Noise fixture scenarios and nine transport regressions, with exact
paid responses and explicit capacity; 31 frontend tests and Clippy/fmt pass.
[The sender page ledger](OUTGOING_HISTORY_PAGES.md) now preserves exact bodies
and independent ACK sets with paid-peer binding, quota, cold trust and SQL fences;
116 backend /31 frontend checks and Clippy/fmt pass on 137 focused inputs.
[Matched network replies](HISTORY_PAGE_SENDER.md) now reach this ledger through
ordinary request/peer/connection authorization. One bounded client handles existing
leaf and typed page requests; the ordinary publisher uses exact page ACKs already.
All 27 targeted backend /31 frontend, node all-target Clippy and fmt pass. The
ordinary paid CLI lifecycle passes its exact-ledger/cold-restart guard and actual
18-data/18-index-copy loss/recovery. The four-stage sender SQL fault/restart gate
also passes; both runs use 790 unchanged captured inputs. The flat v1 directory
remains in use.
[Core receive admission](RECEIVE_ADMISSION.md) now applies from initial contact
creation/Welcome and guards the shared direct/custody reducer. The original
130-live-original test keeps arrivals 1..129,0 and passes after deferred imports
and fresh retries, with cold state and constant row writes. Profile v2 makes
legacy/protected policy explicit; old unguarded contacts require recovery before
v2 root or retained-token use. All 141 targeted backend /31 frontend, all-target
Clippy and fmt pass. Each run holds 798 captured inputs unchanged.

The first ordinary native replay remains failed: an intentionally unpublished
setup original in the same MLS chain blocks later paid messages. A separately
reviewed positive fixture isolates that unrelated payment sentinel in another
real MLS conversation. That native scenario passes payment, two CLI sends,
budget/retry, SQL faults, page ACKs, retirement and recovery after actual
18-data/18-index-copy loss, with six verified QC signatures. It does not prove
recovery across the original gap or renew the GUI gate.

[Durable fair retry](HISTORY_SCAN.md) now separates ReceiveGap from generic
rejection, releases the whole old fetch attempt, and persists selection across
bounded work and cold restart. The real Runtime fixture reaches original 17 after
16 gaps and recovers all originals on retry. 72 targeted backend /31 frontend,
Clippy/fmt and the ordinary two-original paid loss/recovery regression pass on
802 stable inputs per candidate. The 130-original Core gate remains green. This
is flat v1 scheduling, not a complete recoverable-range or paid graph claim.

[Core graph checkpoint and shared publication queue](PUBLISHED_GRAPH.md) now pass
current-root/member/pointer atomic retirement, SQL/cold retry and real epoch
separation. Both index and history preparation yield cooperatively. 117 backend /
31 frontend, Clippy/fmt and normal paid native C8 pass. Earlier IPC failures and
diagnostic C5/C7 remain recorded; a single callback has no universal time bound.
Ordinary v2 child ACK ordering and graph traversal remain open.

[Core graph attempts](GRAPH_SCAN.md) now persist reference ordinals across a
128-original wrapped legacy leaf, bounded passes and cold restart. Signed expired
subtrees skip atomically; exact pointer/root, real epoch, SQL and wrong-parent
fences pass. Seven new cases plus existing custody/sender regressions yield 105
backend /31 frontend, Clippy/fmt on 811 unchanged inputs. Native was not rerun;
ordinary receiver traversal and sender child ACK ordering still need integration.

Next connect bounded graph publication/traversal and expose a durable recoverable
range plus explicit product gap/legacy/rejoin outcomes. Keep current-root fences, atomic original/progress writes,
child ACKs before root/pointer publication, pending work after restart and existing
queue quotas. Then run the ordinary network gate above 128 simultaneously live
paid originals. The [complete lifecycle](../../../spec/custody-retained-lifecycle-v2.md),
first offline Welcome, real control/epoch transitions, independent repair and the
full 67-card/22-E2E/three-platform acceptance remain open. Historical failed
[Core import](HISTORY_PAGE_IMPORT.md) and [crypto prerequisite](MLS_CONTIGUOUS.md)
reports retain their original source and outcomes.
