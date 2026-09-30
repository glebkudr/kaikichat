# Paid private storage and recipient read capability

This checkpoint supplies durable paid primary storage and owner Core/daemon read
capabilities. It does not implement custodian network transfer, automatic offline
delivery, independent index placement, repair, or complete V1. The last verified
ordinary desktop package remains the preceding `000f8c9` envelope checkpoint.

On 564 unchanged source inputs, all 671 Rust tests passed with zero failures or
ignored tests, using four test threads. All 51 frontend tests, 7 model tests,
12 EVM model tests, TypeScript, frontend build, formatting and workspace Clippy
passed. See `gates.json`, `regressions.json` and `source-inputs.json` (SHA256
`c2b433904b4fefff0fb7025c53cf27311e44d63ec3337bd6de7a53b2fba4aa63`).
Native WKWebView acceptance, ordinary release packaging, heavy EVM/spend E2E and
cross-platform checks were not rerun here. No new external dependency was added;
Cargo.lock only adds existing internal crypto/protocol dependencies to postage-spend.

Twelve new tests cover:

- Two crypto tests using ten independent signed read vectors and exact wire equality.
- Seven actual paid storage tests: genuine receipt/QC admission, selected operator
  position/role/transport, paid byte/lifetime bounds, durable original receipt and
  full historical evidence, restart, quota, conflict, expiry, private pagination and
  real SQLCipher write failures. They open retained ciphertext after the short
  connection/checkpoint authority expires. These use authenticated historical time.
- Two Core tests and one actual daemon test: incoming MLS direction, conversation
  isolation, fresh nonce, fixed 30 seconds, bounded request, unchanged persisted
  state/outbox, reopened recipient with sender absent, owner authentication and
  agent/time/expiry override refusal. This tests capability preparation, not fetch.

Tests preceded implementation and received independent FINAL ACCEPT decisions.
The paid helper required R1/R2 corrections for retained evidence and durable read
clock assertions; R4 corrected SQL revision type and R5 corrected direct-target QC
encoding. Compile RED and actual daemon `unknown_method` RED are retained in
`execution-excerpts.json`. Production's first StateValue ownership compile error
was corrected before the seven paid tests passed. Review manifests preserve their
historical input hashes; they are not a claim that implementation was executed then.

Six new genuine fixed-image RISC0 receipts are retained in
`crates/postage-spend/tests/fixtures/paid-custody/`. They bind independently encoded
envelopes to actual canonical-issuer funding, including distinct tickets and paid
bound violations. Both the normal verifier and independent upstream oracle accepted
all six; changed-operation and exact-expiry controls failed. The artifact critic
independently repeated these checks. No private owner payment opening was retained.
The public synthetic custody exporter is intentionally fixed for test decryption.

The generator source fingerprint was frozen through proving. Its report refers to
the Cargo.lock before the two internal dependency additions; that lock is available
at `000f8c9:Cargo.lock`. Other fingerprint inputs match the accepted generation
manifest. `fixture-generation.json` and `generated-artifacts.json` preserve artifact
bindings. Generation used historical authenticated time with the source chain
stopped before proving; it is not live daemon admission evidence.

The first full run failed one existing announcement retry test with an unexpected
non-Timeout outbound failure (94 other node unit tests passed). Its report, source
manifest and exact failure are retained. The unchanged test passed in isolation
and when rerunning the original failing binary. The subsequent complete run used
four test threads; no assertion, deadline or test selection changed. The cause of
the original intermittent failure is not established. See `announcement-recheck.json`.

Reproduction uses the repository build-storage wrapper. Run the model suites,
`cargo fmt --all -- --check`, workspace Clippy with `-D warnings`, and
`cargo test --locked --workspace --all-targets -- --test-threads=4`, followed by
`npm --prefix apps/desktop test`, `run typecheck`, and `run build`.
The contract is `spec/paid-custody-store-v1.md`. Actual network custody and R10
repair are the next product gates; 64-validator debugging remains parked.
