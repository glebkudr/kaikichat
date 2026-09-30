# Independent backend test review

Reviewer: /root/spend_recovery_test_critic, spawned without inherited context.
Baseline a9fbad3. Production was unchanged until all revisions were accepted.

## R1: REVISE

The two-case native exercise violated the shared helper's assumption of competing
tickets. It failed during setup, not recovery. The native contract also lacked
revocation while a delivery was deferred. Both were blocking. Additional useful
checks: no persistence under a mutated target key, explicit positive controls
for alternate/conflicting genuine QCs, and a precise recovery SQL failure wait.

## R2: ACCEPT

The helper now has defaulted options for competing-ticket cases and rejected
receipt probes. This exercise uses distinct tickets and never supplies the lagger
with the owner's original receipt, including before configuration. Native
revocation now occurs while real recovery commitment is failing. With the fault
removed and peers still online, nine seconds require no running service, held
recovery, network writes, record commit or consumer advancement. Cold replay after
re-enabling must recover from one peer without a new quorum. Rust tests cover both
keys and verify the valid QC controls independently.

The actual R2 RED completed afterwards: three original source records, nine QC
signature checks, zero lagger candidate inputs and 30 failed archive effects.
Recovery never reached commitment. Raw diagnostic summary is in red.json.

## R3 and final refinement: ACCEPT

Existing Marshal metadata uses None for an untouched height, so pre-ACK checks
accept None or zero; later ACK heights 1 and 2 remain mandatory. A real selected
test peer now reopens an ordinary stopped profile, serves its genuine transport
binding and an altered original QC. Local rejection, no write/ACK and recovery
from the ordinary peer are required. The carrier's optional field preserves old
wire shapes. Cleanup records failures and still allows ordinary daemon cleanup.
The final refinement changes a QC signature byte while explicitly confirming
CBOR framing remains valid. The critic accepted the exact final tests before
production. Acceptance is of the contract, not a claim that runtime passed.

## Executed fixture diagnosis and correction: ACCEPT

The first implementation run passed the actual recovery SQL failure and role
revocation, but never exercised the hostile response. A separately accepted
diagnostic-only update retained carrier events and current routing state, using
the existing complete-line JSONL reader. The diagnostic run established one live
service and deferred delivery, but zero routes and sent requests; the carrier
only logged its Noise connection. Its ordinary postage-response mode lacked
service announcements, while this cold validator had no explicit peer/key pin.

The test carrier gained an opt-in mode that serves its genuine own announcement
through existing code as well as full proofs and the supplied hostile response.
Older carrier modes remain unchanged. The native test now requires announcement,
full proof and receipt-free request observations before rejection can pass. The
critic accepted this change without weakening any record/cursor assertions.
These were test infrastructure changes only; production stayed unchanged.

The next run reached QC signature rejection through a genuine compact binding.
The carrier had not needed to serve a full-proof RPC, because Core authenticated
the compact binding against its stored roster. The critic inspected that path
and accepted replacing the redundant servedProof event assertion with exact
authenticated route equality for peer, key and base committee. Announcement,
receipt-free request, signature rejection and no-record/no-ACK checks remain.
The final complete native gate subsequently passed with 24 signature checks.
