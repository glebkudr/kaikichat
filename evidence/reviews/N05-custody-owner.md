# N05/D03 — durable owner custody preparation and funded assignments

Status: Core/owner IPC implemented; focused and full backend/frontend/EVM/native/Linux gates pass. Full V1 remains open.

## Test-first and separate review

Contract: `spec/registry/custody-owner-v1.md`. New seal/Core/daemon tests and the independent actual two-chain runner preceded production. Retained RED logs fail on absent APIs. The existing no-context-fork node test critic returned REVISE for a late request whose refusal could be masked by an old head, and missing exact retry after preparation lease expiry. The corrected test first validates the real current seeded proof, then refuses fresh preparation under that actual head. Exact retry after preparation expiry and reopen retains the same intent and rows. A semantically equal reformatted preparation proof now requires the exact raw-digest idempotency conflict; unauthorized owner responses expose no result. Separate FINAL ACCEPT arrived before production. A second narrow FINAL ACCEPT approved only the test-helper dead-code annotation and adding the already reviewed live runner after all prior EVM gates. No production work occurred while either verdict was pending.

## Implementation

The existing bounded account/storage verifier and registry parser are shared by separate pre-beacon seal and seeded snapshot capabilities. A seal must have zero captured seed and satisfy sealedBlock <= actualHead < beaconBlock; at-beacon preparation is too late even with zero seed. It cannot authorize selection.

Core generates independent Ed25519 intent seeds and salts after verifying the seal, stores strict bounded private metadata in SQLCipher, and atomically commits the intent together with observed checkpoint time before returning public metadata. Read-only metadata and exact original retries survive head changes and lease expiry. Conflicting retries cannot rotate keys. Shared identifier/hex helpers are reused with operator identity code.

Funding binds only a known private intent and is proved through the daemon's existing authenticated durable checkpoint archive. Its funding anchor, commitment and bounded proof persist atomically. Semantic funding retries preserve original proof bytes/revision and reauthenticate both supplied and stored evidence. Queries always use the actual selected head, current registry proof, stored funding and monotonic clock. The existing adapter produces the assignment; its serialize-only display DTO is shared with the CLI. It cannot be deserialized into authority.

Four strict owner-only IPC methods expose preparation metadata, preparation, funding binding and assignment. Agents cannot supply owner keys, time, history, root or selected profiles. Existing scoped message delivery remains available. No dependencies, Solidity contracts or trust defaults changed.

## Focused results

- `N05-custody-owner-adapter.log`: 3 new seal tests and all8 previous funded-custody tests pass on independent actual fixtures.
- `N05-custody-owner-core.log`: all25 registry tests pass, including6 new custody cases for fresh keys, exact retry/restore, resource limits, real SQL faults, clock rollback, own funding, current proof, archive loss and corruption.
- `N05-custody-owner-node.log`: the real owner/agent daemon scenario passes with a valid scoped-message positive control and actual restart/receipt preservation.
- `N05-custody-owner-clippy.log`: targeted all-targets Clippy with warnings denied passes after the limited helper annotation and archive-method rename.

## Remaining product requirements

This persists private request intent and authenticated funding/placement metadata. It does not reserve or spend a ticket, discover selected custodians by ordinal, upload/retain ciphertext, issue custody receipts or restore ten independent copies. Required historical evidence can become unavailable under current archive pruning; that path fails closed and needs a retention lifecycle before full admission/repair acceptance. User-facing registry/payment controls, full custody, jobs, groups and other mandatory E01–E26 requirements remain open. No real-money deployment occurred.

## Actual owner IPC/EVM result

`N05-custody-owner-live.log` exits0. `output/evm-e2e/custody-owner.json` records28 exact
assignments through124 actual owner calls on chains31337 and31338. Fresh daemon-generated
keys were committed and paid before beacon. The independent Python CBOR/full-list oracle
checks all output fields. Both source chains stop; the same daemon state is reopened after
historical funding certificate expiry and continues to verify until actual current expiry.
Expired assignments refuse before a status read can observe time on their behalf. Cleanup
errors are empty. The before/after source hash is
`da85703a56e74e55530cd5e2f6df143ec46439ab1eb3fbda6240fb98ba73e581`.

## Linux network regression

`N05-custody-owner-network.log` exits0. All7 actual Linux outcomes pass with run
`ain-nat-e00ad5e7`, source hash
`35c7ffcaa60ff86ffb50c08794f4c93058377d306337f4b7d3e45226f41a4de8` and empty cleanupErrors. The freshly hash-locked daemon
preserves LAN discovery, direct MLS/receipts, NAT/relay failover, direct QUIC after DCUtR
and AutoNAT firewall/service behavior. Only run-labelled resources were removed.

## Packaged native and aggregate acceptance

`N05-custody-owner-native.log` exits0. All351 Rust,24 Solidity,7 Python model and40 frontend
tests pass, with formatting, Clippy, TypeScript and production frontend build. All earlier
actual EVM gates remain, including28 old funded CLI checks; the new custody daemon gate
repeats28 exact assignments and124 calls with the same source hash and no cleanup errors.
All5 real hidden packaged WKWebView flows pass. The ordinary macOS arm64 release bundle
builds, passes strict deep codesign verification, and excludes the automation plugin from
its default dependency graph. It is ad-hoc signed and not notarized.

The new `alice-chat.png` and `checkpoint-restored.png` were displayed together with their
67cbb56 references saved before the run under
`/Users/glebk/Library/Caches/agentic-internet/recovery/native-reference-67cbb56`.
Own visual inspection confirms matching layout, text wrapping and control positions;
only expected message/checkpoint times differ. Current screenshots and native result are
retained in `output/native-e2e`. These five flows do not substitute for full E01–E26.
