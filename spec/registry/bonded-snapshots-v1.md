# Node registry: bonded holder units (V1)

`NodeRegistry` (`contracts/src/NodeRegistry.sol`) records holder units: one
fixed bond per unit, owned by the address that paid it. Mailbox holders are
the nodes whose unit is active ([Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md](../../Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md));
`OperatorPool` pays them ([Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md](../../Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md)).
A bond proves capital committed to a unit, not availability, independence
or honesty; one operator may own many units.

Deployed on Base mainnet (`deployments/base.json`, genesis
`agentic-internet-base-v1`: a bond of 0.0001 ETH, hourly epochs, a
two-hour admission lease, a seven-day obligation, a beacon five blocks
ahead) and on the retired Base Sepolia testnet (`deployments/base-sepolia.json`).

## The unit commitment

A unit's commitment is

`keccak256(keccak256("AIN_UNIT_V1") ‖ domain ‖ transport_key ‖ receipt)`

with the network domain, the holder node's Ed25519 transport key and the
Ethereum account that receives its payouts (`unit_commitment` in
`crates/mailbox-swarm/src/directory.rs`; `OperatorPool` opens it the same
way). A node reports its own commitment in `node_info` and, in the
network's container, in `/data/published.json`; the operator bonds it.

## The contract

- **Configuration** (immutable): a nonzero genesis digest, `unitBondWei` > 0,
  `epochSeconds` 60–86 400, `snapshotLeaseSeconds` from one epoch to 7 days,
  `maxObligationSeconds` up to 366 days, `futureBlockDelay` 1–128. The
  constructor records the start time and chain id; the domain is
  `keccak256(abi.encode(keccak256("AgenticInternet/NodeRegistry/v1"), chainId, address, genesis, configHash))`.
  Every mutation refuses another chain id. No admin, proxy, pause, allow
  list, confiscation or withdrawal by anyone but a unit's owner.
- **`bond(commitment)`** takes exactly one bond, refuses a zero commitment and
  one the same owner already used (even after an exit), and appends the
  unit; indices are never reused (up to 2^32).
- **`requestExit(index)`** (owner only) removes the unit from the active set
  at once and allows withdrawal after `snapshotLeaseSeconds +
  maxObligationSeconds`; an exit cannot be undone. **`withdraw(index,
  recipient)`** (owner only, nonzero recipient) pays the bond once, updating
  state before the transfer; a failed transfer reverts everything. Deposits
  equal held plus withdrawn; ETH forced in creates no entitlement.
- **Reading:** `unit(index)` (owner, commitment, exit times, state `None`,
  `Active`, `Exiting` or `Withdrawn`), `nextIndex()`, and
  `activeUnits(start, limit)`: the commitments of active units among
  positions `[start, start + limit)` and where the next page starts.

## What the node reads

A node with `--registry` reads, at a block `--chain-confirmations` deep, the
active commitments page by page (256 positions a call, at most 65 536
positions) and, for payouts, `unit(index)` and `nextIndex()`. A unit
record (transport key, receipt account, addresses, signed by the transport
key) is taken into the holder directory only when its commitment is active;
access and admission for units follow [access-by-book-v1.md](../access-by-book-v1.md).

## Snapshots and seeds (unused)

The contract also keeps a Merkle-sum tree of active units and can freeze it
per epoch, built for the committee placement of the replaced storage path.
No current code seals epochs or captures their seeds; `OperatorPool` draws
its own seed. The mechanism stays in the deployed bytecode:

- A 32-level tree over positions: an empty leaf is
  `keccak256(abi.encode(EMPTY_TAG, domain))` with count 0, an active one
  `keccak256(abi.encode(LEAF_TAG, domain, index, owner, commitment))` with
  count 1, a parent `keccak256(abi.encode(BRANCH_TAG, leftHash, leftCount, rightHash, rightCount))`,
  the tags being `keccak256` of `AgenticInternet/RegistryEmpty/v1`,
  `RegistryLeaf/v1` and `RegistryBranch/v1`. `activeRoot()` returns the root
  and the active count.
- `currentEpoch() = (now − start) / epochSeconds + 1`. Anyone may call
  `sealEpoch()` once per epoch with a nonempty tree; it stores the root,
  count, time, block, `beaconBlock = block + futureBlockDelay` and
  `admissionUntil = time + snapshotLeaseSeconds`. Later bonds and exits do
  not change it; missed epochs stay empty.
- `captureSeed(epoch)` works while `beaconBlock < block ≤ beaconBlock + 256`
  and stores `keccak256(abi.encode(SEED_TAG, domain, epoch, root, count, beaconBlock, blockhash))`
  with `SEED_TAG = keccak256("AgenticInternet/RegistrySeed/evm-blockhash-v1")`;
  a repeat returns it, a missed window has no fallback. A block producer can
  bias or withhold such a seed.

Tests: `contracts/test/NodeRegistry.t.sol` (funding and accounting, exits
and withdrawals, callback failures and reentrancy, sealing and the seed
window, randomized roots); the holders' side in the node's mailbox swarm
tests and `crates/node/tests/support/swarm_native.rs` (ten holders bonding
on a local chain).
