# Successor spending requires complete state and current authority

Full V1 remains 67 cards / 22 E2E / three platforms. This contract enables genuine
application spending in a chosen successor; actual ordinary-peer bootstrap and
client lineage must follow before AR1 is complete. Do not replace those gates
with owner-copied pages or fixture signatures.

`SpendSession::new_with_continuity(authority, core, store, continuity, now)` must
check complete owner-bound continuity for exactly this issuer/code/policy/committee
and current selected Core authority before returning a session. Historical state
does not revive expired or revoked authority. Warm use rechecks the retained
completion state. Cold `SpentContinuity::open` retains its full historical scan.

The legacy `new` guard remains for epochs other than 1. In a successor session,
legacy proposal/verification APIs without store access must fail closed. New
`propose_indexed_with_state` / `verify_indexed_with_state` validate the current
receipt/fence, continuity and issuer-global old spent key before consulting the
current-epoch prefix and bounded consensus suffix. Absence in an empty successor
log is insufficient. A malformed retained record cannot become absence.

Closing decisions have corresponding `_with_state` variants. Both ordinary
finalization and closing finalization already receive the store and must enforce
continuity. A valid new-committee QC for a previously spent ticket cannot create
a new effect or overwrite the original record, even with the same journal.
Same-committee exact retries retain original QC/time. Current history closing
rules, selected role, checkpoint/roster revocation and monotonic clock remain.

`lookup_continuous(core, store, nullifier, now)` authenticates retained evidence
under the original committee. A bounded unverified finality-target parser may be
used only as a lookup hint; the QC and exact chosen predecessor membership must
then be checked. Do not accept merely any registry committee/ClosingRecord found
in the store. Predecessor membership can use the already cold-validated immutable
progress chain with bounded memory; no lifetime epoch array is introduced. New
proposal negative membership remains independent of predecessor history length.

Tests use the genuine 160-ticket three-epoch EVM fixture and reusable archive
helpers. Run actual receipt/proposal/finalization in epochs 1, 2 and 3, not a
manually constructed epoch-2 positive carrier. Prove refusal of old tickets from
both predecessors, exact historical records after cold restart, current-authority
and completion-state revocation, wrong owner/target, legacy API refusal, and
genuine duplicate later QC without mutation. An unrelated but valid historical
committee carrier must not be accepted as chosen history. Existing import tests
must retain their behavior after helper extraction.

Write tests and obtain an independent backend-test-critic ACCEPT before any
production changes. Node/network integration needs its own meaningful tests and
critic before changes, followed by actual funded multi-node consensus and the
required backend/frontend regression checks.
