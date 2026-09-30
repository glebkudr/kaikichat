# N03: finite selected connection reservations

Core-approved local services retain connection capacity while ordinary clients fill
their existing 64 slots. The node keeps the ordinary 16 pending-dial budget, adds a
bounded 64-PeerID reservation catalog and enforces 192 total established connections,
48 total pending outgoing, 32 unauthenticated incoming and the existing 1/2-per-peer
limit. The original libp2p absolute guard remains first, before stateful protocols.
Its bypass feature is not used. These limits grant neither signatures nor work authority.

Only live local service membership combined with an original authenticated finite
service hint or a newer actually verified route can reserve a transport identity.
Only one identity per selected key is chosen; shared sources may keep a peer live.
The lease ends no later than the binding and local authority, with a 60-second ceiling.
Role/head/network changes revoke shared signals. No owner/MCP mutation API or
unverified Client::targets() source was added. Input is bounded at 256 grants and 64
unique peers. node_info.connectionCapacity exposes actual ordinary/reserved occupancy.

The ordinary class includes demoted reserved sockets until their real Closed events.
Retirement selects newest entries into the ordinary class, so a client that connected
while other sockets were reserved survives their later demotion. Close requests are
exact and nonduplicated; neither a queued close nor a refused dial frees a live slot.

Seven trait-level tests drive real NetworkBehaviour admission and lifecycle callbacks.
They cover 64 ordinary plus 128 reserved sockets, rotation at the 192 absolute ceiling,
per-peer and pending bounds, actual close/failure reuse, catalog refusal, shared
revocation, real finite expiry, partial retirement order and a future observation.
They are synthetic lifecycle tests, not network/authority evidence.

The independent critic first required bounded initial handshake waves and a separate
ordinary-client authority test boundary. R2 accepted eight waves of eight real peers
and the explicit deferral of client reservations. Actual baseline TCP then reached 64
ordinary connections and failed specifically on late selected reconnect. The new
module also had a compile-only baseline RED. R3 accepted a lint-equivalent iterator
correction. Reviewed final tests remain byte-exact.

The first implementation passed all Rust/frontend tests and the late selected join,
65th-peer refusal and released-slot reuse, but failed role-revocation cleanup: sorting
by original socket age also closed the newer ordinary replacement client. Its failed
TCP trace and exact source manifest are retained in retirement-attempt-1. Its pre-fix
source was reconstructed and verified byte-for-byte against that manifest. QUIC was
not run in that failed attempt. Production now orders entry into the ordinary class,
including demotion. The same reviewed test then required a complete rerun.

Final validation: 588 Rust tests across 74 suites,
zero failed/ignored; 40 frontend tests; workspace fmt/Clippy. Full TCP/Noise and QUIC
both preserve 64 ordinary clients, reconnect 3 selected peers after the ordinary pool
fills, refuse a 65th ordinary peer, reuse its released slot, revoke/restore reservations,
and finalize effect 13 on all 4 selected profiles under that fan-in. Earlier scope replay,
independent QC checks, crash/durable cursor, slow effect, authority replacement and
actual expiry checks remain. Signature checks per transport:
72, 69; 1102 owner calls; cleanup errors empty.

The unchanged announcement/DHT/paid-finality gate also passed with two newly generated
genuine proofs, signed route renewal and unseeded cold recovery. Its reservation flag
remains false because that runner does not test contention; the full finalizer runner
above supplies that evidence. The ordinary Tauri package was rebuilt and its ad-hoc
signature, driver/test-helper exclusion and both historical genuine receipt verifications
passed. All 495 frozen source inputs remained exact through packaging. Native UI and
Linux were not rerun; this app is not notarized.

Evidence: validation-summary.json, run.json, source-inputs.json, release.json, independent
review inputs/verdicts, baseline red/, failed retirement-attempt-1/, finalizer-network/
and announcements-*.json. Build/run logs remain in ignored output/service-discovery-planning,
identified by SHA256 in the gate reports. Test-only carriers are absent from the app.

Reproduce from the repository root:

```sh
python3 scripts/build-storage.py run cargo fmt --all -- --check
python3 scripts/build-storage.py run cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/build-storage.py run cargo test --workspace --all-targets
python3 scripts/build-storage.py run npm --prefix apps/desktop test
python3 scripts/build-storage.py run python3 tests/evm/finalizer_service.py
python3 scripts/build-storage.py run python3 tests/evm/service_announcements.py
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
```

Full V1 remains open. Required next work includes ordinary postage-client authenticated
reservations, bounded shared processing capacity, DHT client/server roles, aligning
service-cache/routes 16 with larger committees without reducing quorum, actual private
ciphertext custody and R=10 repair, remaining UI/MCP behavior and all E01–E26 acceptance.
The 64-peer guard alone is not evidence of a 64-member daemon mesh.
