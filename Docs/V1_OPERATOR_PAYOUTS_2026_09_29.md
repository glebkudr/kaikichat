# Node operator payouts: OperatorPool — September 29, 2026

**Status:** user decisions from 29.09.2026; implemented in the
`feature/v1-operator-payouts-20260929` branch and deployed to the testnet on 29.09
(OperatorPool `0x7B9e1DDEc7deb6ed69ef7f2E9c3ae9Ee924029Bc`, BookShop
`0x20d1013D45f2C99472df833293375A811BF1e406`, preset #4). Verification —
[evidence](../evidence/reviews/operator-payouts-2026-09-29/). Replaces the “operator
pool accumulates in V1” item from [V1_AGENT_FIRST_SCOPE_2026_09_25.md](V1_AGENT_FIRST_SCOPE_2026_09_25.md)
and the move of `OperatorSettlement` (L05) to V2.

## Why

`BookShop` sends every payment to `RoyaltySplitter`: 10% to the treasury,
90% to the operator pool. Today the pool is an ordinary wallet (`.local/testnet/pool.key`),
meaning the operators' money is held by its owner and nothing is distributed.
We need an ownerless contract that pays holders for their work, with the
operator interface showing how much the operator has earned in dollars.

## User decisions (29.09)

- Only paid stamps pay out (`BookShop` books). Grant messages are not paid
  directly.
- Payouts are probabilistic tickets: a 10 ¢ prize to each designated holder
  (the user compared 1 ¢, 5 ¢, and 10 ¢ by gas and spread).
- A ticket is valid for 360 days from the book purchase.
- There is no withdrawal minimum, neither in the contract nor in the node.
  Withdrawal is manual only, initiated by the operator from their own node.
- Whatever is not withdrawn on expired tickets goes to the treasury.
- The contract is ownerless and unmanaged, like `RoyaltySplitter` and `BookShop`.

## Economics

A book is 1000 stamps for $1, and the pool gets 90%: $0.0009 per paid stamp.
It is split among the 10 mailbox holders — $0.00009 each.

Why only paid stamps. Holders are assigned by mailbox hash
(rendezvous), so each node carries the same share of grant traffic as the
network average. Paying for paid stamps gives a node, on average, “pool revenue ×
its share of all work” — the same “revenue / all work” rate, but without counting
grant work, which can be inflated with free Google accounts.
Self-dealing with paid stamps does not pay off: pay $1, get back at most
$0.90.

Lottery. Verifying each $0.00009 stamp in the contract would cost more than
the stamp itself. A paid stamp wins with probability `p`; the prize is
`P = $0.10` to each of the `S = 10` holders named in the stamp:

```
p = (book price × pool share / book size) / (P × S)
  = ($1 × 0.9 / 1000) / ($0.10 × 10) = 0.0009 ≈ 1/1111
```

The expected value per stamp equals the pool share. Spread: at $10 earned (100 wins)
±10%, at $1 ±32%.

On average a node receives 90% of network revenue divided by the number of units. For
each unit to earn $10 a year, revenue must be $11 per year per unit.

## Protocol

### The stamp names the holders

The mailbox stamp operation changes (it was `H("AIN_OPERATION_V1", mailbox, period, envelope)`):

```
swarm     = H("AIN_SWARM_V1", holders[0..10])          // 10 × 32 bytes
operation = H("AIN_OPERATION_V2", mailbox, period(8 BE), swarm, keccak256(envelope))
```

- `holders` are the commitments of the units the sender addresses the envelope to, in
  rendezvous order; if there are fewer than 10 units, the tail is zeros.
- The list travels with the stamp (≈320 bytes per message, only for stamps that name
  holders) and is stored with the record; the holder checks that the operation matches
  the list and finds itself in it. It needs no own view of the registry: otherwise,
  after a unit left or joined, the remaining holders could not reconcile the mailbox
  list until the end of the day and would lose payouts.
- The sender pins the list to the mailbox on the first stamp: later messages and
  retries to the same mailbox name the same holders, so that a retry is the same
  stamp the receipts were issued for. A unit that joined later stores copies but gets
  no winnings from them; a unit that left receives no new copies.
  Mailboxes rotate every period, so this lasts no longer than a day. The
  node passes registry units to core, and core keeps them across restarts.
- `H(tag, …) = keccak256(keccak256(tag) ‖ fields)`, as in `crates/mailbox-swarm`.
- The envelope is hashed separately so the contract can verify the stamp without 64 KB of data.
- Without the list, any node that saw a winning stamp (the notary, the recipient)
  could claim it. A holder not named in the list (joined during a rotation)
  stores the message but gets no winnings from it.
- The catalog and card operations (`lookup_operation`, `card_operation`) do not
  change: they have no holders, and their pool share stays with the treasury.

A stamp without a list (old apps) is accepted as before and yields no winnings to
anyone. The stamp signature is unchanged: the book signs `H("AIN_STAMP_V1", domain, book,
index(4 BE), operation)`.

### Which stamps win

```
ticket = H("AIN_TICKET_V1", domain, book, index(4 BE))   // stamp slot
win    = uint256(H("AIN_WIN_V1", seed[day], ticket)) < threshold
day    = (validUntil − validity) / 86400                  // UTC day of the book purchase
```

- `seed[day]` is fixed in the contract after the day ends: anyone calls
  `armSeed(day)` (the target is the block `seedDelay` blocks ahead), then
  `captureSeed(day)` records the target's `blockhash` as is, within 256
  blocks. If the window is missed, the day is armed again. Nodes do this themselves
  right after UTC midnight. The seed needs no separate tag: `win` already
  separates domains through `ticket`.
- The buyer does not know the seed at purchase time and cannot grind books. The
  outcome depends only on the slot, so picking a different envelope or mailbox does
  not change it.
- The node sees the result as soon as the seed of the book's purchase day is known:
  for books from past days, on message receipt; for today's books, tomorrow.
- `threshold = ⌊(2²⁵⁶ − 1) / (P × S × bookSize × 10000)⌋ × (priceUsdc × pool share in bps)`;
  for the testnet `0x3afb7e90…c6a2df8000`. The contract exposes `score(seed, ticket)`,
  `winThreshold()`, `wins(book, index)`, `seedOf(day)`, `ticketId`,
  `swarmDigest`, `operation`; the vectors are pinned in the contract tests and
  repeated in the node tests.

### Pool remainder

Money stays in the pool when: a book's stamps are unspent; a prize was never
withdrawn; a stamp was spent on something other than a mailbox (catalog, cards);
a mailbox has fewer than 10 holders. Non-winning stamps contribute no remainder —
on average they pay for the wins.

## The `OperatorPool` contract

Constructor: `(BookShop shop, NodeRegistry registry, address treasury,
uint256 prizeUsdc, uint64 ticketLifetime, uint16 seedDelay, uint64 firstDay)`.
From `shop` it reads `splitter`, `usdc`, `ethUsd`, `maxPriceAge`, `priceUsdc`,
`bookSize`, `validity`, `domain`. The contract finds its pool share in `splitter`
by its own address; if it is not there (or is there more than once), deployment
is rejected. Also rejected: `p ≥ 1`, zero lifetime, zero seed delay.

Testnet parameters: `prizeUsdc = 100000` ($0.10), `ticketLifetime = 360 days`,
`seedDelay = 5`, `firstDay` is the UTC day the shop was deployed.

### Withdrawal

`claim(unitIndex, transportKey, Ticket[] tickets)`:

- Called by the unit's owner from `NodeRegistry` or by the node itself with the
  receipts key: `unit.commitment = H("AIN_UNIT_V1", domain, transportKey, msg.sender)`.
  No one else starts a withdrawal.
- Each ticket: the book was sold by this shop; the stamp signature is the book key;
  `index < count`; 360 days since purchase have not passed; the day's seed is known
  and the stamp won; `holders[position]` is the unit's commitment and appears
  nowhere else in the list; this position has not yet been paid for the slot.
- The first withdrawal for a slot pins the operation: after it, only holders of
  that same operation are paid for the slot. Two signatures on one slot (a double
  spend) do not produce two sets of payouts.
- Any invalid ticket reverts the whole call: the node checks the list via
  `eth_call` before sending.
- The ticket total is credited to the unit (`owed`) and immediately paid out to the
  unit's owner: USDC first, the remainder in ETH at the Chainlink rate (freshness
  as in the shop). Whatever could not be paid (pool empty, stale rate) stays
  credited; `payOut(unitIndex, transportKey)` pays it later, with the same rights.
- Before paying out, the pool collects its own credits from `splitter`.

There is no minimum. Gas is paid by the transaction sender in ETH; it is not
deducted from the payout.

### Treasury

`sweep(maxDays)` is called by the treasury only. The contract walks purchase
UTC days starting from `firstDay` whose tickets have all expired
(`(day + 1) × 86400 + ticketLifetime ≤ now`), and accumulates
`soldOn(day) × priceUsdc × share / 10000 − paidOn(day)` into a shared remainder.
Lucky days (more wins than expected) offset unlucky ones: taking each day's
remainder separately would gradually drain the pool. A negative remainder (the
overspend of lucky days) carries over to subsequent calls. The positive
remainder is paid out, but no more than the pool holds above what is already
credited to units (`owed`); shortfalls caused by an ETH price drop do not carry
over — otherwise the treasury would take money from unexpired days. The
treasury does not touch money of unexpired tickets or credited amounts.

The ETH rate is read only when USDC is insufficient: with a stale rate, `claim`,
`payOut`, and `sweep` pay USDC and nothing breaks. A `sweep` that needs ETH at
a stale rate is reverted (`StalePrice`), so the rate cannot burn the treasury's
remainder.

### `BookShop`

Adds `soldOn(day)`: the number of books sold during a UTC day. The shop does
not change beyond that.

### Risks we accept

- An operator with units can route their own paid traffic to their own units and
  recover up to 90% of their spending — a discount to themselves, not other
  people's money.
- ETH price: payment in ETH uses the rate at purchase time, the payout the rate
  at withdrawal time. If ETH falls, the pool may come up short; the shortfall
  stays credited.
- Seed: the Base sequencer can shift the `blockhash`; missing the 256-block
  window gives whoever arms the seed a choice between blocks — nodes capture the
  seed immediately.
- The contract cannot distinguish a stamp signature made after the book expired;
  this gives the buyer the same “self-discount” on unspent stamps.

### Gas (prototype, Base mainnet 29.09: 0.006 gwei, ETH $2719)

| Payout | Tickets | Gas | Data | Cost |
|---|---|---|---|---|
| $0.10 | 1 | 153K | 0.7 KB | ≈ $0.0026 |
| $1 | 10 | 608K | 6 KB | ≈ $0.011 |
| $10 | 100 | 5.5M | 61 KB | ≈ $0.10 |

Contract (forge, 29.09): withdrawing one ticket costs 126K of execution gas,
and each subsequent ticket of the slot's first holder about 42K (for the other
holders it is cheaper: the slot record already exists). The pool collects its
credits from splitter only when its own USDC is not enough. A test pins the
ceilings: 140K for the first ticket and 50K for each subsequent one. About
200 tickets fit into a transaction (128 KB); the node splits larger withdrawals.

## Deployment

- New `RoyaltySplitter`, `BookShop`, and `OperatorPool`: the pool recipient in
  `RoyaltySplitter` is immutable. The pool address is predicted from the deployer
  nonce (splitter → shop → pool); the pool constructor checks that the splitter
  names it.
- `NodeRegistry` and `GrantIssuer` stay as they are.
- Nodes no longer accept old-shop books; the balance of the old pool wallet is
  transferred to the new pool.
- Deployment gas spending is approved separately.

## Node and UI

- The holder opens a ticket for every paid (non-grant) stamp that names its
  unit exactly once, with the data for `claim` (≈0.6 KB). The ticket waits for
  the seed of the book's purchase day; a winner is kept until withdrawal or expiry
  (360 days from purchase; tickets outlive the mailbox and the book), a loser
  is deleted.
- Seed: after the day ends, each unit-node, having waited out its share of the
  first ten minutes (by the hash of the unit and the day), reads `seeds(day)`;
  not armed, or the target window missed (more than 256 blocks) — it arms;
  armed and the target passed — it captures. Every transaction is
  estimated first (`eth_estimateGas`): what the contract would revert (someone
  else already armed) is not sent.
- The node signs transactions itself with the receipts key: gas is the estimate
  plus one fifth, the price ceiling is double the base fee plus tips, the nonce
  is pending. If there is not enough ETH at the receipts address, nothing is sent;
  the operator sees `no_gas` and the address.
- Withdrawal (`operator_withdraw`, command only): all winning tickets in one
  `claim` (up to 150 tickets per transaction). If estimation rejects the batch,
  the node estimates the tickets one by one and sets the rejected ones aside
  (`refused`: position already paid, different slot operation, ticket expired).
  What the pool credited but could not pay out is shown as `owed`; the next
  withdrawal without tickets calls `payOut`.
- `operator_earnings` / `kaiki earnings`: won, awaiting the draw, withdrawn
  (`claimed`), refused, `owed` per the pool's data, what expires within the
  next 30 days and in how many days (rounded up), how many messages the node
  stored and how many of them were paid, the receipts address and its ETH for
  gas, the state of the last withdrawal.
- Only registry unit nodes have the operator section; the app is a client.
- Holders still accept books of the previous shop (people's coins do not
  disappear) but do not draw tickets on them: the node reads each ticket's book
  in the current shop; a read error deletes nothing, and “not sold” is final
  only after the purchase day has passed.

## Order of work

1. This document.
2. Contract tests, the test critic, `OperatorPool` and `soldOn`.
3. The V2 operation and the holder list in the client and on the holders.
4. Tickets, seed, and withdrawal in the node; `kaiki`.
5. Deployment script, deployment to Base Sepolia, a new preset.
