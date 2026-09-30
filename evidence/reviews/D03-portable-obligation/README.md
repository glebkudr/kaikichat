# D03 — portable original custody evidence

This checkpoint retains original public zkVM proof/context through custody
admission, sender receipt aggregation and restart. It does not close D03,
E05–E07 or full V1. Automatic target10, independent metadata and autonomous repair
remain open; the 64-validator/R24 engineering gate remains RED and parked.
Orders, ratings and reviews remain V2, with existing regression tests retained.

The genuine fixed-image receipt is retained only after existing verification and
Core fences succeed. Its original checkpoint/history/registry evidence stays
together after current-head advancement; the actual operation replaces the
retained policy placeholder. Incoming and outgoing encrypted commits retain the
carrier atomically with the original obligation. Outgoing positions share one
proof/context per operation. Existing serialized quotas still bound actual bytes.

Owner-only `custody_obligation` returns a live incoming original bundle after the
clock commit. Legacy missing proof is explicit unavailability, while private
ciphertext reads continue. Renewed bindings do not rewrite any original evidence.
Historical audit is distinct from current admission and grants no new-copy,
repair, spend or signer permission.

Independent context-free test review R1 returned REVISE for a nonce type error;
R2 and strengthened R3 returned FINAL ACCEPT before production. Six new paid-store
tests cover real proofs, cold independent reconstruction, SQL failures, expiry,
shared evidence and legacy reads. The existing Core scenario now checks original
evidence after 80 heads/restart; the real daemon scenario checks owner access and
agent denial. All reviewed R3 test hashes remain unchanged. Compilation RED is
reported as compilation, not executed failing scenarios. The corrected daemon
RED actually returned `unknown_method`. An initial production re-export typo was
fixed without test changes; both unsuccessful build logs remain hashed.

All 708 workspace Rust tests passed, zero failed/ignored, four threads and 47
nonempty suites, including nine actual Tauri command tests. The run took
972.304654 seconds on 587 frozen inputs. All 59 frontend, 7 model and 12 EVM model
tests, fmt/all-target Clippy, TypeScript and Vite passed. All seven hidden native
WKWebView scenarios passed, including 1051 actual messages, recipient restart,
22 UI pages with exact IDs/text and a reply. Seven screenshot pairs were
personally compared with baseline 48ed794 without visual regressions.

A new real proof paid for an actual 872-byte MLS ciphertext. Three actual quorum
signatures were verified. Sender SQL failure/retry and restart passed. A restarted
custodian exported the exact original proof, context, envelope, full SpendRecord,
storage receipt and presentation. The returning recipient automatically retrieved
one original message with sender absent, through storage failure/retry,
custodian/recipient restarts and unavailable endpoint fallback. One live selected
primary was used, not target10. Cleanup reported no errors.

The fresh proof took 452227 ms; SHA256 is
`3b812b81d3943efb4d16850ee047d9629f84b3e3bbd65a4f4e3a9aef404a5890`.
Both the independent oracle and verifier in the ordinary rebuilt macOS arm64 app
verified it; changed operation and expiry were rejected. The app's ad-hoc signature
and exclusion of driver/test helpers passed. Its proof/verifier hashes match the
baseline bundle, and both historical receipt checks passed. Live proof-worker
binary hashes are recorded separately. Notarization/Linux/Windows remain open.

The checked network trace is losslessly compacted JSON; exact public proof bytes
also appear in `fresh-receipt.json`. `native-e2e/` retains only the seven compared
screenshots and two current result files. Raw logs and other runtime captures are
outside Git under `output/portable-obligation/`, with root log hashes retained.
The full EVM matrix was not rerun for this persistence module; the fresh paid
network, required workspace/frontend gates and native app checks were run.

Contract: `spec/portable-custody-obligation-v1.md`.
