# Pending paid request across a successor checkpoint

Status: focused native GREEN R3 passes both pending boundaries, two real peer
successors, exact original tickets/QCs and cold sender-absent data/index recovery.
All833 inputs stayed unchanged; six signatures verified, clean teardown. This
closes the focused renewal gate. Full130 release R6 failed on an over-budget snapshot-comparison oracle after128 stored originals (834 unchanged inputs, clean teardown);
the corrected oracle passes focused release BO-R3 on837 unchanged inputs. Full130 release R7 terminated after130 stored originals and390 verified signatures on838 unchanged inputs. The first SQL loss helper refused before any loss/recovery; its obsolete keep-two limit is now reproduced and removed with accepted actual keep11 data/index regression, exact survivors and cold reads (1 backend/14 frontend, Clippy/fmt). Cleanup was clean; the full67/22/3 product goal remains open.

## Current acceptance

[Native GREEN R3](pending-renewal-native-green-r3.json) proves both before-finality
and refused-result-commit continuation. Each original reaches ten data receipts,
ten index receipts and100 location ACKs, keeps its original ticket and frees its
pending slot across cold restart. The inherited ordinary CLI flow then passes
real copy loss, cold index/recipient recovery, refused import commits and exact
retry with the sender absent. It verifies six genuine QC signatures, not a
130-original range or autonomous repair.

Local pending-client checks pass31 backend/14 frontend. The storage-admission
boundary passes54 backend/14 frontend; the final shared compact-location follow-up
passes37 backend/14 frontend, postage/node all-target Clippy and fmt. These are
separate overlapping targeted groups, not one additive full-suite count. The
failed native R1/R2 runs and their independently reviewed regressions remain below.

## Observed ordinary failure

[Full130 release R5](history-range-native-r5.json) reached64 stored originals in
four batches. The third real successor was issued four seconds after the fourth
batch completed, while the fifth batch was entering the postage client. The
original fifth-batch deadline then expired with eight jobs finalizing and eight
blocked by `postage_limit`. Alice had client revision4, lineageReady=true,
eight pending requests, zero rejected results/failed result commits and three
accepted peer authority updates. All826 source inputs stayed unchanged; exit1,
empty cleanup errors and removal of temporary profiles are recorded. Full390
signature verification, copy loss/recovery and130 cold cursor claims were not
reached. No quota or deadline was changed.

The focused [renewal R2](WALLET_RENEWAL.md) passes but changes authority only
between completed originals. Its acceptance does not establish continuation of
an already admitted, unfinished postage request.

## Focused reproduction

[Native RED R2](pending-renewal-native-red-r2.json) now reaches the intended boundary: all four actual finalizers report absent under candidate INSERT/UPDATE SQL faults; the client obtains one genuine successor through peers, refreshes its ordinary wallet, restarts, then fails to resume the same pending original within the existing180 seconds after faults are removed. All827 inputs remained unchanged, teardown is clean. The later after-QC phase remains unreached. R1 exposed a fixture issue: an UPDATE-only fault did not block first candidate INSERT; the corrected test/helper received independent FINAL ACCEPT.

## Code evidence and hypothesis

`postage_client.rs::configure` resets `Pending.verified`. Its `reconcile` method
then calls `prepare_client` on the original saved `Request.candidate` using the
new authority. `postage_spend.rs::Submit::context` authenticates that candidate's
saved checkpoint/proof. Current Core context requires the actual current head.
Unverified pending rows remain in the eight-slot active list until expiry.

The ordinary sender now constructs fresh wallet-backed context, but its client
path reads an existing pending request by ID and does not replace that request's
stale transport context. This is the leading explanation for R5. A focused
reproduction must distinguish it from lack of peers, failed proof acquisition,
finalizer faults or result-commit failure before a production fix is accepted.

## Implementation and local regression

The ordinary sender now passes its fresh wallet-backed transport candidate to internal `Client::refresh_pending` for an existing pending request. The client reuses current Core receipt/context checks, requires the original receipt plus journal/nullifier/operation, keeps the original external request hash, and commits candidate/snapshot/finite expiry together with the unchanged active-ID list through the existing atomic store helper. Only a successful commit releases a refreshed transport permit. Exact repeats are write-free; finalized/conflict/expired results remain read-only. On a refused refresh the sender still exposes the original durable pending payment in diagnostics.

The historical verifier already retains the immutable committee registry window below the current checkpoint lower bound. A freshly authenticated snapshot therefore verifies an earlier chosen QC and also supports a later QC beyond the prior checkpoint expiry, without granting admission beyond the new finite authority. Both cases are exercised with the existing real paid fixture and independent P256 QC oracle. No unverified historical snapshot or extended ticket interval is introduced.

Three independently accepted API tests cover eight pending slots and their release, exact external retry and immutable completed results, same-ticket/different-operation and other-ticket rejection, SQL failures on request and active-index rows with genuine cold reopen, and the exact authority/ticket expiry cutoff. [Local checks](pending-renewal-checks.json) pass: client4 (three new plus prior permit maintenance), sender scheduling5, wallet RPC5 and shared retention17, totaling31 backend; frontend14, node all-target Clippy and fmt. The focused native GREEN R1 ended with829 unchanged inputs: the before-finality phase passed; after a refused result commit the cold client finalized and released its slot (pending0), but the sender did not reach stored. The downstream historical custody/admission boundary requires a separate regression. See `pending-renewal-native-green-r1.json`. The full130 gate remains pending after that result.

## Required regression and fix

### Earlier finality and successor storage admission

The second phase of [native GREEN R1](pending-renewal-native-green-r1.json)
completed its cold client result (pending0, sent1, no rejected results), but did
not reach stored. A separate accepted regression now reproduces the downstream
fault: real data and index puts under a genuine successor accept the unchanged
earlier QC, then independent cold verification rejects both. No retained object
or payment was replaced to make the regression.

`renewed_admission.rs` reuses real TTL900 paid cases and a signed successor.
Its two tests cover exact retry, cold read, alternate genuine QC and certificate
substitution, signed pre-head receipts, late flat-origin copy and exact object
expiry. Fresh data/index input is accepted at head cutoff-1 and rejected at the
cutoff while its operator binding is still live. The first fixture review caught
unsuitable TTL10 cases and an expired-binding false positive; both were corrected
before [FINAL ACCEPT](renewed-admission-critic.json) and production edits.

The cold verifiers now separate QC finalization time from original primary
storage admission. Payment is checked at the signed original receipt time; the
QC remains independently authenticated and must precede that receipt. Related
primary receipts each obey the finite paid interval. A later copy retains the
origin's admission time. Both new tests pass after the observed RED; 54 backend /14 frontend, postage/node all-target Clippy and fmt also pass.
Native GREEN R2 reached ten data and ten index replicas in the after-QC phase,
but location publication was rejected (indexLocations0). All832 inputs stayed
unchanged and teardown was clean. The remaining compact location verifier still
used the earlier QC time. The extended test received FINAL ACCEPT, reproduced
that exact cold location refusal, and now passes with a shared original-admission
time helper reused by full and compact custody verification. It additionally
publishes original/copy locations, reads them cold and preserves the original
paid index. See `pending-renewal-native-green-r2.json`. Follow-up checks pass37 backend/14 frontend and Clippy/fmt; native GREEN R3
passes the complete focused gate described above.

Use the real ordinary wallet/CLI setup and existing real-chain Source helper.
Force a successor while a genuine original has an actual pending postage-client
row. Prove that it remains the same original, ticket, nullifier and operation;
Alice obtains the head through peer sync and refreshes through the ordinary
wallet command. It must resume after cold restart without another send/work RPC,
another ticket or relaxed admission/retention limits.

Cover both a request still awaiting a chosen QC and a chosen QC whose local
result commit failed. Existing test-only candidate-write and client-write SQL
faults can hold those boundaries. Install faults before the relevant event and
observe the actual pending row/transmission or verified failed commit. Remove the
fault as the positive control. A status string alone is insufficient evidence.

Any internal refresh API must reauthenticate fresh current public evidence and
match the original receipt/journal/nullifier/operation before committing a change.
Preserve external request-ID retry semantics and historical QC verification.
In particular, a later authority snapshot cannot simply replace the only evidence
needed to authenticate an earlier chosen result. Wrong context or SQL failure
must leave pending state and capacity accounting intact; exact refresh must be
idempotent. Completed results remain immutable and must not be re-enrolled.

Write these backend tests first and obtain independent backend-test-critic
acceptance before production edits. Keep the existing eight pending requests,
R10 limits, message/purchase TTL3600 and original test deadlines. Then run targeted
backend/frontend checks, both focused native renewal boundaries, the standalone
graph sender regression and the unchanged full130 release gate. User-visible
recovery range/gap/legacy/rejoin work follows; the full67/22/3 goal remains open.
