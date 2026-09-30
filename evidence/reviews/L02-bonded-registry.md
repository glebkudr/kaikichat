# L02 bonded registry contract and local EVM evidence

This increment implements `contracts/src/NodeRegistry.sol`, a prerequisite for authenticated
operator selection and independent custody. It does not accept the complete L02/N05/D03 cards
or the full V1 goal. There is no public deployment or economic parameter recommendation.

## Test-first review

Production was absent when both Solidity and Anvil tests were written. The initial Solidity
RED is `L02-registry-contract-red.log`; the revised Anvil RED is
`L02-registry-anvil-red.log`, failing during forced compilation of the missing source.

The separate context-free `node_test_critic` first returned FINAL REVISE:

1. A duplicate withdrawal could be denied merely because the contract was empty. The revised
   malicious callback runs while a second real, matured bond remains in the contract. It
   asserts one payment, preserved Bob state/accounting, then Bob's actual full withdrawal.
2. Loading cached bytecode did not bind the Anvil run to current source. Both existing issuer
   and new registry runners now share the issuer's forced-build helper and pre/post source
   fingerprint checks. The registry report records the deployed creation-bytecode hash.
3. Ordinary behavior assertions would miss an additional administrative method. The reused
   strict ABI helper compares the complete signature/mutability list, retaining overloads
   and counts, and rejects receive/fallback. Registry permits exactly 23 functions/getters.

After these revisions and additional ordinary invalid configuration boundaries, the critic
returned FINAL ACCEPT, without required changes. No production work occurred while either
review was running. Existing issuer behavior assertions were preserved by the helper refactor.

## Implemented behavior

One exact fixed-price bond creates an append-only index and one active count. A 32-level
ordered Merkle-sum path authenticates its owner-bound opaque commitment and the full active
count. Exiting deletes the active leaf while old snapshots remain immutable. Commitments
cannot be reused by the same owner, including after withdrawal; an unrelated owner cannot
prevent registration by copying opaque bytes first.

Anyone can seal the current nonempty epoch once. The snapshot copies domain/root/count,
seal time/block, one specified future block and a finite admission deadline. Post-seal joins
and exits cannot change it. Capture uses only that block's nonzero hash inside the EVM's
256-block history window; saved retries preserve the seed and original admission deadline.
Missing epochs and missed captures are not synthesized from caller/time/latest-block data.

The owner can withdraw principal only after exit time plus admission lease plus maximum
obligation duration. Checks and state/accounting precede the recipient callback. Failure
rolls back the whole withdrawal; another owner's claim remains independently usable. There
is no admin, pause, proxy, confiscation, withdrawal of others' bonds or treasury API.
The immutable deployment domain binds chain ID, registry address, genesis and all numeric
parameters. Mutations on a different chain ID fail.

The wire/hash/configuration contract is `spec/registry/bonded-snapshots-v1.md`.

## Direct verification

- Ten Solidity tests pass, including 256 randomized 48-operation sequences with independent
  full-array root/count reconstruction and multi-owner bond conservation at every step.
- `tests/evm/registry.py` force-compiles and deploys actual bytecode on disposable Anvil chains
  31337 and 31338. It verifies exact ABI, config/domain/getters, bond event topics/data,
  actual gas-adjusted balances, immutable snapshots, independently reconstructed membership
  paths, a genuine state rollback, process/disk restart, early withdrawal denial, exact
  maturity, duplicate withdrawal denial and permanent owner/commitment non-reuse.
- Each chain and mutation workspace is cleaned. The report keeps `localOnly=true`,
  `chainFinalityVerified=false` and `operatorIndependenceVerified=false`.
- The existing aggregate `scripts/check-evm.sh` now runs registry acceptance after all prior
  issuer, funding-proof, trusted-checkpoint and owner-daemon tests.

Evidence: `L02-registry-contract-green.log`, `L02-registry-anvil-green.log`,
`output/evm-e2e/registry.json`. Foundry 1.8.1 and Solidity 0.8.36 remain pinned; no dependency
was installed. `dynamic_test_linking=false` keeps constructor failures at actual CREATE depth.

## Negative controls

`L02-registry-mutations.py` copies the reviewed current source/tests to a disposable APFS
workspace, first proves the unmodified baseline, and requires actual test failure after
successful compilation for each mutation. All seven mutations were killed:

- registration without exact payment;
- withdrawal before the obligation horizon;
- recipient callback before marking the claim withdrawn;
- rewriting the current epoch;
- omitting the membership root from the seed;
- counting only the left subtree;
- adding a confiscation function.

The confiscation mutation deliberately passes all ten ordinary contract tests, then fails
the independent exact ABI gate. This demonstrates why that review requirement matters.
All mutation details/logs and empty cleanup errors are retained in
`L02-registry-mutations.json`. Production source was never mutated by this runner.

## Remaining integration

The complete native gate terminated with exit 0: 288 Rust tests, 24 Solidity tests (256 runs
for each fuzz test), 40 frontend tests, all real Anvil/CLI/daemon checks, five packaged hidden
WKWebView flows, release bundling, strict/deep ad-hoc codesign and driver exclusion. Evidence:
`L02-registry-native-green.log` and `output/native-e2e/result.json`. Current chat and restored
checkpoint screenshots were visually compared alongside their prior committed references;
layout, clipping, text and state presentation remain consistent. No UI source was changed.

The registry source fingerprint is
`0ff6bd7b7e6685174ba613384c616f5b8f455cf5d3eaf8e8cf4b0e0204a94c2f`.
The Linux network inputs were evaluated with the existing gate's exact sourceHash function:
`bf6b526469681740190503978ea3907c15ecbd04abf0e04452eff000c283515e`, identical to the passing
`ain-nat-0f2c57b4` run (seven outcomes, empty cleanup errors). No network source changed and
that unchanged gate was not rerun for the registry increment.

Rust must authenticate registry account/storage and membership proofs against the selected
checkpoint, pin the exact deployment/code/config, verify commitment openings/key possession,
and enforce admission/obligation bounds. A bonded unit proves committed capital, not endpoint
availability, physical independence or an honest keeper. Fixed units eliminate a nonlinear
per-key weight bonus; they do not prove resistance to all assignment grinding or Sybil attacks.
The explicit EVM blockhash profile remains producer-biasable and withholdable, as documented
in the spec. Actual placement, spend authorization, R=10 custody and sender-independent repair
remain required. No replica count or spend authority was added to the UI by this increment.
