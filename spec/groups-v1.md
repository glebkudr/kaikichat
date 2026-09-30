# Groups (V1 phase A, step 5; V1-AF04)

A group is an MLS group of up to 2000 profiles with a name and roles: one
owner (its creator), admins and members. Everyone sends; the owner and
admins add, remove and ban; only the owner changes who is an admin. Every message
and commit is paid like any other. Commits are ordered by the ticket notary,
so members never fork, and a member removed from the group reads nothing
written after its removal.

## The roster

The roster is a document signed by the owner's root key:
`{group ref, version, admins}`, where the group ref is
`G = H("AIN_GROUP_V1", domain, owner root, group id)`.

- Version 1 is made with the group and names no admins.
- Only the owner makes a new version, one higher, inside its own commit: a
  change of admins replaces the list; removing an admin drops it from the
  list in the same new version.
- Admins are members; only the owner removes an admin.
- The owner is never removed and never leaves in V1.

## Bans

The bans ride in the MLS group context, in a private-use extension
(`0xF1A0`, the group data) that the group context's required capabilities
name: `["bans-v1", [[id, by owner], …]]`, one entry per network id (its 32
bytes), ascending, at most 256. A commit that changes them carries a
GroupContextExtensions proposal; any other keeps them. So every member
agrees on them, and a newcomer reads them from its Welcome.

- Whom one may remove one may ban: the owner any id but its own, an admin
  any but the owner's and the admins'. An entry records whether the owner
  made it; only the owner lifts or re-marks the owner's bans, and the owner
  and admins lift an admin's.
- No banned id is a member: banning a member removes it in the same commit
  (the owner banning an admin drops it from the roster too), and nobody adds
  a banned id until it is unbanned (`banned`). An id that never was a member
  can be banned ahead.
- A client made before bans lists no such extension among its leaf's
  capabilities: it is not invited into a group that has bans, and a group
  with such a member takes none (`member_outdated`). A group made before bans
  never takes any; the owner makes a new one.

## The group mailbox

Every message and commit goes, sealed like a conversation's envelopes, into
the group's mailbox of each period. Its secret comes from the MLS exporter
(context `AIN_GROUP_MAILBOX`, the domain and the group id) of the epoch of
the last commit that removed someone, or of the group's first epoch: a
commit that removes nobody keeps the mailbox, one that removes a member
gives the next epoch a fresh one, so a removed member cannot find, let
alone read, what follows its removal
(Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 3). One copy and one
stamp pay for a message to the whole group. A newcomer gets the mailbox's
secret with its invitation; what was written before it joined lies in the
same mailbox and is passed over unread. Members keep the secrets of their
last four mailboxes, each from the epoch it began, and read the newest two:
a message or commit is always stored in the mailbox of the epoch it was
written in, however late it goes out. Once a reader applied the commit that
removed a member, it takes nothing more of that member's from the epochs it
was in, not even a message written before; what it read before stays. A
removed member thus cannot go on writing into the epochs it still knows. Group messages and commits have no
direct copy; invitations do.

While a commit of its own waits for the notary, a member sends nothing else
in the group (`group_busy`, retryable).

## Commits

1. An owner or admin makes a commit at epoch `E` (adds with the new
   members' KeyPackages from their intro cards, removals) and carries, in
   the commit's authenticated data, the roster in force after it.
2. It stores the commit, with a commit statement for round 0, in the group
   mailbox of `E` at a quorum, paid.
3. It puts the statement on record with the ten notaries of
   `H("AIN_GROUP_COMMIT_V1", G, E, round)`. A statement wins a round when at
   least seven of them recorded it first; no two can.
4. The winner applies its commit, moves to `E + 1`, and only then sends the
   Welcome to each new member through its intro mailbox (sealed to its card,
   paid) and directly. A loser drops its commit and sends nothing; it applies
   the winner's when it reads it and makes its change again at `E + 1`.

A group's Welcomes carry no ratchet tree (every committer keeps it apart).
When the tree fits the invitation beside the Welcome, it goes in it;
otherwise the winner signs a `GroupTree` document `{G, E + 1, tree}` and
queues it, with the commit's application and so before any invitation,
into the tree's own mailbox: `H(domain, secret, period)` with the secret
`SHA-256("AIN_GROUP_TREE_V1" ‖ group mailbox secret ‖ E + 1)`, sealed with
it. Members never read it; the invitation names it by its whole's hash and
number of parts.

### Documents in parts

A commit or tree too big for one envelope is signed as one document of up to
4 MB and sent in parts (spec/wire/signed-document-v1.md, "Documents in
parts"), each its own envelope and stamp. It stays one message in the
outbox; the swarm outbox lists one delivery per part not yet stored
(`{message id}:{index}`), and the message is stored — and a commit's claim
put on record — only once every part is. A reader keeps parts per author
and whole, a few wholes per group (the oldest gives way), and takes the
document once it is whole.

Rounds: when a round's answers show that no statement can reach seven (all
ten answered, or too few are left to lift any), each committer still
contending signs its statement again for the next round and stores the new
statement in the group mailbox of `E` too, so readers can put it on record.
The epoch's commit is the winner of its first round that has one. An epoch
whose round ends without a winner after every contender left before signing
again waits for one of them; the owner can make a new group.

Readers apply, for each epoch, only the commit named first; a decision that
comes before its commit is read waits for that commit, and any other commit
of the epoch is not applied. A reader that finds a commit whose statement is
not yet on record puts that statement on record itself (it keeps the
committer's signed statement to do so). The first recorded commit is always
one stored in the mailbox, so the group never waits for a committer that
went away after storing its commit.

### Talk that waited too long

Members keep the MLS keys of three past epochs. A text or notice of the
sender still waiting in its outbox (no quorum of holders stored it) is
sealed again in the group's current epoch once its epoch is three or more
behind, or once members no longer read the mailbox of its epoch. It keeps
its id and its first `issued_at` at the sender, drops its pin in the swarm
and takes a new envelope and stamp for the current mailbox; receipts for the
old envelope no longer count. Commits, trees, invitations, knocks and posts
in the clear are never sealed again, and a group with a commit of the sender
waiting is left until it is decided. Messages of one epoch are sealed again
in the outbox's order. A reader passes over an application message signed by
its own root key. Accepted: a later message of the same epoch already stored
stays behind the gap its predecessor left; a reader who opened a partly
stored first envelope sees the message twice.

### The commit statement

A document signed by the committer's root key (kind GroupCommit):
`{owner root, group id, epoch, round, commit hash, roster}`, the roster
being the owner-signed document in force before the commit. Its notary key
is `H("AIN_GROUP_COMMIT_V1", G, epoch, round)`.

A notary records it only when:
- the roster is signed by the owner root of the group ref;
- the committer is the owner or one of the roster's admins.

So no plain member can take an epoch first. A demoted admin still holding an
older roster could; V1 accepts that.

Readers also check the commit itself: the roster in its authenticated data
is a document signed by the group's owner (checked by the caller), and
`agentic_protocol::group::check_transition` holds:
- the committer is the owner or an admin of the current roster;
- its roster is the current one unchanged, or the next version (one higher)
  made by the owner;
- an admin removes neither the owner nor another admin, and nobody removes
  the owner;
- a removed admin is gone from the roster after the commit;
- `agentic_protocol::group::check_bans` holds for the bans before and after
  it (see Bans), and no member after it is banned.

A named first commit that breaks these rules stops the group at that epoch;
the owner then makes a new group.

## Joining

The invitation names the group, the inviter, the roster, the invitee's own
Welcome (the batch's Welcome with only its entry, so an invitation stays a
few kilobytes however many were added with it), the group mailbox's
secret, the epoch it joins and its tree (inline, or the whole and count of
the tree document in parts). It meets the
recipient's contact policy (spec/contact-by-id-v1.md) like a contact request,
except that being a contact lets the inviter in rather than out:
- an inviter that is a contact or on the list joins at once;
- with `all`, others join within the daily limit;
- otherwise the invitation waits, listed with the group's name and id
  (`group` in the waiting request); one waits per inviter and group.

An invitation whose tree came along joins at once. One whose tree goes in
parts, once let in, waits (`AwaitingTree`, kept across restarts, at most as
long as the tree's mailbox lives): the node reads the tree's mailbox for
the current and previous period, takes only the inviter's parts of the
named whole, and joins when the tree is whole.

A member removed earlier can be invited again, unless it is banned: its old
state of the group is dropped and it reads from the epoch it joins.

## At the node

- A committer's node stores its commit at a quorum first and only then puts
  its claim on record; a reader's node puts on record every undecided claim
  it read. Each asks the claim key's ten notaries and counts the first
  claim each recorded: a commit with seven is the epoch's commit
  (`decide_group_commit`); when none can reach seven, a committer still
  contending signs its claim for the next round (`renew_group_claim`) and
  stores it in the group mailbox after a random delay of up to two seconds,
  so two contenders rarely split again; claims with no answer are asked
  again. With fewer than seven notaries in its directory a node waits: no
  round is decided or given up.
- Notaries keep a commit claim at least as long as its epoch's mailbox lives
  (31 days after the period it was put on record in, and two more): a late
  reader never finds another claim first. Claims carry no stamp; a notary
  takes them within its request-rate limits, and only a group's owner and
  admins can make one.
- It reads the group mailboxes of the newest two epochs like conversation
  mailboxes (the current and previous period each poll, closed periods since
  the last one read through caught up).
- Owner IPC:
  - `create_group {name, members: [network id], operationId}` → the group,
    once every member's card is read (`card_pending` meanwhile,
    `card_not_found`);
  - `change_group {groupId, add, remove, admins?, ban?, unban?, operationId}`
    → `{epoch, commit, messageId}`, the commit made and waiting for the
    notary; `group_busy` (retryable) while another of this profile's commits
    waits; `not_allowed` for a role that may not make it; `banned` for adding
    a banned id; `member_outdated` (see Bans); the same operation id is the
    same commit, and asked again after it lost, a new one;
  - `groups {}` and `group {groupId}` → what a member sees (`unknown_group`),
    its bans as `banned: [{id, byOwner}]`;
  - messages go with `send_message` to the group's id.

## The door

Decision: [Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md](../Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md),
part 5. The owner-signed roster's `access` is private (0), public (1) or by
request (2); only the owner changes it. A public group is read by anyone
(open-read groups, below) and anyone joins it through its door; one by
request keeps its talk MLS-closed and its door lets people in once the
owner or an admin decided; a private group has no door.

- The door mailbox of `G` for a period is
  `H("AIN_GROUP_DOOR_V1", domain, G, period)`. Its entries are
  `0x01 ‖ door card` and `0x02 ‖ door key (32) ‖ ephemeral X25519 key (32)
  ‖ ciphertext`, an application sealed like a contact request
  (spec/contact-by-id-v1.md) with the door key in place of the card id.
- The door card is a document of kind `GroupDoor` (16) signed by the owner
  or an admin: `["door-v1", group id, owner root, owner-signed roster, name,
  door key]`. An applicant takes it when `G = group_ref(owner, group id)`,
  the roster is the owner's for `G`, its access has a door, and the signer
  is the owner or one of its admins. The door key is X25519 from
  `SHA-256("AIN_GROUP_DOOR_KEY_V1" ‖ group mailbox secret)`: it changes
  with the mailbox, and applications sealed to any kept mailbox's key open.
- An application is a `GroupControl` document of the applicant:
  `[9, group id, its intro card envelope, note (≤ 280 characters), rejoin]`,
  one stamp, into the door mailbox of the period it goes out in.
- The owner's and admins' nodes publish the card into each period's door
  mailbox, read the door (current and previous period) about every 20 s,
  and once a minute make one commit adding everyone let in who is not yet
  a member, not banned, and whose card holds (at most 300). In a public
  group an applicant is let in when read; by request it waits
  (`door_requests`, `door_decide`); a refusal holds for applications issued
  up to it. Each node keeps its own intake; the notary decides between
  batches made at once, and the loser adds the rest at its next one.
  Closing the door drops who was let in or waiting.
- The applicant's node takes the invitation of a group it knocked on
  whatever its contact policy, and forgets the knock once in.
- Owner IPC: `change_group {…, access: public | request | private}`;
  `join_group {groupRef, note?, operationId}` → `{groupId, messageId}` once
  the door card is found (`card_pending` meanwhile, `card_not_found`);
  `door_requests {groupId}` → `[{requestId, networkId, note, receivedAt,
  rejoin}]`; `door_decide {groupId, requestId, accept}`.

- A refusal goes to the group as a notice — an MLS application message
  whose plaintext is `0x00 ‖ ["door-refused", network id, issued at]`,
  never shown as talk — and every owner's and admin's node holds it too.

## Falling behind

A member that misses commits for longer than the mailboxes are kept (30
days) cannot catch up. Its node finds such a group stale by itself: the
first period after the one it read through (or joined in) is no longer
kept; moving its reading past such a period marks the group behind until
it joins anew (`set_swarm_read_through` takes the time for this). Once a
minute, once a day per group while it stays behind, the node asks for its
place again with no human action: a rejoin application at the group's door,
or — when no door card is found — sealed into the owner's intro mailbox
(an intro request carrying the application). An owner's or admin's node
lets a current plain member back without a decision whatever the mode,
and its next batch replaces the member's leaf in the same commit
(`GroupChange.replace`): no removal, so the mailbox stays. The member's
node takes the invitation of the group it still holds, replacing its state.
A member removed or banned meanwhile does not return so; an owner or admin
who fell behind is not replaced so in V1.

## Limits

- 2000 members; one commit adds at most 300 (their Welcome still fits an
  invitation).
- The group name follows the contact name rules.
- The roster names at most 49 admins.
- At most 256 bans.

## Open-read groups

Decision: [Docs/V1_DISCOVERY_2026_09_27.md](../Docs/V1_DISCOVERY_2026_09_27.md),
part 2.

- The owner-signed roster is `roster-v2`: `{group ref, version, admins,
  access}`, `access` private (0) or public (1). Only the owner changes it,
  in its own commit, in a new version; a new group is private. It applies
  from the epoch after that commit.
- A public group's messages are posts: documents of kind `PublicPost` signed
  by their author's root key, `post-v2` `{G, epoch, operation, text,
  membership}` (`operation` the SHA-256 of the operation id, `membership`
  the author's certificate), stored in the clear as `0x01 ‖ post` in the
  public mailbox `H("AIN_PUBLIC_GROUP_V1", domain, G, period)`, one stamp
  each. Commits still go through the epoch's mailbox and the notaries.
- A certificate of membership is a document of kind `GroupMember` (17),
  `member-v1` `{G, member, epoch, roster}`: whoever let the member in says
  it is a member of `G` since `epoch` (the epoch its Welcome joins);
  `member` is the network id's digest. The owner's stands alone (`roster`
  empty); an admin's carries the owner-signed roster in force, which names
  it an admin. It comes with the invitation (`GroupWelcome`, field 11); the
  owner and admins sign their own for each post. When the owner demotes or
  removes an admin, its node sends those that admin let in certificates of
  its own in a notice among the members (`["member-certs", [cert…]]`, at
  most 200 each); it knows whom from the commits it applied.
- The owner and admins publish a `PublicRoster` there, signed by
  themselves: `public-roster-v2` `{G, epoch, roster, removed}` with the
  owner-signed roster inside and `removed` the latest 1024 removals — a
  network id digest and the last epoch it was a member in — and every ban
  (epoch `2^64 − 1`), ascending by digest (`0x02 ‖ roster`). It is
  published once a day and when what it says changes (a removal, a ban, a
  new roster, a closing), not on every epoch; in the epoch and on the day
  it closed, and the next, the closing roster.
- A reader takes a post whose certificate is for `G` and for the post's
  author, stands on the owner it knows (the roster's signer, or the issuer
  of a lone certificate), was signed by that owner or by an admin of the
  newest roster the reader knows (the newer, by version, of the one inside
  and its own), and whose author was not removed at or after the
  certificate's epoch nor banned. Once a removal is known nothing more of
  that member under an older certificate is taken, not even a post dated
  before. A member goes by its own view of the group (its roster, its
  removals, its bans) and also knows the author as a member; a post by
  someone it does not know yet, under a certificate of a later epoch,
  waits until the member moves on. A follower takes a roster whose inner
  roster is signed by the owner it follows, for this group, by a committer
  who is that owner or one of its admins, not older than the newest it
  took; it keeps the newest roster, the removals (the latest 2048) and the
  newest roster's bans. A closing roster ends the follow.
- A follower takes posts the holders stored within the day before it
  followed, a newcomer those stored within the day before it joined; the
  day is counted by the holders' storage time, not the author's date.
- A member reads the public mailbox (current and previous day) every poll;
  a follower once a minute from one holder, from four every fifth time.
- The public roster also carries how far back followers read
  (`retention`, days, 0 for ever): a day for a group.
- Owner IPC: `change_group {…, access}`, `follow_group {group, owner,
  name}` → `{id, name, owner, since, closed, kind, retention}`,
  `unfollow_group {groupId}`, `follows {}`; `group` and `groups` answer
  `access`, `groupRef`, `kind` and `retention`.

## Channels

Decision: [Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md](../Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md),
parts 8–10b.

- A channel is a group whose owner-signed roster is `roster-v3` `{group
  ref, version, admins, access, kind = 1}`; a group's stays `roster-v2`.
  The kind is set when it is made and no commit changes it.
- Its members are its owner and admins only (at most 50): whoever the owner
  adds joins as an admin in the same commit, removal from the team is
  removal from the channel, admins are never named apart, and only the
  owner changes the team. A reader of a commit refuses one that leaves a
  member outside the owner and admins. A public channel has no door: it is
  followed.
- A reader takes a channel's post only from its owner or an admin of the
  newest roster it knows, besides the certificate rules of open groups.
- How long it keeps its history is group data beside the bans: `bans-v2`
  `["bans-v2", entries, days (0 for ever)]` for anything but the default 30
  days (`bans-v1`). The owner and admins change it among 30, 90, 180, 365
  days and for ever; the public roster carries it.
- Archive: its team's nodes keep the posts they write and take, and pack
  them, oldest first, into parts `0x03 ‖ ["archive-v1", C, [post…]]` of at
  most 64 KiB, unsigned: every post keeps its author's signature and
  certificate. A part closes when the next post would overflow it or when
  its oldest post is 25 days old. With a retention above 30 days it is laid
  in the public mailbox of the day for one stamp, and again 25 days after
  its latest copy while its newest post is younger than the retention (for
  ever: always); raising the retention archives what is still alive. A
  part is laid that day once the node stored it at a quorum or read a copy
  of it back; until then the same part is handed out under the same stamp.
  A team node that reads another's part does not pack its posts again.
- Readers take a part only for a channel, its posts one by one; a part
  read before is passed over. A follower's history is the channel's last
  30 days (a group's: a day); followed conversations are shown in the order
  posts were written.
- At the node: a channel's follower reads every 5 minutes from one holder,
  from four every sixth time, and reads the last 30 days' mailboxes once,
  in three rounds from four holders; the team's nodes look for parts to lay
  every 5 minutes.
- A closed channel (access private or by request) keeps a key tree in its
  group data: `bans-v3` `["bans-v3", entries, days, [generation, seed,
  [removed leaf…], [[digest, branch]…]]]`. The tree is made with the
  channel and neither comes nor goes after; removals are only added within
  a generation; a reseed starts the next with none. Leaves are `branch ‖
  24-bit number`, a branch per team member. A node's key is
  `HKDF-SHA256(salt "AIN_CHANNEL_KEY_V1", seed, C ‖ generation ‖ node ‖
  version)`, its version the removed leaves under it; the channel key is
  the root's.
- Its entries go sealed into the mailbox derived from the channel key:
  `0x04 ‖ ["sealed-v1", C, generation, version, nonce, AEAD]`
  (ChaCha20-Poly1305 under a key derived from the channel key, the nonce
  from that key and the entry), holding a post, the roster, or a key
  update (`0x05 ‖ document`). Without its key tree the channel sends
  nothing; it keeps no archive and no retention other than 30 days, and
  stays closed.
- Keys go to one subscriber at a time, like an invitation: `ChannelKeys`
  (packet 10: group id, card, name, owner, roster, generation, leaf, root
  version, the path's 33 versions and keys) to its intro mailbox and
  directly; the team is told where they hang by a notice `["subscribed",
  id, leaf]`, kept until sent. A subscriber takes keys from the owner or an
  admin under a roster no older than the one it knows, for a card of its,
  not behind the keys it holds; let in when it knocked at that very
  channel's door, or by its contact policy, else waiting for its decision.
  It reads from when the keys were given, no history before.
- A removal or ban (`change_group {unsubscribe}` or `ban`) adds the leaves
  to the log; the commit's author publishes, from a plan kept with the
  commit, a `ChannelKeys` document (kind 18) per leaf: `["keys-v1", C,
  generation, root version, leaf, [sealed…], roster]` — the path's new
  keys, each under the key beside and under the new key below, from the
  leaf up — and for a reseed (`reseed`, or a log past 1000) documents of
  the next generation's key of every occupied node under its key now, at
  most 700 each; all sealed under the key before the commit, in its
  mailbox, until stored at a quorum. Subscribers apply them in the log's
  order, holding later ones; they read the current key's mailbox and those
  of keys replaced within two days.
- A subscriber's own key: on taking keys a subscriber makes an X25519 key
  for the channel (kept across keys given again) and publishes, paying one
  stamp, a `ChannelSubscriber` document (kind 19) `["subscriber-v1", C,
  leaf, key]` signed by its root key, sealed like a post (`0x06 ‖
  document`) in the channel key's mailbox — again after keys given anew or
  a reseed under the old keys, not after one onto its key. The owner keeps
  each by who signed it and the leaf named; it counts only for a leaf the
  team's notes give that id.
- A team member out of a closed channel knows its seed, and the commit
  that took it out carries its group data to it. So once that commit is in,
  a reseed onto subscribers' own keys is due at the owner: a commit of its
  own after it (not one that also takes a member out; a reseed under the
  old keys meanwhile leaves it due), with a new seed. Its documents are
  `["keys-v2", C, generation, root version, null, [[node, sealed]…], roster,
  ephemeral key]`, signed by the owner alone: each occupied leaf's new key
  sealed to its subscriber's own key (X25519 of the ephemeral key and the
  subscriber's, HKDF, ChaCha20-Poly1305), each node's under the new keys
  of its occupied children; nothing under a key the member knew. A
  subscriber takes them from any version of its generation, in any order,
  and from the generation before when a former admin moved it on under a
  roster older than the one they come under; that roster is then the one
  it knows. Subscribers of whom no own key was known get their keys again
  from the owner's node (`ChannelKeys`, two coins each); a subscriber
  already following the channel takes keys given again without asking.
- Owner IPC: `channel_subscribe {groupId, members, operationId}`,
  `change_group {…, unsubscribe, reseed}`.
- Owner IPC: `create_group {name, members, kind: "channel", access}`,
  `change_group {…, retention: 30 | 90 | 180 | 365 | "forever"}`,
  `channel_storage {groupId}` → `{retention, parts, bytes, stampsPerMonth,
  addedLastMonth}`: stamps a month are `⌈parts × 30 / 25⌉`.
