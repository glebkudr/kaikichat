# L06 explicit trusted checkpoints — authenticated proof composition

Date: 2026-09-05. Full V1 remains incomplete. This increment implements the explicit
trusted-checkpoint option in ADR-05 as a bounded Rust verifier and real CLI. It does not
prove chain-specific finality, preserve a durable high-water mark or authorize spending.
The existing desktop was rebuilt and regression-tested; its balance/admission UI does not
use this utility yet. No public chain, real funds or private user credentials were used.

## Test-first review

`L06-checkpoint-red.log` records unresolved TrustProfile/TrustedCheckpoint imports and the
missing CLI main while both production files contained only comments. Nine integration
tests preceded implementation. The independent Python generator used actual fresh Anvil
state roots, block hashes/numbers and EIP-1186 proofs, then signed canonical envelopes with
cryptography Ed25519 and the reused independent CBOR reference. No Rust checkpoint producer
or homemade cryptographic primitive generated the expected profile/checkpoint hashes.

The separate originally context-free `/root/node_test_critic` returned REVISE for two gaps:

- A wrong issuer codeHash was already rejected by the Ethereum verifier, so it did not
  isolate the checkpoint manifest binding. A foreign-profile successor also had the wrong
  sequence. Correctly signed certificates now pass under separately changed issuerDomain
  and issuerCodeHash profiles, then fail with the unchanged real issuer/proof. A foreign
  successor has the correct sequence, parent and increasing block/time before profile denial.
- Only3-of4 was tested. A5-authority manifest now rejects threshold3, accepts threshold4,
  rejects three valid signatures and accepts four. A fixed three-signature implementation
  cannot satisfy this test.

The CLI wait also gained a10second deadline that terminates only its own child. FINAL ACCEPT
preceded production implementation. The additional live EVM runner/gate composition was
separately accepted. The parent waited for each final critic verdict before further work.

## Observed implementation and trust boundary

`crates/l2-adapter` reuses SignedDocument v1 and the funded-state verifier. A strict selected
manifest pins network, issuer domain/codeHash, authority epoch,4..32 unique nonweak keys,
a >2/3 threshold, and finite lease/root-age/skew bounds. Canonical profile IDs normalize key
order. No built-in manifest, attestor key or privileged RPC exists in production sources.

A certificate must contain a bounded canonical array of distinct authorized valid signatures
over one purpose/profile/body/time statement. Wrong kinds, extensions, mixed statements,
duplicate/unknown/invalid signers, malformed and noncanonical encodings fail. The checkpoint
ID is independent of signer subset/order. Fresh signatures cannot extend root usefulness
beyond its configured age. Expiry is checked again when the certificate object is used for
funding verification. Issuer identity is matched before invoking real Ethereum membership
and exact resource-allocation checks. No success creates a spend token or available balance.

The pure successor check recognizes identical retries and rejects forks, missing parent
links, skipped/decreased sequences, profile switches and non-increasing block height/time.
It cannot detect forgotten history: durable storage, clock/high-water protection and startup
trust-manifest provenance remain required. Quorum signatures authenticate selected attestors'
claims, including false claims if that quorum lies. The certificate does not validate Ethereum
headers/ancestry, OP withdrawal finality, sequencer/governance behavior or attestor independence.

The live fixture deliberately authenticates a historical funded root after a local rollback.
It succeeds only as `funded_at_trusted_checkpoint` under the declared attestor assumption.
At the surviving attested root, absence and substitution of the old proof both fail. This
distinction is intentional: the report says chainFinalityVerified:false, and neither this
result nor an RPC finalized tag is described as trustless or current chain finality.

## Verified evidence

- `L06-checkpoint-green.log`: all9 Rust integration tests pass. Their real CLI cases use
  normal and >u128::MAX funded amounts from independent Anvil vectors, exact IDs/domain/leaf/
  allocation, explicit provenance/expiry, unsigned-RPC denial and duplicate-key rejection.
- `L06-checkpoint-clippy.log`: targeted all-targets Clippy passes without exceptions in
  production. The test-file unwrap allowance follows the existing project convention.
- `L06-checkpoint-native.log`: final exit0,261 Rust tests,14 Solidity tests including256
  fuzz sequences,30 frontend tests, formatting/Clippy/types/build, all previous EVM checks,
  all four actual packaged hidden WKWebView scenarios, release bundle and deep/strict
  ad-hoc signature. A normal release still excludes the automation driver.
- `output/evm-e2e/trusted-checkpoints.json`: passed:true,12 actual new Rust CLI verifications,
  two fresh local Anvil profiles, cleanupErrors:[], trusted_attestors_v1,
  chainFinalityVerified:false, sourceHash
  `417b4006348776582d503deae1b0991817737e2ae38cf099059b3dc3e00cfbd3`.
  The runner preserves all earlier issuance/funding gates, builds the current locked CLI,
  invalidates its report before work and checks unchanged source hashes afterward.
- The preserved funded-state runner passes26 actual CLI verifications on two other fresh
  Anvil profiles; sourceHash `ac9e413fba4330ed0ac18a74ea3352fb515eeda9004e8f5eb08c78b166903e76`.
  Three additional local profiles retain contract issuance/restart/rollback acceptance.
- `L06-checkpoint-mutations.json` and the retained runner/logs: clean9-test baseline, then
  eight isolated compiling mutants caught by real assertions: wrong quorum formula,
  ignoring configured threshold, duplicate signers, stale-root renewal, missing issuer
  code binding, missing issuer domain binding, successor profile switch and expiry at use.
  No compile failure counts as a killed mutant; main sources/targets were never mutated.
- `L06-checkpoint-network.log`: all seven actual Linux network scenarios pass against current
  source; run `ain-nat-7801dec3`, sourceHash
  `05e77b2a15a04f2436f3f50921ad88410d5adb8de4931f9ab57ffe818b170ddf`, cleanupErrors:[].
- The actual restored-native-network screenshot was viewed beside the previous screenshot.
  Layout, saved fields, relay policy and live status remain intact; fixture PeerIDs/ports
  change between runs. No visible browser or focus-stealing application launch was used.

Text logs normalize trailing whitespace only; test output and exit outcomes are preserved.
No external Rust dependency version changed; the new workspace package reuses locked
libraries. The Python Ed25519 oracle uses the existing cryptography46.0.5 installation.

## Traceability and remaining work

L06.T02/N01 now distinguish a selected signature trust profile from agreeing RPC answers and
compose it with actual funding membership. L01.N01 gains an explicit authenticated-profile
boundary. Successor and time tests contribute to L06.T01/F01, but do not yet implement a
durable replay/reorg adapter or E21's end-to-end outage/lease behavior.

Next: atomic durable checkpoint/high-water/clock state, authenticated profile installation,
bounded live source/attestor transport and a documented public test profile; then finite
private admission/consensus, retained ciphertext/indexes, independent R=10 custody and repair.
Existing retrieval must continue under its own valid policy when new admission is blocked.
No L01/L06/E21 or full E01–E26 acceptance is claimed complete by this increment.
