# Private custody placement checkpoint

The ordinary Tauri app has been rebuilt and checked. This checkpoint implements
private custody placement and membership verification; it does not implement
ciphertext storage, admission or repair and does not complete V1.

The owner command `plan_postage_custody` derives positions from a genuine
fixed-image ticket's nullifier and an authenticated frozen registry/future beacon.
It returns `admission: false` and queues no spend. Selection reuses the shared
sampler, current Core context checks and actual gapped-rank membership proofs.
The checked context owns its registry snapshot; retained contexts refresh that
snapshot at the current authenticated head. No funded owner opening is exposed.

## Test-first evidence

R1 required genuine class-boundary cases. R2 accepted three newly generated real
receipts before production implementation. Actual compile RED demonstrated the
absent APIs. The first runtime run passed four tests and failed one because the
test attempted to replace immutable registry code; production correctly refused.
R3 accepted correction of that impossible setup. All five placement tests and the
retained-context test across 80 heads on each of two chains then passed. The
original R2 test is retained byte-identically. The independent no-context reviewer
ran both receipt verifiers for every new class 2/3/4 fixture. No fake verifier or
candidate was used. See `test-review.json`, `review-r1.md`, `review-r2.md`,
`review-r3.md` and `class-fixture-generation.json`.

## Actual validation

- 545 Rust tests across 72 suites, zero failures or ignored tests; 40 frontend
  tests; seven models; nine independent oracles; formatting and Clippy passed.
- The live ordinary-daemon placement gate passed with 223 owner calls, two fresh
  genuine receipts and no cleanup errors. Exact ordered 10+4 placement survived
  cold restart. Wrong operation, damaged seal and caller resource override were
  refused. Client pending/sent and selected spend candidates remained zero.
  Zero QC signature checks are expected because this API grants no admission.
  Captured receipts also passed independent fixed-verifier/upstream-oracle replay.
- The unchanged paid-client regression passed with 1,283 owner calls and 18 QC
  signature checks, including repeated verification. Forged results, SQL failure,
  partition/healing, cold offline results and real expiry behavior were checked.
  Cleanup errors were empty. The two gates rebuilt their own debug binaries;
  their distinct hashes are recorded rather than described as one binary.
- The ordinary macOS release passed deep/strict ad-hoc signature verification,
  automation-driver exclusion and both genuine historical receipt compatibility
  checks. It is not notarized. Native UI and Linux matrix were not retested here.

## Frozen inputs and documentation handoff

All code, tests and 29 accepted inputs remained fixed. During the client gate,
the user's separate architecture handoff changed only the frozen document
`spec/finalizer/routing-v1.md` and added `spec/service-discovery-v1.md`. All eight
actual gates passed, then the original wrapper correctly stopped at its manifest
barrier. The original exit 1 and 464-input manifest are preserved in `phase1-*`.
The strict resume verified this exact documentation-only transition and ran the
two remaining release gates against the updated 465-input manifest. That manifest
stayed fixed through packaging. The original entire manifest was not unchanged;
`run.json` and `architecture-handoff.json` record the distinction explicitly.

`validation-summary.json`, `run.json`, `release.json`, captured public receipts
and source manifests retain the checked evidence. Raw build/proving logs remain
in ignored output directories; their hashes are in the reports. The fixed image
is unchanged. The next architecture work is bounded service discovery through
existing Kademlia and authenticated transport bindings, followed by actual paid
ciphertext custody and autonomous repair. Full V1 acceptance remains open.
