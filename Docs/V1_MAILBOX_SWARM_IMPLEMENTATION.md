# Mailbox swarm: implementation plan (2026-09-24)

Implements [the storage redesign](V1_STORAGE_REDESIGN_2026_09_24.md). Every
phase is tests-first (RED → independent backend-test-critic → production), with
as many scenarios as possible in the single-process managed-time reproducer.
User decisions (24.09): legacy ZK (`postage-zk`, `postage-proof`, RISC Zero,
V1-A05) is removed. Superseded on 25.09 by
[the agent-first scope](V1_AGENT_FIRST_SCOPE_2026_09_25.md): the finalizer
crate is removed entirely (group commit order goes through the notary by
`H(group_id, epoch)`); one stamp book per user, not per conversation (payment
privacy is not a V1 goal); every message carries a stamp, direct delivery
included; grant books of the identity server (`crates/grant-book`, another
session) are verified by holders like purchased books.

## Parameters

| Parameter | Value | Where |
|---|---|---|
| Swarm size / quorum | 10 / 7 | `agentic-mailbox-swarm::select` |
| Mailbox period | 86 400 s (same slot as today's mailbox pointer) | `address` |
| Holder selection | rendezvous (HRW) over registry units, weight 1 per unit, keyed by unit commitment (ordinals shift on exit) | `select` |
| Signed formats | keccak256(keccak256(tag) ‖ fixed-width big-endian fields); secp256k1 r‖s‖v, low s, v ∈ {27, 28} — EVM `ecrecover` compatible | `digest` |

## Phases

0. **Pure rules** — new crate `crates/mailbox-swarm`: selection, mailbox id,
   stamp and ticket id, holder receipt, sender/holder equivocation proofs.
1. **Holder service** in `crates/node`: protocol `/agentic-internet/mailbox/1`
   (store → receipt, read by cursor), node-local store
   `<profile>.mailbox.db`, book terms verified once per book from the existing
   funding proof (`ProvenFunding`) and cached per checkpoint, secp256k1
   receipt key.
2. **Directory**: per unit a self-verifying record (unit index, commitment,
   member proof against the snapshot root, binding signed by the node key to
   transport key, receipt account and addresses); distributed and cached per
   registry epoch so every node computes swarms without querying peers.
3. **Send**: per-conversation book with a secp256k1 book key; envelope +
   stamp to all 10 holders; "stored" at 7 receipts. The sender job is small
   (message id, ticket, receipts) — no 60 KB rows, fences or queues of 128.
4. **Receive**: read the current and previous period mailbox by cursor from
   several holders, merge, detect gaps by the sequence inside the encrypted
   envelope and the sender-signed conversation head.
5. **Replication and repair** inside the swarm (push + range reconciliation),
   handover to the next rendezvous member on membership change, storage
   challenges.
6. **Notary** (`ticketId → hash(operation)` on the 10 units nearest to
   `H(ticketId)`, first writer wins, background check by holders),
   propagation of equivocation proofs and local blocking; contract functions
   to burn a book or slash and exclude a unit from a proof.
7. **Acceptance spike** (258 messages, 9 of 10 copies lost, sender offline,
   stamp double spend, malicious holder) in the reproducer, then native;
   compare with A04. Then switch over and delete the replaced path in one
   series (see the redesign's list), update plan, specs and AGENTS.md.

## Status

- Phase 0, 1a (holder service), 3a (per-conversation books) and 3b/4 (send
  and receive end to end) are implemented; 1b (verified purchases) follows the
  end-to-end path, 2 (directory) is injected in tests until then.
- 3b/4 contract: holders store only a sealed envelope (the signed wire under a
  key derived from the direction's MLS exporter and the period, AAD = mailbox
  id, padded to 1 KiB buckets, deterministic so a retry reuses its stamp
  slot). A Welcome stays on direct delivery. A message is pinned to the
  period of its first attempt while the recipient still reads it (current and
  previous period) and moves to the current period with a new slot after
  that. "Stored" = 7 verified receipts (listed holder key and unit, pinned
  mailbox/operation/ticket, distinct signers of the selected swarm); the
  sender keeps delivering to the rest of the swarm after that and leaves a
  holder that failed three more times to swarm replication. The reader polls
  every conversation's incoming mailbox of the current and previous period
  from every selected holder by cursor, dedupes by operation and holds an
  envelope that arrives before its MLS predecessor. Holder receipts
  acknowledge swarm deliveries; no MLS receipt travels through the swarm.
- Client limits: at most 8 concurrent mailbox requests per node and 2 per
  holder (the ordinary processing budget has 16 slots and 4 per peer).
- Phase 5 (replication and repair) is implemented: every 30 s each holder
  sends each other member of a mailbox's swarm a summary of the mailboxes
  they share (operation count and XOR of the operations, chunked by 64). A
  side that sees a difference pulls the other's entries with ordinary cursor
  reads and stores them through the verified store path — only for mailboxes
  it is itself a member of; a unit that left a swarm still offers what it
  holds. This repairs copies the sender missed and copies a holder lost, and
  hands a mailbox to a joining unit. A slot spent on two operations is kept
  as a `SenderEquivocation` proof (once per ticket, never replacing the
  stored entry). Storage challenges move to phase 6: a non-answer is not a
  cryptographic proof. Diagnostics: `mailboxSwarm.replication` and
  `mailboxHolder` in node info.

- Phase 5b (retention) is implemented: a stamp's operation commits to the
  mailbox period; holders take fresh entries only for periods written now
  (period(now) ± 1), keep a mailbox 30 periods after its own and then drop
  it, keep a ticket record while its book can sign or its mailbox is kept,
  and judge replicas at their period (a copy paid in time is repaired after
  its book ended). Core keeps a per-conversation read-through period; the
  reader reads from it (or from the period before the conversation began),
  counts a closed period as read once seven holders were read to its end by
  requests sent after its writes closed, and spreads catching up over polls
  (four closed periods per conversation and poll, in turn).

- Phase 3c (one book key per profile) is implemented: core keeps one
  secp256k1 stamp key per profile and the verified books under it (bought
  or granted); a new operation takes the book lasting at least another
  period that ends soonest, a book ending within a day only when nothing
  else is left; a spent operation keeps its book and slot.
- Phase 6a (notary) is implemented: the ten units nearest to
  `H("AIN_NOTARY_V1", key)` keep the first verified statement per key with
  its first-seen time — a stamp per book slot (signer and index checked, not
  expiry), a grant per issuer serial (server signature checked). A different
  statement for a taken key is kept as a proof (`SenderEquivocation` or
  `GrantEquivocation`). Senders put each new stamp on record; holders put
  every newly stored entry (fresh or replica) on record and keep the proof
  when a notary answers with another first stamp. Records last while the
  book can sign (slots) or until the grant expires.

- Phase 6b (proofs and blocking) is implemented: a node keeps a proof only
  if it verifies there (a sender proof against a book it knows, a holder
  proof against the signing account, a grant proof by its issuer) and only
  one per offender; every node pulls the proofs a random directory member
  learned since last time (an append-only log paged by sequence), so proofs
  spread without trusting complaints. A proven book pays for nothing new
  (fresh stores and new notary keys are refused); replicas of it are taken
  only for periods written before the node learned the proof, so copies paid
  before the cheat are still repaired. A proven holder is never asked by
  senders, readers or replication.

- Phase 6c (grant books at holders) is implemented.
  - Core keeps a grant of its own network naming the profile's book key as
    a book whose id is the grant's, and keeps the grant to show holders.
    The node puts each grant it adds on record with the grant's notaries
    right away, so a grant received today pays on later days too.
  - A holder that does not know a granted book refuses the store as
    `unknown_book`; the sender then shows it the grant (`LearnGrant`) and
    keeps retrying the store.
  - The holder checks the grant against the issuer's rules for the grant's
    day (active days, book size, daily cap, maximum validity, not ahead, not
    expired). It then asks the grant's ten notaries when they first saw it.
  - The book is learned (and kept until the grant expires) when at least
    four notaries saw it on its day, within the clock tolerance, and they
    outnumber the late, unanswered and unreachable ones together.
  - A grant shown only after its day, or a serial another grant took first,
    is refused and the refusal is remembered for five minutes. A serial
    granted twice leaves a `GrantEquivocation`, and that issuer grants
    nothing after the serial's day. Unreachable notaries that could still
    tip the decision leave it open: the holder asks again after 30 s.

- Phase 6d (grants in replication) is implemented.
  - A read page carries the grants of its own granted books, each once.
  - A member pulling a page checks only the grants its entries name, the
    same way as a grant a sender shows.
  - Its cursor stops before the first entry of a grant still being checked,
    so the next round takes it. Entries of a refused grant, or of an
    unknown bought book, are passed over and not read again.
  - An issuer's grants stop after the earliest day proven against it. An
    earlier-day proof replaces the kept one and is passed on, so nodes that
    learned proofs of different days end at the same day.

- Notary and request load (found by the phase 7 spike) is bounded.
  - Every node still puts each slot on record with all ten of its notaries.
    A notary request carries up to 64 statements and gets one answer per
    statement, in order.
  - The notary lane keeps at most two requests in flight per node and sends
    at most one batch per notary per second. Each node starts its round of
    notaries at a point of its own. So statements pile up into batches
    instead of taking the notaries' request rate from stores and reads.
  - The mailbox client starts at most 128 requests per second, 32 per
    holder. The node's shared processing budget (256 and 64 per peer)
    counts outbound streams too; above it a sender's own stores failed.
  - Failures are counted by cause (capacity, rate, dial) in node info.

- Phase 1b, holder side, is implemented (2026-09-27; plan below).
  - `contracts/src/BookShop.sol` sells books and sends each payment to the
    royalty splitter.
  - The node's chain reader (`crates/node/src/chain.rs`) and its lane
    (`mailbox_chain.rs`) read books and grant rules; `serve` takes
    `--chain-rpc`, `--chain-id`, `--book-shop`, `--grant-issuer` and
    `--chain-confirmations` (default 6).
  - Holders persist the books they read (`mailbox/book/…`) and keep grant
    rules per issuer and day; a proven issuer is refused before any read.
  - Diagnostics: `node_info.chain` (`configured`, `running`, `absentBooks`,
    `reads`, `learned`, `failures`).
  - The buyer's side: owner IPC `coins_buy` and `coins_balance`; core
    keeps one unpaid request (`swarm/purchase/…`) until paid, and the node
    reads it every 30 s for an hour, then every ten minutes. A confirmed
    purchase becomes a book and releases messages that waited for one.
  - The listener retry is a scheduler deadline only while a listen address
    has no listener; the managed-time rig no longer steps every second, and
    the node lib runs 121 tests in about 45 s.
  - `coins claim`: owner IPC `coins_claim` with `serve --identity-server`;
    core keeps the claim (`swarm/claim`), the node posts it (again every
    30 s while unanswered), reads it every 5 s and adds the grant. Phase 1b
    is complete; its CLI commands come with phase A's owner CLI.

- Phase 2 (directory) is implemented (2026-09-27; plan below).
  - `agentic_mailbox_swarm::directory`: the unit commitment and the unit
    record. `NodeRegistry.activeUnits(start, limit)` pages the active
    commitments; `serve --registry` joins the chain flags.
  - `crates/node/src/mailbox_directory.rs`: registry reads every ten
    minutes (a failed one keeps the membership and is retried after a
    minute), the node holding as its active unit, its own record re-issued
    on address change or after six hours, and `Directory` pulls (a random
    member every ten minutes; every connected peer every 30 s while units
    lack a record). Diagnostics: `node_info.directory`.
  - The native spike passed (2026-09-27,
    `evidence/reviews/mailbox-swarm-native-2026-09-27/`).
  - Next: stamps on direct delivery, then the owner CLI, the intro
    mailbox and groups via the notary (IMPLEMENTATION_STATUS.md).

## Switch-over (user decision 2026-09-26: delete before 1b and 2)

The replaced path goes first, so that the build and test loop is fast for
phases 1b and 2. Native delivery of offline messages is unavailable until
those two phases land (testnet only; GUI is phase B). Stages, each leaving
the workspace building and its tests green:

1. Finalizer. Removes:
   - `crates/finalizer` and the node's `finalizer_*` and `checkpoint_*`;
   - the core's finalizer and checkpoint code;
   - `FinalizerRegistry.sol` and `CanonicalPostageIssuer.sol`;
   - the commonware dependencies and their `sysinfo` patch.

   The reproducer's runtime helpers move out of `finalizer_frames`.
2. Old payment and custody path. Removes:
   - the `postage-spend`, `postage-zk` and `postage-proof` crates (legacy
     ZK and RISC Zero);
   - the node's `custody_*`, `paid_custody`, `public_sender*` and the
     `postage_*` client, spend and history lanes;
   - the core's `custody_*`, public postage and historical postage context;
   - `PostageIssuer.sol` (replaced by the 1b purchase contract),
     `OperatorSettlement.sol` (V2) and `SubsidyVault.sol`.
3. Jobs: `crates/core/src/jobs.rs`, the node's `postage_jobs`, `jobs.*` in
   MCP, `JobsPanel`, job scenarios and evidence.
4. Identity leftovers: Telegram, site/org OIDC, k-of-n attesters,
   `TrustProfile`, the client's native Google PKCE.
5. Native scenarios and scripts of the removed paths, then `plan.json`,
   specs, AGENTS.md (the V1-C05 A04/H11 section) and IMPLEMENTATION_STATUS.

Progress:
- 2026-09-26: stages 1–5 are done.
  - Node: gone are the finalizer frames, network and services; checkpoints;
    operator proofs; service records and discovery; custody, paid custody
    and the public sender; the postage client, spend and history lanes; the
    L2 wallet; the old mailbox rendezvous (pointer records) and its DHT
    record store; the blocking execution pool. The DHT now only routes to
    peers and refuses record requests.
  - Crates: `finalizer`, `postage-spend`, `postage-zk`, `postage-proof`,
    `l2-adapter`, `l2-types` and the vendored `sysinfo` and
    `commonware-utils` patches.
  - Core, crypto and store: the custody, checkpoint, public-sender, jobs and
    history-recovery code; jobs capabilities (grant wire version 3).
  - Contracts: `FinalizerRegistry`, `PostageIssuer`,
    `CanonicalPostageIssuer`, `SubsidyVault`, `OperatorSettlement`.
  - Desktop: the wallet, checkpoint, history recovery and jobs panels.
  - Tests and scripts: `tests/evm`, `tests/models`, the fault and postage
    scripts; native cases are `chat` and `network`.
  - The node's lib tests went from 480 (about 8 minutes) to 99 (about 30
    seconds). The peer routing fixture gained Identify, so its two process
    tests that failed before now pass.

Kept for phases 1b and 2:
- `NodeRegistry.sol` (units), `RoyaltySplitter.sol`, `GrantIssuer.sol` and
  `crates/grant-book`;
- direct online delivery.

The L2 reads (book purchases, `GrantIssuer`) and the directory's records are
written anew in 1b and 2 rather than kept from the replaced code.

## Phase 1b plan: books and grant rules from the chain

- **`BookShop.sol`.** A book is priced in USD: `priceUsdc` in USDC units
  (2026-09-27: $1.00 for 1000 stamps, a tenth of a cent a message). It is
  paid either way and recorded as the book
  `keccak256(keccak256("AIN_BOOK_V1") ‖ domain ‖ key ‖ salt)` (the node's
  `book_id`) with `(key, bookSize, now + validity)`:
  - `buyWithUsdc(key, salt)` takes exactly `priceUsdc` of the allowed USDC;
  - `buy(key, salt)` takes ETH at the Chainlink ETH/USD feed's rate:
    `quote()` is the price in wei, rounded up; at least the quote is paid,
    the rest goes back to the sender. A feed answer older than
    `maxPriceAge` or not positive stops ETH purchases (`StalePrice`,
    `BadPrice`); USDC still buys.
  - A book id is bought once. Buying someone else's id only gifts them a
    book for their own key.
  - The whole payment goes to an immutable `RoyaltySplitter` (treasury
    share and operator pool), which credits ETH and tokens alike. The shop
    keeps no balance and has no owner.
  - Price, size, validity, domain, splitter, token, feed and the feed's
    maximum age are immutable. The contract is testnet only: changing them
    means a new deployment. On a mainnet the shop would also check the
    L2 sequencer uptime feed.
- **Chain reader in the node.** Ethereum JSON-RPC over HTTP, from a node
  operator's own RPC choice (no attesters).
  - It checks `eth_chainId` once (again after a failed check) and reads
    with `eth_call` at block `head − confirmations`, at least one
    confirmation (a local anvil needs one more mined block).
  - Reads:
    - `books(id)`;
    - the `GrantIssuer` immutables `bookSize` and `maxValidityDays`;
    - `issuerActiveOn(server, day)`, `capForDay(day)` and `today()`.
  - Configured by `serve` flags. Without them a node knows no bought books
    and takes no grants.
- **Holders look books up themselves.**
  - A store, notary statement or replicated entry of an unknown book
    starts one read per book. Until it ends the store is refused as
    `unknown_book` at once (the sender retries), and a replication cursor
    stops before the book's first entry.
  - A book not on the chain (or not confirmed yet), or a failed read, is
    remembered for a minute; the next store or round after that reads
    again.
  - An entry of an absent book holds a replication cursor for up to ten
    minutes from the book's first absent read (a lagging RPC), then is
    passed over (a bad holder's entries never block a mailbox).
  - A learned book is kept in the holder store until every mailbox it
    could pay for has ended (`expires_at` of the period of its end), so
    late copies are still repaired.
- **Grant rules per issuer and day**, read when a grant is checked.
  - An issuer's window and a day's cap cannot change for days before the
    chain's today, so those answers are kept. Answers for today are read
    again when a check needs them more than ten minutes later: an owner
    ending a stolen key from today stops new grants of today within ten
    minutes. Grants already learned stay learned.
  - A node without chain flags takes no bought book and no grant.
  - The clock tolerance is a node constant.
- **Buyer's flow** (owner IPC now, the CLI with phase A's CLI):
  - `coins_buy` answers a payment for a book of the profile's key under a
    fresh salt: book, key, salt, shop, chain id, value in wei, count, an
    EIP-681 URI and the `buy(address,bytes32)` calldata. It answers
    `chain_pending` until the shop's terms were read (a failed read is
    tried again on the next call) and `chain_not_configured` without chain
    flags.
  - One unpaid request at a time: asking again gives it back. A request is
    kept until paid, however late; the node reads it every 30 s during its
    first hour, then every ten minutes, and confirms it in core when the
    chain has it for the profile's key. Requests live in core, so polling
    resumes after a restart.
  - `coins_balance` lists the books (bought or granted, used, end), the
    pending request and the stamps remaining.
- **`coins claim`** against the identity server's existing API
  (`POST /v1/claims`, `GET /v1/claims/{id}`); `serve --identity-server URL`.
  - Core keeps one claim: the request signed by the profile's book key, and
    the server's claim id, login link and end once it answered. It lasts
    until granted, denied or expired, so a restart never loses a grant
    already signed in for.
  - `coins_claim` answers `claim_pending` while the request is being posted,
    then `{status: "open", claimId, loginUrl, expiresAt}`, the same link
    while the claim lasts; `identity_not_configured` without the flag.
  - A request the server did not answer is posted again every 30 s by the
    node itself. A request it refused (`stale_request` after an outage longer
    than the claim TTL) closes the claim; the next `coins_claim` signs a
    fresh one.
  - The node reads an open claim every 5 s until the server decides, also
    past `expiresAt` (the server expires it). A grant is added like
    `add_mailbox_grant` (and put on record with its notaries); a denial
    closes the claim, and so does a grant core refuses (reported as
    `invalid_grant`). Server failures (5xx, unreadable answers) are
    transport errors and never close a claim. `coins_balance` shows the
    open claim and the last outcome.

## Phase 2 plan: the directory (2026-09-27)

Decisions (the open items "receipt account registration" and "directory
publication channel"):
- **Unit commitment.** An operator bonds a unit in `NodeRegistry` with
  `keccak256(keccak256("AIN_UNIT_V1") ‖ domain ‖ transport key ‖ receipt
  account)`: the node's Ed25519 transport key and its secp256k1 receipt
  account. The account is thus bound on-chain for later slashing, and
  the key for the transport.
- **Unit record.** A self-verifying record per unit: transport key, receipt
  account, up to eight addresses and the time it was issued, signed by the
  transport key. It proves itself against the commitment; the newest
  record of a unit wins. A node re-issues its record when its addresses
  change and every six hours.
- **Membership from the chain.** Nodes read the active units' commitments
  from `NodeRegistry` (a new paged view `activeUnits(start, limit)`: a page
  scans `limit` registry positions and skips exited units, so it can be
  short before the end) at one confirmed block, every ten minutes. Swarm
  membership is the active units with a verified record; an exiting unit
  leaves at the next read. A node whose own commitment is active holds as
  that unit. A failed read keeps the last membership: an RPC outage never
  empties the directory.
- **Distribution by pulling.** A node asks a random directory member for
  its whole directory every ten minutes, over the mailbox protocol
  (`Directory` request: the verified records of active units, up to 1024,
  in one answer). While its directory lacks active units it asks every
  connected peer (at most 16, such as bootstrap nodes) every 30 s. No DHT
  records and no gossip.
- A unit whose record lists no address is still a member: listed, not
  dialable, like any unreachable holder.
- **Records are checked one by one.** A malformed, forged or unbonded
  record is skipped without spoiling the rest of a page; a record dated
  more than ten minutes ahead is refused, since it would beat every later
  correction.
- Without chain flags a node's directory stays as set by tests.

## Direct delivery pays (2026-09-27)

- A sender with a book delivers an application message directly as the
  sealed envelope and stamp of its swarm copy (`Delivery.stamped`: the
  conversation, mailbox, period, envelope and stamp). One slot pays for
  both paths; a retry reuses it.
- The recipient's node checks it like a holder: its own incoming mailbox of
  the conversation for that period, the operation over those bytes, and a
  stamp of a book it knows (an unknown book is read from the chain and the
  delivery refused as `unknown_book` meanwhile; the sender retries). Core
  opens the envelope from its author's node (`receive_stamped_from`) and
  answers the usual receipt.
- A slot pays for one message. The recipient records each ticket it takes
  (as a notary does, locally): a different operation on a taken ticket is
  refused as `conflict`, kept as a `SenderEquivocation` and blocks the
  book; the same stamp again (a retry, the swarm copy) is no conflict. It
  also puts the stamp on record with the ticket's notaries, so a slot
  reused across recipients is proven and the book blocked network-wide.
- A node that reads the chain takes no unpaid application message: an
  unstamped delivery may carry only a control message (a Welcome, receipts;
  `receive_control_from`, `PaymentRequired` otherwise).
- A node without chain flags neither sells nor checks payment: it accepts
  unstamped messages (a free local network for tests and private setups).
  A sender without a book sends unstamped, which only such nodes take.
- A stamp of a granted book carries its grant (`StampedDelivery.grant`,
  2026-09-30; absent for a bought book, whose wire is unchanged for nodes
  not updated yet). The recipient checks it as a holder does
  (`offer_grant`): the issuer's rules for its day from `GrantIssuer`, then
  the grant's notaries; meanwhile the delivery is refused as
  `grant_pending` and the sender retries. A grant refused by its rules or
  notaries is refused. A learned grant is kept, so its later stamps are
  checked without the chain.
- Low trust (the owner's decision, 2026-09-30): when a stamp cannot be
  checked now (its book is unknown here and the latest read of its source
  failed: the book from `BookShop`, or a granted book's issuer rules for its
  day from `GrantIssuer`; as on a LAN without the Internet), a one-to-one
  contact's message is taken anyway and shown with low trust (`Message.lowTrust`, also in the
  owner's and the agents' inboxes). Its slot is not recorded: an unchecked
  stamp is not evidence. Stamps defend against Sybil senders; a contact was
  accepted by its owner. A stranger's contact request by ID and group
  traffic are never taken this way; a book the chain answered absent, and a
  grant whose rules were read while its notaries have not vouched yet, are
  refused. Once that source is read again, stamps are checked again; the
  message whose delivery starts the read may still be marked.

## Next

- 1b: books and grant rules from the chain (plan below).
- Order held envelopes by MLS generation.
- The group epoch key comes with groups.

## Open technical items

- Receipt account registration: the registry unit commitment (or a unit
  field) must bind the holder's secp256k1 account for on-chain slashing.
- Book key: the purchase commitment binds today's Ed25519 owner key; the new
  book key is secp256k1 so a double-sign proof verifies on-chain.
- Directory publication channel (DHT records per unit vs. gossip of a
  directory document) and its freshness per registry epoch.
- A holder conflict is two receipts by one key for one ticket with different
  operations; whether the receipt's `holder` unit must also match is open
  while one receipt key binds one unit.
- Reader cost: polling every 5 s from every holder of every conversation is
  a development default. Before the switch-over: holder push or long-poll,
  fewer holders per read once gap detection by the signed conversation head
  exists, adaptive polling.
- Held envelopes are placed once in their sender's MLS order and
  released in order, stopping at the first gap (fixed 2026-09-27 after the
  native spike stalled on it).
- The mailbox follows the MLS epoch: a group commit changes the exporter and
  therefore the mailbox; readers must also read the previous epoch's mailbox
  until its messages drain.
- Delivery phase is still "delivered" once stored; the UI/spec phase for
  "stored at a quorum" is decided at the switch-over.
- Replication is pull-only on a 30 s round; its cursors and the reader's
  completed periods are in memory (a restart re-reads, deduplicated).
- Store retries back off per message and holder; an undialable holder costs
  every queued message its own attempts and the request rate. Back off per
  holder.
- Under load a sender's own notary batches yield to its stores; a sender that
  leaves right after its quorum never sends them (holders still register
  every slot they store). Evidence:
  `evidence/reviews/mailbox-swarm-spike-2026-09-26/`.
- A statement goes on record with the notaries the directory lists when it
  is queued; a grant added before the directory is known is recorded with
  nobody and pays only on its own day. Recompute notaries when the phase 2
  directory arrives.
- Proofs against books a node does not know yet are dropped, not deferred;
  there is no margin for proof propagation delay in the replica rule; how
  holder equivocations are detected (beyond proofs handed to a node) is open.

