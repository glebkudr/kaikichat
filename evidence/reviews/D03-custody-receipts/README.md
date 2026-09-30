# D03 — custody receipt verification

Four tests added after D02: original saved presentation across restart/renewals,
plus three sender-verification tests using genuine fixed-image proof/QC fixtures.
Eleven paid-storage tests, six existing operator-authentication tests and 51 frontend
tests passed; workspace/all-target Clippy passed. Full-workspace regression and
native/release packaging were not rerun for this increment.

`put_receipt` returns the exact committed receipt with its original presentation.
`verify_receipt` verifies full paid QC and selected membership at current candidate
authority, the expected peer signature and every receipt field. Original binding
validity is checked at signed admission; a renewed binding cannot replace it, and
an expired binding cannot authorize a new admission. A fresh-Core expiry control
prevents clock rollback from masking an ignored current-authority fence.

Shared paid/object/admission and operator-binding comparisons avoid separate
validation implementations. Existing store rejection and SQL-failure tests exercise
the same code through the `put` compatibility wrapper. The evidence does not prove
network receipt counting, retrieval, R10 or repair. The real-daemon network gate is
the next increment; its baseline currently reports `unknown_method`.

The prior full 671-Rust/51-frontend checkpoint and preserved intermittent failure
remain in D02. The verified ordinary application remains commit `000f8c9`.
