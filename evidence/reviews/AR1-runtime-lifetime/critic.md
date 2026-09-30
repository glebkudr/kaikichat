# Independent backend test review

Reviewer: `/root/runtime_test_critic`, separate agent, no inherited context.
Production baseline: `9dece5a`. No production edits until final ACCEPT.

## R1 — REVISE

1. The isolated replay stage demanded entries129..136 on node0 while observing
   them only on other peers. A fresh effect failure establishes one delivery,
   not that every later entry is in that node’s durable archive.
2. A fault before spend insertion alone did not prove durability after that
   insertion and before derived history/candidate cleanup/consumer ACK.

## R2 — ACCEPT

Only delivery129 is outstanding during the initial SQL failure. A fresh counter
increment and consumer cursor128 identify the failed real delivery. Solo replay
requires129, without assuming unobserved peer progress.

A separate real trigger rejects shrink of the candidate array at delivery130.
Assertions require readable original spend, retained candidate, unchanged cursor,
then isolated cold replay with exact QC, completed cleanup and cursor130. The
reviewer independently exercised the trigger SQL: admission succeeds, shrink
fails atomically, removing the trigger permits cleanup.

Stale/foreign-profile check_operation tests cover both operation and anchor-only
queries. The role-disabled historical read preserves pending cleanup and does
not run the consumer. No additional blocking scenarios were required. A separate
history-write fault remains an optional extension. Acceptance concerns the test
contract; the native gate must still pass after implementation.

## Compatibility assertion — ACCEPT

After successful lifetime execution, the existing 100-client gate's post-finality
assertion was updated from two retained terminal candidates to zero active
candidates. Both pending-two assertions before quorum remain. The independent
critic accepted this narrow semantic update. Its full legacy gate was not rerun;
the native lifetime gate independently tests both competing candidates and terminal
retirement. No production changed for this compatibility update.

## Cold pending read and retained verifier inputs — ACCEPT

A second independent pass reviewed the discovered historical-reader fallback:
unresolved inputs plus unavailable current authority must not return absent.
The focused native test first had a missing preflight harness error (not RED),
then genuinely reproduced the incorrect absent result. After critic ACCEPT,
production now preserves the authority error while unresolved inputs remain.
The focused native check passes and retains the pending input across restart.

The full trace now includes the actual public committee, selection and issuer
evidence. Its offline helper rechecks committee derivation, all retained QC
signatures and chain continuity. It does not newly authenticate checkpoint or
EVM state proofs; those inputs are retained for audit. A fresh full run is required
and the initial successful run is kept separately in initial-run/.

## Isolate the tested daemon executable — ACCEPT

The final workspace and shared native harness can rebuild the same target path.
The unpinned native run passed on unchanged source, but a later executable hash
differed, so one binary across every restart was not established. Retain this
run under unpinned-run/. An opt-in test harness now copies the daemon into managed
temporary storage, verifies its hash before and after the scenario, and passes
that path to every selected daemon/restart. The independent critic accepted this
change. Other exercises retain their default behavior. No production changed.
The pinned native run supersedes the unpinned run for final acceptance.
