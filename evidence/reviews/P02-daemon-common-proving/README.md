# Common-policy proving in the packaged daemon

The owner can start a paid proof with start_common_postage_proof without choosing
its time interval. Core authenticates public checkpoint history and registry
evidence, then prepares the owned ticket against that exact opaque authority.
The existing proof read/cancel methods and eight-field response remain compatible.

Legacy proving, common proving and foreign receipt verification share the same
single worker, sixteen retained jobs, sixty-second retention and bounded child
I/O/cancellation/reaping implementation. The common job's deadline is the shorter
current-authority lease. Completion repeats owned-ticket preparation through Core
before publishing a receipt. Caller time/context/private input/executable overrides
are rejected. A ready local proof remains a relation receipt, not spend admission;
foreign verification still requires fresh authority. Every response has admission:false.

Tests preceded implementation and received separate no-context critic FINAL ACCEPT.
The Rust and Python entry points both demonstrated real missing-method RED. The
three targeted process tests then passed, as did 493 ordinary Rust tests, seven
models, nine independent-oracle tests, formatting and workspace Clippy. All forty
frontend tests, TypeScript and Vite passed. No approved test input changed during
implementation or subsequent validation.

The complete combined EVM scenario passed using the ordinary release app's bundled
daemon/prover/verifier. Its legacy proof took 316091 ms and was 585044 bytes; the
new common-policy proof took 309412 ms and was 584978 bytes. Independent full-journal
verification confirms identical context, nullifier and resource envelope. A foreign
daemon without sender wallet/prover accepted the new genuine receipt.

The scenario passed input/ownership failures, exact retries, request-kind conflicts,
capacity shared in all directions, real worker cancellation, daemon SIGKILL and
wallet recovery, MLS responsiveness, and all previous foreign-verifier and legacy
proving regressions. Late genuine output after head change was refused. New current
authority prepared the same ticket/nullifier; an actual short checkpoint lease
stopped a running genuine prover while the common statement still had time left.
Paid rows remained unchanged. Test profiles and owned processes were cleaned up.

Five hidden packaged WKWebView scenarios passed: messaging/receipts, history
recovery, scoped MCP/revocation, network settings/restart and checkpoint trust/
recovery. Chat, agent access, network and restored-trust screenshots were visually
compared with the preceding verified native evidence; the layout remains consistent.
Those native tests used the debug automation bundle. The subsequent ordinary
release app passed deep/strict ad-hoc signature verification and excludes the test
driver from its default dependency graph. The EVM proof test then used that release
package, whose bytes remained unchanged. It is not notarized.

All 394 frozen source inputs, six accepted test inputs and five native gate inputs
matched their fingerprints. Raw logs stay in ignored output; validation.json records
their paths and hashes. Genuine public receipts, compact results, review, screenshots
and cleanup evidence are retained here.

This increment does not complete public epoch-history availability, issuer-global
canonical spend, paid custody/repair or full V1 acceptance. It does not repeat the
Linux network matrix, full EVM remainder or every separate real-proof suite.
The collective one-MiB IPC limit remains unchanged. No new guest, verifier image,
schema or dependency is introduced. See spec/postage-circuit/daemon-common-proving-v1.md.
