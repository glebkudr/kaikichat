# Public testnet acceptance (AF07, AF06, AF05) — 2026-09-29

Runner: [scripts/check-testnet.mjs](../../../scripts/check-testnet.mjs). The
clients run in OrbStack containers from the Linux image of the network E2E
(`tests/network/Dockerfile`, built from the current sources), each behind a
NAT router of its own (iptables in a container: masquerade, no forwarding
to the other private network or to the other router's public side, a public
resolver through the router). They reach the Base Sepolia testnet: the
nodes' flags from `deployments/base-sepolia.json`, four of the ten routes of
the signed preset on kaikichat.com, and two of the nodes (4103 over QUIC,
4107 over TCP) as circuit relays. Nothing changed on the host or its
firewall; the server changed only by the fix below, deployed through
Coolify.

```
node scripts/check-testnet.mjs --case af07 --case af06 --case double-spend --case stolen-key --pay --keep
node scripts/check-testnet.mjs --attach RUN_ID --case double-spend --case stolen-key
```

The owner delegated the decisions (2026-09-29): AF06 as isolation on the
newcomer's side, books bought from the deployer's key, a forged grant
within the cap as the control of the stolen-key check.

## AF07 — two agents behind NAT through a relay, then another: passed

Passed in three runs; the times are from the main run
([af07-af06-af05.json](af07-af06-af05.json)); the others:
[second](af07-second-pass-af06-bookless-peer.json),
[first](af07-first-pass-stale-price.json).

| Step | Seconds |
|---|---|
| both clients reserve circuits at both relays | 1.5 |
| each node reads the shop's terms (a payment request) | 2.2 |
| books bought and noticed (each) | 28–31 |
| holder directories complete and cards published | 88 |
| Alice asks Bob by id through both NATs; Bob has her | 25.6 |
| Alice's message read by Bob / Bob's reply read by Alice | 1.1 / 3.2 |
| Alice's connection to Bob: a circuit through node 3 (QUIC) | — |
| node 3 cut off for both NATs: each keeps one relay | 25.0 |
| the next message / reply read | 1.1 / 1.1 |
| a circuit through node 7 (TCP) in its place | 107 (1.2 in the second run) |

The two clients never reach each other directly: TCP probes to the other
client and the other router fail, and their routers drop what goes there.
While the new circuit comes up, messages go through the holders.

## AF06 — our web services gone, a newcomer through an independent peer: passed

The newcomer's router drops HTTPS to 51.91.126.3 (kaikichat.com with the
preset, id.kaikichat.com, the directory: 78 packets dropped). Its only
bootstrap route is an independent peer in another container: someone
else's node, not in the preset, not a holder, no bond, with a book of its
own.

| Step | Seconds |
|---|---|
| a minute of `coins claim`: always `claim_pending`, no login link | 62 |
| the newcomer buys a book, gets the holders' records from the peer, publishes its card | 65 |
| it finds Alice (AF07) by id; she has it | 7.6 |
| its message read / Alice's reply read | 0.05 / 2.2 |

What only we give — free coins — is the one thing missing. The holders are
still ours: there are no others yet, and with ours stopped no mailbox could
reach a quorum (holders are chosen among all registry units).

Found: a peer without a book has no holders' records to hand on (holders
show records only to units and book owners), and a newcomer does not look
for holders through the DHT. With a bookless independent peer the newcomer
never completed its directory
([second run](af07-second-pass-af06-bookless-peer.json)). A newcomer thus
needs an entry peer that takes part in the network with a book.

## AF05 — purchase, network share, double spend, stolen issuer key: passed

- **Purchase and balance:** thirteen books bought over the runs
  (`BookShop.buy` from the deployer, 0.000375 ETH each at the Chainlink
  rate plus 1 %, the excess refunded); each node saw its book of 1000 in
  28–31 s (five confirmations).
- **Network share:** in the main run the treasury's credit in the
  `RoyaltySplitter` grew by 74 347 784 064 296 wei for two books: a tenth
  of the price at the rate.
- **Double spend** ([af05-double-spend-stolen-key.json](af05-double-spend-stolen-key.json)):
  the newcomer's profile, backed up, sends three messages, is restored from
  the backup and sends three others with the same stamps. Before, the
  directory takes a search pass of its book (200); 1.3 s after the second
  sends, it refuses it (401 `book_required`), Alice's node reports the book
  `blocked`, and holders refuse its pass (`blocked`).
- **Stolen issuer key:** a grant signed with the identity server's key
  (`crates/node/examples/grant_pass`, one grant per serial all day) passes
  the directory's check at serial 9999 (200), the last within the day's cap
  of 100 000 000 coins in books of 10 000, and is refused at 10 000 and
  10 001 (401). The check is the holders' rule, read from `GrantIssuer`.
  `reportEquivocation` was not called on the testnet (it would end our
  issuer from the next day); Foundry tests cover it.

## Found and fixed on the way

- **The testnet's holders were unreachable from outside, but for the
  preset's routes** (fixed in `deploy/node/run-nodes.sh`, dad2d575,
  deployed): listening on 0.0.0.0 on the host's network, each node's
  record listed eight docker bridges (10.0.x.1) and never 51.91.126.3. A
  client dialled only its four preset routes and never stored at a quorum:
  no card, no message. Now the nodes listen on the public address; a kept
  client published its card once it pulled the new records.
- The image of the network E2E did not build (the owner's skill,
  `services/`, `vendor/` left out); fixed.
- Harness: clients on internal networks had no DNS; a failed `cast`
  repeated its command line, the deployer's key with it, into a local log
  and report (removed from them; the runner now redacts); a check counted
  one of two block rules.
- The Base Sepolia ETH/USD feed once went over the shop's hour without an
  update: `buy` reverted with `StalePrice` before spending gas; the runner
  retries a minute later.

## Also seen

- A client retries holders it cannot dial about once a second (about 2100
  failed dials in 55 minutes in a kept client): no back-off per holder.
- Without the identity server `coins claim` answers `claim_pending` for
  ever (status `starting`), never that the server is unreachable.
- The preset carries no relays and `kaiki` has no `--relay`: agents behind
  NAT get no circuit by default; their messages go through the holders.
