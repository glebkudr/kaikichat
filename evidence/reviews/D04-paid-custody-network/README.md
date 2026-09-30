# Actual paid ciphertext over the ordinary daemon network

A fresh, genuinely paid MLS ciphertext was stored by its selected primary over
TCP/Noise, retained across custodian restart, and fetched by the original recipient
while the sender was stopped. The recipient then restarted and fetched again;
exactly one original message remained. This is one primary with explicit owner
orchestration. Automatic discovery/retrieval, independent indexes, R10 and
sender/recipient-independent repair remain open; full V1 is not accepted.

On 567 unchanged source inputs, all 675 Rust tests passed (zero failed/ignored,
four test threads), alongside 51 frontend tests, 7 model tests, 12 EVM model tests,
TypeScript, frontend build, fmt and Clippy. Eight Tauri command tests and all six
hidden WKWebView scenarios passed. Five result/reference screenshot pairs were
personally inspected; see `visual-review.json` for the exact scope. The ordinary
macOS arm64 app was rebuilt and ad-hoc codesign verified, with test driver/helpers
excluded and both retained genuine receipt compatibility cases passing. It is not
notarized. `app-release.json` records all five binary hashes.

The daemon uses `/agentic-internet/paid-custody/1`, shared bounded processing and
existing connection policy. SQLCipher commits precede durable storage receipts and
read pages. The sender authenticates the receipt's actual transport signer, original
operator binding at admission, selected position, complete genuine QC and exact
immutable paid envelope under current authority. The saved original presentation
survives renewed bindings and exact retries. Scoped agents cannot invoke owner APIs.

`network-evidence.json` and `network-trace.json` retain the live result and public
synthetic protocol evidence. The fresh fixed-image RISC0 proof binds the actual
872-byte envelope, rather than a historical corpus entry. Three P256 certificate
signatures and the complete Ed25519 storage receipt were verified independently.
The fresh ZK receipt was verified during execution; only its hash, operation,
nullifier and proof-worker hashes were retained by this harness. Its complete proof
bytes were not separately retained for independent offline re-verification. Existing
historical genuine proof fixtures remain in the repository and are independently
verifiable. The source chain was stopped after renewal and before storage; the
sender was present for storage and absent throughout recipient fetch checks.

The live gate also passed disabled-new-work refusal with retained reads, wrong
selected position, malformed ciphertext and read capability paired with valid
requests on the same independent Noise connection, copied genuine receipt from a
different peer, forged advancing cursor, real SQLCipher INSERT/UPDATE failures,
four held requests with exact-ID retry and changed-input conflict, fifth-request
capacity refusal, daemon deadline release and successful subsequent read.
Temporary daemon profiles and private keys were cleaned up successfully.

`review-history.json` records independent preimplementation decisions. R1 required
the fixture to outlive the daemon deadline and stronger same-connection/forgery
controls; R2 accepted those corrections. A later two-line Runtime constructor fix
was independently accepted without changing assertions. The missing-owner-method
RED and actual daemon GREEN are retained. Earlier sender-verifier review and tests
are in `../D03-custody-receipts/`. A first Clippy run caught the constructor and large
enum issues; transparent boxing and the reviewed constructor corrected them.

The pre-existing intermittent announcement retry failure remains documented in
`../D02-paid-custody/`; no assertion or deadline was changed. The 64-validator/R24
engineering gate remains RED and parked. This checkpoint does not rerun that gate,
Linux/Windows native acceptance, or the entire historical EVM suite.

Reproduce the live gate from the canonical repository:

```sh
python3 scripts/build-storage.py run python3 tests/evm/paid_ciphertext_network.py
```

All build/test commands use the repository storage wrapper. `run-gates.py` and
`verify-app.py` preserve the bounded regression/native/release invocation for this
checkpoint; their output path is local build storage. Native UI acceptance uses a
hidden debug-feature WKWebView bundle from the same source; the ordinary release
excludes its driver and test helpers. No external dependency was installed.
