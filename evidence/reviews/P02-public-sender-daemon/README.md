# Ordinary public-postage sender: initial ten-primary execution

The daemon now executes the durable jobs created by ordinary owner and scoped
runtime messages. It reauthorizes the original preparation, obtains a real
native spend QC, resolves selected custodians over authenticated Noise, stores
the exact paid MLS envelope and commits ten independent transport/position
receipts before publishing the private mailbox pointer through actual DHT ACKs.
The public owner IPC exposes configuration, sponsorship policy and execution
evidence. Signed messaging grants cannot invoke these global owner routes.

## Evidence

- `red.json`: the initial real gate fails at the absent sender status API.
- `critic-review.md` and `critic-inputs-r*.json`: independent no-context test
  review before production, including the accepted current-ancestry regression
  and position-specific proof-refusal observation. Revisions are retained.
- `failed-run-{1..6}.json` and matching traces preserve development failures.
  They are not acceptance evidence. Raw logs remain local and ignored.
- `module-checks.json`: 22 Core sender tests pass, zero failed or ignored.
- `e2e-green.json` and `e2e-green-trace.json`: fresh canonical EVM funding,
  actual daemons/SQLCipher/OpenMLS/Noise, six independently verified finalizer
  signatures and twenty independently verified custody receipts for two
  ordinary messages. Owner orchestration and retrieval work calls are zero.
  Real outgoing-receipt commit failure/retry, sender restarts, copied genuine
  offers on a different Noise peer, unchanged holder ledgers after successful
  sender restart, sender-absent recipient recovery/restart and first-cache loss
  all pass. Self-tested prover/verifier tripwires record zero workflow calls.
- `final-checks.json` and `final-inputs.json`: all 791 workspace Rust tests
  pass (zero failed/ignored, 50 nonempty suites), 59 frontend, 7 model and
  12 EVM-model tests, fmt, all-target Clippy, TypeScript and Vite pass on
  635 unchanged frozen inputs. The interrupted earlier workspace invocation
  and process recovery are retained separately, not counted as successful.
- `native-checks.json` and `native-e2e/`: all seven hidden WKWebView scenarios
  pass, including actual MCP/revocation, relay/restarts and 1051 real history
  messages over 22 pages. Existing V2 job behavior is regression only.
- `visual-review.json`: all seven old/new screenshot pairs were personally
  inspected together. No new layout regression observed; live timestamps vary.
- The ordinary macOS arm64 bundle was rebuilt and its ad-hoc signature verified
  with `codesign --verify --deep --strict`. The default dependency graph excludes
  the automation driver. Bundle and executable SHA256 values are recorded in
  `native-checks.json`. Build output is not committed.

All build/test commands use `python3 scripts/build-storage.py run ...` from
the canonical repository. No packages or versions changed. The parked live
64-validator/R24 gate was not run. Current bundle:
`/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.

## Limits and remaining V1 work

This is initial ten-primary execution, not full V1 acceptance. Receipt storage,
spend QC, pointer publication and recipient delivery ACK remain distinct facts.
Local separate processes prove distinct transport identities, not independent
physical machines. The test configures funding/sponsorship through owner IPC;
the native regression does not accept a new graphical wallet setup flow.

Independent durable indexes/control metadata, autonomous R10 repair with both
clients offline, terminal queue retirement/capacity recovery, authority refresh
for pending remote spends, committee/epoch handover and admission-expiry
lifecycle, complete wallet/sponsorship desktop and CLI/MCP UX, remaining
messenger acceptance and Linux/Windows release evidence remain open.
The mailbox pointer currently uses up to four cached endpoints shared by the
stored messages; this is not acceptance of independent durable indexes.
See `spec/postage/public-sender-daemon-v1.md` and the effective 22-scenario
release graph. Orders/reviews remain deferred to V2.
