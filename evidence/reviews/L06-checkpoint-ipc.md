# L06 owner-authenticated checkpoint daemon API

Date: 2026-09-05. The running daemon now exposes the four durable checkpoint Core operations
through its existing owner socket. This advances V1 integration without claiming that the
whole application, full L06/E21 or E01–E26 acceptance is complete.

## Test-first review and RED

Two actual Rust process tests and a new live Anvil/daemon runner preceded production code.
The separate originally context-free `/root/node_test_critic` reviewed both, their reused
helpers, the fixture-generator extension, aggregate gate composition and the IPC contract.
The parent waited for the final verdict before further work.

The critic returned REVISE for an incorrect shared override test: issuerProfile is a valid
required field for install, so replacing it with the same value must be accepted. That one
combination is now excluded from the invalid-field loop; the same field remains forbidden
for other commands. Wrong-owner/agent denials also assert absence of result, and live funding
compares the exact commitment. FINAL ACCEPT preceded production changes.

`L06-checkpoint-ipc-red.log` records both actual process cases failing unknown_method.
`L06-checkpoint-ipc-evm-red.log` records all previous EVM verifications passing, then the new
actual daemon call failing unknown_method. The initial repeat briefly waited for another
process's Cargo lock; its owner exited without intervention, and the same waiting test
process completed its expected RED. No process or build was restarted because of that wait.

## Implemented boundary

`crates/node/src/checkpoint_ipc.rs` validates bounded owner requests and delegates to Core.
The existing strict owner parser is shared. The normal actor supplies SystemTime; no new
agent broker method, caller time, caller state root or issuer override is introduced.
Checkpoint/issuer manifests and proof arrive as JSON strings so duplicate fields reach the
existing strict verifiers without Value normalization. Certificates are bounded 0x hex,
checkpoint/commitment hashes are exactly 32 bytes, and proof text is at most 512 KiB.
The existing 1 MiB frame, private socket/token and 5-second IPC bounds remain unchanged.

Responses retain Core's exact IDs, lease, trust mode and U256 allocation. Errors distinguish
malformed requests, rejected evidence, changed revision, unavailable/corrupt storage and
clock rollback. Denials never include a result. There is no available balance, spending
permit, new mint, settlement or automatic source observer in this increment.

## Evidence and scope

- `L06-checkpoint-ipc-green.log`: both real process cases pass. They exercise identity
  prerequisites, owner-token and working scoped-agent denial, valid certificate acceptance,
  immutable profile/retries, malformed/oversized requests, preserved head/history across
  restart, real SQL INSERT/UPDATE triggers and subsequent recovery. Actual MLS traffic and
  signed receipts continue through a checkpoint disk failure and a damaged-state restart.
- The Rust signed headers are synthetic IPC fixtures, explicitly separate from chain-finality
  evidence. The live runner independently signs real Anvil roots with Python Ed25519/CBOR.
- `tests/evm/checkpoint_node.py` keeps all earlier contract/proof/checkpoint gates. Its normal
  and full-width cases re-fund the surviving chain tip after the shared rollback fixture,
  then feed exact real proofs through framed IPC into the actual Rust daemon. It checks
  exact domain/leaf/allocation/commitment, profile/checkpoint IDs, duplicate JSON rejection,
  bad proof substitution and valid-typed time/root/issuer overrides.
- Source and daemon are stopped; the same profile/proof verifies after daemon restart within
  lease. In the ordinary-amount case, both remain stopped until actual wall-clock expiry,
  and the first funding call after restart fails. No host clock or daemon time hook is changed.
- Independent subprocess deadlines, finally cleanup, report invalidation before the run and
  source hashes prevent stale pass reports. Generator defaults and existing vector corpus
  remain unchanged; no dependencies were installed or upgraded.

Final aggregate evidence:

- `L06-checkpoint-ipc-native.log`: exit 0; formatting/Clippy, 269 Rust tests, 14 Solidity tests
  including 256 fuzz sequences, 30 frontend tests, TypeScript/Vite, all earlier EVM checks,
  the new actual daemon gate, four packaged hidden WKWebView scenarios, release bundling
  and deep/strict ad-hoc signature verification pass. Release excludes the automation driver;
  Apple notarization remains unconfigured.
- `output/evm-e2e/checkpoint-node.json`: passed:true, 53 owner IPC calls, four successful
  funded verifications across two new normal/full-width profiles, actual expiry denial and
  cleanupErrors:[], sourceHash `5b10557e232e0cde49260d49757c3d82c9537b865d0023abcac305c413a3916f`.
- Prior live verifiers still pass 26 funding-proof and 12 trusted-checkpoint CLI checks. Their
  reports retain explicit trust boundaries and empty cleanupErrors.
- `L06-checkpoint-ipc-network.log`: all seven current-source Linux outcomes pass, run
  `ain-nat-6c500602`, sourceHash `213e2993088f8c3512a48b773cff804637185801b534f2f1786228df070549e5`.
  cleanupErrors is empty; exact-run container/network inventories are empty.
- The new actual restored-settings screenshot was viewed beside the preceding run. Fields,
  layout and relay status remain intact with new fixture PeerID/ports. All visual checks ran
  in the background; no focus-stealing browser launch was used.

Logs normalize trailing whitespace only. Test/Anvil/daemon resources have been cleaned.

The selected attestors remain an explicit trust assumption, with chainFinalityVerified:false.
Anvil and all funds/keys are local fixtures. This is owner-supplied proof verification during
source outage, not automatic RPC/source failover or a chain consensus light client. The
Tauri profile-selection/payment UI, bounded live sources, spend consensus, retained ciphertext,
independent R=10 custody/repair and the other mandatory V1 modules remain required work.

Contract: `spec/postage/checkpoint-owner-ipc.md`. This adds an observable daemon/API boundary
to L06.N01/F01 and the durable Core work; it does not close E21's full admission/retrieval case.
