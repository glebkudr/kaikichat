# L01 funded state proof — Rust verifier checkpoint

Date: 2026-09-05. This increment verifies funded EVM storage at a supplied state root.
It does not authenticate finality, issue private spend authority, populate available balance
or complete L01/E01–E26. The complete V1 goal remains active.

## Tests first and independent critique

`L01-proof-rust-red.log` records missing public APIs while production was comment-only.
Ten Rust integration tests use independent actual Anvil `eth_getProof` fixtures. The separate,
originally context-free `/root/node_test_critic` returned REVISE before implementation:

- Changing RPC values without replacing Merkle proofs only exercises membership rejection;
  it cannot prove that the allocation policy checks payment/count/domain independently.
- An incomplete duplicate-key JSON example could fail for missing fields instead of duplicates.

The fixture generator now injects eight deliberately invalid local storage states, mines
actual blocks and obtains valid proofs under their new roots. It recomputes unrelated fields
consistently to isolate wrong payment, zero/excess count, zero/unknown class, reserved bits,
wrong leaf and wrong domain. These are explicitly tagged test-injected states, not states
the contract can produce. Rust checks specific Allocation/Domain errors, so a failed Merkle
proof cannot conceal an absent semantic check. Complete valid JSON with a duplicate field
and correct last value independently demonstrates generic-parser collapse; strict parsing
must reject it. The actual CLI also rejects duplicate nested profile/proof fields.

FINAL ACCEPT preceded production implementation. Subsequent narrow changes were separately
accepted: a test-file-only unwrap lint allowance, comparing CLI domain/leaf with Anvil eth_call,
and changing an indexed node loop to enumerate without changing its attacks. The parent waited
for each final verdict before continuing. `L01-proof-native-green.log` is the earlier failed
Clippy attempt; `L01-proof-native-final.log` supersedes it.

## Verified behavior

`agentic-l2` uses Alloy Ethereum account/storage trie proofs, RLP, Keccak and Solidity ABI.
The selected profile pins chain/genesis/issuer/runtime codeHash and the immutable resource
configuration. Four unique storage keys must prove exactly one requested commitment under
the authenticated account storage root; proofs may arrive in any order. Nonzero words use
RLP values; zero words require exclusion. No RPC account field or storage value is trusted
without its proof. Absence yields no funded record.

Packed storage and ABI hashes match independent compiler-layout/getStorageAt/eth_call
oracles. Full-width U256 payment, class/count/expiry bounds, reserved bits and leaf binding
are checked after membership. Bounds on JSON size, node count, node size and total proof
bytes limit untrusted work. Strict schemas reject duplicate/unknown fields. The private-field
result has no Deserialize constructor and carries no finality or spend authority.

The CLI accepts one bounded JSON request on stdin and returns `funded_at_supplied_root`.
It preserves nested raw JSON until strict parsing. Failure exits2, writes only an error to
stderr and leaves stdout empty. It performs no network requests or persistent writes.
The caller must separately authenticate the root, selected profile and time/lease semantics.
An old proof can remain historically valid after a reorg; it cannot prove membership at the
surviving root. The API does not interpret an RPC `finalized` tag as finality evidence.

## Validation

Retained text logs normalize trailing whitespace only; exit outcomes and test output are preserved.

- `L01-proof-native-final.log`: exit0;252 Rust tests,14 Solidity tests including256 fuzz
  sequences,30 frontend tests, formatting/Clippy/types/build, four actual packaged hidden
  WKWebView scenarios, normal release bundle and deep/strict ad-hoc signature.
- Existing issuance scenarios still run on three fresh local Anvil profiles. The new runner
  adds two fresh proof profiles and26 actual calls to the freshly built locked Rust CLI.
  It covers normal and >u128::MAX payments, absent/rolled-back records, wrong roots/genesis,
  malformed proof fields, duplicate raw JSON and eight valid-proof invalid-state cases.
  `output/evm-e2e/funding-proof.json`: passed:true, localOnly:true, finalityVerified:false,
  cleanupErrors:[], sourceHash `a8766fea644a7a837b89be145b32249d9982aab58bde266b7e9b9ca92b601688`.
- `L01-proof-mutations.json`, its runner and seven individual logs retain a clean10-test
  baseline followed by six isolated compiling mutants. Removing account membership,
  storage membership, exact payment, domain binding, expiry or duplicate-key preservation
  causes real assertion failures. No compile failure counts as a killed mutant, and no
  production file or live gate target is modified by the isolated copies.
- `L01-proof-network-final.log`: exit0, all seven actual Linux network scenarios passed
  against final sources. Run `ain-nat-5bbeddaf`, sourceHash
  `de787c11c53dbb8f9e2393fba7c44abc65fb53ab559cf762f4e23e3a62d6711f`, cleanupErrors:[].
  The previous successful run preceded the final test-only lint change; this final run
  refreshes the full source fingerprint without weakening network assertions.
- Actual native restored-network screenshot was viewed beside the previous screenshot.
  Layout, saved fields, relay status and reachable peer counts remain correct; disposable
  port/PeerID values differ as expected. The app's payment UI is not implemented by this CLI.

The wide-value fixture uses zero base fee and gasPrice0 to avoid Anvil1.8.1's u128 mempool
cost overflow, with actual EVM balance checks still enabled. The normal profile pays gas;
both independently verify balances and receipts. This is documented in dependency decisions.
No public chain, real wallet, real funds or third-party credentials were used.

## Traceability and next integration

L01.T01 now has an off-chain exact allocation check; L01.N01 has independent chain/genesis/
issuer/code binding and rejects unproven RPC state. L01.F01 has actual rollback plus proofs
that fail against surviving history, but still lacks authenticated finality. L01.T02/E01
public testnet and desktop purchase/consumption remain open. L02/L06 must authenticate the
root under an explicit chain-specific finality or honest trusted-checkpoint profile before
P02/P04 may authorize finite private tickets. Multiple agreeing RPCs alone cannot supply that.

The new crate is not linked into custody admission or the desktop balance. Registry, private
proofs, spent-nullifier consensus, retained ciphertext/indexes, independent R=10 custody and
autonomous repair remain required. No upstream task is marked complete for this increment.
