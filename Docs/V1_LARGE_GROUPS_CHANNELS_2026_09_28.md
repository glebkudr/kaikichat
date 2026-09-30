# Large groups and channels — September 28, 2026

**Status:** user decisions of 28.09.2026. The base is `implementation/v1`
with discovery merged (`f916d9ce`): read-open groups, subscription,
book-based access, catalog. Complements
[V1_DISCOVERY_2026_09_27.md](V1_DISCOVERY_2026_09_27.md) and
[spec/groups-v1.md](../spec/groups-v1.md).

## Why

A group today is MLS with up to 50 members. Communities of thousands and
channels read by hundreds of thousands are needed. Simply raising
`MAX_MEMBERS` does not work: at 2000, document sizes, holder load, epoch
count, and moderation break. MLS itself handles 2000: tree paths are
logarithmic.

## User decisions (28.09)

- **Groups of up to 2000 members**, three modes, switched by the owner at any
  moment:
  - *public* — anyone can read (open posts, as in discovery), entry through
    the door is automatic;
  - *by request* — content is closed (MLS), entry through the door after a
    decision by an admin or the agent;
  - *private* — invitation only, as now.
- **History for newcomers:** in a public group — the last 24 hours; in
  closed ones (by request, private) — none, a newcomer sees messages from
  the moment of entry.
- **The entry batch runs once a minute, always**, with whatever has
  accumulated. Speed matters more than price.
- **A delay of up to ~20 s** in a 2000-member group is accepted; push
  notifications from holders are not needed in V1.
- **Long offline** (more than 30 days) — return is implicit, without human
  or admin action.
- **The ban list** already exists (`720f71e`).
- **Channels:** only the owner and admins write; up to 1 million subscribers
  (we do not go higher for now); the same three modes.
  - A public channel — subscription without admin involvement; the number
    of subscribers is limited by nothing except load.
  - A closed channel (by request, private) — we build it; the channel key is
    issued to each subscriber individually, **without batches**.
- **Channel history is paid.** The retention term is configured by the owner
  and admins: from 30 days to "forever" (the channel's maintenance price then
  grows over time). The client shows how long history is kept and how much
  the storage costs. Posts vary in size, so history is kept as an archive to
  pack them densely.

## Prices

1 coin = 1 stamp = $0.001. A stamp pays for one document of up to 64 KB for
30 days across 10 holders. Reading is free with an active book.

MLS sizes were measured (`crates/crypto/tests/large_group_measure.rs`,
release, our X25519/Ed25519 suite, id `ain1…` 68 bytes):

| What | Size or time |
|---|---|
| KeyPackage | 348 B |
| Tree for 2000 (Welcome with one newcomer) | 488 KB ≈ 8 parts |
| Tree after removing 30% | 343 KB |
| Commit with 1000 additions | 351 KB |
| Commit with one addition | 590 B |
| Removal commit: 2000 / after removing 30% | 165 KB / 115–119 KB |
| Profile MLS state with one 2000-member group | 4.1–4.9 MB |
| Message encryption / decryption | 22 ms / 9 ms |
| Joining via Welcome | 73 ms |
| Removal commit: create / apply | 52–128 ms / 17–25 ms |

### A 2000-member group

| What | Who pays | Stamps |
|---|---|---|
| Application at the door | applicant | 1 |
| Entry batch: commit, tree in parts, shared response | admin | ≈ 3 / 4 / 6 / 10 at 100 / 500 / 1000 / 2000 members |
| Invitation to a private group | admin | 1 per invitee + batch |
| Message to everyone | sender | 1 |
| Removal or ban (commit 115–165 KB) | admin | 2–3 |

- The batch runs every minute whenever there is at least one application.
  With a continuous flow of applications to a 2000-member group — up to
  ≈ 14,400 stamps (≈ $14) per day for admins.
- Filling 0 → 2000 with rare applications (each in its own batch) ≈ 12,600
  stamps (≈ $12.6); with a flow of 10 per minute ≈ 1,300 (≈ $1.3). Plus one
  coin from each entrant.
- Coins are not transferable in V1: the applicant's coin goes to the holders;
  nobody reimburses the admin's expenses.

### A channel at 200,000 (up to 1 million)

| What | Who pays | Stamps |
|---|---|---|
| Subscribing to a public channel | — | 0 |
| Post to all subscribers | author-admin | 1 |
| Storing history beyond 30 days | admin | 1 per 64 KB of archive per 30 days |
| Card in the catalog | owner | 10 for 30 days |
| Application to a closed channel | applicant | 1 |
| Issuing a key to a closed-channel subscriber | admin | 2: keys and a note to the team on where they sit (decided during implementation, item 10c; 200,000 subscribers ≈ $400) |
| Removing or banning a closed-channel subscriber | admin | 2 |

Example: 10 posts of ~1 KB per day — 300 stamps per month for posts and
≈ 5 archive parts per month.
- History for 1 year: each part is paid for 12 months; in steady state
  ≈ 60 stamps per month; in total ≈ 360 stamps (≈ $0.36) per month.
- Forever: each part is renewed every month. After a year, renewal costs
  ≈ 60 stamps per month; after three years ≈ 180, growing further.
- Storing each post separately is ~10x more expensive: a stamp pays for
  64 KB while a post takes ~1 KB.

## Design

### 1. MLS at 2000

- `MAX_MEMBERS` 50 → 2000; admins remain capped at 49.
- Group Welcomes are sent **without the tree**
  (`use_ratchet_tree_extension(false)`): after applying, the committer
  exports the tree (`export_ratchet_tree`) and puts it as a composite
  document (item 2) into the epoch tree mailbox; the newcomer receives its
  hash in the entry package and joins with
  `StagedWelcome::new_from_welcome(…, Some(tree))`. MLS itself verifies the
  tree's authenticity by the tree hash in GroupInfo. One-on-one conversations
  do not change.
- A removal commit encrypts a new secret to almost every member (the tree is
  nearly empty: only admins commit) — ≈ 82 bytes per member, 165 KB at 2000.
  Such a commit, like a commit with more than ~180 additions, goes as a
  composite document.
- Profile MLS state is a single openmls record store with a
  `MAX_STATE = 32 MB` ceiling. A 2000-member group takes 4–5 MB. Before
  phase 1b every operation loaded and copied the entire state (22 ms at
  4 MB), and a profile held about six such groups. Phase 1b (done 28.09):
  an operation loads from SQL only the profile's shared records (signature,
  KeyPackage, leaf encryption keys, PSK) and the records of its own group or
  conversation — ranges by openmls key prefixes
  (`label ‖ json(GroupId)`, for the proposal queue
  `label ‖ "[" ‖ json(GroupId)`), `record_scope` in crypto and
  `state_records_in` in store. Changes touch only the loaded records; the
  total record counter in the header is recomputed as "before − removed +
  added". `MAX_STATE` limits what a single operation loads, not the whole
  profile. The client knows its scope: an operation on another group is
  `OutOfScope`, as is modifying records outside the scope. `forget_group`
  cleans up the group records that openmls leaves behind on removal (epoch
  keys).
- The body of a signed document is at most 48 KB (`MAX_BODY_BYTES`), so a
  commit inside `GroupCommit` currently fits ~130 additions, and a Welcome
  inside `GroupWelcome` about 350 newcomers without the tree. Anything larger
  goes as a composite document.
- Once-a-minute batches advance the epoch frequently, while MLS keeps keys
  for only three past epochs (`max_past_epochs(3)`). A message or notice
  waiting in the sender queue (a quorum of holders has not stored it yet) is
  resealed by the sender's node into the current epoch when its epoch is
  three or more behind (one more and participants lose the keys) or when
  participants no longer read its epoch's mailbox (after two removals).
  `reseal_stale_group_sends` does this every 30 s before sending:
  - the message id stays the same at the sender; only the wire in the queue
    changes; the signature keeps the original time, pinning in the swarm is
    reset, and delivery gets a new envelope and stamp for the current
    epoch's mailbox; receipts for the old envelope no longer count;
  - only text and MLS notices are resealed: commits, the tree, invitations,
    door knocks, posts of open groups and channels are not; while the
    sender's own commit is pending, the group is skipped;
  - the notice body is stored separately (`groups/notice/{id}`, up to 32 MB;
    a notice with 200 credentials would not fit into a message record) and is
    deleted after the holders confirm;
  - several messages of one epoch are resealed in queue order; when reading
    the mailbox, the sender skips its own resealed envelope.

  Accepted remainder: if the next message of the same epoch is already
  stored and the resealed one came before it, readers hold it as a gap and do
  not accept it; if the old envelope was partially stored and someone
  managed to decrypt it, that reader ends up with the message twice. Both
  cases require the sender to go offline in the middle of sending for three
  epochs.
- The ban list in group data is limited to 256 entries and travels in full
  in every commit with a ban and in every Welcome. For a 2000-member public
  group this is not enough: the limit is raised (per size measurement), and
  if needed the list is moved to a composite document with only its hash
  left in the group data.

### 2. Composite documents

- A document larger than 64 KB is cut into parts; each is a separate signed
  document of the author with kind `Part` and a separate envelope with its
  own stamp. A part's body takes almost the whole envelope (up to 64 KB
  rather than the usual 48 KB body), so the 2000-member tree is 8 parts. A
  part carries the hash of the whole, its own number, and the part count;
  there is no separate manifest. A reference to a document is its hash and
  part count.
- The whole is a signed document of up to 4 MB (64 parts of 65,280 bytes);
  its own signature establishes its author. Parts are assembled by the pair
  "part author, whole hash", so foreign parts with the same hash do not
  interfere with an honest assembly.
- The reader assembles parts in any order and with duplicates, verifies the
  hash of the whole, and only then parses it; the whole leaves assembly once.
  A part with an impossible position (number not less than the part count,
  zero parts, more than 64) is rejected. Incomplete assemblies are bounded
  in count and age.
- Applied to the tree, large commits, large public rosters, and ban lists;
  not needed for archive parts (an archive part is itself ≤ 64 KB).

### 3. The group mailbox changes only on removal

- The epoch `E + 1` mailbox secret equals the `E` secret if the commit
  removes nobody (additions, admin or mode changes, unbans); otherwise it is
  taken from the MLS exporter of epoch `E + 1`. Replacing the same member's
  leaf on return (item 7) does not count as a removal.
- The newcomer gets the current mailbox secret in the entry package. They
  can open envelopes of past epochs in the same mailbox but do not decrypt
  their MLS content and silently skip it (closed groups have no history).
- A member keeps the secrets of the last four *mailboxes* (not epochs).
- The epoch tree and door replies live in separate mailboxes so that members
  do not download them on every batch: the tree mailbox is
  `H(domain, HKDF(mailbox secret, "tree", E), period)`.

### 4. Polling in large groups

- Up to 100 members — as now.
- Larger: the interval grows linearly to 20 s at 2000; each poll goes to one
  random holder; once a minute — to four (4 + 7 > 10: any four intersect
  with the seven that confirmed the record). The node polls mailboxes every
  5 s, so the interval rounds up to that step (a 283-member group — every
  10 s). A message reaches a member no later than about a minute after a
  quorum stored it.
- Channel subscribers: once every 5 minutes from one, once every 30 minutes
  from four. With 1 million subscribers online — up to ≈ 550 requests/s per
  holder; with a realistic 10–20% online — 55–110.

### 5. Modes and the door

- The owner-signed roster (`roster-v3`) gets
  `access: private | request | public` and `kind: group | channel`. Only the
  owner changes it, in their own commit, effective from the epoch after it.
- The **door** exists in public mode and in "by request" mode:
  - mailbox `H("AIN_GROUP_DOOR_V1", domain, G, period)`;
  - the door card is an owner or admin document
    `{G, epoch, X25519 door key, mode}`, published to the door mailbox every
    period (like an intro card). The door key is derived from the group
    mailbox secret, so it changes together with it — on removals;
  - an application is an applicant document sealed to the door key
    `{root key, fresh KeyPackage, reply key, note up to 280 characters,
    return flag}`, one coin;
  - owner and admin nodes read the door every 20 s.
- **Public mode:** any online admin node takes valid applications (not
  banned, not a member, signature valid) into the nearest batch.
- **By request:** applications wait for a decision. All admins and the agent
  see them (`door_requests`, `door_decide {accept | reject}`); accepted ones
  go into the nearest batch, a rejection is sent to admins as a control
  message in the group so the application disappears from everyone's lists.
  They wait at most 30 days (the door mailbox lifetime).
- **Private:** no door, no card in the catalog; invitations as now, via the
  invitee's intro mailbox under their contact policy.
- **Invitation after the batch:** the newcomer receives an invitation as
  usual — their own Welcome (only their entry from the batch Welcome) to
  their intro mailbox. Their node accepts the invitation to the group they
  knocked on, bypassing its contact policy, and forgets the knock after
  joining.
- **Switching:** public → by request or private — content is MLS again from
  the next epoch; in private the door closes, waiting applications are
  dropped, the card is removed. In the other direction — the door card is
  published.

### 6. Batches

- Every minute, any online admin node that has valid or accepted
  applications makes one commit with all of them, after a random delay of up
  to 10 s.
- Two admins in the same minute — the notary resolves the dispute, as for
  any commit; the loser moves the remaining applications to the next epoch.

### 7. Return after long offline

- A node that has not read the group longer than the mailbox retention
  (30 days) considers itself stale and sends an application with the return
  flag on its own: in public and "by request" — to the door; in private — to
  the intro mailboxes of the owner and the admins it knows.
- An online admin node accepts it without question if the applicant is still
  a member (their leaf is in the tree, not removed and not banned): in the
  nearest batch the old leaf is replaced with a new one. An admin decision in
  "by request" mode is not needed for a return.
- The returning member's old group state is reset on the new Welcome (as
  now).
- At least one admin node must be online.

### 8. Channels

- A channel is a group with `kind: channel`. Its MLS members are only the
  owner and admins (up to 50); only they write. Subscribers are not in MLS.
  Roster, bans, commits, notary — as for a group.
- **Public:** posts are open in the public mailbox (like an open group);
  subscription is `follow` without a door and without admin involvement. The
  catalog card has kind "channel".
- **Closed** (by request or private): posts are encrypted with the channel
  key.
  - Subscriber keys are a binary key tree (LKH). Admins derive node keys from
    the common channel seed: `HKDF(seed, node ‖ version)`. The seed and the
    removal log live in the admin MLS group data (like bans); the notary
    orders removals.
  - Leaves are issued without coordination: each admin has their own branch
    of the tree.
  - Entry: an admin (or the agent) issues the subscriber the key packet for
    their path (≈ 1 KB), sealed to the key from the application or
    invitation — one stamp per subscriber, without batches.
  - Removal or ban: a commit in the admin group (the log) and a rekey
    document ≈ 3 KB (new path keys of the removed subscriber, encrypted to
    the keys of neighboring subtrees) — two stamps.
  - The post mailbox is derived from the current channel key and changes with
    it; the rekey document lives in the previous mailbox.
  - The removal log is bounded (≈ 1000 entries); on overflow — a reseed: a
    new seed, new keys for all subscribers under the old node keys
    (≈ 104 bytes per subscriber, ≈ 1,600 stamps at 1 million). Rare.
  - V1 limitation: a removed admin stops writing but can keep reading (they
    know the seed, and the reseed happens under the old keys). To cut them
    off entirely, the owner recreates the channel. The client warns about
    this when removing an admin.
  - Closed-channel history: the past-version keys needed for the archive
    within the retention term are kept as a list under the current key; a
    newcomer reads it and the archive.

### 9. Storage beyond 30 days: re-placement, no holder changes

Decided 28.09 during implementation instead of extending records at the
holders: the same in price, but the holder protocol, replication, and repair
do not change.

- An archive part lives, like any record, 30 days after its period. While
  its post is younger than the retention term, an admin node places the same
  part again into the current-period mailbox — once per ~25 days, one stamp.
- The archive pointer (see item 10), published daily, says where each live
  part lies; the reader fetches a part by period and hash.
- Price: one stamp per part per 30 days, same as with extension. No paying
  ahead: history is kept by an admin node appearing at least once a month;
  if nobody appears within a month, old parts are lost. The client warns
  about this.

### 10. Archive and channel history

- Live posts sit in the mailbox for 30 days, like any record.
- An admin node packs posts into archive parts in order: a part closes when
  the next post would overflow it (64 KB) or when its oldest post is 25 days
  old. The part: `{C, reference to the previous part, posts as is}`; author
  signatures are preserved, so the packer does not need to be trusted; in a
  closed channel the part is encrypted with the current key.
- A part is placed into the channel mailbox of the current period and
  re-placed (item 9) while its newest post is younger than the retention
  term.
- The archive pointer is an admin document `{C, [(period, part hash)]}` of
  all live parts, in the channel mailbox of every period; the reader fetches
  parts by it, so the number of requests equals the number of parts, not
  days.
- Two packers: the part number is the notary key
  `H("AIN_CHANNEL_ARCHIVE_V1", C, number)`; the first to write wins.
- The retention term is in the group data (like bans): 30 days (no archive),
  90, 180, 365 days, or forever; the owner and admins change it. Decreasing
  — they stop re-placing; increasing — they re-place whatever is still
  alive.
- "Forever": admin nodes re-place every live part once per ~25 days. If no
  admin node appears for longer than a month, old parts are lost.
- Display: `channel_storage` → term, number of parts and bytes, stamps per
  month now, paid ahead, growth per month for "forever". The subscriber sees
  the history term.
- Public group: 24-hour history without an archive — a newcomer and a
  subscriber read the public mailbox for the last 24 hours (not "since
  subscribing").

### 10a. Open-group posts are verified without the full roster

Decided 28.09 during implementation: the public roster listed all members
(no more than 64), but a group with a door grows to 2000 and changes epoch
once a minute — a roster per epoch would cost up to ~4,000 stamps per day,
and a subscriber could not verify a post older than two epochs.

- Every member receives a **member credential** on joining — a document of
  whoever added them (owner or admin): `{G, member, joining epoch,
  owner-signed roster}`; the owner issues one to themselves too. The
  credential travels with the invitation and is attached to every post.
- The reader accepts a post if the credential was issued to the post author,
  signed by the `G` owner or an admin of **the newest roster known to the
  reader** (the newest by version of the two: the one inside the credential
  and the last read one — for a subscriber from the public roster, for a
  member their own), and the author was not removed after the credential
  epoch and is not banned. Per-epoch rosters are not needed.
- A removed or demoted admin (this is always an owner commit; the roster
  version grows) no longer issues valid credentials, including
  retroactively. For those they admitted, the owner's node sends its own
  credentials into the group with the same commit (it knows who added whom
  from the applied commits).
- The **public roster** shrinks to `{G, epoch, owner-signed roster, removed,
  banned}`: the last 1024 removals (the epoch after which a credential is
  invalid) and all bans (any credential is invalid). Only a group with more
  than 1024 removals forgets the oldest ones. Published once a day and
  whenever it changes (removal, ban, roster, mode), not on every epoch — for
  one stamp.
- The 24-hour history is counted by the post's storage time at the holders,
  not by the author's date: a subscriber takes what was stored in the 24
  hours before subscribing, a newcomer in the 24 hours before joining.
- Channel: only the owner and admins write; their credentials are issued by
  the owner.

### 10b. Channels: decided 28.09 during implementation

- **Team.** The channel's MLS group members are the owner and admins; there
  are no others. Whoever the owner adds to the channel joins as an admin by
  the same commit (`roster-v3` with `kind: channel`, the kind does not
  change); a channel has no explicit admin list, leaving the team means
  leaving the channel. Therefore only the owner changes the team. Every
  commit reader checks that the channel has no members outside the admins. A
  channel has no door: a public channel is read by subscription, without an
  application.
- **Who writes.** A channel reader takes a post only if the author is the
  owner or an admin of the newest roster known to them (besides the rules of
  item 10a).
- **History term** — in the group data next to the bans (`bans-v2`): 30 days
  (default, no archive), 90, 180, 365, or forever; the owner and admins
  change it by commit. The public roster carries the term (`retention`; for
  groups — 24 hours), the subscriber sees it.
- **Archive without a pointer and without a notary.** A part is an unsigned
  record `0x03 ‖ archive-v1 {C, posts as is}` in the day's public mailbox,
  up to 64 KB; posts are each verified by their own credential, so the
  packer does not need to be trusted. Team nodes collect posts (their own
  and the ones read), close a part on overflow or when its oldest post is 25
  days old, and place it if the term is longer than 30 days; increasing the
  term takes still-live posts into the archive. Having read someone else's
  part, the node does not repack its posts; having read its copy within the
  last 25 days, it does not re-place it itself. Collisions within seconds
  are possible; the reader discards duplicates by part and post hash.
- **Placement is confirmed by reading.** `channel_archive` keeps emitting
  every part that is due for placing, again and again under the same stamp
  of the same day (the stamp slot is determined by the mailbox, period, and
  record), until a team node stores it with a quorum or reads its copy in
  the public mailbox (it reads it on every poll). Storing by a quorum and a
  read copy — one's own or another admin's — count as placement for that
  day. A lost send does not lose a part and costs no extra stamp.
- **Re-placement:** a part is placed again every 25 days into the current
  day's mailbox while its newest post is younger than the term (forever —
  always). An archive pointer is not needed: live copies of all parts sit in
  the mailboxes of the last 30 days.
- **A channel subscriber** reads the last 30 days of mailboxes once (live
  posts and parts); afterwards — like a group subscriber, but less often
  (item 4).
- **Ordering at the subscriber:** history arrives after the new, so the
  subscription feed is shown by post writing time.
- **Not covered by rig tests (residual risk):** a node subscribed to both a
  group and a channel at once (separate timers), and history reading when
  some holders did not receive the record (history goes to four holders).
- **Price:** `channel_storage` → term, number of live parts, their bytes,
  stamps per month (`⌈parts × 30 / 25⌉`), and parts added in the last 30
  days (growth for "forever").

### 10c. Closed channels: decided 28.09 during implementation

- **Key tree** of depth 32 (up to 4 billion leaves). A leaf is
  `branch (8 bits) ‖ number (24 bits)`. The owner assigns each team member a
  branch in the group data (`branches`) when creating the channel or in the
  same commit that adds them to the team; branches are not reused. The
  number is one past the largest known leaf of its branch: the node that
  issued the keys reports each subscriber to the team with a notice in the
  group (`["subscribed", id, leaf]`), so any admin can remove a subscriber.
- **Tree node key** — `HKDF-SHA256(seed, C ‖ node ‖ version)`; a node's
  version is the number of removed leaves of the log under it. The seed,
  generation, and removal log live in the group data next to the bans
  (`bans-v3`); only the team sees them. The channel key is the root key.
- **The closed-channel mailbox** is derived from the root key and changes
  with it. Its records are `0x04 ‖ sealed-v1 {C, generation, root version,
  nonce, AEAD(public mailbox record)}`: inside are the same posts with
  credentials and the same roster as a public channel. Closed channels have
  no archive (the retention term does not change).
- **Key issuance** — without batches: the `ChannelKeys` packet (leaf path
  nodes with versions and keys, ≈1.5 KB, owner roster, name) travels like an
  invitation — to the subscriber's intro mailbox and directly, with a
  receipt; one stamp, plus one more for the note to the team. A private
  channel — an admin issues by card; a "by request" channel — an admin
  decision on a door application issues the keys immediately, without a
  commit. The subscriber accepts keys for a channel they knocked on, or per
  their contact policy, like an invitation; under a manual policy the keys
  await their decision.
- **A newcomer reads from the moment keys are issued:** closed channels have
  no history; posts stored before key issuance (the packet signature time)
  are not fetched; what was written while the keys were in transit or
  awaiting the subscriber's decision is theirs.
- **Kind and access do not change** (owner decision of 29.09: a channel
  cannot be switched between public and closed): a channel stays public or
  closed as created (switching between "by request" and private is allowed).
- **A removed admin stops writing:** the roster is published as a sealed
  record in the key mailbox; the subscriber checks credentials against it.
- **Removing a subscriber** (or a ban) — a commit appending a leaf to the
  log; its author publishes the `ChannelRekey` document (signed by the owner
  or an admin, roster inside): new path node keys, encrypted to neighboring
  subtrees and to the new keys of nodes below, ≈3 KB, one stamp, into the
  previous key's mailbox. The subscriber applies documents strictly in log
  order; one who missed a document (offline longer than 30 days) knocks
  again.
- **Reseed** — when the log has grown to 1000 entries, or by owner/admin
  decision: a new seed and generation, the log is empty. The commit author
  publishes reseed documents: each occupied node's new key under its old key
  (≈70 bytes per node), in 64 KB batches, into the previous key's mailbox.
  Removed entries (in the log) get nothing. Such a reseed does not cut off a
  removed admin: they know the old seed and all old keys.
- **Removing an admin (done 28.09, owner decision: "reseed onto subscriber
  keys").** Having received the keys, the subscriber creates their own X25519
  key for the channel and publishes it once, paying a stamp: the
  `ChannelSubscriber` document (kind 19) `["subscriber-v1", C, leaf, key]`,
  signed with their root key, sealed into the channel key mailbox (`0x06`).
  Again — after a new key issuance or a soft reseed (the owner might not
  have read the key before the generation change), but not after a hard one.
  The owner stores keys by (signer id, leaf) and uses a key only if the
  team's notes give that leaf to that id.
  When the owner removes a team member, a hard reseed is "owed": a commit in
  which the removed one no longer participates (they read the group data in
  the removal commit), with a new seed; a commit that both removes and
  reseeds, or a soft reseed after the removal, does not settle the debt. The
  `keys-v2` documents are signed by the owner only: each occupied leaf's new
  key is sealed to the subscriber's key (X25519 with an ephemeral key, HKDF,
  ChaCha20-Poly1305), each node's key is under the new keys of its occupied
  children; nothing is sealed with keys known to the removed one. The
  subscriber accepts them from any version of their generation, in any
  document order, and if the removed admin managed to move them with their
  own reseed under an old roster — also from the generation before that,
  since the owner's reseed roster is newer than the one under which the
  subscriber took the current generation; after that this roster becomes
  known and the removed one moves nobody. Subscribers whose key the owner
  did not know get keys re-issued by the owner's node (two coins each); an
  already-subscribed one accepts them without question if the owner issued
  the keys. Cost: one stamp per subscriber once, and about one stamp per
  ~300 subscribers when removing an admin.
  Security review (28.09, four passes) and what was done: the subscriber key
  record is sealed with their leaf key (other subscribers cannot see who is
  subscribed); a low-order key is not accepted and does not block
  publication; a former team member loses their subscriber leaves forever
  (they leave the map on a hard reseed); team notes cover only leaves of
  their own branch and ones not taken by another id; key packets are ordered
  by (roster, generation, version) only for the owner's packets, for an
  admin strictly by generation; the subscriber remembers the roster of
  everything accepted and the maximal roster of their keys, so a forgery by
  the removed admin under an old roster does not pass after any document
  under a new one; hard-reseed accumulation survives a generation change;
  keys from still-unknown ids for a leaf — at most four and only near the
  issued leaves of the branch.
  Residual risks: fake subscribers in the removed admin's branch (they
  admitted them themselves — the owner can remove them); a subscriber more
  than two days behind on removals cannot open reseed documents and waits
  for re-issuance; leaves that two ids occupied before this fix are given to
  nobody, and the owner is not warned; the card search for re-issuance
  repeats once a minute.
- **Security review (28.09) and what was fixed:** a closed-channel record
  without the tree does not leak into the open mailbox; the tree neither
  appears nor disappears after creation; reseed documents — no more than 700
  records, the publication plan is stored with the commit; notes to the team
  are not lost, a duplicate application at other admins is dropped, an id
  may have several leaves, branch leaves are not issued twice; keys and
  updates are accepted only under a roster no older than known; the knock is
  bound to G; old key packets do not roll the subscriber back; the
  closed-channel archive is not assembled; key updates are sealed with the
  previous key.
- **Remaining V1 limitations:** between removing an admin and the hard
  reseed (a minute or two, two commits) the removed one reads whatever was
  written in time; a subscriber whose key the owner did not read gets keys
  re-issued for a fee; a lagging author who has not yet applied the removal
  can seal a post with the previous key; "from the moment keys are issued"
  is a client rule, not cryptography (the channel key is shared until the
  nearest change).
- **Not covered by tests (residual risk):** automatic reseed at 1000 log
  entries (the same path as a decision-driven reseed), forged documents from
  a non-admin (rejected by signature and roster), finishing reads from the
  previous mailbox after a key change (the node's job).

### 11. Interfaces

- IPC: `create_group {kind, access}`, `change_group {…, access, retention}`,
  `join_group {card | G + owner, note}` (door application), `door_requests`,
  `door_decide`, `channel_storage`; subscription is the existing
  `follow_group`.
- CLI `kaiki groups …` and `kaiki channels …`, MCP tools, skill, GUI: modes,
  the door, application decisions, history term and price.

## Phases

Each phase is tests first (RED), then an independent backend-test-critic,
then code; the affected clusters are checked, not the whole suite.

1. **MLS at 2000.** Measurement at the MLS level (done: commit, Welcome, and
   tree sizes at 2000, including after removing 30%; profile state and
   operation times). Welcome without the tree, the tree separately.
   1b. An operation loads only its own group's records; the state ceiling is
   higher.
2. **Composite documents** in the core and the node.
3. **The mailbox changes only on removal**, the tree mailbox, the entry
   package, the polling schedule for large groups.
4. **Modes and the door:** `roster-v3`, applications, once-a-minute batches,
   application decisions, auto-accepting the Welcome, return after long
   offline.
5. ~~Extension at the holders~~ — replaced by re-placement (item 9); the
   holder protocol does not change.
6a. **Member credentials** and the compressed public roster (item 10a);
   24-hour history in a public group.
6. **Public channels:** `kind: channel`, the archive, history term and
   price, the subscriber schedule, the "channel" card; 24-hour history in a
   public group.
7. **Closed channels:** key tree, issuance, rekey, reseed.
8. **CLI, MCP, skill, GUI**; a native run.

## Checks

- **MLS:** sizes and times at 2000 within seconds; joining via a Welcome
  without the tree using the tree from the mailbox; a substituted tree is
  rejected.
- **Composite documents:** assembly from parts in any order, a foreign or
  substituted part, assembly limits.
- **Rig:** the door (public, by request), the once-a-minute batch, two
  admins in one minute and the notary, the mailbox changing only on removal,
  return after long offline, the channel archive and history reading, the
  closed channel: entry, removal, reseed.
- **Holders:** extension (the `1..k` chain, retry, conflict, expiry),
  replication and repair of extended records.
- **Load:** requests per holder by the polling schedule in the rig with
  simulated readers, without MLS.
