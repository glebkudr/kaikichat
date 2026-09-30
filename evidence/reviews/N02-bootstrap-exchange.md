# N02: bounded bootstrap self-record exchange

Status: implemented; focused and complete regression gates PASS on2026-09-05. This is a foundation for N02, not full N02/E02 acceptance.

## Contract

- Optional `agentic-node serve --bootstrap <multiaddr>` supplies up to four explicit IP/PeerID hints. A hint may include one Circuit Relay v2 hop. Multiple sources are tried independently; there is no mandatory company seed or DNS lookup.
- The existing encrypted cache supplies at most64 currently valid, signed peer records on startup and during bounded refresh. The runtime never distributes that cache to other peers.
- `/agentic-internet/bootstrap/1` uses a bounded CBOR object containing only `nodeRecord`, as a byte string, for request and response. The existing canonical signed NodeRecord verifier checks network domain, root signature, TTL, actual authenticated transport PeerID and valid routes before persistence. The response is the responder's own record. No third-party lookup or peer list is exposed.
- A bootstrap hint does not create a conversation, authorize an agent, attest independent operators or replace the pinned network domain. Registry/genesis checkpoint consensus is outside this exchange and remains unfinished.
- At most four discovery requests run concurrently. Candidate state is bounded by four explicit hints plus64 cache entries; retry uses finite backoff. Connections and frames retain independent transport limits. Inbound record verification needs bounded per-peer/global admission.
- Owner `node_info.bootstrap` reports the pinned domain, currently connected verified peers, cache/policy counts, failed/rejected attempts and an actionable `bootstrap-needed` state. A cached signature alone is not evidence of a live connection.
- Relay-only constrains discovery as it constrains application delivery; a direct bootstrap hint cannot silently bypass the owner's policy.

## Tests-first evidence

New process tests: `crates/node/tests/support/bootstrap.rs`, using real daemon subprocesses, actual MLS/receipts and an independently defined raw libp2p peer. Existing helpers remain shared.

Initial RED: `/tmp/ain-bootstrap-red.log` (retained as `N02-bootstrap-red.log`), four tests failed on the unmodified bf76147 production: missing `--bootstrap` and absent actionable bootstrap state. Separate context-free `node_test_critic` returned REVISE for attack synchronization, exact disk checks, inbound/routing boundaries, resource bounds and relay-only evidence. The revised suite received FINAL ACCEPT before implementation. A compile-only `.clone()` correction for the zeroizing StateValue test helper received a separate FINAL ACCEPT; assertions were unchanged.

- Six real-process tests pass: `/tmp/ain-bootstrap-process-green.log`. Invalid foreign-domain, signature, session PeerID, expired, wrong-route PeerID and unsupported-route responses run with isolated peers/daemon restarts and preserve exact encrypted-state revision/bytes. An independent peer verifies self-only disclosure, exercises inbound invalid→valid requests and actual ninth-request admission refusal. Cache reopening initiates a connection to a raw peer that never dials back.
- Three scheduler/admission tests pass: `/tmp/ain-bootstrap-limits-green.log`; RED was unresolved bootstrap module in `/tmp/ain-bootstrap-limits-red.log`. Controlled time verifies four held slots across source refresh, source bounds/deduplication,500ms/1s/capped30s retries, finite refresh after success, per-peer/global limits and recovery after60seconds.
- The first common gate found an obsolete direction assumption in the existing address-change test: autonomous bootstrap lets Bob initiate the TCP connection, giving Alice an ephemeral remote source port. The fixture now clears only Bob's disposable discovery cache after shutdown and independently checks Alice's signed persisted new route; the original outgoing endpoint, exact message/receipt/history assertions are retained. This prevents reverse dialing from masking broken route persistence. The separate critic returned FINAL ACCEPT for the fixture change; `/tmp/ain-bootstrap-route-fixture.log` confirms its actual GREEN.
- Initial Linux run executed all six network outcomes, but correctly invalidated its report because the reviewed fixture changed during the run. It is not accepted as current-source evidence; the full gate was rerun against the final source hash below.

## Implementation and compatibility

`crates/node/src/bootstrap.rs` integrates a new libp2p request-response behavior, existing Core signature/cache verification and owner diagnostics. `bootstrap_schedule.rs` holds the bounded scheduler/admission. Delivery and discovery share route-policy/first-hop preparation, including fresh ordinary TCP dial ports and relay-only enforcement. No dependencies were added.

Each frame is capped at8192bytes before application admission; the signed record remains capped at4096bytes. Up to68 merged candidates retain four in-flight slots across refresh/eviction. The cache is refreshed every5seconds; successful peers are rechecked every30seconds. Cached-root responses must match that root. Live verified state disappears on disconnect or record expiry. Network reconfiguration clears obsolete requests/connections and restarts discovery under the new policy.

Peers lacking `/agentic-internet/bootstrap/1` fail discovery negotiation with bounded backoff; existing delivery remains a separate protocol. The prior v2 NodeRecord compatibility restriction still applies. Incoming exchange reveals the sender's root-to-transport binding and advertised addresses to the contacted peer; this is not an anonymity protocol. No other user's record, contact inventory or message history is returned.

## Remaining acceptance

LAN mDNS, private rendezvous, partial Kademlia, authenticated registry/checkpoints, deployed diverse seed operators and company-offline acceptance are not provided by this step. Actual IP-hint fallback does not establish operator independence or a validated registry.

## Final application evidence

- `/tmp/ain-bootstrap-native-final.log`:196 Rust tests (including node unit11/process39),27 frontend tests, strict fmt/Clippy, TypeScript/Vite, all4 actual packaged hidden WKWebView flows and rebuilt macOS arm64 release with deep/strict ad-hoc signature check. Normal bundle excludes WebDriver. The released app still lacks notarization and other-platform acceptance.
- `/tmp/ain-bootstrap-network-final.log`: PASS, all6 current Linux scenarios. `output/network-e2e/result.json` sourceHash `725a8bd4c64c211577c3fba7529995acb26cd967110830fc19e3be5e5015d509`, run `ain-nat-40fe101e`, cleanupErrors empty. No run-labelled containers remain.
- Own visual comparison of the current `output/native-e2e/network-alice-restored.png` with `output/playwright/component-network-wide.png`: saved provider and relay-only state, verified address/relay1of1, intact chat sidebar and disabled unchanged Save. No visible regression.
- Source and release artifacts include the new daemon behavior. Explicit bootstrap hint entry has not yet been added to native controls; automatic saved-record reconnect already runs in the desktop daemon. Full E01–E26, N02/E02, registry and R=10 acceptance remain open.
