# L02 — durable operator keys and daemon role signing

Status: implementation, focused checks and both aggregate gates pass. This increment does not close V1 or L02/N05/D03.

## Contract and test review

The owner prepares idempotent public registration intents for multiple units, keeps independent operator seeds in SQLCipher, and signs a role only after the stored opening authenticates under the selected current checkpoint. Daemon uses its own persistent Ed25519 transport key. Contract: `spec/registry/operator-identity-v1.md`.

Tests were written first. RED: `L02-operator-identity-red.log`, missing Core APIs. The existing separate `/root/node_test_critic`, created without forked context, reviewed only explicitly supplied new tests/contract and returned **FINAL ACCEPT**, no blockers. No work was performed while awaiting its verdict. It accepted real rollback/restart/idempotency/member/time/owner boundary assertions and independently verified live Anvil bonds, signatures, wire and PeerID. Optional additions suggested: assert seeds never appear in any public value (including salt), and directly test preparation-owner conflict advancing time. They are not required for acceptance; implementation uses separate CSPRNG seed/salt and the same denial/clock transaction covered by signing and existing Core tests.

## Implementation

- The registry ABI commitment function is shared by verification and registration; exact independently generated Anvil openings cover both members/two chains.
- `crates/core/src/operator_identity.rs` adds bounded, strictly validated encrypted operator state. Private entries zeroize seeds. Existing Store StateValue/StateChange already zeroize serialized bytes. Operator units are not root identities or transport identities.
- Creation and observed time share one CAS transaction. Same operation/owner returns the exact original registration; a conflicting owner is refused. Independent operations get separate seeds and salts. No checkpoint head is required before bonding.
- Role signing reuses current registry membership verification with the stored opening. A successful durable clock write precedes any wire result. Lifetime is at most60 seconds and bounded by snapshot admission, with effective acceptance also capped by the checkpoint lease.
- Owner IPC supplies system time and the actual daemon transport key. Signed agents and caller overrides are refused. No new packages, private-key import, deterministic production seed, new trust policy or transaction submission API was added.

## Focused evidence

- Core registry target:14 tests pass (six new operator tests), `L02-operator-identity-core.log`.
- Actual daemon process target:two new tests pass, `L02-operator-identity-node.log`; queued MLS survives operator registration/restart and corrupt operator state.
- Clippy all targets for touched crates passes, `L02-operator-identity-clippy.log`.
- Four isolated **compiling** mutations are killed after a green14-test baseline: repeated seed, ignored retry owner, unchecked stored opening, excessive binding lifetime. See `L02-operator-identity-mutations.json` and mutation logs. Scratch cleanup errors empty; authoritative source never mutated.

## Remaining boundary

Public commitments still require an owner-funded bond transaction; the local gate uses actual Anvil accounts. A produced transport binding is not a remote-handshake verification, placement, independent custody, spend admission or evidence of deployed chain finality. Native registry controls, remote operator discovery/verification, committed-before-beacon selection, actual R=10 custody/autonomous repair and all other V1 acceptance remain open.

## Completed aggregate acceptance

- `L02-operator-identity-native.log`: exit0,326 Rust tests,24 Solidity tests (including256-run fuzz cases),40 frontend tests, TypeScript/Vite, five actual packaged hidden WKWebView flows, macOS release bundle, deep strict ad-hoc codesign verification and no webdriver plugin in normal release dependencies.
- `L02-operator-identity-anvil.json`: two genuine chains31337/31338, newly generated private daemon keys, actual paid units3/4 in a four-unit snapshot,16 independently verified exact signatures and transport PeerID bindings,72 owner calls, source shutdown/restart within lease and refusal after real expiry. Source hash `a0d80682c654c5fc6d0c8e6e9e2e2678e62150c0e8b4f1f6fb5b4d79171a6df6`; cleanup errors empty. The old32 operator CLI calls/8 successes and64 registry daemon calls/10 successes remain.
- `L02-operator-identity-linux.json` / network log: exit0, seven actual Linux topology outcomes, run `ain-nat-81a53b9a`, source hash `f47cc666c173b37ba35048f4adc56103a46129074b3b36f383a02ccc3f50165d`, cleanup errors empty.
- Own visual inspection displayed current `output/native-e2e/alice-chat.png` and `checkpoint-restored.png` alongside the corresponding preceding84f917f files under `/Users/glebk/Library/Caches/agentic-internet/recovery/native-reference-84f917f`. Layout, text flow and controls agree; only expected timestamps differ. Other three native flow assertions also passed.
- No packages installed, no failed acceptance hidden by reruns, no source changes during aggregate gates. Initial compile ownership error was corrected before the first focused GREEN and aggregate runs. Local release remains ad-hoc signed and not notarized; platform-wide V1 release is not claimed.
