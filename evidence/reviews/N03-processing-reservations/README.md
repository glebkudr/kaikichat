# Shared processing reservations

One per-swarm resource budget now covers all ten application request/response
protocols. It shares the existing authenticated connection-reservation catalog.
Actual authenticated handler PeerID selects ordinary versus selected capacity;
public-key claims, protocol names and owner-provided pins do not grant resources.
The bound worker codec retains its original independent authority/request sources
and owns a physical slot and byte charge through decode, application-response wait,
write, cancellation or timeout. Revocation cannot free still-owned memory or allow a
new grant to revive an old token. Network replacement drops the old workers.

Limits: 16 ordinary + 48 selected operations, 4 ordinary/peer, 16 combined/peer;
4×(16MiB+1024bytes) per class and 2× that bound per peer. Operations conservatively
charge max(request bound,response bound). Independent monotonic one-second windows
admit 256 ordinary / 64 per peer and 1024 selected / 256 per peer; completion releases only
active/byte charge. No waiting queue is added. Existing codecs, per-connection bounds,
timeouts, outbound-job limits, Core verification and cryptographic fences remain.
These are operation/wire size reservations, not a total-process heap or CPU quota.

Finalizer frames are polled before ordinary application protocols. Unknown/inactive
scopes return `unavailable`; unrelated transports are refused before profile/Core
reconciliation and cannot populate the selected frame-admission table. Current Core
membership, authority, transport scope and engine/application admission remain required.
Owner node_info exposes bounded processingCapacity diagnostics without mutation.

## Test-first evidence

Independent no-context critic R1 requested actual decoded-worker lifetime and measured
finality/load overlap. Both were added; R2 ACCEPT preceded production. R2 RED:
missing processing API compilation and actual TCP early-refusal failure before new
diagnostic-field checks, all reviewed/production/helper hashes unchanged. R3 accepted
an equivalent Clippy lint correction. R4 accepted a stronger completed-request load
(staggered 20 ms cadence, one outstanding request per peer); held/idle behavior and all assertions unchanged,
so original RED remains valid. See review-history.md and exact manifests/patches.

The first implementation passed focused tests and 596 Rust and 40 frontend tests, then failed
live completed-rate traffic: reader concurrency masked the rate boundary. A subsequent
transport-before-reconciliation change introduced an unknown-scope response
regression caught by an unchanged test, which was fixed without changing that assertion. Both failed traces are
retained. The final workload separates incomplete-reader and completed-traffic loads.

## Final validation

All 596 Rust tests in 74 suites passed, zero failed or ignored;
40 frontend, fmt and Clippy passed on final inputs. Real TCP/Noise and QUIC each retain 64
ordinary clients and selected peers, certify effects 13 and 14 on all four selected
profiles under held/complete load, independently verify QCs, then certify effect 15
and refuse 16 after actual authority expiry. Original replay, real SQL/cursor failures,
slow cold replay, role/network replacement and connection refusal/reuse remain tested.

| Transport | Distinct early-refused peers | Held ordinary readers | Fresh complete writes during finalization | Rate refusals |
|---|---:|---:|---:|---:|
| tcp | 48 | 16 | 215 | 2036 |
| quic | 48 | 16 | 301 | 3916 |

Network gate: 1199 owner calls;
independent signature checks [96, 90].
Genuine ordinary-client TCP/QUIC regression: 2243 owner calls,
30 signature checks and two fresh genuine proofs.
Automatic discovery, wrong pin refusal, cold pending restore, 64 ordinary + 4 selected peers,
SQL failure, durable release/reacquisition and conflicting-result verification remain.
Announcement/DHT/genuine-spend regression: 3599 owner calls,
two fresh proofs and unseeded cold recovery with moved endpoint. Cleanup errors: none.

All 503 exact inputs stayed fixed through ordinary Tauri build, ad-hoc
codesign, exclusion of test carriers/driver and both historical receipt compatibility
checks. Fixed proof image: `ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de`.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`. Not notarized; native UI and Linux matrix were not rerun.
No UI changed in this module. Previous-version client DB migration was not tested here.

Full V1 is not achieved. DHT client/server roles, route/cache capacity alignment for
larger accepted committees, actual private ciphertext custody and R=10 repair,
authenticated spent-state handover, remaining UI/MCP workflows and complete E01–E26
product/native acceptance remain required. This checkpoint does not redefine the goal.
