# Decision: storage and payment via the recipient mailbox swarm — September 24, 2026

**Status:** accepted by the user on 24.09.2026 after an architecture review.
Work on the current custody/history scheme (including the A04 optimization)
stops; the scheme below is implemented, and the replaced parts are removed
together with their tests, specs, and plan tasks. We leave no dead code or
parallel "legacy" paths: there are no real users or mainnet data, only
testnet.

## Original problem statement (unchanged)

From the ideation thread and R05–R07: there are no company servers, only the
recipient can decrypt; a message is stored by 10 nodes, and when a holder
fails another node picks up the copy; every message is paid for with a
"postage stamp" purchased with crypto or issued by the identity provider;
free stamps are limited by campaign and term.

## Why we are changing

Review of the current implementation (HEAD `5701428`) showed that the system
is an order of magnitude heavier than the task, and the key original
requirement — redistribution of copies — is not implemented
(`autonomousRepair: false`, R01–R05 not started).

| Metric per message (A04, 258 messages) | Value |
|---|---|
| Signatures / unsigned confirmations | ~26 / ~103 (of which 100 location ACK = 10 index × 10 data) |
| Sender requests without retries | 55–65 |
| Sender state | ~138 KB (61 KB job row + 67 KB outgoing object) for a ~2 KB envelope |
| "stored" condition | 10/10 at every level: data, index, addresses, graph pages, pointer |
| A04 run | 37–57 min; recovery ~20 min due to the 32 reads/min limit |
| Size | custody/history ≈32k lines of code + 54k tests (62% of `crates/node`); postage/finalizer/checkpoint ≈36k + 60k |

Root causes:

1. **Placement is tied to the stamp** (`ticketId` → 10 holders), not to the
   recipient. The recipient cannot compute where their messages are, hence
   index rosters, 100 location ACK, the signed history graph, the pointer,
   and an ordinal resolver that scans peers.
2. **Exact global single-spend of the stamp** without a server and without an
   L2 transaction per message required a BFT committee, checkpoint attesters,
   leases, epochs, closing, and O(N) handover, while every holder verifies
   and stores the entire chain of proofs in every object. A stamp costs a
   fraction of a cent; the worst outcome of a double spend is one extra
   message.
3. **The sender conducts everything and demands unanimity**, so redundancy
   became a condition of liveness rather than a safety margin.

In terms of attacks, the current scheme created global failure points (a
failure of >f finalizers or expiry of attester leases halts paid messages
across the whole network) and did not close the main privacy threat (see
"Open user decisions").

## New architecture

### Storage and delivery

- **Mailbox.** The mailbox address rotates: `mailbox_id = H(MLS exporter, period)`.
  Only conversation participants know it; an outsider cannot address spam
  there.
- **Swarm.** The mailbox holders are the 10 operators closest to `mailbox_id`
  in the registry snapshot (rendezvous hashing by stake/ordinals). The sender
  and the recipient compute the composition themselves. The "ordinal → peer
  address" catalog is published in signed registry records rather than looked
  up by polling peers.
- **Sending.** The sender sends the envelope with the stamp to all 10 swarm
  members; "stored" means 7 signed receipts. Sending through 1–2 members in
  the hope of relaying is forbidden: with 1/3 malicious stake, three random
  members are all malicious with ~4% probability.
- **Replication and repair inside the swarm.** Swarm members themselves bring
  up missing copies (push + periodic range reconciliation, anti-entropy).
  When the composition changes (a holder disappears, a new registry
  snapshot), the next closest operator takes over the mailbox contents from
  the survivors. This is exactly the original requirement "another node picks
  up the copy", as the primary mechanism rather than a separate repair
  subsystem.
- **Retrieval.** The recipient reads the mailbox by cursor from several
  swarm members and merges the responses. Gaps are visible from message
  numbers inside MLS and the conversation counter in the encrypted header.
  Against hiding the "tail", a small **conversation head** (the last number)
  signed by the sender is kept and replicated in the swarm. Index, addresses,
  the history graph, and the pointer to the graph root are not needed.
- **Storage checks.** Holders answer challenges against stored data (a hash
  with a nonce). This is the basis for repair and for excluding lazy holders.

### Stamps and trust

- **Book** is purchased on L2 as now. The holder verifies the payment fact
  **once per book** (cached by checkpoint), not with proofs in every object;
  objects and jobs carry only a reference to the book. A book is valid for
  months, with no binding to a registry epoch. An L2 or attester failure
  blocks only new purchases, not sending on already verified books.
- **Stamp** — a signature of the book key over `(book, index, operation)`;
  the holder verifies the signature, `index < count`, the term, and the
  class.
- **Stamp notary.** Together with the message, the sender writes a
  `ticketId → hash(operation)` record to the 10 nodes closest to
  `H(ticketId)`, first-writer-wins. Any repeat spend of a slot inevitably
  lands there too. Mailbox holders accept the message immediately and check
  the notary in the background. There is no consensus, committee, epochs, or
  handover.
- **Proofs of violations.** Punishment only on a self-contained
  cryptographic proof, never on complaints or voting:
  - sender: two of their signatures on one slot with different `operation`;
  - holder: two of their receipts on conflicting spends of one slot, or
    a receipt plus a failed storage challenge.
- **Punishment.** The proof spreads across the network; every node that
  verifies it immediately blocks the book or the holder locally. Final
  settlement is on-chain: anyone submits the proof to the contract, the
  sender's entire remaining book burns, the holder gets stake slashing and
  removal from the registry. For this, receipt keys must be cheaply
  verifiable on L2 (secp256k1 or P-256 with the RIP-7212 precompile), not
  Ed25519.
- **Identity provider stamps** are a "grant book", signed by the provider
  and funded through `SubsidyVault`; verification is the same. The campaign
  subject is punished. `SubsidyVault.claim()` must verify the credential
  (currently it does not).

### Changed guarantees

| Was | Became |
|---|---|
| R14: globally exactly one final spend before storage | a double spend is detected by the notary and provable; the offender burns the book or the stake |
| R10/D05: 10 verified index promises, a signed completeness graph | 10 replicas in the swarm, reconciliation, a signed conversation head, gaps by numbers |
| sending waits for committee finalization | "stored" at 7 of 10 receipts within seconds |
| an L2/attester failure longer than the lease halts paid messages | only new purchases are blocked |

## What is kept

MLS and Core/SQLCipher; libp2p/Kademlia; the book purchase and operator
registry contracts with stake and beacon; stamp books; signed receipts (with
a different key type); L2 checkpoint roots for verifying books and the
registry (by attesters for now); CLI/MCP/UI; the managed clock and the
single-process reproducer (C05).

## What is replaced and removed

Removed when switching to the new path, together with tests, native
scenarios, specs, and plan tasks:

- placement by `ticketId` (`crates/postage-spend/src/placement.rs`) and the
  ordinal resolver `/custodians/1`, route hints, resolver admission lanes;
- index rosters, index store/network, 100 location ACK
  (`custody_index*`, `spec/paid-index-*`, `spec/index-*`,
  `spec/custody-holder-location-v1.md`);
- the history graph: leaves/branches/root, pages, the pointer to the root,
  their sync, scan, cache, and retry (`custody_history_*`, `spec/*history*`);
- the spend path through finalizers: SpendRecord/QC in sending, epoch
  closing, lineage, handover, successor spending, authority lease in the
  send path (`crates/postage-spend` spend part,
  `spec/postage/epoch-closing-v1.md`, `native-handover-v1.md`,
  `successor-spending-v1.md` and related);
- the 128-job sender queue, fence/`sender_queue`, 60 KB job rows and their
  GC, publisher/collection/placement priority/arrival fences;
- native scenarios `tests/evm/public_sender_capacity_a04.py`,
  `public_history_full130_h11.py`, Diagnostic32/H10, and their oracle.

## User decisions of 24–25.09

- The finalizer crate is removed entirely: ordering of group commits goes
  through the notary by `H(group_id, epoch)`, without BFT.
- Legacy ZK (`postage-zk`, `postage-proof`, RISC Zero, V1-A05) is removed.
- Payment privacy is not required: one stamp book per user (25.09).
- Every message, including direct online delivery, carries a stamp; the
  holders and the recipient's node verify it.
- Contact by ID: the introduction mailbox is a swarm from `H("intro", pubkey)`
  with published KeyPackages; a contact request is paid.
- Free coins are grant books of the identity server per node book account
  with a daily limit from the `GrantIssuer` contract (changed no more than
  once a day and by no more than 10x); the notary by
  `H("grant", domain, server, day, serial)` catches repeat and backdated
  books.
- Coin purchase immutably splits the payment: the network share (royalty)
  and the operator pool.
- Details: [V1_AGENT_FIRST_SCOPE_2026_09_25.md](V1_AGENT_FIRST_SCOPE_2026_09_25.md).

## Open user decisions (original edition of 24.09)

1. **The finalizer crate.** Besides stamps, it is planned for ordering the
   group control log (V1-G03, ADR-02). Whether to remove it entirely or
   first settle the ordering of MLS commits in groups without BFT (the
   research document calls total ordering a "deliberate overpayment").
2. **Legacy ZK** (`crates/postage-zk`, `postage-proof`, RISC Zero, V1-A05):
   whether we keep compatibility with old testnet stamps or remove it.
3. **Payer anonymity is the next priority.** Public stamps link the on-chain
   wallet of the book buyer to every message it pays for; a staked holder
   sees the sender's social graph. The immediate minimum is a separate book
   per conversation and an honest note in the UI; later, lightweight
   anonymous stamps (RLN as in Waku or blind tokens from a threshold of
   operators).
4. Parameters: swarm size 10, quorum 7, the address rotation period,
   reconciliation and challenge intervals; the behavior of a "hot" group
   mailbox.
5. Replacing attesters with a light client.

## Transition plan

1. A separate branch; new backend tests → RED → an independent
   backend-test-critic → production, as AGENTS.md requires.
2. A spike on the acceptance scenario: 258 messages, loss of 9 of 10 copies,
   the sender turned off, a stamp double spend, a malicious holder. Compare
   time to "stored", delivery time, and code size against A04.
3. Switching and removing the replaced parts in a single series of changes,
   without feature flags or compatibility layers.
4. Update the plan (`plan.json`, COVERAGE, chapters 01, 03, 05, 10, 12), the
   specs, and AGENTS.md for the new acceptance scenarios.

Review evidence: `evidence/reviews/a04-retention-gc-2026-09-24/` (branch
`evidence/v1-a04-retention-gc-20260924`),
`evidence/reviews/custody-backpressure-2026-09-18/`.
