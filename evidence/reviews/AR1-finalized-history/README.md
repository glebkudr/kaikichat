# AR1 finalized-history index prerequisite

Baseline: `fb78e4e` (architecture review and expired sender retirement).
Specification: `spec/finalized-history-index-v1.md`.

The new P-256 index derives operation membership from a complete canonical
hash-chain prefix under an actual Simplex QC. It reuses ProfileStore's SQLCipher
database, keyed state rows, lifetime lock and atomic CAS transactions. No new
package, signer, wire format, committee or independent spend domain was added.

Eleven new index tests exercise 130 retained entries, duplicate rejection,
different-quorum retry preservation, restart, transfer across owners in pages
of seven entries, resumable partial transfer, SQL failure rollback, invalid
pages/genesis/quorum/scope, stale/wrong-owner handles, snapshot rollback and missing
or misdirected index rows. Existing fixture helpers independently encode entries
and create real Commonware P-256 signatures. These opaque entries are not proof
of funded postage or runtime consensus beyond 128.

The ancestry suite has ten tests across Ed25519 and P-256, including two new
cross-boundary cases. A short proof can target a sequence greater than 128;
paths longer than 128, bad links/quorum and expired live authority remain rejected.
The exact canonical Entry, journal and QC formats are unchanged.

- `test-contract.md`: scope and expected behavior written before production.
- `red.json`: missing API and two actual pre-implementation ancestry failures.
- `critic-revisions.json`: independent no-context REVISE → ACCEPT before production.
- `checks.json`: final regression results and artifact fingerprints.
- `validated-inputs.json`: frozen code/spec/test/build inputs for those checks.
- `plan-validation.json`: unchanged full scope and preserved original review.
- `documentation-updates.json`: proof-spec clarification made during regression.
- `post-regression-test-style.json`: the sole Clippy test-style correction, followed
  by all 21 targeted tests and workspace Clippy/fmt; production remained unchanged.
- `NEXT.md`: mandatory runtime/archive/spend and epoch integration work.

Final validation: 825 workspace tests (53 nonempty suites), zero failed/ignored;
59 frontend tests, TypeScript, Vite, workspace Clippy/fmt, 7 models and 12 EVM
models pass. The 21 final targeted tests are included in the workspace count.
660 final source/spec/test/build fingerprints are verified; the two explicitly
recorded documentation/test-style changes are not hidden as unchanged inputs.

Run logs belong to managed `output/ar1-finalized-history/`, not Git. Test data
and assertions are retained in source; counts include targeted tests, not duplicate
acceptance credits. This evidence is a prerequisite, not AR-R01/AR1 or full-V1 PASS.

The engine, service adapter, proof archive exporter and spend policy still have
their original runtime caps. A later-epoch index cannot reset an existing log;
actual epoch transfer is not implemented. Total cold transfer/revalidation and
retained storage are O(N); page/transaction size and working memory are bounded.
Archive reconciliation is still required before the derived index's absence
answers can participate in voting. No fresh live-EVM, packaged-native, independent
operator or three-platform gate is claimed by these tests.
