# N03: ordinary client discovery and reservations

An owner request_postage_spend may omit peers or pass an empty array. Both forms
canonicalize to the same request and use the authenticated current public committee
keys. An explicit one-to-four key/PeerID array retains its serialization and input
hash algorithm. Pins constrain a current authenticated binding and cannot create
one. The API remains owner-only; no signer or custody admission is granted.

Only genuine-verified, unexpired pending requests contribute discovery interest.
Service and client reservations reuse one current binding selector, preferring a
newer actually verified route over an older signed hint before enforcing any pin.
Sending chooses an available verified route; the candidate is serialized once per
pending request rather than once per possible key. Existing concurrency limits remain.

A client reservation has independent public-authority and pending-request signals.
Its deadline is bounded by authority, request and binding. Durable completion removes
that request source; failed SQL commits retain the pending request. Other live local
or client sources sharing the same peer survive. Renewed public authority cannot
reactivate the completed request's old signal. Catalog input work is bounded at 768
(four local services plus eight requests, at most 64 keys each); at most 64 distinct
PeerIDs, 192 total established, 64 ordinary established, 48/16 total/ordinary outgoing,
32 unauthenticated incoming and the existing 1/2-per-peer physical bounds remain.

Test-first R1 FINAL ACCEPT preceded all production changes. The original seven
lifecycle tests remain byte-exact; one new test covers shared local/client sources,
two requests on one peer, revocation, fresh reuse and non-revival. Baseline RED was
missing Reservation::client at compilation and omitted peers rejected at the API
schema. That cheap preflight ran before genuine proofs; it is not a baseline live
network failure. Four reviewed test/spec inputs remained exact through final gates.

The existing ordinary client scenario retains damaged real receipts, wrong outer
fields, actual selected transport carrying a foreign genuine QC, request/result SQL
failure, pending crash, partition without quorum, healing, conflicting operation,
offline cold reads and historical verification after actual authority expiry.

Two additional ordinary clients run TCP/Noise and QUIC. Each knows only two ordinary
seed addresses, has no operator or service, and discovers all four selected keys.
The selected key paired with a wrong PeerID stays pending with zero sends/reservations even after
the authentic binding arrives. A real SQL failure holds automatic-A pending. Cold
restart removes only ordinary bootstrap-cache state, preserving every other main
profile state and the pending request. With selected nodes and seeds stopped, all 64
ordinary peers connect. Starting selected nodes alone restores all 4 selected routes
using original signed cached addresses under full ordinary fan-in. A genuine QC
cannot finish until SQL succeeds. Completion releases selected capacity while all 64
ordinary clients and the unrelated wrong-pin request survive. Automatic-B reacquires
capacity, verifies its conflict's winning QC, and releases it again. These new clients
retrieve an already certified outcome; fresh ingress/consensus is proved by the
original scenario, not claimed as a second consensus run by the new helper.

Validation: 589 Rust tests across 74 suites, zero failed/ignored;
40 frontend tests; workspace fmt/Clippy. Client E2E independently verifies
30 QC signatures with 2343 owner calls and two fresh genuine
proofs. Full local-service TCP/QUIC regression retains 13 durable effects per profile,
64 ordinary connections, late selected reconnect, refused 65th ordinary/reused slot,
role disable/re-enable, slow-effect/cold-cursor, replay and real expiry. It verifies
81 signatures per transport and makes 1034 owner calls. The announcement/DHT
regression also passes two fresh genuine proofs and unseeded cold address recovery,
with 3636 owner calls. All runners report no cleanup errors.

All 497 frozen source inputs stayed exact through the ordinary Tauri rebuild,
ad-hoc deep/strict codesign, WebDriver/helper exclusion and verification of both
historical genuine receipt fixtures with the unchanged image ID. Native UI and Linux
were not rerun; the app is not notarized. Old explicit serialized fields/hash logic
are preserved, but a database artifact from a previous binary was not tested.

Reproduce from the repository root using the build-storage wrapper:

```sh
python3 scripts/build-storage.py run cargo fmt --all -- --check
python3 scripts/build-storage.py run cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/build-storage.py run cargo test --workspace --all-targets
python3 scripts/build-storage.py run npm --prefix apps/desktop test
python3 scripts/build-storage.py run python3 tests/evm/postage_spend_client.py
python3 scripts/build-storage.py run python3 tests/evm/finalizer_service.py
python3 scripts/build-storage.py run python3 tests/evm/service_announcements.py
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
```

Evidence: validation-summary.json, run.json, source-inputs.json, release.json,
review inputs/verdict, red/, client-live/, finalizer-network/ and announcements-live/.
Build/run logs remain in ignored output/service-discovery-planning, identified by
SHA256. The raw fan-in carrier and SQL failure helper are excluded from the app.

Full V1 remains incomplete. Required next work includes shared processing capacity,
independent DHT client/server roles, alignment of 16-entry route/cache limits with
larger committees without reducing quorum, actual private ciphertext admission and
custody/R=10 repair, spent-state handover, remaining UI/MCP and complete E01–E26
product acceptance. This run proves neither a 64-member mesh nor ciphertext custody.
