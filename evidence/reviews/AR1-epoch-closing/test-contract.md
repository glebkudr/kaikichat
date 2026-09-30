# Actual postage epoch closing — test-first contract (work in progress)

Full V1, AR1 and successor spending remain open. This work connects the durable
terminal boundary to actual issuer policy and the ordinary node/network paths.
Do not relax epoch !=1 spending until complete predecessor spent-state transfer
and successor bootstrap are separately established.

## Current authority and canonical control entry

Core::prepare_postage_successor(current: &PostageClientAuthority, roster, now)
returns an opaque PostageSuccessorAuthority. It preserves the published old
roster and its fence, authenticates the successor with installed trust at the
same current checkpoint, and requires a strictly increasing registry epoch.
Expose predecessor_id, source_snapshot, and authority (the new public committee,
snapshot and a fence bound to the old published roster). It cannot supply a
signer or authorize empty successor spending. Role-free clients may verify the
public successor; actual closing proposals require a live selected SpendSession.

The unique old-committee operation is SHA256(CBOR[
"ain-postage-epoch-terminal-v1", issuerDomain, oldScopedCommitteeId]).
The canonical payload is CBOR["ain-postage-epoch-close-v1", issuerDomain,
issuerCodeHash, spendPolicyId, oldScopedCommitteeId, successorScopedCommitteeId,
successorRegistryEpoch]. Different proven successors compete for the same old
operation, not separate journals. Check both session and successor fences before
proposing/validating/finalizing. Bound all evidence using existing limits.

SpendSession gains terminal_operation, prepare_closing,
propose_closing_indexed/verify_closing_indexed, finalize_closing. Indexed spend
and closing decisions require a CheckedPrefix bound to this terminal policy;
legacy full-prefix spend validation rejects any terminal ancestor. The node binds
history before starting its signer. Preserve the current revocation/generation
shutdown path when replacing configuration.

ClosingRecord retains exact original QC, canonical payload, verification time,
and both original public authority snapshots. Its own QC must finalize the
terminal target directly; descendants are not closing certificates. Historical
verification re-authenticates both snapshots with installed Core trust, requires
their checkpoint IDs to match, verifies issuer/policy/committee/epoch/body/time,
and never revives current authority. Cold lookup/recovery preserves the first
record, refuses a different chosen successor and is atomic through SQL failure.
Spend records preceding closure remain readable; new child spend effects fail.

## Fixture and tests

New tests/evm/postage_epoch_fixture.py reuses real contract/payment/registry
helpers. It captures three genuinely overlapping epochs and their independent
selection references at one actual signed checkpoint, plus a later genuine
checkpoint renewal. Receipts are public signed stamps over real paid funding.
No ZK process and no fabricated EVM state are used. Existing private-key material
is explicitly public fixture material. Generic signature fixture helpers may
certify test entries; such tests do not claim live consensus.

Backend tests must cover role-free successor authentication without replacing the
old roster, wrong/stale/foreign/incomplete/same-or-older evidence, clock-write
failure, session revocation, canonical competing closing payloads, unbound-index
refusal, exact first-QC preservation, direct-versus-descendant closing evidence,
SQL/restart/historical recovery, immutable old spends and refusal of new children.
Current successor authorization alone must still yield HandoverRequired for a
later-epoch SpendSession.

The revised test suite additionally isolates an obsolete successor candidate from
a fresh session, rechecks all prepared-candidate decision/effect paths after role
revocation, rejects a same-committee prefix bound to a different terminal, and
independently damages source/successor evidence without changing checkpoint IDs.
Historical/recovery paths reject a genuine but payload-mismatched successor,
descendant-certified closure, and out-of-window original time. After cold restart,
recovery preserves the first QC despite a different valid QC and refuses an
independently certified competing closing. These addressed the first critic REVISE. Repeated Core/application review returned
ACCEPT before production, and all seven resulting tests passed. The separate node
critic also returned ACCEPT before node production; actual outcomes are recorded
separately from this test-first contract.

## Actual node gate

Add a strict closing input alongside existing Submit JSON; old spend carriers
remain byte-compatible. Closing candidates use the same bounded queue and replica
transport, external validity and durable-before-ACK path. Add owner submission
and truthful epoch-state read APIs. Recovery carriers distinguish closing records
from spends while retaining the old spend wire. A lagger must recover the actual
original closing record after its input has retired elsewhere. No refunds or
silent deletion of unresolved ordinary tickets when an epoch closes.

The native funded gate must use ordinary generated keys (including keys in the
future active registry), a pinned daemon, actual EVM proofs and live clock. Spend
once, close the old committee through ordinary gossip, recover on a lagger through
SQL failure/revocation/cold restart without a new quorum, preserve original QC,
and reject post-close spends and competing successors. All epoch !=1 guards stay
closed. Full state handover/next-epoch spending, old-lease outage closure and the
rest of V1 are not accepted by this closing gate.

Tests and fixture work precede production. Obtain a separate no-context
backend-test-critic ACCEPT before each production phase; run backend/frontend
regression after implementation. This file records the intended gate, not a pass.
