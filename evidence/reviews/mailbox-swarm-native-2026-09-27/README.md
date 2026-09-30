# Mailbox swarm native spike (2026-09-27)

The second half of phase 7 of `Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md`: the
acceptance scenario with real daemons and a real chain. It complements the
managed-time rig run in `../mailbox-swarm-spike-2026-09-26/`.

Test: `native_swarm_spike` in `crates/node/tests/support/swarm_native.rs`,
ignored by default. `AIN_SPIKE_MESSAGES=<n>` sets the number of messages
(default 20).

```sh
python3 scripts/build-storage.py --worktree .local/verification-workspaces/mailbox-swarm \
  run sh -c 'AIN_SPIKE_MESSAGES=258 RUST_MIN_STACK=8388608 cargo test --locked -p agentic-node --test processes -- --ignored native_swarm_spike --nocapture'
```

## Scenario

- **Chain.** Anvil runs locally with a block every second. `RoyaltySplitter`,
  `BookShop` (1000 stamps per book), `GrantIssuer` and `NodeRegistry` are
  deployed with the node's network domain.
- **Holders.** Ten `agentic-node serve` processes read the chain with one
  confirmation.
  - Each reports the commitment it bonds (`node_info.directory.ownCommitment`)
    and is bonded in the registry.
  - Holders 1–9 use holder 0 as their bootstrap peer; nobody is handed a
    directory.
- **Purchase.** Alice buys a book with `coins_buy` and `cast send`. Her node
  notices the confirmed purchase. The treasury is credited its tenth of the
  price.
- **Offline send.** Bob stops; Alice sends and stops once every message is
  stored at a quorum.
- **Disk loss.** Holders 1, 4 and 7 lose their mailbox stores and restart
  empty; the swarm repairs them.
- **Read.** Bob starts and reads every message, in order.

## Results (debug build, one Mac, n = 1)

| | 20 messages | 258 messages |
|---|---|---|
| Directory complete after the holders started | 30 s | 30 s |
| Purchase noticed after payment | 30 s | 30 s |
| All stored at a quorum | 2.5 s | 27.4 s |
| Three lost holders repaired | 26 s | 37 s |
| Read by the recipient after coming online | 0.9 s | 13.4 s |
| Sender store requests | 214 | 2592 (258 × 10 + 12 retries, no failures) |

The 30 s in the first two rows are polling intervals: the registry is read at
start (after the bonds confirmed), and a purchase request is read every 30 s.

`native-258.json` is the test's report for the 258 run.

## Found and fixed by the spike

- **The recipient's daemon stalled on 258 envelopes.** It held envelopes that
  arrived before their MLS predecessor and retried all of them after every
  import, each retry running a full candidate decrypt. An owner IPC call
  timed out after 5 s.
  - Fix: a held envelope is placed once in its sender's order
    (`AppCore::swarm_envelope_order`, the authenticated generation) and
    released in order, stopping at the first gap.
  - Rig test: 60 envelopes around a gap took 1395 decrypt attempts before
    and take at most three each now.
- **A bond in the head block is not visible with one confirmation.** A node
  reads the registry at start and every ten minutes, so an operator's bond
  shows up within ten minutes. The test waits two blocks after bonding.

## Compared with A04 (old path, native)

Condition for "stored":
- A04: 10/10 holders at every level;
- swarm: 7 verified receipts, with copies restored by the swarm.

| | A04 (258 messages) | swarm, native |
|---|---|---|
| Recovery after copy loss | about 20 minutes, no autonomous repair | 37 s, autonomous |
| Recipient read | bounded to 32 reads a minute | 13 s for 258 |
