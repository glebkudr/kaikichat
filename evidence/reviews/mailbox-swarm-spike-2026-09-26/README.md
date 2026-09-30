# Mailbox swarm acceptance spike, managed-time rig (2026-09-26)

Phase 7 of `Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md`, first half: the
acceptance scenario of `Docs/V1_STORAGE_REDESIGN_2026_09_24.md` on the
single-process managed-time rig (real libp2p over loopback TCP, virtual
clock, debug build). The native run comes after phases 1b and 2.

Test: `acceptance_spike_258_messages` in
`crates/node/src/reproducer_mailbox_swarm_tests.rs` (ignored by default).

```sh
python3 scripts/build-storage.py --worktree .local/verification-workspaces/mailbox-swarm \
  run sh -c 'RUST_MIN_STACK=8388608 cargo test --locked -p agentic-node --lib -- --ignored acceptance_spike --nocapture'
```

`AIN_SPIKE_MESSAGES=<n>` runs it with fewer messages.

## Scenario and what is asserted

- **Setup.** Ten holders, sender Alice, recipient Bob offline.
  - Holder 3 receipts as a unit the directory does not list (malicious).
  - Holders 1 and 6 are listed for Alice without addresses (out of her
    reach).
  - So exactly seven valid receipts are possible: the quorum.
- **Load.** Alice sends 258 messages at once. Meanwhile another book spends
  one slot twice, at holders 0 and 5.
  - With ten holders, every one is in both mailboxes' swarms, so
    replication and the notaries both see the double spend.
  - Detection by the notaries alone is covered by
    `a_notary_catches_a_slot_spent_in_two_swarms_that_never_meet`.
- **Stored.** Each message is stored with receipts from exactly the seven
  reachable honest holders.
  - The malicious holder's 258 receipts arrive and are all rejected.
  - Alice's book spent exactly 258 slots.
  - Her only failures are dials to the two unreachable holders: no
    capacity or rate refusals.
  - Every holder she reaches served exactly one store per message.
- **Double spend.** The cheating book ends up refused by every holder (time
  of the first step at which all ten block it).
- **Notaries.** For a sample of slots (first, middle, last), every one of
  the slot's notaries holds the record.
- **Sender leaves.** The two unreached holders have every copy by
  replication (each pulled at least 258 entries).
- **Disk loss.** Nine holders lose their disks; the malicious one keeps
  receipting as an unlisted unit. The nine honest holders again hold the
  same 258 copies.
- **Read.** Bob comes online and reads all 258 texts, in order.

## Results (n = 1 per configuration)

Virtual seconds are rig time: the rig advances its clock only when nothing
is in flight, and its periods are 1 s (request rate window), 5 s (read
poll) and 30 s (replication round). Real seconds are wall time of the
single-process debug run.

`final-258.json` is the scenario above.

| | virtual | real |
|---|---|---|
| All 258 stored at a quorum | 30 s | 96 s |
| Double spend refused by every holder | first instant | 2.4 s |
| Sample of slots on record with all their notaries | ≤ 30 s (checked after the stored phase) | |
| Unreached holders complete after the sender left | 1 s (they pulled during the load) | |
| Restored after 9 of 10 disks lost | 59 s (two replication rounds) | |
| Read by the recipient after coming online | 6 s | 66 s |
| Whole run | | 268 s |

**What the stored time measures.** It is set by the mailbox client's own
request rate: 128 starts per second. The client made:
- 2064 stores to the eight reachable holders;
- 1888 failed dials to the two unreachable ones.

That is about 3950 starts, or about 31 s. Retries back off per message and
holder, not per holder. An unreachable holder therefore costs every queued
message its own attempts. This is an open item.

**Requests per message, all nodes, until the sender left** (`sent`
counters, failed dials included):

| store | notarize batches | pulls | reads | summaries | proofs |
|---|---|---|---|---|---|
| 15.3 (8 served + 7.3 failed dials) | 8.8 | 1.4 | 0.5 | 0.4 | 0.07 |

- A notary batch carried 8.3 statements on average. Every holder puts every
  slot it stores on record with all ten notaries.
- The sender did not send its own notary batches before leaving: under load
  they yield to its stores. That branch is not exercised by this run.
- Bob sent 2.0 reads per message.

### Before notary batching (same scenario without unreached holders)

`before-batching-258.json` (commit `a634b64`) and
`after-batching-258-no-unreached.json`. Neither has holders out of the
sender's reach.

| | before | after |
|---|---|---|
| All stored at a quorum (virtual) | 33 s | 20 s |
| Sender store requests per message | 16.7 | 10.0 |
| Sender request failures | 4211 | 0 |
| Notary requests served per message | 79 | 7.2 |

- **Stored time.** 20 s is again the request rate bound: 2580 starts at 128
  per second.
- **Cause of the earlier failures.** The before run has no split of failures
  by cause. The later burst test showed rate refusals on the sender's own
  outbound streams: the node's shared processing budget counts them.
- **Part of it is the rig.** The rig puts a whole burst into one virtual
  rate window, so part of the earlier failures is an artifact. A native
  run spreads the same burst over real time.
- **The other copy times in those runs are not replication times.** The
  sender reached every holder itself.

## Compared with A04 (native, old custody path)

| | A04 (258 messages) | swarm, rig |
|---|---|---|
| Condition for "stored" | 10/10 at every level: data, index, addresses, history pages, pointer | 7 of 10 verified receipts (a weaker condition by design; copies are then restored by the swarm) |
| Sender requests per message, without retries | 55–65 (the redesign's line 25) | one store per reachable swarm holder (8 here) plus failed dials; no notary batches of its own in this run |
| Network requests per message, until the sender left | not measured | about 19 without failed dials (8 stores to the reachable holders, 8.8 notary batches carrying about 73 statements, pulls and reads); Bob's reads (2.0) and the repair traffic are not included |
| Recovery | about 20 min, limited to 32 reads/min; no autonomous repair | 59 s virtual after 9 of 10 disks lost, bound to the 30 s replication round |
| Code | custody/history about 32k lines + 54k tests | about 5.0k lines + 6.7k tests |

What the swarm code count includes:
- the swarm crate: 406 lines;
- six node modules (`mailbox_client`, `mailbox_holder`, `mailbox_grants`,
  `mailbox_notary`, `mailbox_proofs`, `mailbox_replication`): 3840 lines;
- the core's `mailbox_swarm`: 702 lines;
- envelope sealing in `crates/crypto`.

`crates/grant-book` (776 lines) is not included. Tests are the swarm crate
tests, the holder tests, the reproducer scenarios and the core tests.

Whole-run times are not comparable: the rig skips idle virtual time and runs
in one process. Times become comparable in the native run. The sender's
requests per message and the code size compare directly; the network total
was not measured for A04.

## Open items found

- Store retries back off per message and holder. A holder that cannot be
  dialed costs every queued message its own attempts and eats the request
  rate (1888 failed dials here). Back off per holder instead.
- Under load the sender's own notary batches yield to its stores. A sender
  that leaves right after its quorum never sends them. Holders still
  register every slot they store; a grant is registered when it is added,
  not under load.
- The rig's wall clock advances only by whole seconds per step
  (`Virtual::advance`). The numbers above use the instant clock.
