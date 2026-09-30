# L06 automatic checkpoint exchange — verified increment

The full V1 goal remains active. This increment provides real automatic public peer transport;
it does not produce attestations, verify live chain finality, authorize spending or establish
independent R=10 custody. Synthetic signed test headers exercise transport and lease boundaries.

## Contract and integration

Protocol /agentic-internet/checkpoints/1: strict CBOR request(profileId,after) and response
(available,certificates,more);256-byte request and132096-byte response limit, up to4 certificates
of32768bytes. Outgoing sources have live unexpired signed bootstrap identities; these identities
confer no attestor authority. Only the user's previously selected profile authorizes certificates.
Core requires exact retained head and every signed successor; absent history never resets it.

Read-only checkpoint_sync_anchor validates the saved profile/head and observed-time barrier
without background clock writes. Actual advancement uses existing atomic Core/SQLCipher history,
head and clock transitions. Serving returns only public verified certificates, with no private
contact/identity/funding state. Corruption/disk failure is fail-closed and chat remains independent.

The shared scheduler holds one global request, starts at most once per second, preserves the
budget across source refresh and fresh swarms, retries1..30seconds and polls idle sources every
30seconds. At most64 candidate states; request timeout5seconds and4 protocol streams. Public
serving is limited to32/minute globally and16/peer using the existing generic admission limiter.
These are finite resource limits, not Sybil/eclipsing protection. Relay-only excludes direct
candidate connections and denies direct inbound serving. No separate checkpoint dial path.

Owner node_info.checkpoints provides bounded counters/local scheduler status. Empty responses
are not evidence of global freshness; selected checkpoint expiry remains authoritative. Native
status polling automatically reflects accepted heads; its help now describes automatic and manual
imports without claiming a balance. There are no new agent/WebView mutation commands.

## Tests and critique

Tests preceded implementation. Separate review history and final ACCEPTs are in
L06-checkpoint-network-critic.md. Retained RED proves absent Core API/module/network protocol.
The reviewed suite adds1 real Core persistence test,2 deterministic scheduler tests and6 actual
process scenarios: expired multi-page catch-up/source loss/fresh explicit selection; SQL failure
and automatic retry during real MLS exchange; nine independent bad-wire variants and recovery;
two held sources, stale owner-response race and live swarm replacement; read-only public serving
at exact per-peer/global limits; actual relay-only fetching/serving policy and encrypted chat.

Targeted Core and node tests passed. Initial production compilation found module visibility
errors, fixed; initial strict Clippy found two unit-test unwraps, resolved with the reviewed local
test-only allowance. All subsequent aggregate formatting/Clippy/backend/frontend checks passed.

Six compiling mutants were isolated in a separate APFS workspace. A passing baseline precedes
each set, and a mutant is killed only by the named reviewed business-test failure. All6 killed:
shared slot, global start throttle, response rebasing to current owner head, retained stale slot
on swarm replacement, serving admission and oversized response frame allowance. Source hashes
and empty cleanupErrors: L06-checkpoint-network-mutations.json. The real source tree was never
mutated by that script; scratch was removed after completion.

## Aggregate acceptance evidence

scripts/check-native.mjs exit0:288 Rust,40 frontend,14 Solidity tests with256 fuzz runs; TypeScript,
Vite, debug/release app bundling, strict/deep ad-hoc codesign and release driver exclusion passed.
Apple notarization and other release platforms remain open.

All five packaged hidden WKWebView flows passed. The checkpoint flow now launches a second
actual desktop/daemon, explicitly selects trust and enters the source's listener through native
network settings. It never enters/submits a certificate. Exact expected signed checkpoint ID,
lease and trust appear in the follower UI and survive source shutdown plus follower UI/daemon
restart with identical identity. Native reference, checkpoint-peer-received.png and
checkpoint-peer-restored.png were viewed together: readable layout, wrapped help, same trust
and lease; connection status changes from one connected peer to no connection after shutdown.

Existing real Anvil daemon acceptance:53 owner IPC calls,4 successful funding verifications;
sourceHashe28080cb4b018ea075985f213c3ae6c0ed587878a22270e028460972000c4ad4.
Trusted CLI:12 verifications/two profiles;
sourceHash5f79f02d6546bfd080322b908f2890c6afb37ebdc1443b46c6f42f2a1a762d16.
Both reports passed with empty cleanupErrors.

Linux gate exit0: all7 actual LAN/direct/NAT/relay/DCUtR/AutoNAT outcomes;
run ain-nat-0f2c57b4, sourceHashbf6b526469681740190503978ea3907c15ecbd04abf0e04452eff000c283515e,
cleanupErrors empty; separate exact-label container inventory empty. Existing current-source
Linux gate checks transport regressions; new checkpoint scenarios run actual local TCP/circuit
daemons and packaged native clients, not a claimed full checkpoint-under-NAT acceptance suite.

No dependencies were installed. All builds ran on APFS and used the existing locked toolchain.
Required V1/E01–E26 scope and all84 original task cards remain open until complete acceptance.
