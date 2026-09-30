# P01 selected finalizer transport routing

Baseline:66599d7f54a5bd294ff0ba90bdec682195c1f355. The full V1 goal remains open.
This increment connects enrolled/selected P-256 keys to live daemon transport peers.
It does not start the selected daemon voting service or complete P01.

## Tests before implementation

Core/finalizer RED failed on the absent bind/verify, roster publication and serving APIs.
The separate no-fork core_test_critic required isolated TTL60/61 signatures under a live
renewed committee, a correctly signed real unselected registry key, a corrupt binding
under an otherwise valid Core roster and query negatives on an enabled selected provider.
Those revisions received FINAL ACCEPT before production. Accepted test hashes:

- crates/finalizer/tests/p256_route.rs:27539081d0b0f8e2563aa00438a3ed1e4a04bae9b124d649737f6f73c474e3ea
- crates/core/tests/support/finalizer_route.rs:2f1d32bb6bc452ac99edaa9a36b6200d9526c8bdc0a4b3b6381b32cfe5aa293f

Node process and fresh EVM RED then reached the real daemon and failed on unknown
`publish_finalizer_roster`. The separate no-fork node_test_critic required actual endpoint
proof around TCP/QUIC success, retained binding expiry before head expiry, proof that four
raw requests arrived and no fifth was sent, and valid forbidden field values. A further
revision made the result read the first daemon call after binding expiry, before any
checkpoint status call could advance its clock. FINAL ACCEPT preceded node production.

- crates/node/tests/support/finalizer_owner.rs:78a0dbce255d092d22259599fd1e379e891b2f4a70f5f55facbfb5a34a0fb7e9
- tests/evm/finalizer_network.py:42319980b2f4c28fa2b4aff41224d59a9b975ead946baeb59803507fdb755b10

The existing generic raw-peer test carrier gained an optional capture request, preserving
its existing replay/hold behavior. It captures a real provider response over its own
connection and serves the copied proof to a consumer from a different authenticated key.
No production secret is exported or copied to the adversarial peer.

## Implemented behavior

The shared Commonware P-256 signer produces canonical domain-separated CBOR bindings with
a maximum60-second lifetime capped by the current verified authority. No new dependency
version or handwritten crypto was introduced. Signature, selected member, transport key,
network, committee, epoch, canonical encoding and both time limits are checked together.

Core reuses full selection verification. Owner publication persists a bounded complete
roster using CAS plus the durable checkpoint clock. Serving additionally requires a local
enabled selected encrypted key. Head changes require a new current proof; restart cannot
extend authority. A fresh proof of the same committee may validate an old signature only
until its original deadline. SQLCipher write/clock failures cannot leak a partial success.

The daemon adds owner roster publication, asynchronous peer verification and result reads.
The latter repeat full Core verification against the same live authenticated connection.
Network replacement, connection loss, stale proof or expiry invalidate cached authority.
Inbound admission, relay checks and Peer ID decoding reuse existing transport helpers.
Queues, stream counts, wire sizes, retention and timeouts are bounded.

## Focused evidence

Four new finalizer tests and all16 Core finalizer tests passed. Three actual daemon
process tests passed, including ordinary pending MLS delivery through owner configuration
and failures. Targeted all-target Clippy passed for finalizer, Core and node.

The focused fresh local EVM run used17 paid random daemon-generated P-256 registrations,
full public registry proofs and an independently sampled committee. It passed actual
TCP/Noise and QUIC endpoint checks, copied-peer rejection, four received pending requests,
current-head retirement, provider restart, disable/re-enable, short binding expiry with
live head, subsequent head expiry and consumer restart. The chain was stopped during
proof exchange;333 actual owner calls completed, cleanupErrors was empty. Full report is
P01-finalizer-routing-focused-evm.json. Aggregate/native/Linux results are recorded in the
validation JSON after their terminal completion.

## Boundaries

A valid route is discovery evidence only. Disabling a provider stops new bindings but
cannot remotely revoke an already issued public signature. The selected voting runtime
must separately supervise enablement, head/lease changes and application admission.
No quorum is reduced and no absent member is replaced. Groups, paid spend authorization,
offline R=10 custody/repair, jobs/credentials/trust and full E01–E26 remain required.

## Completed aggregate and application validation

`scripts/check-native.mjs` terminated with exit0:445 Rust,29 Solidity,7 model and40 frontend
test functions (521 total),18 fresh EVM reports, formatting/Clippy/TypeScript/frontend build,
five actual hidden native WKWebView flows, a fresh release `.app`, strict deep ad-hoc
signature verification and no WebDriver in the default dependency graph. Four paired
baseline/current screenshots were inspected together; no new clipping or overlap was found.

A separate fresh OrbStack Linux build passed all seven existing network scenarios and
cleaned up its own containers/networks. The run was ain-nat-a03cb1ce; subsequent Docker
queries found no remaining matching resources. New selected P-256 proof exchange ran on
macOS; Linux coverage here is the existing messaging/NAT/relay regression on the new daemon.

The source manifest remained unchanged through the full run. Every top-level EVM report is
successful and newer than the aggregate start, with empty cleanup errors where applicable.
The reports, signed bundle/standalone hashes, logs and screenshot provenance are retained
beside this review. Tauri re-signs Mach-O sidecars when bundling, so signed bundle and
standalone test executable hashes are recorded separately without claiming byte identity.
The release is local macOS arm64 and is not notarized. Full V1 remains incomplete.
