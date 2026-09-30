# Discovery, open groups, and book-based access — September 27, 2026

**Status:** user decisions of 27.09.2026. To be done before the public
testnet launch. Complements
[V1_AGENT_FIRST_SCOPE_2026_09_25.md](V1_AGENT_FIRST_SCOPE_2026_09_25.md) and
[V1_MAILBOX_SWARM_IMPLEMENTATION.md](V1_MAILBOX_SWARM_IMPLEMENTATION.md).

## Why

A newcomer knows friends' email or GitHub login, not their `ain1…`, and does
not know which communities exist on the network. Needed:

- finding a person by their Google or GitHub account;
- a catalog of public groups and profiles with interest search;
- groups that anyone can read while members write.

This also closes a hole along the way: all reading at holders is currently
free for any peer id, and a peer id costs nothing.

## User decisions (27.09)

- Google and GitHub lookup is direct only, by exact address or login, no
  listing. Each checked address costs 1 coin, found or not. The address book
  is checked once at startup, then manually; there are no continuous
  re-checks.
- Linking your own account (making yourself findable) is free.
- Publishing a card in the catalog is a separate method, 10 coins for 30
  days; editing and renewal are a new publication. Card search is free. A
  Google account is not required for publishing.
- Coin emission for Google and GitHub accounts is a separate topic, not
  here.
- No supergroups: a group remains MLS with up to 50 members. What is needed
  are read-open groups with a "public / private" switch at any moment.
  History for new readers is not needed.
- Free operations (reading, subscription, search) are available only to
  active book owners, with a limit of 30 requests per second per book at
  every holder. There is no anonymous quota, so it cannot be filled up to
  keep newcomers out.
- The default limit on new contacts is 100 per day instead of 20. This is
  the recipient node's policy; it enforces it itself and the sender cannot
  bypass it (a separate change, branch `feature/contact-daily-limit-100`).
- Mutual auto-acceptance is not being built.

## 1. Book-based access

An active book is a purchased or identity-server-issued profile book whose
term has not expired and which the node has not blocked with a double-spend
proof. The coin balance is not visible to the holder, so a fully spent but
unexpired book is also active; this is accepted.

### Pass

Once per UTC day the node signs a pass with the book key:

```
AccessPass { book, peer, day, signature }
digest = keccak256(keccak256("AIN_ACCESS_V1") ‖ domain ‖ book ‖ peer ‖ day)
```

`peer` is the node's transport key, `day` is the UTC day number, the
signature is secp256k1 in the stamp conventions (low s, v ∈ {27, 28}).

- A new request `Access { pass, grant? }` to the holder. For a grant book
  the node attaches the grant. For the pass, a grant passing the
  `GrantIssuer` rules (active key, number within the daily limit, term)
  works, even if the notary has not seen it yet: otherwise a newcomer could
  not find notaries to register the grant. The holder reads a purchased book
  from the blockchain and answers `unknown_book` until then; the node
  retries.
- The holder accepts the pass if the book is known and active, `day` is
  today (±1 for clock skew), `peer` matches the connection, and the
  signature was made by the book key. It remembers `peer → book` in memory
  until the end of the day. After a holder restart the node gets
  `access_required` and presents the pass again.
- One book can issue passes to several peers; their limits are shared.

### What requires a pass

There is no anonymous quota. The node decides whether to admit a request
before reading its bytes, so it cannot distinguish requests by type: from a
peer that is not a unit and has not presented an accepted pass, only
`Access` is taken. Everything else is refused with `access_required`, even
requests carrying their own payment (`Store`, `Notarize` with stamps or
grants, `LearnGrant`).

| Request | Available to | Charged to whose quota |
|---|---|---|
| `Access` | everyone | the presentation path (below) |
| `Read`, `Store`, `Directory`, `Proofs`, `Notarize`, `LearnGrant` | with a pass or to units | the pass's book; a unit — without the book limit |
| `Summaries` | units only | the unit |

- A unit is a holder the node knows from the catalog, or one that identifies
  itself with its signed record (`Access` with `Unit { record }`): its
  commitment is among the registry's active units, and the record's
  transport key is the connection's peer. The identification lasts until the
  end of the day, like a pass.
- Nodes without blockchain flags (local network) do not ask for a pass, same
  as with stamps.
- A pass accepted before the book was blocked by a proof remains valid until
  its own end: it is reading only. That book's next pass will not be
  accepted.
- A grant not passing the `GrantIssuer` rules (wrong issuer key, number
  beyond the daily limit, term) is refused with `grant` and a penalty, like
  an invalid pass.

### Newcomer entry

1. The book is obtained outside the p2p network: an L2 purchase or a grant
   via the identity server over HTTP.
2. The node connects to a bootstrap node (also a holder); bootstrap, DHT,
   and relay are separate protocols with their own per-peer limits, no pass
   is needed there.
3. Presents a pass with the book or grant.
4. Then — the unit catalog, grant registration, reading and sending under
   its own book's quota.

### Limits

- A peer with an accepted pass is counted by its book: **30 requests per
  second per book** at every holder (user decision). A thousand peer ids
  sharing one book share one quota.
- When a holder's total capacity is busy, it is split evenly among books,
  not first-come-first-served: an attacker's N books take no more than N
  shares, and each book costs $1 or a Google account.
- The `Access` path is a separate budget, not shared with book owners. The
  check is cheap: one signature and an in-memory lookup of the book
  conditions. The budget is 64 requests per second total, 2 per peer, and 8
  per IP (per-IP accounting is new; the address is the first IP in the
  connection address, loopback is counted by peer only). A peer with an
  invalid pass and its address get refused for 5 minutes. An unknown book
  triggers a blockchain read at most once a minute per peer and per IP; book
  absence is remembered for 60 seconds, as now.
- Units — as now.
- An attacker without a book can only interfere with pass presentation, and
  only by flooding this path at every holder from many IPs; book owners are
  unaffected. The numbers will be refined with a load scenario in the rig.

### Client

- The node picks the active book with the latest term, signs a pass for
  today on the first free request to a holder, presents it again on
  `access_required` and on a new day.
- A node that is not a unit and has no active book sends holders nothing: it
  has nothing to present. Diagnostics are
  `node_info.mailboxSwarm.access.credential` (`book`, `unit`, or `none`).
  Direct delivery while the node is online works: the sender pays for it.
- A unit sends requests immediately: other holders usually know it from the
  catalog. On `access_required` it identifies itself with its record.
- Holders learn nothing new: the record already travels from the same peer
  id with stamps of the same book.

## 2. Read-open groups

A group remains an MLS group of up to 50 members. In public mode anyone can
read its messages; only members write.

### Mode

- The owner-signed roster gets the field `access: private | public`
  (`roster-v2`). Only the owner can change it, in their own commit, like the
  admin list. The mode takes effect from the epoch following that commit.
- A new group is private.

### Public mailbox

- `H("AIN_PUBLIC_GROUP_V1", domain, G, period)`, where `G` is the group
  reference (`group_ref`). Anyone who knows `G` and the owner can compute
  the address; holders do not treat it specially — stamps, quorum, and
  retention are as for any mailbox.
- A **public post** is a document signed by the author's root key:
  `{G, epoch, operation, text}` (`operation` is the SHA-256 id of the send
  operation, so two identical posts do not merge). It lies in the open, one
  stamp per post.
- The **public roster** is a document signed by the committer (owner or
  admin): `{G, epoch, members, ownerRoster}`, where `ownerRoster` is the
  owner-signed roster (admins, version, `access`). The commit winner places
  it right after applying the commit while the group is public. Owner and
  admin nodes place the current roster once per epoch and per day so that a
  new reader finds it without history. The closing roster
  (`access = private`) names no members and is placed only in the closing
  epoch.

### Reader

- Accepts a roster if `ownerRoster` is signed by the `G` owner, the
  committer is the owner or one of their admins, and `access = public`.
  Takes the roster with the newest epoch and remembers the last two.
- Accepts a post if the author is in the roster of the epoch the post names
  and the epoch is not older than the previous one. Everything else in the
  mailbox (garbage, posts of removed members) is skipped. A post newer than
  the known roster waits for that roster.
- A roster with `access = private` means the group has closed: the
  subscription stops.
- Two rosters of one epoch are possible only from a malicious admin; this is
  accepted, just as a demoted admin is accepted in V1.

### Member

- In public mode `send_message` places a public post in the group instead of
  an MLS message. Commits still go through the epoch mailbox and the notary.
  The member reads both mailboxes and verifies posts against their own MLS
  roster (author is a current member, post epoch not older than the previous
  one), independent of whether anyone published a public roster.
- Private → public: from the next epoch messages are open; everything
  written earlier via MLS stays closed.
- Public → private: from the next epoch MLS again; the last public roster
  with `access = private` tells readers about the closure.
- What is published is not revoked: it sits at holders for up to 30 days and
  in copies. The CLI requires explicit confirmation when switching to public
  mode.

### Subscription

- `groups follow` by card or by `G` and owner: the node reads the current
  and previous period of the public mailbox every 60 seconds from one random
  holder and every 5 minutes from four (4 + 7 > 10, intersection with the
  record quorum). There is no history: posts stored before subscribing are
  skipped.
- Reading uses the pass and consumes the book's quota.
- In the inbox a subscription looks like a read-only group;
  `groups unfollow` removes it.
- Load: 1000 readers generate about 17 requests per second per group, about
  2 per holder.

## 3. Catalog service

### Design

- `services/directory` — axum and rusqlite, like the identity server; a
  separate process with its own database and binding signature key. Google
  OAuth is the same flow as the identity server's; GitHub OAuth with no
  extra permissions.
- Our node runs alongside; the service talks to it over owner IPC in
  service mode (a node flag):
  - `redeem_stamps {stamps}` — verifies the book (blockchain or grant), the
    signature, and records the ticket locally and with the ticket's notaries,
    like the recipient node. The same stamp on a different operation is
    `SenderEquivocation`; the book is blocked across the whole network.
    Reusing the same stamp on the same operation is not a conflict, so the
    request is idempotent. So that one stamp cannot track an address forever,
    the lookup operation includes the day (UTC): the service accepts today's
    and yesterday's; beyond that — pay again. A card is paid once: a repeat
    does not extend it, raise it in results, or restore a removed one.
  - `book_status {book, grant?}` — whether the book is active.
- Coins burn as for messages; the server earns nothing from them.
- Free card search presents the book's pass (an `AccessPass` with a one-time
  nonce instead of a peer); paid requests are signed with stamps. Without an
  active book — `book_required`. The limit is per book: behind the Coolify
  proxy all clients share one IP.
- The login link opens a page with the profile and the code from the
  terminal; the login counts only in the browser that confirmed it (cookie),
  so someone else's link cannot bind the victim's account to an attacker's
  profile. Consents are one-time. The service stores neither the address nor
  its hash — only an HMAC — and signs the binding at paid lookup.
- The client node talks to the service at the address from the
  `serve --directory URL` flag, like with the identity server.

### API

Exact routes, request bodies, refusal codes, and node IPC are in
[spec/discovery-v1.md](../spec/discovery-v1.md). In brief:

- linking — signing in with Google or GitHub via the link, the consent
  signed with the profile's root key; free; one address — one profile, one
  account — one address, a profile has one address of each kind;
- account lookup — only by the exact hash of the normalized address (Gmail:
  without dots and the `+suffix`), one stamp per address; no listing or
  dump; the service stores only an HMAC of the hash;
- cards — 10 stamps for 30 days, republishing replaces, only the author
  removes; search by words, tag, language, and kind — for active book owners
  (a pass with a one-time nonce);
- stamps and books are verified by our node next to the service
  (`redeem_stamps`, `book_status`); newcomer grants are passed along with
  the stamps.

### CLI, MCP, skill

```
discover link google|github
discover unlink google|github
discover lookup [--email ADDR]… [--github LOGIN]… [--file PATH]
discover publish group GROUP --about TEXT [--tag T]… [--lang L]…
discover publish profile --about TEXT [--tag T]… [--lang L]…
discover withdraw CARD
discover search TEXT [--tag T] [--lang L] [--kind group|profile]
groups access GROUP public|private [--confirm]
groups follow CARD | --group G --owner ID
groups unfollow GROUP
```

`--file` reads vCard, CSV, or a line-per-entry list. Skill: during
onboarding, with the person's consent, the agent collects addresses (vCard,
`gh api user/following`) and checks them in one batch, showing the price;
afterwards — only at the person's request.

## Prices

| Operation | Price |
|---|---|
| Linking and unlinking your own account | free |
| Address check | 1 coin |
| Publishing a card for 30 days | 10 coins |
| Removing a card | free |
| Card search, subscription, reading | free with an active book |

## Order of work before launch

1. **Book-based access** — changes the holder protocol, so first.
2. **Open groups** — changes the group roster format.
3. **Catalog service**, node service mode, CLI, MCP, skill, and Coolify
   deployment next to the testnet nodes.

Each phase is tests first (RED), then an independent backend-test-critic,
then code. Scenarios live in the rig; for 1 and 2, a native run before
launch.

## Checks

- **Access:** pass verification by a holder (wrong peer, wrong day, expired,
  blocked, unknown book, grant); quotas (1000 peer ids with one book share
  the quota, a stream of invalid passes does not displace book owners, under
  overload capacity is split evenly among books); rig: a node without a book
  does not read, with a book it reads, unit replication and group
  applications work.
- **Open groups:** `roster-v2` transitions; post and roster verification;
  switching both ways; a removed member's post is rejected after the next
  epoch (at the core level); rig: a reader sees posts, sees the closure, and
  stops.
- **Catalog:** service tests modeled on the identity server with fake
  Google, GitHub, and node: lookup pays one stamp per address per day, a
  repeat is idempotent, a reused stamp on a different operation is a
  conflict, a card requires 10 stamps and is paid once, search without an
  active book is rejected, there is no listing, someone else's login link
  binds nothing.
