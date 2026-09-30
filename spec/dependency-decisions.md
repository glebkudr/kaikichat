# Dependency decisions

2026-09-26: the replaced storage and payment path was deleted. The entries
"Trusted checkpoint composition", "Public finalizer-key registration", "Local
private-postage proof backend" and "Local postage CLI and in-memory execution"
are history: their commonware, RISC Zero and FinalizerRegistry dependencies are
gone. Of "Ethereum state proofs" the trie crate is gone with the L2 adapter;
Alloy primitives stay: `grant-book` and `mailbox-swarm` use them for keccak and secp256k1 accounts.

2026-09-05: versions checked against the official Cargo registry (`cargo search`) and primary documentation before installation. Rust 1.97.1 is installed; workspace currently targets MSRV 1.88 and Cargo's compatible resolver.

| Dependency | Selected stable version | Purpose |
|---|---|---|
| minicbor | 2.3.0 | Strict application CBOR codec |
| ed25519-dalek | 3.0.0 | Strict Ed25519 signatures |
| sha2 | 0.11.0 | SHA-256 object IDs |
| thiserror | 2.0.20 | Typed errors |
| serde_json | 1.0.151 | Test fixtures; later local JSON APIs |
| hex | 0.4.3 | Test vector decoding |

Cargo.lock records exact transitive versions. No cryptographic primitives are implemented locally. Fixture keys are public RFC examples and never application defaults.


## Store and desktop shell, 2026-09-05

Registry checks before installation selected rusqlite 0.40.2 with bundled-sqlcipher, fs2 0.4.3, getrandom 0.4.3, serde 1.0.229, tempfile 3.27.0, zeroize 1.9.0. SQLCipher builds on macOS arm64; real encrypted DB/WAL tests pass. OS keychain adapter is still pending.

Frontend uses React/React DOM 19.2.8, Tauri API 2.11.1, Vite 8.2.2, TypeScript 7.0.2 and the exact versions in package-lock.json. npm registry metadata was checked before installation. The host's `min-release-age=2` rejects Vitest 5.0.0 and @types/react-dom 19.2.7 as too recent. Selected newest compatible versions allowed by that configured policy: Vitest 4.1.11 (Vite 8 supported) and @types/react-dom 19.2.5. No global npm policy was modified. Official setting: <https://docs.npmjs.com/using-npm/config/#min-release-age>.

Official Node 26.8.1 arm64 archive was downloaded into the isolated build cache and checked against nodejs.org SHASUMS256.txt. `.node-version` pins this runtime; existing system Node remains unchanged. Frontend tests and build use that runtime. Browser component visual verification used the already installed Playwright runtime headlessly; fixture pages are outside the production entry graph and are not network E2E evidence.

## OpenMLS, 2026-09-05

Official crates registry and release notes verified OpenMLS 0.9.0, openmls_traits/openmls_rust_crypto/openmls_basic_credential 0.6.0. Supporting tls_codec 0.5.0 is re-exported by OpenMLS, so no duplicate direct dependency is introduced. MSRV raised to 1.91 as required by these releases; installed Rust 1.97.1 is compatible. No test-utils or crypto/content logging features are enabled. The memory provider's plaintext file-persistence feature is not used; its records are serialized into bounded secret buffers for the existing SQLCipher transaction boundary.

Primary sources: <https://book.openmls.tech/releases/0.9.0.html>, <https://docs.rs/openmls_rust_crypto/0.6.0/openmls_rust_crypto/struct.OpenMlsRustCrypto.html>, downloaded cargo registry source for these exact versions.

## Local EVM issuance, 2026-09-05

Official release metadata was checked before downloading Foundry1.8.1 and Solidity0.8.36.
The local macOS arm64 toolchain lives in the APFS build cache; system versions are unchanged.
The downloaded assets matched their GitHub release API SHA256 digests:

- Foundry `foundry_v1.8.1_darwin_arm64.tar.gz`: `8a2d1bb1ac76530c973f1677a7276dad66f47663f485a899bab5cc4d837f5ba4`.
- Solidity `solc-macos`0.8.36: `d4abcf0b3e24b7948ddfd64c374d26c3214648717777790ecb936979054a129d`.

This is checksum verification over authenticated downloads, not a claim of Sigstore verification.
Compiler0.8.36 and Cancun bytecode are pinned in `contracts/foundry.toml`. No forge-std,
wallet SDK or other Solidity package is required. The small test Vm interface exposes only
cheatcodes used to set up actors/time and inspect actual EVM outcomes. Python's standard
library drives isolated loopback Anvil JSON-RPC; no public endpoint or funded wallet is used.

Foundry1.8 makes dynamic test linking a default. Mutation testing here proved that its
`new` -> `vm.deployCode` rewrite can end a multiple-constructor `expectRevert` test at its
first case. `dynamic_test_linking=false` preserves actual CREATE call depth. The retained
trace and the previously surviving17-class mutant prove the correction; all6 mutants are
now caught. Anvil explicitly uses the Ethereum execution profile even for the chain-ID84532
fixture, to avoid an automatic Optimism profile selection. This is not an OP/testnet proof.

Primary sources: [Foundry1.8.1](https://github.com/foundry-rs/foundry/releases/tag/v1.8.1),
[Solidity0.8.36](https://github.com/argotorg/solidity/releases/tag/v0.8.36),
[Foundry release defaults](https://github.com/foundry-rs/foundry/releases),
[expectRevert call-depth semantics](https://getfoundry.sh/cheatcodes/expect-revert).


## Ethereum state proofs, 2026-09-05

The official Cargo registry (`cargo search` / `cargo info`) and exact primary crate sources
were checked before selecting Alloy primitives/sol-types1.7.2, trie0.9.5 and RLP0.3.16.
Alloy MSRV1.85 and RLP MSRV1.71 fit the existing workspace1.91. Primitives enables serde/rlp,
trie enables ethereum; the CLI enables the existing serde_json raw_value feature to preserve
duplicate keys until strict parsing. MPT verification, RLP, Keccak and standard Solidity ABI
come from these libraries; the project only implements its bounded issuer/allocation policy.

The lockfile adds92 package versions, including optional dependency families; no previously
locked package version changes or disappears. This does not imply all92 additions execute
in the runtime. Builds and test installations used the isolated APFS workspace.

Anvil1.8.1's mempool cost calculation saturates transaction value into u128 before adding
gas cost. A valid test payment above u128::MAX therefore overflows the pool calculation with
nonzero gas, despite sufficient U256 account balance. Only the wide-value local fixture mines
a zero-base-fee block and submits gasPrice0; actual EVM balance checks remain enabled. The
ordinary fixture pays gas, and both assert exact balances/receipts. This is a test harness
adaptation, not a change to contract payment bounds or a public-chain assumption.

Primary sources: [Alloy primitives1.7.2](https://docs.rs/alloy-primitives/1.7.2),
[Alloy Solidity ABI1.7.2](https://docs.rs/alloy-sol-types/1.7.2),
[Alloy trie0.9.5](https://docs.rs/crate/alloy-trie/0.9.5),
[EIP-1186](https://eips.ethereum.org/EIPS/eip-1186),
[Solidity storage layout](https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html),
[Anvil1.8.1 pool validation source](https://github.com/foundry-rs/foundry/blob/v1.8.1/crates/anvil/src/eth/backend/mem/mod.rs).


## Trusted checkpoint composition, 2026-09-05

No external Rust package version was added or changed. The new workspace crate reuses the
locked agentic-protocol SignedDocument verifier, Ed25519-dalek3.0.0, SHA-256/minicbor, Alloy
and agentic-l2; a read-only issuer codeHash getter avoids duplicating issuer JSON/ABI parsing.
There is no new signature, hash or Merkle primitive and no RPC-dependent authority source.

The independent Python fixtures/live EVM gate reuse the existing cryptography46.0.5 runtime
for Ed25519 and the existing CBOR reference function (loaded without executing its mailbox
fixture generator). No Python package was installed or upgraded. This package is now also
required for the aggregate EVM gate; the previous standard-library-only description applies
to the issuance runner. Test signing seeds are public fixtures and never product defaults.
The new lockfile entry is only agentic-l2-adapter and references existing locked packages.

## Public finalizer-key registration, 2026-09-06

OpenZeppelin Contracts5.7.0 is the latest stable release reported by the official GitHub API
on this date (released2026-07-29; commit cab19933c33c2ad1d4c7a84864a3601dddfd16f3).
The unchanged P256.sol import closure and MIT license are vendored under
contracts/vendor/openzeppelin-contracts with exact original URLs/SHA256 in provenance.json.
The six retained files total90906 bytes. Solidity0.8.36/Cancun compiles them successfully.
FinalizerRegistry calls verifySolidity explicitly; it needs standard modexp, without using
chain-specific code or a P-256 precompile at0x100. The vendored code has not been reformatted
or patched. No external Rust, npm or Python version changes were needed.

Commonware2026.9.0 already includes the attributable P-256 Simplex scheme. A separate real
upstream signer/verifier test matches an independent Python cryptography46.0.5/OpenSSL
registration vector, including the exact RFC6979/low-s signature. The P-256 registration
namespace uses existing Commonware union_unique encoding. This is a compatibility result;
the application's existing Ed25519 runtime remains separate until its P-256 integration.
Full design and consequences: spec/registry/finalizer-keys-v1.md.

Sources: [OpenZeppelin5.7.0](https://github.com/OpenZeppelin/openzeppelin-contracts/releases/tag/v5.7.0),
[pinned P256 verifier](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/cab19933c33c2ad1d4c7a84864a3601dddfd16f3/contracts/utils/cryptography/P256.sol),
[Commonware P-256 scheme](https://docs.rs/crate/commonware-consensus/2026.9.0/source/src/simplex/scheme/secp256r1.rs).

## Local private-postage proof backend, 2026-09-07

RISC Zero zkVM/build3.0.6 are the latest stable versions in the primary crates.io
sparse index; prerelease SDKs are excluded. Pin both exactly and compile the existing
postage relation with upstream Rust r0.1.97.0. Use a recursive Succinct STARK receipt,
an explicit in-process LocalProver, and compile-time disable-dev-mode. Default SDK
features, Bonsai and the Groth16 wrapper are disabled. The transitive Groth16 crate
is part of the upstream proving dependency graph; this application neither requests
a Groth16 proof nor invokes its setup/container service.

The verifier compiles its own guest identity, never taking that identity from the
sender. A different program with the same journal must fail. A real same-program
Composite receipt must also fail: the upstream security model says raw execution
receipts expose execution length. A recursive receipt avoids that particular leak,
but does not establish privacy against small anonymity sets, timing or public-context
fingerprints. Upstream also states that its zero-knowledge analysis is not a complete
formal mathematical argument. Guest correctness, authenticated checkpoint provenance,
application anonymity and global one-use spending remain our responsibilities.
No external cryptographic audit or completed anonymity theorem is claimed.

The host and both nested guest lockfiles are retained; project Cargo configuration
forces locked nested builds. Tooling is isolated in RISC0_HOME on the existing
external build volume. The installed manager is rzup0.5.2: docs.rs search initially
reported stale0.5.1, then the current primary index was checked and the project-local
manager updated. General Rustup and other projects' tools were not changed. Apple
Metal17B54 was installed using xcodebuild's compatible component selection for
Xcode26.1.1. Upstream RV32IM4.0.5 selects CPU proving on this platform; recursion
build materials are downloaded with their upstream SHA256 check.

This is a backend implementation decision within P02, not P02 acceptance. The real
proof gate, measurements and independent reviewer outcomes are recorded separately.
Full shared-root anonymity, durable proving interruption and Core/daemon spending
integration remain required. Contract: spec/postage-circuit/zkvm-v1.md.

Sources: [zkVM3.0.6](https://github.com/risc0/risc0/releases/tag/v3.0.6),
[guest Rust r0.1.97.0](https://github.com/risc0/rust/releases/tag/r0.1.97.0),
[upstream security model](https://dev.risczero.com/api/security-model),
[receipt options](https://docs.rs/risc0-zkvm/3.0.6/risc0_zkvm/struct.ProverOpts.html),
[rzup primary version index](https://index.crates.io/rz/up/rzup),
[Apple component installation](https://developer.apple.com/documentation/xcode/downloading-and-installing-additional-xcode-components).

## Local postage CLI and in-memory execution, 2026-09-07

The CLI reuses the existing locked workspace serde and zeroize dependencies; no
package versions or guest dependencies change. Replace LocalProver's default
filesystem-backed executor with the SDK3.0.6 public `run_with_callback` and
`SimpleSegmentRef`, then pass that session to the same local `ProverServer`.
The own compiled image, Succinct policy, verifier, guest relation, hard cycle limit
and disable-dev-mode remain unchanged. `RISC0_PPROF_OUT` is rejected because the SDK
otherwise imports it and writes a profile. This removes explicit private segment
files; it does not claim control of OS swap/core dumps or complete wallet recovery.
The exact tagged executor/session/prover source downloads match the installed SDK.
Contract and primary source links: spec/postage-circuit/cli-v1.md.
