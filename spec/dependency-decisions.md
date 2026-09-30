# Dependency decisions

How the direct dependencies were chosen. Versions are pinned in `Cargo.toml`,
`Cargo.lock`, `apps/desktop/package.json` and `contracts/foundry.toml`.

2026-09-26: the replaced storage and payment path was deleted with its
dependencies: commonware, RISC Zero, the FinalizerRegistry, `alloy-sol-types`
and `alloy-trie`; the OpenZeppelin P-256 verifier vendored for the
FinalizerRegistry followed on 2026-09-30. Their
selection notes (trusted checkpoints, finalizer-key registration, the private
postage prover and its CLI) remain in
[the first commit](https://github.com/glebkudr/kaikichat/blob/7563f614931f26e7dd1148a5c5053bb1e1537847/spec/dependency-decisions.md).
Other choices are noted where they are used: `rmcp` in
[agent-grants-v1.md](agent-grants-v1.md#agentic-mcp), `keyring` in
[desktop-host-v1.md](desktop-host-v1.md#secrets), the patched `libp2p-kad`
in [vendor/libp2p-kad/README-PATCH.md](../vendor/libp2p-kad/README-PATCH.md).

2026-09-05: versions checked against the official Cargo registry (`cargo search`) and primary documentation before installation. Rust 1.97.1 is installed; the workspace then targeted MSRV 1.88 (1.91 since OpenMLS, below) and Cargo's compatible resolver.

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

Registry checks before installation selected rusqlite 0.40.2 with bundled-sqlcipher, fs2 0.4.3, getrandom 0.4.3, serde 1.0.229, tempfile 3.27.0, zeroize 1.9.0. SQLCipher builds on macOS arm64; real encrypted DB/WAL tests pass. The OS keychain adapter came later (`keyring` 4.2.0, [desktop-host-v1.md](desktop-host-v1.md#secrets)).

Frontend uses React/React DOM 19.2.8, Tauri API 2.11.1, Vite 8.2.2, TypeScript 7.0.2 and the exact versions in package-lock.json. npm registry metadata was checked before installation. The host's `min-release-age=2` rejects Vitest 5.0.0 and @types/react-dom 19.2.7 as too recent. Selected newest compatible versions allowed by that configured policy: Vitest 4.1.11 (Vite 8 supported) and @types/react-dom 19.2.5. No global npm policy was modified. Official setting: <https://docs.npmjs.com/using-npm/config/#min-release-age>.

Official Node 26.8.1 arm64 archive was downloaded into the isolated build cache and checked against nodejs.org SHASUMS256.txt. `apps/desktop/.node-version` pins this runtime; existing system Node remains unchanged. Frontend tests and build use that runtime. Browser component visual verification used the already installed Playwright runtime headlessly; fixture pages are outside the production entry graph and are not network E2E evidence.

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
cheatcodes used to set up actors/time and inspect actual EVM outcomes (`contracts/test/Vm.sol`).
Native tests start their own loopback Anvil (`crates/node/tests/support/swarm_native.rs`);
no public endpoint or funded wallet is used.

Foundry1.8 makes dynamic test linking a default. Mutation testing here proved that its
`new` -> `vm.deployCode` rewrite can end a multiple-constructor `expectRevert` test at its
first case. `dynamic_test_linking=false` preserves actual CREATE call depth. The retained
trace and the previously surviving17-class mutant prove the correction; all6 mutants are
now caught.

Primary sources: [Foundry1.8.1](https://github.com/foundry-rs/foundry/releases/tag/v1.8.1),
[Solidity0.8.36](https://github.com/argotorg/solidity/releases/tag/v0.8.36),
[Foundry release defaults](https://github.com/foundry-rs/foundry/releases),
[expectRevert call-depth semantics](https://getfoundry.sh/cheatcodes/expect-revert).


## Alloy primitives, 2026-09-05

The official Cargo registry and the crate sources were checked before
selecting `alloy-primitives` 1.7.2 (MSRV 1.85, within the workspace's 1.91).
`grant-book`, `mailbox-swarm` and the node use only its Keccak-256;
secp256k1 signatures and account recovery use `k256` 0.13.4. The contracts' ABI is encoded by hand in `crates/node/src/chain.rs`.

Primary source: [Alloy primitives 1.7.2](https://docs.rs/alloy-primitives/1.7.2).
