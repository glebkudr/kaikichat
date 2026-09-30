# Current issuer-bound Core service authority

The Rust-only preparation API now authenticates each supplied issuer binding proof
at the exact current checkpoint against the installed finalizer policy. It derives
one canonical application/log scope per issuer and reuses full roster validation,
enabled selected keys, finite authority, revocation fences and durable clock commits.

Tests preceded production and received independent no-context FINAL ACCEPT after
one strengthened stale-proof case. See [review.md](review.md) and the twelve frozen
[accepted inputs](accepted-inputs.json). The missing-interface RED exited 101.

Verification passed 507 Rust tests (zero failed/ignored), seven model tests, nine
independent oracle tests, formatting/workspace Clippy, forty frontend tests,
TypeScript and Vite. All 24 Core finalizer tests passed. A fresh two-chain actual
Anvil scenario validated six issuer-policy and eight P256-selection CLI results,
then all four Core scenarios with the chains stopped. A genuine paid purchase
changes the root without changing the binding; stale proof rejection is exercised.
No source provider was needed during Core verification and cleanup reported no errors.

The ordinary release Tauri app was rebuilt. Deep/strict ad-hoc codesign passed and
its dependency graph excludes the automation driver. Its bundled verifier accepts
both existing genuine historical receipts with unchanged fixed guest image and
matching independently encoded full journals; changed operation and expiry fail.
No new proof was generated and no current-time authority is claimed for old receipts.
The package is not notarized. The 415 frozen source hashes remained unchanged.

Reports: [ordinary gates](validation.json), [actual EVM/Core](evm.json),
[fresh public proofs](fixtures.json), [release](release.json),
[receipt compatibility](compatibility.json), [counts and scope](summary.json).
The accepted ordinary-test fixture is separately committed under Core tests.
Raw logs and disposable profiles stay in ignored output/postage-service.

This API starts no consensus runtime, creates no spent ledger and grants no
admission. Epoch handover and issuer-global spend, UI/MCP spending, paid custody/
repair and full V1 acceptance remain unfinished. Native UI, the Linux network
matrix, remaining full EVM gates and new-proof process suites were not repeated
for this Core-only increment. The domain contract is
[spec/postage/current-spend-service-v1.md](../../../spec/postage/current-spend-service-v1.md).
