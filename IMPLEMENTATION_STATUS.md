# V1 implementation status — 2026-09-29

Every V1 acceptance scenario has passed (AF01–AF08, GF01–GF02; the last,
AF05–AF07 on the public testnet, on 2026-09-29), except one step no script
can take: signing in with a real Google or GitHub account on the claim link
of a Linux install (the link and its redirects are checked). V1 is not
released: the items before mainnet are at the end.

## Scope

[The agent-first scope](Docs/V1_AGENT_FIRST_SCOPE_2026_09_25.md) (2026-09-25)
defines V1 as a permissionless agent chat on macOS and Linux:
- phase A: headless daemon, CLI, skill, contact by ID, direct messages,
  groups, paid messages, our Google identity server with a network-checked
  emission cap, royalty on coin purchases, public testnet;
- phase B: GUI.

Windows, attachments, devices and backup move to V2; agent orders are
deleted. Operator payouts came back into V1 on 2026-09-29 (below).

Storage and payment follow [the storage redesign](Docs/V1_STORAGE_REDESIGN_2026_09_24.md)
(2026-09-24): a recipient-mailbox swarm of 10 holders with a quorum of 7,
in-swarm replication and repair, one stamp book per user, a stamp on every
message, a ticket notary and equivocation proofs. Mailboxes live 30 days.

## Current result

The mailbox swarm is implemented through phase 6 of
[the implementation plan](Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md): pure rules,
holder service, send, receive, replication and repair, the notary with
batched requests, equivocation proofs and grant books checked by holders.

[The acceptance spike](evidence/reviews/mailbox-swarm-spike-2026-09-26/) ran on
the single-process managed-time rig. It is not a native run. Results:
- 258 messages were stored at a quorum in 30 virtual seconds, with 2 holders
  unreachable and 1 malicious;
- a double spend was refused by every holder;
- the swarm repaired itself after 9 of 10 disks were lost;
- the recipient read all messages in order after coming online.

## Replaced path deleted (2026-09-26)

By user decision the replaced path was deleted before phases 1b and 2, so
that the build and test loop is fast. Removed:
- custody/history storage and the public sender;
- finalizers and checkpoints;
- private ZK stamps (RISC Zero);
- the L2 adapter;
- the stamp issuing, subsidy and operator settlement contracts;
- agent orders (jobs);
- the wallet, checkpoint, history recovery and jobs panels;
- the scenarios and scripts of all of these, including A04/H11.

Their descriptions and evidence remain in Git history.

Checks after the deletion:
- workspace Clippy with `-D warnings` and formatting;
- all Rust tests, including 58 process tests. The node's lib tests went from
  480 (about 8 minutes) to 99 (about 30 seconds);
- 27 Foundry tests;
- 40 frontend tests, TypeScript and Vite.

Phases 1b and 2 have since landed (below). The native spike passed on
2026-09-27: 258 offline messages stored, repaired and read with real daemons
on a local chain ([evidence](evidence/reviews/mailbox-swarm-native-2026-09-27/)).

## Done since the deletion (2026-09-27)

- Phase 1b (12645bf): `BookShop.sol`; holders read bought books and
  `GrantIssuer` rules over JSON-RPC; `coins_buy`, `coins_balance` and
  `coins_claim` (identity server) over owner IPC.
- Phase 2 (83cb2a8): the directory — unit commitments bonded in
  `NodeRegistry`, records pulled from peers.

## Remaining V1 plan (phase A, in this order)

1. **Native spike (V1-AF03, local AF05)** — done 2026-09-27: ten bonded
   units, a bought book, 258 offline messages stored in 27 s, three lost
   disks repaired in 37 s, read in order in 13 s. It found and fixed a
   recipient stall on held envelopes. The double spend stays covered by
   the rig.
2. **A stamp on direct delivery (part of AF02)** — done 2026-09-27: a
   direct message carries its swarm copy's stamped envelope (one slot for
   both paths); a node that reads the chain checks it like a holder, takes
   each slot once and refuses unpaid application messages; a node without
   chain flags stays a free local network.
3. **Owner CLI and headless secrets (AF01)** — first part done 2026-09-27
   ([spec](spec/owner-cli-v1.md)): `kaiki` with `daemon
   start|stop|status`, `init`, `contacts invite|add|list`, `send`,
   `messages`, `coins balance|buy|claim`; the agentic-cli JSON and exit
   codes; the desktop's profile and keychain, or a password-sealed secrets
   file; saved daemon flags; one daemon per profile. Second part done
   2026-09-28: `inbox watch|poll|ack` (the owner's own cursor per
   conversation with a lease), `grants create|list|revoke` for scoped
   agents, `kaiki` in the app bundle; on Linux arm64 the CLI tests pass
   and the default is the Secret Service when one answers, the password
   file otherwise ([evidence](evidence/reviews/owner-cli-linux-2026-09-28/)).
   On Linux x86_64 (2026-09-29, a clean Ubuntu 24.04 container): the
   `install.sh` build installs in 3 s; `init`, the signed preset, a claim
   link to Google and GitHub, a payment request, exit codes and a restart
   pass; the sign-in itself needs a person
   ([evidence](evidence/reviews/af01-linux-x86_64-2026-09-29/)).
4. **Contact by ID (AF02)** — done 2026-09-28
   ([spec](spec/contact-by-id-v1.md)). Each profile publishes a signed card
   (a last-resort KeyPackage, its addresses, a seal key) in its intro
   mailbox `H(AIN_INTRO_V1, domain, id, period)` once a period. A request is
   the Welcome, sealed to the card and paid, sent into that mailbox and
   directly; the requester's node finds the card newest period first. The
   recipient's policy: `all` with a daily limit (20 by default), `list`,
   `manual`; 32 waiting requests at most, one per requester; decided ones
   stay decided. Unpaid requests are refused by holders and by a paying
   node. CLI: `contacts request|requests|accept|reject|policy`. Covered by
   core tests, managed-time rig scenarios (a recipient away for days, the
   direct path, owner decisions) and a native run
   ([evidence](evidence/reviews/contacts-groups-native-2026-09-28/)).
5. **Groups (AF04)** — done 2026-09-28 ([spec](spec/groups-v1.md)). MLS
   groups (up to 50 at first, up to 2000 since the large groups below) with
   an owner-signed roster of admins; one group
   mailbox per epoch (one copy and one stamp per message); invitations
   through the invitees' intro mailboxes under their policy. Commits are
   stored first, then claimed with the notaries of `H(G, epoch, round)`:
   seven first records decide, a split round is renewed; readers claim what
   they read, so an epoch is decided without its committer; holders accept
   only claims under the owner's roster. A removed member reads nothing
   new. CLI: `groups create|add|remove|ban|unban|admins|list|show|send`.
   Covered by core tests, rig scenarios (a forced split, a committer gone, a
   message written before a commit) and the same native run: a removal
   decided by the notaries in 5.7 s. Bans (2026-09-28): the owner and admins
   ban ids as they remove them; the list rides in the MLS group context, so
   every member and newcomer agrees on it; nobody adds a banned id until the
   ban is lifted, and only the owner lifts the owner's bans. A removed
   member's later messages sealed for an epoch it knew are refused. Not in a
   native run yet ([evidence](evidence/reviews/group-ban-2026-09-28/)).
6. **Skill and thin MCP (AF08)** — done 2026-09-27: the owner skill
   (`integrations/agent-skill/kaiki/SKILL.md`, kept in step with every
   command of the CLI spec by a test), `kaiki skill show|install`, and
   `kaiki mcp` exposing the CLI's everyday operations as MCP tools
   (identity, grants, policy and payments stay with the CLI). Accepted with
   a real agent host (Codex, `gpt-6-sol`) on a local network of ten
   holders: contacts, messages, a group and the inbox through the CLI and
   the MCP; injected messages caused neither a send nor a leak; a grant
   revoked between a prepared reply and its send refused the send
   ([evidence](evidence/reviews/af08-codex-host-2026-09-27/)). The run found
   and fixed four defects: the owner inbox now covers groups, a member's
   group messages are taken in the order written, `--text-stdin` drops a
   here-document's final line break, and an agent's inbox lease may be up to
   600 s.
7. **Public testnet (AF05–AF07).** Contracts on an L2 testnet, our bootstrap
   nodes and identity server, a client joining through an independent peer
   with ours off, the NAT/relay matrix.
   - A book of 1000 stamps is priced in USD: $1.00, a tenth of a cent a
     message, paid in ETH at the Chainlink ETH/USD rate or in USDC
     (2026-09-27).
   - Contracts deployed to Base Sepolia on 2026-09-27 with
     `scripts/deploy-contracts.py`; addresses, parameters and the node's
     flags in [deployments/base-sepolia.json](deployments/base-sepolia.json).
     Since 2026-09-28 the grant book is 10000 coins (the app's "up to ten
     thousand messages a month") and the daily cap 100000000 coins (10000
     grants a day); the two earlier `GrantIssuer`s are listed as superseded.
   - **Deployed** (Coolify project `chat`, application `chat-production`,
     from `main`, `deploy/docker-compose.yml`): ten holders on UDP/TCP
     4101–4110 of 51.91.126.3, each unit bonded in `NodeRegistry` (0.0001
     ETH; the registry lists 10 active units, checked on 2026-09-29); the
     identity server at https://id.kaikichat.com with Google and GitHub
     sign-in (its `/v1/policy` matches the `GrantIssuer`); the discovery
     service at https://directory.kaikichat.com; kaikichat.com with the
     signed network preset (serial 3: the ten routes, Base Sepolia, the
     identity server, the directory with its key, and `welcome`) and
     `install.sh` for the command line (macOS arm64 and Linux x86_64
     builds); the welcome agent (`deploy/node/welcome.py`, in the nodes'
     container, `ain1db67…f0d8`, English only) with its open group "Kaiki
     Lobby", both listed in the directory. Checked 2026-09-29 from a fresh
     macOS client: contact by id and the agent's welcome arrive; joining the
     lobby did not complete (the client reached 4-5 of the ten holders).
     After the fixes below, a fresh macOS client (a granted book) is in the
     lobby 76 s after its first start, its card and knock stored at a
     quorum, let in by all ten holders; its contact request to the welcome
     agent succeeds on the first ask and the welcome arrives 22 s later.
   - **Accepted on the public testnet** (2026-09-29,
     [evidence](evidence/reviews/testnet-acceptance-2026-09-29/)), with
     clients in containers behind NAT routers of their own
     (`scripts/check-testnet.mjs`):
     - AF07: two agents behind different NATs reserve circuits at two of our
       nodes, find each other by id (26 s), talk in 1–3 s through a
       circuit; the relay in use cut off for both, they keep the other one
       and go on (the next message in 1.1 s).
     - AF06: with kaikichat.com, its preset, the identity server and the
       directory unreachable, a newcomer given only an independent peer
       (someone else's node with a book, not in the preset, not a holder)
       buys a book, gets the holders' records from that peer, finds a user
       by id and talks; only free coins are missing. The holders are still
       ours. A peer without a book cannot hand on the holders' records.
     - AF05: thirteen books bought in ETH at the Chainlink rate, each seen
       by its node in about 30 s; the treasury's credit grew by a tenth of
       the price; a profile restored from a backup spent its stamps again
       and its book was blocked across the network within seconds; a grant
       signed with the identity server's key passes at serial 9999 and is
       refused from 10 000, the day's cap.
   - Fixed on the way: the testnet's nodes advertised only the host's
     docker bridges, never their public address, so outside clients reached
     only their preset's routes and stored nothing at a quorum; they now
     listen on 51.91.126.3 (deployed 2026-09-29). A node now ranks its
     listener addresses public first and takes `--public-address` for a host
     whose public address is not on an interface. After the move one holder
     reached no one for hours: its cached records of the others (kept a day)
     listed eight dead bridge addresses, which crowded out its bootstrap
     routes; a given route now goes before a cached record's addresses (the
     same cut off every client that had cached a holder before the move). A
     newcomer's first `contacts request` or `groups join` answered
     `card_not_found` while the holders were still reading its new book or
     grant; the lookup now waits for them, and at worst answers to ask
     again. Asked in the node's first half minute, before it has read the
     directory, it now waits instead of answering `network_unavailable`;
     `kaiki daemon status` shows the node's reach (`network`: holders known
     and letting it in, card stored, failures). The command line on
     kaikichat.com was published again with these (d66870ad): installed
     from install.sh, a new profile's first `groups join` is answered in
     37 s and it is in the lobby two minutes later.

## Since phase B (2026-09-28)

- **Access by book** ([spec](spec/discovery-v1.md)): a holder serves only
  units and peers that showed a pass signed daily by an active book's key
  (bought, or a grant within the `GrantIssuer` rules); 30 requests/s per
  book at each holder, a fair share between books under load, a separate
  budget for showing passes. Rig: a node without a book reads nothing, a
  granted newcomer is let in before the notaries saw its grant, a copied
  grant lets in only its owner, a new day needs a new pass.
- **Discovery** ([design](Docs/V1_DISCOVERY_2026_09_27.md),
  [spec](spec/discovery-v1.md),
  [evidence](evidence/reviews/discovery-gui-2026-09-28/)): open-read groups
  (anyone follows and reads, members write; `groups access|follow`); the
  service `services/directory` (bindings of one's own Google or GitHub
  account, free; exact lookup, one coin an address; cards of groups,
  channels and profiles, ten coins for 30 days; search free with a pass),
  its stamps redeemed at our node so a stamp reused elsewhere is a proven
  double spend; `kaiki discover …`, MCP tools, the window's "Find people".
  A native run (`native_discovery`) covers card, search, follow and a paid
  lookup on a local chain.
- **Network preset** (see phase B): live on kaikichat.com as serial 4 (serial 3 named the welcome agent and its lobby; 4 the operator pool).
- **Bans** (in step 5).
- **Large groups** ([design](Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md)):
  groups of up to 2000; the ratchet tree apart from the Welcome, documents
  in parts (up to 4 MB); the group's mailbox changes only on removals; big
  groups poll less often (up to 20 s at 2000, one holder a read, four a
  minute); three modes — public (anyone reads, the door lets in at once),
  by request (an admin or agent decides at the door), private (invitations
  only); one batch of newcomers a minute, two admins in one minute settled
  by the notary; a member away longer than the mailboxes keep asks for its
  place by itself; messages that waited three epochs are sealed again; an
  MLS operation loads only its own group's records. Rig: a newcomer reads
  the tree from its mailbox, a stranger knocks and is let in at once or
  after a decision, a member away past 30 days takes its place again.
- **Public channels**: only the owner and the admins (the team) write,
  anyone follows; retention 30, 90, 180, 365 days or for ever, kept as
  archive parts laid again every 25 days; `channel_storage` shows the term,
  parts and stamps a month; a channel's card in the directory. Rig: a
  follower reads its month back and the archive beyond.
- **Closed channels**: a key tree whose paths subscribers hold, keys given
  one by one (two stamps), new keys on every removal, reseeds. An admin
  taken off the team is cut off (2026-09-28): each subscriber publishes its
  own channel key once, and the owner's node makes a hard reseed sealed to
  those keys in a commit the removed admin is not in; subscribers whose key
  the owner does not know get keys again. A channel never switches between
  public and closed (the owner's decision of 2026-09-29). Rig: subscribers
  read until one is removed; the owner moves the channel to new keys once
  an admin is out.
- **The command line and the window** gained doors, requests, channels,
  history and cost, a closed channel's keys
  ([evidence](evidence/reviews/large-groups-channels-gui-2026-09-28/)).
- **Native desktop gate** (2026-09-29,
  [evidence](evidence/reviews/native-gate-2026-09-29/)): passed in the main
  checkout once its chat case opened Agents from the gear menu — check.sh
  (684 Rust, 41 Foundry, 113 frontend tests, Clippy), both native WKWebView
  cases, the signed release bundle.
- **Distribution**: the landing's "Let your agent try it" gives an agent
  one text that installs `kaiki` from kaikichat.com/install.sh, the skill,
  and a first free grant.
- **Native acceptance** (2026-09-29,
  [evidence](evidence/reviews/large-groups-native-2026-09-29/)): three
  scenarios on real daemons, a local chain and ten bonded holders passed —
  an admin's ban decided in 1 s, lifted by the owner; a stranger following
  the opened group reads members' posts by their certificates; the unbanned
  member let in at the open door in 31 s; an application by request listed
  at the admin's in 8 s and let in 42 s after the decision; a public
  channel's full archive part laid in 169 s and read, with the history, by a
  later follower; a closed channel's keys given, a subscriber taken off by
  an admin, and the owner's node reseeding onto the subscribers' own keys by
  itself once the admin is off the team — the subscribers read on (Carol
  given keys again), the former admin and the removed subscriber do not.

## Phase B: the GUI — done 2026-09-27

The desktop app is the owner's window onto the same daemon and profile as
the CLI ([spec](spec/desktop-gui-v1.md),
[evidence](evidence/reviews/gui-phase-b-2026-09-27/)).
- **One profile, one daemon:** the window opens the profile like `kaiki`
  (data directory, keychain or a password-sealed file, the flags saved in
  `daemon.json`); whichever starts first starts the daemon, the other joins
  it. A daemon stopped from the CLI stays stopped until the owner starts it
  in the window.
- **Screens:** a five-step first run, one action per screen (what this is,
  the name, free messages through Google or GitHub with crypto as the
  anonymous way, one text to copy into Claude Code or Codex naming the
  bundled `kaiki` CLI, an invitation for friends), a start screen with the
  same next steps and adding a friend from a pasted invitation,
  direct and group chats with delivery states, contacts by id with requests,
  group members with owner/admin/member roles, agents and grants with the
  CLI/MCP settings, credentials path and skill install, the wallet, Settings
  (theme, language, the all/list/manual policy, network).
- **Boundary:** the webview calls only the listed commands, each forwarding
  one daemon method; links open only as the daemon gave them; no secret
  reaches the webview; untrusted text renders as text.
- **Language and look:** English by default and Russian from localization
  tables; monochrome dark (default) and light themes.
- **Identity server:** GitHub login besides Google (one grant per GitHub
  account id).
- **V1-GF01** passed on a native run: three windows on a local chain with
  ten holders and the identity server, every flow through the window, in
  237 s. **V1-GF02:** the macOS `.app` and the Linux x86_64 `.deb` build,
  install and pass a production smoke; the native chat case passes on
  WKWebView and on WebKitGTK; screenshots checked.
- **Name:** the app is Kaiki Chat (window, bundle, sidebar, onboarding in
  all twenty languages, 2026-09-28,
  [evidence](evidence/reviews/kaiki-chat-name-2026-09-28/)); Agentic
  Internet stays the protocol's name, and the bundle identifier and data
  directory are unchanged.
- **Network preset** (2026-09-28,
  [design](Docs/V1_NETWORK_PRESET_2026_09_28_RU.md),
  [evidence](evidence/reviews/network-preset-2026-09-28/)): the app and
  `kaiki` know only `https://kaikichat.com/network.json`, an Ed25519-signed
  preset (routes, chain, identity server) checked with the key built in and
  kept in the profile; within a network it updates quietly, another network
  is only offered (`kaiki network switch`, the window's notice), and a
  preset for a newer app waits for an update. Signed with
  `kaiki-preset` from `scripts/network-preset.py`; the key stays in
  `.local/network-preset/`.
- Fixed on the way: `daemon start` with flags on a fresh machine left the
  profile directory world-readable and refused it; the desktop list and
  history now include groups.

## Operator payouts (2026-09-29)

[The decision](Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md): the pool's 90% of
book sales goes to the holders of paid messages. A paid stamp is a lottery
ticket (1 in ~1111 wins by the seed of its book's purchase day) paying $0.10
to each holder the stamp names; tickets last 360 days; withdrawal is manual,
without a minimum; the treasury sweeps what was not withdrawn once tickets
expire. Branch `feature/v1-operator-payouts-20260929`:
- `contracts/src/OperatorPool.sol` (no owner) and `BookShop.soldOn`: 72
  forge tests; a withdrawal costs 126 thousand gas for one ticket and about
  42 thousand for each further one, about 1% of the prizes.
- Stamps of mailbox messages name their holders (operation V2 with the list
  carried along, fixed per mailbox); holders accept stamps that name nobody
  as before. Holders keep tickets, draw them, fix a day's seed in turn and
  withdraw on `kaiki earnings withdraw`; `kaiki earnings` shows prizes won,
  drawing, claimed, refused, owed, ending, and the receipt account's gas.
- The node signs its EIP-1559 transactions itself (checked against `cast
  mktx`) and was checked against a pool deployed on a local anvil.
- `scripts/deploy-contracts.py --reuse` deploys a new splitter, shop and pool
  next to the running GrantIssuer and NodeRegistry.
- Live on Base Sepolia since 2026-09-29 (OperatorPool `0x7B9e…29Bc`, BookShop
  `0x20d1…e406`, preset serial 4); the testnet nodes fix each day's seed
  themselves. Holders still take stamps of books from the former shop but
  never draw them. [Verification](evidence/reviews/operator-payouts-2026-09-29/).

## Identity penalties (2026-09-30)

[The decision](Docs/V1_IDENTITY_PENALTIES_2026_09_30.md): nodes are
permissionless and bondless, so the one stake is the identity that gets free
coins. A grant proven spent twice (a sender double spend of its book) gets
its account banned at the identity server for 90 days (for good if a grant
got after the ban is spent twice), and the account's grants still in force
are revoked: holders take no new stamps of them, keep what they hold and
repair copies paid before. Escalation rather than a ban for good at once: a
profile restored from a backup, or a claim link opened by someone else's
key, makes an honest person the author of such a proof. Branch
`feature/v1-identity-penalties-20260930`:
- `GrantRevocation` (grant-book crate), signed by the issuer key.
- Identity server: grants linked to accounts, `POST /v1/reports`,
  `GET /v1/revocations`, `account_banned`, 90-day then permanent bans.
- Holders queue a report for every grant they see spent twice (theirs or
  learned from other nodes), send it to their identity server, retry a
  minute later while it is down; they read the revocations every 10
  minutes and right after a report is answered; a revocation of a grant not
  learned yet waits for the grant. The testnet nodes run with
  `--identity-server`.

## Before mainnet (2026-09-29)

In the owner's order — speed for users first, no heavy load on hosts:

- Holder push or long-poll instead of polling every mailbox from every
  holder every 5 s (channels every 5 minutes): faster delivery and far less
  load on holders.
- Back-off per holder: an undialable holder is retried about once a second.
- A holder that moves to new addresses stays at its old ones in a client's
  directory until the client's next pull (every 10 minutes, from a random
  member, often one it cannot dial): pull from a connected member first.
- Independent holders: with all ten ours, the network does not work without
  them; the registry's units must be many and not ours.
- Relays for agents behind NAT: the preset names none and `kaiki` has no
  `--relay`; relay capacity (16 per node) decides how far that scales.
- `coins claim` without the identity server should say so.
- The ETH/USD feed on Base Sepolia sometimes lags past the shop's hour.

V2: holders' accounts bound for on-chain slashing, on-chain checks of a
book's double signature.

## Work and history

Work on branches of this repository and merge them into `main`, which
production builds from. Builds and checks go through
`python3 scripts/build-storage.py run …` as specified in
[AGENTS.md](AGENTS.md). Automated native tests use the isolated E2E vault;
ordinary builds keep the Keychain.

[The preserved implementation history](IMPLEMENTATION_HISTORY.md) contains
earlier entries and evidence links. It is historical context, not current
acceptance.
