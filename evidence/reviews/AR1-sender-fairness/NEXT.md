# Continuation after bounded sender scheduling

This slice preserves the 128-active-job admission bound. Successful stored jobs
still participate in latest-pointer holder intersection. Removing them now would
lose the only existing route to older messages; recipient ACK and paid storage
must remain independent. Full AR-R02 is therefore not accepted by the fairness
gate or by the two-message test, whose data rosters overlap.

The retained [paid-index store continuation](../D05-paid-index-store/NEXT.md)
provides the dependency needed for safe successful retirement. Its next bounded
step is to authenticate original data-holder receipts from a compact signed
index descriptor plus the same native postage, original QC and historical
registry evidence. Reuse the existing historical primary/copy receipt verifier
and receipt subject fields. Do not synthesize ciphertext or treat a descriptor
as a verified received envelope. The result authenticates a finite signed storage
obligation; actual retrieval still must match the descriptor to the real envelope.

Tests must use actual funded fixture spends and original storage/copy receipts,
cover cold verification after admission expiry without the sender's wallet,
wrong descriptor/payment/transport/member evidence, copied obligations and
expiry, and preserve original signed bytes. Use independent backend-test-critic
ACCEPT before production. Then retain bounded locations and index receipts with
SQL failure/restart checks, add index put/read to the existing bounded custody
protocol, and test disjoint data rosters with both clients offline in turn.
Only durable index discovery permits successful jobs to leave the active queue.

Automatic acquisition/renewal of current authority, closure after old-authority
outage, pending input recovery, complete-range history and R10 remain separate
mandatory work. This dependency sequence does not reduce the 67-card / 22-E2E /
three-platform V1 scope or reopen deferred V2 jobs/A2A features.
