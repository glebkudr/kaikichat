# N05 — paid pre-beacon custody selection

Status: bounded adapter and CLI implemented; focused, mutation and aggregate native/EVM/Linux gates all pass. N05/D03 and full V1 remain open.

## Test-first contract and independent review

Contract: `spec/registry/funded-custody-selection-v1.md`. Production was absent when tests were written. The retained RED log fails for missing intended APIs. The fixture's first generation failed because a mapping slot sometimes had an odd number of hex digits; both existing EIP-1186 helpers now request canonical32-byte slot keys. The genuine two-chain fixture generation then passed; its report explicitly stated fixturesOnly with zero actual CLI checks.

The independent existing `/root/node_test_critic`, originally spawned without forked context, first returned FINAL REVISE: historical signatures needed direct corruption controls, limit failures could be masked by malformed content, and the live report/build hash needed invalidation before work. Tests were revised before production: first/intermediate signature-only corruption retains raw signed bodies/IDs/links; limits require the exact Limit error; actual JSON padding has a valid512KiB control and512KiB+1 refusal; and report invalidation and before/after source hashes bracket the run/build. Separate valid chain/genesis mismatch profiles also reject commitment creation. The critic returned FINAL ACCEPT. No production work occurred while waiting for either verdict. A positive large-population memory case remains a nonblocking coverage gap; production allocates by the authenticated class's maximum96 selections, never by registry population.

## Implementation

The existing funding verifier retains its already validated immutable resource class; the existing snapshot retains its proven seal/beacon block numbers. New getters do not change old CLI responses. No contract, dependency or trust profile is introduced.

The checkpoint adapter authenticates every bounded historical certificate and strict successor link through its exact currently selected head. It verifies funding against the first historical root, the current seeded full registry against the current root, the exact public commitment opening and the strict fundingBlock < beaconBlock boundary. An expired historical certificate can establish a historical root, but cannot renew current authority. Funded class, expiry and current snapshot determine all resource bounds.

Canonical domain-separated hashes determine the assignment and each draw. Sparse swap-remove Fisher–Yates sampling uses rejection to avoid modulo bias and bounded memory/work. First positions are primary custodians, then ordered replacements capped by the paid allowance and remaining population. Each selected member must prove its actual ordinal in the complete gapped-index Merkle-sum set, its opening, matching expected head and current finite time. The result has private fields and is not deserializable from peer JSON.

The two actual CLI commands preserve strict raw nested JSON, unknown/duplicate field rejection and existing failure/size contracts. `scripts/check-evm.sh` adds the new real gate after all existing gates.

## Focused evidence

- `N05-funded-custody-focused.log`:all8 new Rust integration tests pass, including exact real CLI DTOs.
- Independent Python canonical CBOR and literal full-list sampling agree with the complete Rust selection on two real16-unit registries with an exited index gap. Primary R=10/replacements4 and a fully selected16-unit population are checked across three paid ticket indices.
- Actual paid-at-beacon/paid-after-beacon, unpurchased intent, changed opening/ticket, incompatible class/population/retention, damaged historical signatures, forks/gaps/foreign roots, selected member mismatch, independent finite expiries and exact CLI limits all reject with positive controls.
- `N05-funded-custody-clippy.log`:whole workspace/all-targets Clippy passes.

## Acceptance boundary

This is authenticated placement planning only. It does not reserve/consume a ticket, prove owner-key possession, publish ciphertext, retain an index, discover selected operators by ordinal, prove custody or execute repair. Future Core integration must durably own intent/idempotency, the actual current head and monotonic clock, then bind selected members to verified live operator roles. Global spend authorization and actual independent R=10 with autonomous repair remain required. Full E01–E26 and other V1 acceptance are not closed. No real-money contracts were deployed.


## Independent fault controls and Linux result

`N05-funded-custody-mutations.py` copies only the three required crates into an isolated
APFS workspace. Its baseline passes all8 tests. Four separately compiling mutations fail
their exact reviewed business test: permitting beacon equality, accepting any selected
position, skipping successor links and ignoring sparse ordinal swaps. Authoritative source
is hash-checked unchanged, scratch removed and cleanup errors empty. Logs and machine result
are retained alongside the runner.

`N05-funded-custody-network.log` exits0. All7 Linux outcomes pass with run
`ain-nat-e6ed05e9`, source hash
`6d4f7b3285402387a82100a44dcb5265b60f0681d78412c8e788fb21af98d2a9` and
cleanupErrors empty. The hash-locked image includes the new Rust sources. Existing real
NAT/relay, DCUtR, AutoNAT, bootstrap/LAN and MLS/receipt behavior remains verified.


## Actual CLI/EVM result

`N05-funded-custody-anvil.json` passes all28 actual CLI checks across chains31337/31338,
with both source chains stopped before verification and actual wall-clock checkpoint
expiry refusal. All earlier issuer/registry/checkpoint/operator network gates remain.
The current run's source hash is
`0a306e6f5babbd90a80a3f9ce6ccc4b7fd7e0fc0e1a0c6d21f018c44d3b80c9e`; the Rust verifier
hash, checked before/after build and after scenarios, is
`7f9f0d50e623f1014a7833d08bbe77d48ee9875d107e51b25fe53b74caa88da0`.
Cleanup errors are empty. The report starts failed before work, so this live success cannot
be confused with the earlier fixture-generation-only report.

The aggregate run has also passed341 Rust tests,24 Solidity tests,7 Python model tests,
40 frontend tests, TypeScript and production Vite build. All5 native flows and the release/signature gates also pass.


## Native UI inspection

All5 actual packaged hidden WKWebView flows passed. Current chat and restored-checkpoint
screenshots were displayed together with their corresponding611afec references saved under
`/Users/glebk/Library/Caches/agentic-internet/recovery/native-reference-611afec` before the
run. Own visual inspection confirms matching control positions, wrapping and layout, with
only expected message/lease timestamps changed. The full native gate exited0, rebuilt the
macOS arm64 release bundle, passed deep strict ad-hoc signature verification and confirmed
that the default release dependency graph contains no WebDriver plugin. The bundle remains
local and unnotarized; this module does not complete V1.
