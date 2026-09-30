# L02 registry owner IPC

Baseline: 8efb47f103b9b5d9788f527626893168a1bfa47d. The actual daemon now exposes
the existing durable Core registry selection and membership verification. This is a V1
increment; it does not finish operator possession, placement, paid custody or R=10 repair.

## Tests before production

Three actual-process tests preceded handlers. They exercise full DTO equality, immutable
byte-preserving selection/retry/restart, wrong owner credentials and signed-agent refusal,
strict raw profile/proof input and byte limits, SQL INSERT/clock UPDATE rollback, corrupt
saved state, and continued real MLS delivery. Both peers restart around the original queued
message; its ID/text remain exact, its receipt marks delivery, and the sender outbox drains.
The existing process checkpoint helper was reused; its original owner test also passed.

The separate context-free backend-test-critic initially returned REVISE: forbidden override
fields had invalid types, potentially masking a permissive schema. Revised requests carry a
real state root, valid profile JSON strings and current time within the lease. The live
positive verification succeeds both before and after these rejected overrides. Expiry uses
a bounded monotonic deadline around the actual wall-clock wait. The critic returned FINAL
ACCEPT with no remaining blockers. Repeated Rust and fresh Anvil RED runs failed only on
the missing owner methods, with no production edits and no cleanup errors.

## Implementation

The existing owner actor supplies SystemTime and routes registry_selection,
install_registry_profile and verify_checkpoint_registry through checkpoint_ipc. The adapter
uses strict schemas, existing 32-byte hash parsing and checkpoint error/DTO encoding.
Core remains the only implementation of profile binding, proof validation, durable time and
atomic persistence. No proof request can choose a new issuer/registry/root or override time.
Agent access is refused before dispatch; no unrestricted signing or keeper authority is added.

## Live integration

The new registry_node.py acceptance runner reuses the real daemon Unix IPC helper and the
independently constructed registry/checkpoint corpus. Current contract bytecode is deployed
on each of two disposable Anvil chains. After the fixture's rollback, the actual surviving
canonical active set is sealed and seeded again. Its fresh MPT root must match the current
chain tip. A short-lived independently signed checkpoint binds this root to explicit trust.

Both gapped-index members have exact complete DTO oracles, including distinct ordinals.
Wrong head/key/salt, altered storage, duplicate raw proof keys and otherwise-valid override
fields are refused. After both source and daemon stop, a reopened daemon verifies both
members inside the original lease. Both proofs are refused after actual expiry while offline.
No caller clock or source override is used. The targeted run passed: 64 actual owner calls,
10 successful registry verifications, two chain outcomes and empty cleanup errors.
scripts/check-evm.sh invokes this runner, which retains every previous registry contract,
proof and CLI check; the funding/checkpoint daemon gate remains present independently.

## Aggregate verification

The full native gate finished with exit 0: 311 Rust tests, 24 Solidity tests (256 fuzz runs),
40 frontend tests, TypeScript/production frontend, fresh funding/registry Anvil gates and
all five actual hidden packaged WKWebView flows. Release bundling, deep/strict ad-hoc signing
verification and production WebDriver exclusion passed. Current chat and restored trust
screenshots were viewed beside the prior committed 8efb47f images; layout and text are intact,
with only expected runtime timestamps changing. Notarization and other platforms remain open.

The Linux network gate finished with exit 0, all seven outcomes and empty cleanup errors:
ain-nat-23779610, source hash 25cbafbb274a9a055c5df5f0048d00035d5fde3f6529d582d3ac05b6eedf6404.
The separate read-only AutoNAT trace is retained. The previous increment's unreproduced
withdrawal timeout did not recur; no transport fix is claimed in this owner API increment.
No dependency was installed. This local attestor fixture does not establish public-chain
finality, physical operator independence, live production attestation, paid storage or full V1.
