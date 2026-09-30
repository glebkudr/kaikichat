# Native live paid history range — in progress

The new `tests/evm/public_history_range.py` gate purchases 131 tickets through
the ordinary wallet flow: one separate setup message and 130 real scoped CLI
originals. Sends use batches of 16 within existing active-job limits. The final
signed graph must declare every original, with independently checked signatures,
commitments, routes, receipt positions and real child-before-parent ACK order.

After the sender stops, actual SQL loss removes nine of each original's ten
ciphertext and index copies. Automatic cold retrieval must recover all 130 exact
IDs/authors/text through the remaining providers. Actual first/last import commit
faults verify atomic MLS and per-operation/sequence progress. A final cold restart
must traverse every one of the 130 live ordinals, observed through bounded SQL
UPDATE audit, without reimporting or changing committed progress. The earliest
original must still be live at the final assertion. TTL3600, R10 and storage/page
allowances are unchanged. The original fixed authority/snapshot1800 fixture has
since been rejected as infeasible; see below.

[Independent critic](history-range-critic.md) initially accepted the corrected
test and explicit release-build option. The later [feasibility review](history-range-fixture-review.md)
supersedes that fixture acceptance with FINAL REVISE. This is not native acceptance.

## Current run

[Release R6 terminated](history-range-native-r6.json) after128 stored originals,
130 accepted/exactly retried CLI sends and five real peer successors. The expected
`postage_budget_exceeded` refusal passed; the subsequent full Message snapshot
equality failed. That comparison includes asynchronous publication/delivery
observations. The run did not retain those before/after local values, so a specific
changed delivery field is not directly established. All834 inputs stayed unchanged,
cleanup succeeded and no temporary profiles remain. Full signatures and loss/recovery
were not reached; R6 is not accepted.

The [independently accepted oracle correction](budget-refusal-oracle-critic.json)
compares every immutable Message field, the exact sponsor reservation map and
absence of a durable operation for the rejected send. Only the existing read-only
policy getter is added to allowed sender observations. Focused native verification
precedes a full rerun. TTL3600, R10, original batch deadlines,390 signatures and
the full130 recovery oracle remain unchanged. [Pending renewal GREEN R3](pending-renewal-native-green-r3.json)
still independently proves both actual successor boundaries and cold loss/recovery.

The corrected oracle passes two targeted real Core budget/rollback cases and14
frontend cases. [Focused BO-R1](budget-oracle-native-r1.json) passed the budget
checks, both publications, six signatures and real18+18 copy loss, but its old
CLI fixture placed an unpublished setup predecessor in the same MLS chain. The
receiver correctly deferred; the first SQL import fault was not reached. All835
inputs stayed unchanged and teardown was clean. The critic accepted reuse of the
existing separate-sentinel real conversation (already explicit in the full130
fixture). [Focused debug BO-R2](budget-oracle-native-r2.json) passed the budget
checks but reached the original180-second publication deadline with both jobs
still publishing after full10/10/100 storage evidence. All836 inputs stayed
unchanged; cleanup was clean. The critic accepted the supported release-profile
invocation for the same scenario, matching the full130 gate profile.
[Release BO-R3 passes](budget-oracle-native-r3.json): exact budget refusal and
absence of its operation, both publications, six signatures, actual18+18 copy loss,
both SQL import failures and cold recovery without the sender. All837 inputs
stayed unchanged, cleanup was clean; the original180-second deadline and every
assertion remain. This does not establish a debug performance fix.

[Full130 release R7 terminated](history-range-native-r7.json) on838 unchanged inputs
with all130 originals stored,390 independently verified signatures,1300 data and
1300 index receipts and13000 location ACKs. The first stopped-profile data-loss
helper exited1 before recording any physical loss. Its old keep-two validation
cannot accept the planned ten/eleven survivors per data holder; the [actual helper regression](custody-loss-fixture-checks.json) now reproduces
that validation failure before the fix and passes after removing only the artificial
count cap. Both data/index retain11 preserve every survivor and unrelated row;
cold reads return exact original evidence. One backend and14 frontend tests,
Clippy and formatting pass. The input4096 bound and all validity/lock checks remain.
The native traceback did not preserve helper stderr. Runner exit code was lost
with the expired PTY handle, so the report records null rather than inventing it.
All processes stopped, temporary profiles were removed and cleanup had no errors.

The original130/390-signature/loss/cold-cycle/lifetime oracle, TTL3600 and all quotas
are unchanged. Recovery and the cold full cycle were not reached. A new native full130 pass is still required. Frozen-input
hashes were checked before resuming source edits. The subsequent [Core recovery observation](history-recovery-core-checks.json)
passes targeted checks; adapter tests remain staged and the worker/UI are not integrated.

## Runs

- R1: fixture failure before funding; authority3600 exceeded the generator's
  existing maximum1800. Profile agreement and remaining oracle gaps were fixed
  before the accepted gate. No production changes were made.
- [R2, debug](history-range-native-r2.json): first 16 originals did not finish
  publication within 600 seconds. All 823 source inputs stayed unchanged and
  teardown succeeded. The exact range/signature/recovery assertions were not
  reached. Read-only CLI diagnostics and a sampled stack showed repeated sender
  validation, queue reads and custody resolution; they do not isolate one cause.
  The three targeted backend groups also ran during part of R2, so it is not an
  isolated performance benchmark.
- [R3, release](history-range-native-r3.json): the identical gate also failed
  its first 16-original publication deadline. Build profile/binary hashes are
  explicit; all 823 inputs stayed unchanged, teardown succeeded. No other test
  or build ran during its native phase. Only one original completed by the
  deadline; the whole range and recovery remain unverified. Release status reads
  were much faster, but repeating holder discovery still delayed publication.
- [R4, release](history-range-native-r4.json), with the discovery optimization,
  completed 32 originals in two batches within their original deadlines, beyond
  R2/R3. It was stopped with SIGINT after the critic proved the fixed snapshot1800
  fixture cannot admit the last two originals. All 823 inputs stayed unchanged,
  cleanup succeeded. Full signatures/range/recovery assertions were not reached.

The [discovery optimization](ROUTING_HINTS.md) now uses the existing missing-position
selector and previously verified addresses as routing hints. The critic accepted
focused fresh-proof/withdrawal/fallback tests before production changes. Actual
responses still require current proof checks; quotas, deadlines and paid selection
stay fixed. The full focused native G4 gate passed on both chains. The separate
two-original fault/retry regression also passed on 823 unchanged inputs. The
snapshot3600 and actual successor-head correction is now independently accepted
as test code. The [focused two-original renewal gate](WALLET_RENEWAL.md) R1 failed
after genuine peer acquisition: the second sender job still uses its old policy
authority. The ordinary wallet-to-sender refresh bridge now passes54 backend /
31 frontend and Clippy/fmt. Native renewal R2 passes on826 unchanged inputs:
real peer successor, exact original evidence/tickets/page ACK rows after cold
restart, and sender-absent copy-loss recovery. [Full130 release R5 failed](history-range-native-r5.json) at the original
fifth-batch deadline after64 stored originals and three genuine peer successors.
All826 inputs stayed unchanged, cleanup passed and temporary profiles were removed.
At failure the ordinary client had lineageReady=true and revision4, but all eight
slots remained pending; the other eight jobs reported postage_limit. No full-range
signature/copy-loss/cold-cycle oracle was reached. Next is a focused
[in-flight request renewal regression](PENDING_POSTAGE_RENEWAL.md). The full range remains unproved.

31 targeted backend and 31 frontend checks, Clippy/fmt and the separate graph
fault/retry regression passed after the discovery optimization. The older
standalone flat-only sender oracle has an independently
[accepted test migration](standalone-sender-graph-critic.json), applied after R5 terminated and its826 inputs were verified unchanged. It preserves all four SQL fault stages,
actual cold retirement, typed remote page reads and child-before-parent ACK
checks. [Native R1](standalone-sender-graph-native-r1.json) now passes with six independently verified QC signatures, 826 unchanged inputs, all four SQL faults, exact cold page ACKs, ten remote page reads and clean teardown.

The >128 native requirement remains open. No full V1, product recovery, independent
repair, external-host/NAT or cross-platform acceptance is claimed. Source notes
and sanitized evidence are retained; raw trace/profile secrets stay in local
ignored output only.
