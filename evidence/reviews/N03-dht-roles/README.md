# Owner-controlled DHT roles

Ordinary nodes now explicitly run Kademlia as clients. Owners can offer DHT
service through the desktop network setting or `agentic-node serve --dht-server`.
Saved preferences override the CLI, including saved false. Existing profiles lacking
the field read false without rewriting original bytes. Validator authority remains
independent. Confirmed external addresses cannot silently promote the selected mode.
Relay-only suppresses direct DHT while retaining the preference for direct networking.

Core reuses encrypted persistence and revision/retry semantics; the node reuses
construct/persist/swap and the guarded cache. The desktop distinguishes saved
selection, unsaved edits and live state, including lost replies and stale revisions.
No dependency, wire protocol, quorum or resource limit changed.

All 600 Rust tests across 74 suites and 42 frontend tests
passed, zero failed or ignored Rust tests, with fmt/Clippy. R2 ACCEPT preceded
production. Baseline RED includes an actual decoded TCP Kad reply before new
diagnostics where refusal was required; Core RED was missing-field compilation,
frontend RED was the absent control. R3 accepted native coverage, R4 adapted the
exact Tauri schema, R5 explicitly configured three DHT servers in the old finalizer
fixture while retaining its recovery subject as a client. Failed runs and exact
review/source hashes are retained. See review-history.md.

Independent stock Kad probes verify replies/refusals over TCP/Noise and QUIC,
including enable/demote, saved restart and actual external-address confirmation
through successful relay reservations. Real encrypted failures preserve role,
revision and queued MLS delivery. Existing moved-recipient and sender-offline
mailbox scenarios pass with explicit ordinary DHT servers.

Real TCP/QUIC finalizer regressions retain 64 ordinary connections and unchanged
quorum, certify 15 effects per selected profile and refuse the next effect after
actual expiry. Signed cache isolation, crashes, real Marshal cursors, cross-scope
replay refusal and processing load controls remain. Genuine client and announcement
regressions generated four fresh proofs, verified QCs and passed cold pending and
unseeded moved-address recovery. Selected clients discover through two ordinary
DHT servers with no operator registrations. Separate controls toggle validator
authority independently of DHT service.

The hidden macOS WKWebView gate passed five product scenarios using the packaged
daemon and real Tauri commands. DHT UI checks cover enable, full daemon restart
with the same identity, disable and actual relay-only policy. Native DHT screenshots
were visually inspected. Separate headless component screenshots at 1440×1100 and
850×650 checked layout; these are UI fixtures, not network evidence.

All 506 source inputs stayed fixed through the normal Tauri release
build, ad-hoc codesign, test-driver/helper exclusion and two historical genuine
receipt compatibility checks. App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
The app is not notarized; Linux and Windows native matrices were not rerun.

Evidence: `evidence/reviews/N03-dht-roles/`; contract: `spec/dht-roles-v1.md`.
Larger committee route/cache capacity, private ciphertext custody and R=10 repair,
authenticated spent-state handover, remaining UI/MCP and full E01–E26 acceptance
remain open. **The full V1 product goal is not achieved.**
