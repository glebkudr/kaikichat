# Operator payouts — verification, 2026-09-29

Decision and design: [Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md](../../../Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md).
Code: `contracts/src/OperatorPool.sol`, `BookShop.soldOn`; named stamps in
`crates/mailbox-swarm` and `crates/core`; holder tickets in
`crates/node/src/mailbox_holder.rs`; the payout lane in
`crates/node/src/mailbox_payouts.rs`; transactions in `crates/node/src/chain.rs`;
`kaiki earnings [withdraw]`.

## Tests

Written before the code; every set reviewed by an independent test critic
(two REVISE rounds on the contract tests, one on the node tests, one on the
old-shop fix; all ACCEPT).

- Contracts: 72 forge tests (30 of the pool). A withdrawal costs 126
  thousand gas for one ticket and about 42 thousand for each further one
  (pinned: 140 and 50 thousand).
- Node, core, swarm rules: pinned vectors shared with the contract
  (`cast keccak`), the EIP-1559 transaction byte for byte as `cast mktx`
  signs it, the two-ticket claim as `cast calldata` encodes it.
- Managed-time rig with ten holders and a contract-like fake pool: named
  stamps, tickets won and withdrawn in one claim, a mailbox keeping its named
  holders after one left, the day's seed fixed by exactly one holder after
  midnight, withdrawals without gas / with a refused ticket / with a short
  pool, books of a former shop never drawn.
- Workspace: 712 passed, 0 failed (before the fix); clippy clean; desktop
  vitest 113, tsc, vite build.

## Real chains

- Local anvil, pool deployed by `scripts/deploy-contracts.py` (fresh and
  `--reuse`): the node arms and captures a seed; a capture before the target
  is refused unsent; an account without ETH sends nothing; the node's claim
  of a winning paid stamp pays the unit's owner $0.10 in USDC and a second
  claim is refused unsent (ignored tests `a_real_chain_takes_the_nodes_seed_transactions`,
  `a_real_pool_pays_the_nodes_claim_of_a_winning_ticket`).
- Base Sepolia (owner's OK): RoyaltySplitter
  `0x1a69B92C9C63d53D12A1B26875dFaBC2f4c608B8`, BookShop
  `0x20d1013D45f2C99472df833293375A811BF1e406`, OperatorPool
  `0x7B9e1DDEc7deb6ed69ef7f2E9c3ae9Ee924029Bc` (deploy 0.000025 ETH, every
  immutable read back). The former pool account's 0.005704 ETH forwarded to
  the pool; the ten nodes' receipt accounts funded with 0.0005 ETH each.
- Deployed to the testnet nodes (Coolify, preset serial 4): all ten read the
  pool's terms and find their units (indices 0–9). The first day after the
  switch, exactly one node (unit 4, receipt `0xe6680bdf…d917`) armed and
  captured day 20724's seed, two transactions for 0.00000059 ETH; the other
  nine sent nothing.

## Release

- Native macOS gate (`scripts/check-native.mjs`, main checkout at 145f1171):
  passed, 717 tests, both native chat and network scenarios.
- CLI published at kaikichat.com/downloads from 145f1171: macOS arm64
  (sha256 `82ba5859…ecff1`) and Linux x86_64 (`32681925…d12b`);
  `install.sh` installs `kaiki` with `earnings`. A fresh profile of it
  follows preset serial 4, joins the testnet and reads the live pool's
  prize ($0.10) as a client (`unit: null`).

## Paid traffic on the testnet

`scripts/check-testnet.mjs --pay` (run `ain-testnet-e17d54ff`, owner's OK):
two NAT'd clients bought a book each from the new shop and exchanged a
contact by id, a message and a reply through the testnet's relays, also
after one relay disappeared. The splitter credited the pool 667759123372362
wei and the treasury 74195458152484 — nine to one. Each of the ten holders
then kept 7 tickets of the two new books, drawing by the seed of day 20725,
which the nodes fix after its end.

No prize has been won on the testnet yet: no book of the new shop was sold
when the switch was made, and messages paid by books of the former shop are
held but never drawn; one in about 1111 paid stamps wins.
