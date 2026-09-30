# Collecting a finished paid batch between passes

Date: 2026-09-14. Component of step 1 of the [R14 plan](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).
The shared graph/pointer publisher and native acceptance remain open.

## Fixed behavior

Node keeps one bounded group per `(conversation, index_id, epoch)`.
It continues the collection on calls from different jobs. The limit is 12 originals;
an unsuccessful paid admission consumes a visit and does not hold the queue
forever. The last expensive candidate also yields control if the pass budget is
exhausted. A single send keeps the former short collection period.

The first variant kept the group but re-ran the full authorization/paid path for
all participants before commit. The independent critic rejected it. The test
observers now count real calls over the whole slow pass, including the final
phase: one primary and at most one full optional admission. The results of
evidence, system time and SQL are not substituted in these tests.

## Trust boundary

`CheckedPublicSenderHistory` creates Core after the ordinary full preparation.
It binds the current profile, the original message, the preparation, the
reservation, book/intents/policy and the verified context. Before commit Core
re-reads the active queue and the source bytes, checks the grant, the current
checkpoint/profile, the time and the exact envelope. Ordinary
observation/failure does not invalidate an unchanged preparation.

`CheckedOutgoingIndexes` creates CustodyStore through the shared verified getter.
It holds only immutable proof results and fingerprints of the real
body/meta/claims/expiry rows. Before use the store compares the actual bytes,
its domain/transport, the current clock and the receipt expiry; the existing
historical trust check remains mandatory. A revision change by itself is not
used as proof. The legacy checked read atomically saves the exact archive through
the existing row engine; the ordinary getter keeps the former semantics without
migration at unchanged time.

A changed participant returns to a bounded full check. Wrong data and revoked
rights exclude it, preserving the advancement of the others. Membership hints
appear only after the former atomic Core record. Payment, receipts, ACK and the
completion of delivery remain individual.

## Evidence and limitations

The [independent review](shared-collection-test-review.json) accepted the
implementation and the additional regressions. The [check results](shared-collection-checks.json)
record the actual runs and hashes.

Real paid fixtures check 16 originals → 12+4 in two signed leaves, the exact set
of operations without duplicates, revocation of a grant after collection started,
a stuck participant with subsequent placement, a body change through a separate
SQL connection without a revision change, ordinary progress/clock updates, an
atomic SQL rollback and a cold retry. A separate owner test checks the legacy
checked read at the former time, archive/head rollback, cold retry, archive
corruption, monotonicity and expiry.

This is not a measurement of the final section's duration within 20 ms. Hashing
entire BOOK/INTENTS/POLICY can repeatedly restart the group check under new
reservations; the constant inflow is checked at the next ordinary publisher
stage. First the ready data/index placements and the shared graph/pointer
coordination, then Diagnostic32 and the unchanged Full130. TTLs, deadlines,
allowances and the whole V1 volume are preserved.
