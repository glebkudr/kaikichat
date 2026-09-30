# D03 — durable outgoing custody receipts

This checkpoint completes a bounded sender-evidence module, not D03, E05–E07 or
full V1. Automatic placement, independent indexes, R10 and autonomous repair remain
open. The 64-validator/R24 engineering gate remains RED and parked. Orders, ratings
and reviews remain V2; their existing regression coverage is retained.

Before production, context-free independent backend-test-critic R1 returned REVISE;
R2 returned FINAL ACCEPT. R2 requires complete original sender envelope, full
SpendRecord and authority snapshot before any incoming object exists. Ten actual
selected obligations fit one operation and one envelope's byte allowance, with
separate incoming quota. Exact SQL INSERT/UPDATE failure, live-process query,
restart, wrong object/position/peer/signature and expiry/clock rollback are covered.
Store RED is compilation failure for absent APIs (16 E0599 in R2), not 16 executed
failed tests. The owner daemon RED is an executed unknown_method failure. A wrong
test-target command was corrected; it did not execute a scenario. Reviewed test
hashes remain unchanged after production.

The encrypted sender ledger commits before `stored`. It preserves original
per-position receipts, selected operator presentations and transport keys, while
sharing envelope/SpendRecord/authority snapshot once per operation. Historical
signature validation is reused from incoming custody without changing that saved
format. Original binding/current-checkpoint expiry does not erase a live
obligation. Expired entries are pruned with a durable monotonic clock. Strict
owner-only `custody_receipts {operation}` survives restart. Signed obligations do
not assert physical independence or recipient delivery. This ledger does not yet
retain the raw zkVM seal or authorize historical new-copy/repair work.

All 702 workspace Rust tests passed, zero failed/ignored, four threads and 47
nonempty suites, on 586 unchanged source inputs. Nine real Tauri command tests are
included. All 59 frontend, 7 model and 12 EVM-model tests, fmt, all-target Clippy,
TypeScript and frontend build passed. All seven hidden WKWebView scenarios passed,
including 1051 real messages, recipient restart, 22 UI pages with exact IDs/text and
a real reply. All seven screenshot pairs were personally compared to 7e567f4.

A freshly generated fixed-image proof paid for the actual 872-byte MLS ciphertext.
The live network verified 3 quorum signatures. Remote storage succeeded while a
real sender SQLCipher trigger forced local commit failure: polling returned only
rejected/storage_error. Retry saved the original signed storage receipt without
another payment; sender restart preserved it while ordinary delivery remained
queued. With sender absent, automatic recipient discovery/read recovered exactly
one original message through independent pointer caches, custodian/recipient
restarts, target storage failure/retry and one unavailable endpoint. It uses one
live selected primary, not ten. Cleanup reported no errors. The proof took 666603 ms.

Fresh proof SHA256:
`b74150ca4f82eb55fc5f2a611dcc39f27f2c8b986b57bc7e4ec60c2f0172f00a`.
The independent oracle and packaged verifier both verified it; operation
substitution and expiry were rejected. The ordinary ad-hoc signed macOS arm64 app
was rebuilt and checked without driver/test-state/fan-in helpers. Its proof and
verifier hashes match 7e567f4, and two historical genuine receipt compatibility
checks passed. Live harness proof binary hashes are recorded separately; no claim
of host-binary identity with the bundle is needed for fixed-image compatibility.
The app is not notarized; Linux/Windows acceptance remains open.

See `regressions.json`, `source-inputs.json`, `backend-run.json`, `support-checks.json`,
`paid-network-evidence.json`, `paid-network-trace.json`, `fresh-receipt.json`,
`fresh-receipt-verification.json`, `app-release.json`, `visual-review.json` and
`native-e2e/`. Raw logs remain outside Git under `output/outgoing-receipts/`, with
hashes in `raw-log-hashes.json`. Full EVM matrix was not rerun for this persistence
change; the fresh real paid network gate and required workspace/frontend gates
were run. Contract: `spec/outgoing-custody-receipts-v1.md`.
