# DHT role test and implementation history

Baseline: `94d4c7fc83662af09b5dc7c5afb354c92f658191` in
`/Users/glebk/Code/chat`, branch `implementation/v1`. Full V1 remains unfinished.

R1 froze 19 test/spec files, 124 unchanged production files and 506 total source
inputs. A separate critic with no inherited context returned REVISE: default and
demotion tests did not exercise libp2p automatic promotion after external-address
confirmation. No production implementation occurred before this review.

R2 added an actual relay server, independent successful circuit reservations whose
returned addresses come from the confirmed external-address set, and TCP/QUIC Kad
reply/refusal probes before and after DHT enablement/demotion. Both owner diagnostic
endpoints consistently check mode, enabled and policy suppression. The critic
verified all hashes and returned FINAL ACCEPT before implementation.

The exact R2 baseline then failed three gates as expected. The daemon connected
over real TCP/Noise and returned one decoded Kad reply while the new default-client
test required refusal; this occurred before any new diagnostic/schema assertion.
Core compilation failed on the absent dht_server field. The frontend failed on the
absent owner checkbox. These are distinct evidence types; Core RED is compilation
only. The reviewed tests and unchanged source/helper hashes stayed fixed through RED.

Production reused the existing encrypted network preference, owner CAS, swarm
replacement and GuardedRouting/libp2p mode boundary. It added no dependencies or
new cryptographic/discovery protocol. Focused Core, daemon role, existing routing,
mailbox, frontend and Clippy checks passed.

R3 added later native product acceptance using existing hidden WKWebViews and
packaged daemon controls: saved role/revision, full daemon restart, identity,
demotion and actual relay suppression. A separate UI-only visual fixture reflects
simulated state; it is not network evidence. The critic verified all 19 R2 files
unchanged, reviewed the two additions and current implementation hashes, and
returned FINAL ACCEPT. This review approved tests, not production correctness.

The first full gate passed fmt/Clippy, then the unchanged Tauri command test failed
because its exact preference JSON omitted the new field. The failed run, source
manifest and logs are retained in failure-tauri-schema. No production change was
needed. R4 added the explicit false default, a true request covered by existing
untrusted-window/origin ACL checks, and actual client/relay-disabled diagnostics.
The original identity, listener, revision, secret exclusion, retry and refusal
assertions remained intact. The critic verified 22 test/spec files, 124 unchanged
current implementation inputs and 506 total inputs, and returned FINAL ACCEPT.

The second full gate passed all 600 Rust and 42 frontend tests, then failed the
finalizer topology: all four selected peers were now default DHT clients, retained
zero remote address hints and correctly did not publish to each other. All reached
ten genuine certified local fixture effects. The old fresh-cache requirement was
preserved. Failed logs/manifests/trace are in failure-finalizer-topology.

R5 explicitly enables DHT service on three existing selected fixture peers through
the owner API; the cold replay/capacity subject remains a default client. Opt-in is
asserted not to enable a validator. Existing quorum, cache freshness, isolation,
64 ordinary fan-in, processing-load and expiry assertions stay exact. No peers,
endpoints or production changes were added. The independent critic verified all
23 test/spec, 124 unchanged implementation and 506 inputs and returned FINAL ACCEPT.

The final run is recorded in run.json on those exact 506 R5 inputs. Completion of
individual phases is not full V1 acceptance. See validation-summary.json and the
final report for the actual outcome and remaining product requirements.

All eleven R5 gates passed. The first native screenshots were at the submit-button
scroll position and did not show the DHT control. Supplemental visual inspection
reused the existing hidden native client/lifecycle helpers and unchanged network
scenario, scrolling the network panel to its top before capture. Those overviews
were viewed together and accepted. Canonical tests, their raw screenshots and all
506 frozen inputs stayed unchanged. capture-overview-inputs.json records the helper
and generated capture source hashes; native-overview/result.json records binary
hashes and successful cleanup-controlled capture. No test assertions were weakened.
