# Next boundary: authenticated historical placement

The new carrier retains the missing original seal, context and custody evidence.
It does not deserialize into SpendCandidate and does not lift a current fence.
Next tests must distinguish an old authentic paid obligation from new spend or
unbounded replacement work.

Core authenticate_postage_history already authenticates a historical finalizer
snapshot against installed profiles while committing only the present observed
clock; it never overwrites the live head. Reuse that pattern for a separate opaque
historical public context, not a CheckedPostageContext acceptable to new work.
Original context ancestry may precede the current head by more than 80 certificates;
its original checkpoint/proof must stay together. A read-only historical verifier
must derive a valid authentication instant from retained signed history (or retain
an explicitly checked original instant), bound it by current observed time, and
verify the full original zkVM relation. Never roll back Core's current clock.

The existing verify_record binds the entire certified journal to issuer policy,
network, operation/nullifier and verified_at. Genuine proof resources can differ
from a quorum-certified altered journal, as demonstrated by the existing negative
fixture. Historical placement must compare those exact journals, not infer proof
validity from QC alone. Registry membership and placement use the original frozen
roster/seed. Reuse the existing placement draw and assignment ID derivation rather
than implement a second sampler or journal verifier.

Current generic operator bindings stop at registry admission_until. Completing
original paid primary positions or replacements after that window needs specific
finite consent tied to the original paid operation/TTL and selected position.
It must not renew generic new-work permission. Authenticate original held receipts,
transport ownership, exact envelope, distinct primary/replacement positions and
finite repair allowance. Malformed historical input, wrong profiles/rosters and
certified-but-different resources must fail without successful state changes.

Only after that authority boundary is reviewed should a bounded restart-safe
scheduler place copies beyond a dead sender. An actual 17-custodian fixture exists;
extend its independent position/receipt oracle instead of treating position0 as
proof of target10. Separate ciphertext placement from independently retained
sender-authenticated manifest/control commitments and finite prefix frontiers.
D03/E05-E07, automatic R10 and autonomous repair remain open. The 64-validator/R24
engineering gate remains parked; agent orders/ratings/reviews remain V2.

Read-only follow-up: TrustedCheckpoint::authenticate_history already performs full
canonical quorum verification using the signed issue instant as a bounded hint;
it neither renews the certificate nor changes the caller's present clock. A
historical context can reuse that path, then verify the original registry proof
at that authenticated head's issued_at and derive the original common statement.
Registry verification still checks block_timestamp <= checked_at < min(head lease,
admission_until). There is no need to invent an untrusted proof-success timestamp
or bypass the existing registry verifier. The historical Core wrapper must commit
only trusted host now, retain explicit installed profile binding and expose no
live CheckedPostageContext constructor. Any later implementation requires reviewed
expiry, head-advance, mismatched proof/QC and permission tests before production.
