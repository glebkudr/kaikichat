# Owner CLI `kaiki` (V1 phase A, step 3)

The headless interface of a profile's owner (a person or the agent running
the node): it starts and stops the daemon, creates the identity, manages
contacts, sends and reads messages and buys or claims coins. It shares the
profile, the secrets and the daemon with the desktop app. Scoped agents keep
using `agentic-cli` with a runtime grant ([agent-grants-v1.md](agent-grants-v1.md)).

## Output and exit codes

The same contract as `agentic-cli`: exactly one JSON envelope on stdout,
`{"result": …}` or `{"error": {"code", "message", "retryable"}}`. While a
newer release of Kaiki Chat is known and not skipped, the envelope also has
`"update": {"current", "latest"}` (every command but `update`).

| Code | When |
|---|---|
| 0 | success |
| 2 | invalid arguments or input, or local secrets that cannot be opened (`secrets_locked`) |
| 3 | a final refusal (`unknown_contact`, `profile_exists`, `chain_not_configured`, …) |
| 4 | retryable: the daemon is unavailable, or the node asks to try again (`chain_pending`, `claim_pending`) |

Help and version are ordinary text.

## Commands

```
kaiki [--data-dir DIR] [--secrets keychain|file] <command>

daemon start [--listen ADDR]… [--bootstrap ADDR]… [chain flags] [--identity-server URL] [--directory URL]
daemon stop
daemon status
network
network refresh
network switch
update [--check | --skip]
autostart [on | off]
init --name NAME
contacts invite
contacts add --name NAME --invitation-stdin
contacts list
contacts request --id NETWORK_ID --name NAME --operation-id ID
contacts requests
contacts accept --request ID
contacts reject --request ID
contacts policy [--mode all|list|manual] [--daily-limit N] [--allow ID]… [--clear-allowed]
groups list
groups show --group GROUP
groups create --name NAME [--member ID]… --operation-id ID
groups add --group GROUP --member ID… --operation-id ID
groups remove --group GROUP --member ID… --operation-id ID
groups ban --group GROUP --member ID… --operation-id ID
groups unban --group GROUP --member ID… --operation-id ID
groups admins --group GROUP [--admin ID]… --operation-id ID
groups send --group GROUP --operation-id ID --text-stdin
groups access --group GROUP --to public|request|private --operation-id ID [--confirm]
groups join --group-ref G [--note TEXT] --operation-id ID
groups requests --group GROUP
groups decide --group GROUP --request ID --accept|--reject
groups follow --card ID | --group-ref G --owner ID --name NAME
groups unfollow --group GROUP
groups follows
channels create --name NAME [--member ID]… --access public|request|private --operation-id ID [--confirm]
channels retention --channel CHANNEL --days 30|90|180|365|forever --operation-id ID
channels storage --channel CHANNEL
channels subscribe --channel CHANNEL --member ID… --operation-id ID
channels unsubscribe --channel CHANNEL --member ID… --operation-id ID
channels reseed --channel CHANNEL --operation-id ID
discover link google|github
discover status --link ID
discover unlink google|github
discover lookup [--email ADDR]… [--github LOGIN]… [--file PATH]
discover publish group --group GROUP --about TEXT [--tag T]… [--lang L]…
discover publish profile --about TEXT [--tag T]… [--lang L]…
discover withdraw --card ID
discover search TEXT [--tag T] [--lang L] [--kind group|channel|profile]
send --to CONTACT --operation-id ID --text-stdin
messages --with CONTACT [--limit N]
inbox watch [--timeout-seconds N]
inbox poll --with CONTACT [--limit N] [--lease-seconds N]
inbox ack --with CONTACT --lease-id ID
coins balance
coins buy
coins claim
earnings
earnings withdraw
grants list
grants create --name NAME --contact CONTACT… [--read] [--send] --operation-id ID [--days N] [--max-bytes N]
grants revoke --grant-id ID
skill show
skill install [--dir DIR]
mcp
```

- `CONTACT` is a contact's name or its conversation id.
- `--text-stdin` reads the message from stdin; one final line break (a
  here-document's) is not part of it, everything else is kept as written.
- The chain flags are those of `agentic-node serve`: `--chain-rpc`,
  `--chain-id`, `--book-shop`, `--grant-issuer`, `--registry` and
  `--chain-confirmations`, and the optional `--operator-pool`.
- `earnings` is for an operator whose node holds as a registry unit
  ([payouts](../Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md)): prizes won,
  drawing, claimed and refused (USDC units, six decimals), what the pool
  credited but did not pay yet (`owed`), the tickets that end within 30 days
  and in how many days, how many messages it held and how many were paid,
  the node's receipt account with its ETH for gas (`gasWei`) and the state
  of the last withdrawal. `earnings withdraw` starts one claim of every won
  ticket from that account (several above 150 tickets), leaving out tickets
  the pool refuses; with no ticket won it pays out what is owed. The pool
  pays the unit's owner, USDC first. Nothing is withdrawn unasked; a
  withdrawal without ETH for gas fails as `no_gas`.
- Every command except `daemon stop`, `daemon status` and `network` starts
  the daemon when it is not running.
- **Network preset** ([the design](../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md)):
  a profile without network flags (`--bootstrap`, the chain flags,
  `--identity-server`) takes its network from the signed preset at
  `https://kaikichat.com/network.json` whenever its daemon starts (never on
  a command that joins a running daemon). `AGENTIC_NETWORK_PRESET` names
  another preset URL or `off`; `AGENTIC_NETWORK_PRESET_KEY` its key.
  - `network` shows `{source: preset|manual|off, state, network, name,
    serial, checkedAt, offered, required, error, welcome, recommended}`;
    `welcome` is the network's welcome agent and lobby, `{agent, name,
    lobby, lobbyName}`, when its preset names them; `recommended` the
    channels and groups the network recommends to a new profile, in its
    order, `[{kind: channel|group, ref, owner, name}]` (empty when none;
    kinds a later version adds are left out); `state` is `current`,
    `cached`, `unavailable`, `switch` (another network is `offered`),
    `update` (the preset needs version `required`), `manual` or `off`.
  - `network refresh` starts the daemon again with a fresh look at the
    preset; `network switch` takes the offered network, which was checked
    when offered, and starts the daemon again. Without an offer it is
    `no_network_offer` (exit 3) and the daemon keeps running.
- **Updates** ([the design](../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md#releases-and-updates)):
  the preset names the latest release, its builds and their SHA-256. A
  daemon start learns it; a command that joins the daemon also asks
  kaikichat.com alongside itself when the last check is 12 hours old (3 s at
  most), for a profile that follows the preset.
  - `update --check` asks now and shows `{current, latest, available,
    skipped, checkedAt, error}`; `update --skip` stops telling of the latest
    (`no_update`, exit 3, when nothing newer is known).
  - `update` asks now and, when the latest is newer, replaces this install
    with its build: downloaded, checked against the SHA-256, unpacked and
    swapped with the install directory whole. The new build then starts the
    daemon again if it ran and rewrites the kaiki skills installed under
    `~/.claude/skills` and `~/.codex/skills`. It answers `{updated: true,
    from, to, restarted}`, or `{updated: false, current, latest}`.
  - Only an install from `install.sh` updates itself: its directory holds
    `install.json`, `{"build": "cli-<platform>"}`. Any other `kaiki` (a
    build, the app bundle's) is `not_updatable` (exit 3) and asks nothing;
    a release without this build is `no_build`, a download other than the
    announced one `hash_mismatch`, an archive of another build
    `bad_archive` (exit 3, nothing changed); a failed download
    `download_failed` and a preset that cannot be fetched `unavailable`
    (exit 4).
- **Start at login.** The profile's daemon starts when the owner logs in:
  a job of the system's login manager runs `kaiki [--data-dir DIR] daemon
  start`, with the secrets settings of the run that wrote it
  (`AGENTIC_SECRETS`, `AGENTIC_PASSWORD_FILE`, the preset's variables) and
  never a password.
  - macOS: a launchd agent, `~/Library/LaunchAgents/com.kaikichat.kaiki.plist`
    (another profile's: `com.kaikichat.kaiki.<hash>`). Linux: a systemd user
    unit, `kaiki.service`, enabled for the login; without systemd an XDG
    autostart entry, `kaiki.desktop`. Only the file is written: nothing
    starts before the next login.
  - An install from `install.sh` puts the job in by itself whenever it
    starts the daemon of the profile it opens by itself (no `--data-dir`,
    no `AGENTIC_DATA_DIR`); other builds and profiles only with
    `autostart on`.
  - `autostart` answers `{state, path}`: `on`, `off`, or `blocked` when
    the system keeps the job from running until the owner allows it (macOS
    Login Items). `autostart on` puts the job in, for any profile, and
    when blocked opens the system's login items for the owner to allow it;
    the automatic start does that once. `autostart off` takes the job out.
  - The owner's `off`, or a job the owner took out in the system itself
    (`systemctl --user disable kaiki`), holds: later starts leave it out
    until `autostart on`. A job that names another `kaiki` is written anew.
  - A password given only in `AGENTIC_PASSWORD` is not written into a job:
    `autostart on` is `password_file_required` (exit 2), and the automatic
    start leaves the job out.
- Flags given to `daemon start` are saved in `daemon.json` in the data
  directory and used by every later start.
- `daemon stop` asks the daemon to shut down (owner IPC `shutdown`) and waits
  until it has released the profile. `daemon status` and `daemon stop` of a
  profile whose daemon is not running need no secret.
- `init` is idempotent for the profile's name; another name is
  `profile_exists`.
- `send` is idempotent per operation id. Its input is checked before the
  contact is looked up.
- Concurrent commands on a stopped profile start one daemon: the start is
  serialised by a lock in the data directory, and the others use it. The
  daemon runs as `agentic-node serve --profile <data dir>/profile.db …`.
- The node's `chain_pending` and `claim_pending` are retryable (exit 4).
- `coins buy` answers a request for one book of the profile's key, the same
  unpaid book until it is paid: `{book, key, salt, shop, chainId, count,
  validSeconds, priceUsdc, eth, usdc, createdAt}`. The price is in USD
  (`priceUsdc`, USDC units) and paid either way:
  - `eth`: `{to, quote, value, calldata, uri}`, a call of the shop's `buy`
    with `value` one percent over the quote at the feed's rate (the shop
    returns what is over); `null` while the shop has no fresh rate. The
    node reads the quote again once it is a minute old; the CLI waits for
    it.
  - `usdc`: `{token, amount, approve, buy}`, two calls in order (each
    `{to, calldata, uri}`): allow the shop the price, then `buyWithUsdc`.
  - The node sees the confirmed purchase itself.

## Contact by ID

[contact-by-id-v1.md](contact-by-id-v1.md): anyone who knows a network id
asks it for a conversation through its intro mailbox.

- `contacts request` finds the id's card and sends a paid request; it waits
  while the node looks for the card (up to about a minute) and answers
  `{conversationId, name}`. `card_not_found` is final (exit 3);
  `card_pending` after the wait and `network_unavailable` are retryable
  (exit 4). The same operation id is the same request.
- `contacts requests` lists the requests waiting for a decision:
  `[{requestId, networkId, name, receivedAt}]`. `contacts accept` joins one
  (`{conversationId, name}`), `contacts reject` drops it; an unknown or
  expired one is `unknown_request` (exit 3).
- `contacts policy` shows `{mode, dailyLimit, allowed}` and changes only the
  fields given: `--allow` (repeated) replaces the list, `--clear-allowed`
  empties it (not both at once). A mode other than `all`, `list` or `manual`, a limit over
  1000 or a malformed id is `invalid_input` (exit 2).

## Groups

[groups-v1.md](groups-v1.md). `GROUP` is a group's name or id; a name
several groups share is `ambiguous_group` (exit 3), and a name a contact and
a group share is `ambiguous_contact` for `messages --with` and `send --to`.

- `groups create` makes a group of this profile (its owner) and the members
  named by id, once their cards are read (it waits like `contacts request`);
  each is invited through its intro mailbox. The same operation id is the
  same group. Without a swarm directory it is `network_unavailable` (exit 4);
  a member without a card is `card_not_found` (exit 3). `groups add` finds
  cards the same way.
- `groups add`, `remove`, `ban`, `unban` and `admins` make one commit each
  (`admins` replaces the list; the owner's only) and answer
  `{epoch, commit, messageId}`: the commit is made and waits for the notary.
  `ban` removes the members it names in the same commit and needs no card.
  `group_busy` (another commit of this profile waits) and `card_pending` are
  retryable (exit 4); `not_allowed`, `unknown_group`, `invalid_request`,
  `banned` (adding a banned id) and `member_outdated` (a member's or
  invitee's app predates bans) are final (exit 3).
- `groups send` sends text to the group, like `send`; `messages --with GROUP`
  and `inbox` work for groups too.
- `groups list` and `groups show` print what a member sees:
  `{id, name, epoch, owner, admins, members, role, banned, access,
  groupRef, kind, retention}`, `banned` being `[{id, byOwner}]`, `kind`
  `group` or `channel`, `retention` the days its readers read back (null
  for ever). Channels are listed with the groups.
- `groups access` opens a group to anyone's reading (`public`), puts a door
  on it (`request`: applications wait for an admin), or closes it
  (`private`): one commit of the owner, like `admins`. Opening without
  `--confirm` is `confirmation_required` (exit 2): what is written while
  the group is open stays public for good. A channel moves only between
  `request` and `private` (else `invalid_request`).
- `groups join` asks to join group `G` at its door with a note of at most
  280 characters (one coin): `{groupId, messageId}`. Its card is found
  through the swarm (`card_pending` and `network_unavailable` are exit 4).
  A public group lets the applicant in at its next batch; the invitation
  is taken whatever the applicant's policy.
- `groups requests` lists the applications waiting at a door of a group by
  request (owner or admin): `[{requestId, networkId, note, receivedAt,
  rejoin}]`, the note being untrusted text. `groups decide` accepts or
  rejects one (`{}`); one another admin already answered is
  `unknown_request` (exit 3). Accepting at a closed channel's door gives
  the keys at once (two coins); it needs no operation id: deciding again is
  `unknown_request`.
- `groups follow` reads an open group or a public channel by its reference
  `G` (hex) and its owner's id, without joining it: a group's last day, a
  channel's last 30 days, then what comes; `groups follows` lists `{id,
  name, owner, since, closed, kind, retention, sealed}`, `groups unfollow`
  takes a follow's id or name. A follow's posts are read in the inbox under
  its id; a follower cannot write (`unknown_contact`).

## Channels

[groups-v1.md](groups-v1.md), "Channels". `CHANNEL` is a channel's name or
id, like `GROUP`; the `groups` commands that change members, send and show
work on channels too.

- `channels create` makes a channel of this profile whose team, besides
  the owner, are the members named by id (each an admin), `public`,
  `request` or `private`; a public one needs `--confirm`
  (`confirmation_required`, exit 2). It answers like `groups create`.
- `channels retention` sets how long a public channel keeps its history
  (30, 90, 180, 365 days or `forever`; owner or admin): one commit. A
  closed channel keeps none (`invalid_request`).
- `channels storage` answers `{retention, parts, bytes, stampsPerMonth,
  addedLastMonth}`: what keeping the history costs now.
- `channels subscribe` gives each member named by id a closed channel's
  keys (cards found like `groups add`; two coins each); `channels
  unsubscribe` removes subscribers (`invalid_request` for one this team does
  not know) and `channels reseed` gives everyone left keys of a new seed,
  each one commit.

## Discovery

[discovery-v1.md](discovery-v1.md). The CLI talks to the discovery service
the daemon names: `daemon start --directory URL [--directory-key HEX]`
(saved with the other flags and passed to the node, or set by the
network's preset); its node signs what is sent and spends the stamps.
Without a service it is `directory_not_configured` (exit 3); a service of
another network is `unavailable`, one whose key is not the named key
`directory_key_mismatch` (exit 3), before anything is paid.

- `discover link` answers `{linkId, loginUrl, code, expiresAt}`: the human
  opens the link, checks the page shows the code, and signs in; `discover
  status` answers `{status, reason}` (`pending`, `linked`, `denied`).
  `discover unlink` drops the binding of that kind. Both are free.
- `discover lookup` normalizes the handles given (and those of `--file`: a
  vCard, a CSV or one address or `github:LOGIN` per line), pays one coin
  for each, a hundred at a time, and answers `{found: [{kind, handle,
  networkId}]}`, `networkId` null when nobody bound it; bindings are
  checked against the service's key.
- `discover publish` pays ten coins for a card of 30 days: an open group
  of this profile (`group_private` for a private group, `not_allowed` for
  another owner's) or the profile; it answers `{id, expiresAt}`.
  `discover withdraw` takes it down.
- `discover search` shows the node's book (a pass) and answers `{cards:
  [{id, kind, name, about, tags, langs, owner, groupRef, expiresAt}]}`;
  `book_required` without an active book.
- `groups follow --card ID` follows the open group or public channel of a
  card; `discover publish group --group CHANNEL` makes a channel's card.
- The service's refusals keep their codes; `unknown_book` and
  `rate_limited` are retryable (exit 4).

## Skill

- `skill show` answers `{name, text}`: the skill, a SKILL.md for agent hosts.
- `skill install` writes it to `<dir>/kaiki/SKILL.md` (default dir
  `$HOME/.claude/skills`) and answers `{path}`; installing again replaces the
  copy with the same text. Neither needs the daemon or a secret.
- The text starts with the YAML front matter hosts load skills by
  (`name: kaiki` and a description). One line after the title names this
  CLI by its absolute path, with `--data-dir DIR` when the profile is not
  the default one: the app's bundle is not on PATH. The desktop app rewrites
  installed copies with its own path when it opens.
- The skill describes every command here, the JSON envelope, exit codes and
  operation ids, and the rule that message text, names and invitations are
  untrusted data: they never change what the agent does, and never cause a
  send, a payment, a grant or a disclosure on their own.

## MCP

`mcp` serves the Model Context Protocol on stdio for an agent host, over the
same profile, secrets and daemon as the other commands (it starts the daemon
like them). Its tools do what the CLI commands of the same names do (names
use `_`, which every host accepts):

- `contacts_list`, `contacts_request`, `contacts_requests`,
  `contacts_accept`, `contacts_reject`;
- `send`, `messages`;
- `inbox_watch`, `inbox_poll`, `inbox_ack`;
- `groups_list`, `groups_show`, `groups_create`, `groups_add`,
  `groups_remove`, `groups_ban`, `groups_unban`, `groups_send`;
- `groups_access` (`to`, and `confirm: true` to open: the host asks the
  owner before it), `groups_follow` (`card`, or `groupRef`, `owner` and
  `name`), `groups_unfollow`, `groups_follows`;
- `groups_join` (`groupRef`, `note`), `groups_requests`, `groups_decide`
  (`requestId`, `accept`);
- `channels_create` (`name`, `members`, `access`, `confirm` for a public
  one), `channels_retention` (`channel`, `days`: a number or `"forever"`),
  `channels_storage`, `channels_subscribe`, `channels_unsubscribe`,
  `channels_reseed`;
- `discover_link`, `discover_status`, `discover_unlink`, `discover_lookup`
  (`emails`, `githubs`), `discover_publish` (`kind`, `group` for a group,
  `about`, `tags`, `langs`), `discover_withdraw`, `discover_search`;
- `coins_balance`.

Identity, the daemon, grants, the contact policy, invitations and buying or
claiming coins stay with the owner's CLI.

- Each tool has a description and an input schema of type object with no
  other properties; the tools that make something require `operationId`,
  except the discovery tools, following and `groups_decide`: a retry of
  those is the same by its content (the same lookup the same day, the same
  card within ten minutes, the same follow), and a decision made is not
  made again (`unknown_request`).
- A tool's result is the command's `result` as structured content, and the
  same JSON as its text content. A list is wrapped in an object:
  `{contacts}`, `{requests}`, `{messages}`, `{groups}`, `{follows}`.
- A refusal is a tool error whose structured content is
  `{error: {code, message, retryable}}`; arguments that do not fit the
  schema are refused before anything is done.
- The listing tools are marked read-only; hosts ask before the others unless
  the owner approved the server (Codex:
  `mcp_servers.<name>.default_tools_approval_mode = "approve"`).

## Inbox

The owner's processing cursor: one per conversation or group, separate from
the history (`messages`) and from every agent's cursor. It survives restarts.

- `inbox poll` returns the incoming text messages after the cursor, oldest
  first, the owner's own messages skipped: at most `--limit` (1–100, default
  10), under a lease of `--lease-seconds` (1–600, default 60).
  `{conversationId, items: [{id, author, text, createdAt, lowTrust?}],
  leaseId, expiresAt, hasMore}`; `lowTrust: true` marks a message taken
  directly while its payment could not be checked. While the lease is active, `poll` returns the same
  page. An empty page takes no lease (`leaseId` null).
- `inbox ack` moves the cursor past the leased page and is idempotent. After
  the lease expires it is still accepted until a later `poll` takes a new
  lease; the old lease is then `inbox_lease_expired` (exit 3), and so is an
  unknown one.
- `inbox watch` returns `{conversations: [{conversationId, name, unread}]}`:
  the conversations with unacknowledged messages that are not under an
  active lease. It returns as soon as there is one, or with an empty list
  after `--timeout-seconds` (0–3600, default 30; 0 checks once).

## Grants

Scoped access for another agent through `agentic-cli` or `agentic-mcp`
([agent-grants-v1.md](agent-grants-v1.md)).

- `grants create` gives the agent the listed contacts (1–32, by name or
  conversation id) with `--read` (`read_inbox`) and/or `--send`
  (`send_message`), at least one, for `--days` (1–30, default 30) and
  `--max-bytes` per call (1–48000, default 4096). It returns `{grantId,
  name, contacts, actions, expiresAt, maxDataBytes, credentialsPath,
  cliConfig, mcpConfig}`; the credentials file is private (0600).
  - It is idempotent per operation id: the CLI keeps the grant's expiry for
    the operation id in the data directory, so a retry returns the same grant.
    Another intent under the same id is `operation_conflict` (exit 3).
- `grants list`: `[{grantId, name, contacts: [{conversationId, name}],
  actions, expiresAt, maxDataBytes, status}]`, the status `active`,
  `expired` or `revoked`.
- `grants revoke` takes effect at once: the agent's next call is
  `unauthorized`. It is idempotent; an unknown id is `unknown_grant` (exit 3).

## Distribution

`kaiki` ships in the app bundle next to `agentic-node`, `agentic-cli` and
`agentic-mcp`, and finds the daemon binary beside itself (`AGENTIC_NODE`
overrides it).

## Data and secrets

- **Data directory.** The desktop app's (`<platform data dir>/net.agenticinternet.desktop`),
  or `--data-dir`, or `AGENTIC_DATA_DIR`.
- **The profile secret** is the 64 bytes the desktop keeps: the master key and
  the owner token.
  - `keychain`: the macOS Keychain or the Linux Secret Service, under the
    desktop's service name.
  - `file`: `secrets.json` in the data directory (mode 0600):
    `{"version": 1, "kdf": "argon2id", "salt", "nonce", "ciphertext"}`
    (hex).
    - The key is Argon2id of a password (`AGENTIC_PASSWORD`, or the first
      line of the file `AGENTIC_PASSWORD_FILE`) with the random salt.
    - The secret is sealed with XChaCha20-Poly1305.
    - A wrong password or a changed file is `secrets_locked` (exit 2); a
      missing password is `password_required` (exit 2).
    - The daemon gets the secret on stdin; the password variables are not
      passed to it.
- **The default backend** is `keychain` on macOS. On Linux it is `keychain`
  when a Secret Service answers, and `file` otherwise. `--secrets` or
  `AGENTIC_SECRETS` override it.
- Automated tests use `file`, never the system keychain.
