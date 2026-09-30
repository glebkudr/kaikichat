# Test critic — automatic public sender

R1: FINAL REVISE before production. Five blockers: one-entry QC helper rejects
the second common-log spend; all-provider iteration includes excluded index2;
SQL failure must actually be observed before/after restart; successful outgoing
receipts must survive another restart without repeated puts; the new public
resolver needs a genuine copied-offer rejection on another Noise transport.

R2: fixes submitted for independent review; no production implementation yet.
Inputs are frozen in critic-inputs-r2.json. The baseline RED stops at the
missing public_sender_status route; it does not prove the later workflow.
The independent signed-agent wire also passed a separate real-daemon diagnostic
(send, idempotent retry, owner-route refusal), using no owner token in the proof.

R2: FINAL REVISE, only the copied-offer observation race remains. R3 waits for
a new resolution created after the copier connects, then requires completed
partial state, the full exact query, an increased rejection counter, no
transport failures and the still-live genuine binding. Other R2 changes were
accepted. R3 is submitted before production.

R3: FINAL ACCEPT before production. All input hashes match; no remaining blockers or additional required scenarios for the bounded initial-ten gate. This accepts tests, not V1 completion.

R4: FINAL ACCEPT, two diagnostic trace fields only; failed run 2 still stops at finalizing. R5 adds portable current-ancestry behavior and isolates the real receipt commit fault. Production unchanged pending review.

R5: FINAL ACCEPT before production. All four frozen hashes match. Targeted RED has exactly five missing-method E0599 errors. No blocking or additional mandatory scenarios for current ancestry and receipt-only fault. Full daemon gate remains unverified.

R6: FINAL ACCEPT for diagnostics and Rust formatting only. Run4 still FAILED: copied-offer capture empty, cached empty-candidate partial resolver, zero rejections. Follow-up oracle audit FINAL REVISE: actual portable proof begins at target, not genesis. R7 corrects independent target-to-tip QC plus two-record common-log linkage; no copied-offer assertions or timeouts changed.

R7: initial FINAL REVISE for leftover second assert message; only that syntax error removed, then FINAL ACCEPT (all hashes matched, AST valid) before retry implementation. Corrected independent finality and copied-offer checks retained. Run4 diagnoses retry starvation on a cached empty partial result; production now waits without resetting its due time.

Run5 passes both real QCs, copied-offer rejection, 20 verified receipts, durable exact restart and unchanged holder ledgers. Original messages appear on offline recipient, but immediate scheduler counter assertion races its next tick. R8 reuses existing delivery.completed_reads (5s) before the same positive counters; adds recipient counter evidence. Production unchanged.

R8: FINAL ACCEPT for existing completed_reads wait and recipient diagnostics. Run6 failed earlier on unrelated global transport failure while cached partial had zero candidates/captures. R9 requires per-position rejected nonempty responses and exact capture; transport errors cannot increment the new diagnostic. New production diagnostic absent pending critic.

R9: FINAL ACCEPT before adding the public per-position refusal diagnostic. Exact captured query, completed resolution, actual rejected nonempty response, absent copier and live binding remain required; network errors never increment this position counter. No GREEN inferred.

Run7: full real daemon/EVM gate GREEN, 6 independent finalizer signatures, 20 independently verified receipts, exact sender restarts/no repeated holder puts, copied public offer refusal, two offline recipient starts and first-cache loss, zero ZK worker calls. This accepts initial ten-primary execution only; full workspace checks pending.

Final unchanged-input regression: all 791 Rust, 59 frontend, 7 model and 12 EVM model tests pass; seven native scenarios and seven personal screenshot comparisons pass. Ordinary release build/signature/default graph pass. Full V1 remains open.
