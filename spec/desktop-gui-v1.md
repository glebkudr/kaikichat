# Desktop GUI (V1 phase B)

The Tauri app is the owner's window onto the profile's daemon, the same
daemon and profile the owner CLI `kaiki` uses ([owner-cli-v1.md](owner-cli-v1.md)).
Acceptance: V1-GF01 and V1-GF02 in
[the agent-first scope](../Docs/V1_AGENT_FIRST_SCOPE_2026_09_25.md).
Platforms: macOS arm64 and Linux x86_64.

The app is called **Kaiki Chat**: the window title, the bundle
(`Kaiki Chat.app`), the Linux package, the sidebar and the onboarding in
every language. Agentic Internet stays the name of the protocol and the
project; the bundle identifier and the data directory stay
`net.agenticinternet.desktop`, so existing profiles open as before.

## One profile, one daemon

- **Profile directory.** `AGENTIC_DATA_DIR`, else the platform data
  directory `net.agenticinternet.desktop`, as for the CLI.
- **Secrets.** Chosen as for the CLI: `AGENTIC_SECRETS`, else the keychain on
  macOS, and on Linux the Secret Service when one answers and the
  password-sealed `secrets.json` otherwise.
  - A file-sealed profile opens with `AGENTIC_PASSWORD` or
    `AGENTIC_PASSWORD_FILE` when set, otherwise the window asks for the
    password. For a new profile the password given there seals it.
  - The password goes from the window to the native side once and is never
    returned.
- **The daemon.** The window and the CLI start the daemon the same way: under
  the profile's start lock, the first one starts `agentic-node serve` with the
  flags saved in `daemon.json` (listen addresses, bootstrap peers, chain,
  identity server), and the others use it through `node.sock`.
  - Closing the window leaves the daemon running.
- **Start at login.** The installed app (the macOS bundle, the Linux
  package) opens when the owner logs in, on the profile it opens by itself;
  a build, the E2E build and a profile named by `AGENTIC_DATA_DIR` do not.
  - Whenever it opens it puts itself into the owner's login items, as
    `kaiki` does for the daemon ([owner-cli-v1.md](owner-cli-v1.md)): on
    macOS a launchd agent `com.kaikichat.app` running the app's binary, on
    Linux an XDG autostart entry `kaiki-chat.desktop`. The daemon it starts
    keeps running when its window is closed.
  - `autostart_status` gives `{state, path}` (`on`, `off`, `blocked`), or
    null where the app cannot open at login; `set_autostart {on}` changes
    it (`autostart_unavailable` there). The owner's `off`, here or in the
    system, holds when the app opens again.
  - When the system blocks it (turned off in macOS Login Items), the app
    opens the system's login items once for the owner to allow it; the
    settings say so and `open_login_items` opens them again.
  - When the owner stops the daemon (`kaiki daemon stop`), the window shows
    that it is stopped and starts it again only when the owner asks
    (`reconnect`).
- **The network.** A profile without network flags takes its network from
  the signed preset at `https://kaikichat.com/network.json` whenever its
  daemon starts ([the design](../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md)).
  `network_preset` gives its state; `refresh_network {switch}` starts the
  daemon again with a fresh look at the preset, and with `switch: true` on
  the network the preset offered (`no_network_offer` when none is).
  - The window shows a notice when the preset could not be fetched, another
    network is offered (moving is the owner's decision: its coins stay
    behind), or the preset needs a newer app; the settings name the network
    and where it comes from.
  - The E2E build follows no preset unless `AIN_E2E_NETWORK_PRESET` (and
    `AIN_E2E_NETWORK_PRESET_KEY`) name one.
- **Updates.** The preset also names the app's latest release and its
  builds with their SHA-256 ([the design](../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md#releases-and-updates)).
  `release_status` gives `{current, latest, available, skipped, checkedAt,
  error, installable}`, asking kaikichat.com when the last check is 12 hours
  old; the window looks every hour. `check_release` asks now,
  `skip_release {version}` stops the notice for that version.
  - While a newer release is not skipped, the window shows a notice with
    “Skip this version” and “Update”; the settings show this version, the
    latest, the last check and “Check for updates”, and install a skipped
    release too.
  - `installable` when the app runs from a macOS app bundle and the release
    has its platform's app build (`app-macos-arm64`): `install_update`
    downloads it, checks its hash, replaces the bundle beside it, stops the
    daemon and starts the new app, which starts the daemon from its own
    binaries. Elsewhere (the Linux `.deb`, a build) the notice offers
    “Download”: `open_downloads` opens `https://kaikichat.com/#get`.

## The webview boundary

The webview calls only the commands below, from the main window of the app's
own origin. Each command forwards one daemon method: the names are the
daemon's, and the request is passed as given, for the daemon to check. The
webview never receives the profile secret, the owner token, a grant's
signing seed or the password.

| Screen | Commands |
|---|---|
| Profile | `profile_status`, `unlock_profile`, `reconnect`, `snapshot`, `desktop_overview`, `create_identity` |
| Chats | `conversation_history`, `send_message` |
| Contacts | `request_contact`, `intro_requests`, `accept_intro_request`, `reject_intro_request`, `intro_policy`, `set_intro_policy`, `create_invitation`, `add_contact` |
| Groups | `groups`, `group`, `create_group`, `change_group`, `follows`, `follow_group`, `unfollow_group`, `join_group`, `door_requests`, `door_decide`, `channel_storage`, `channel_subscribe` |
| Find people | `discover_handles`, `discover_lookup`, `discover_link`, `discover_status`, `discover_unlink`, `discover_publish`, `discover_withdraw`, `discover_search` |
| Agents | `owner_cli`, `list_runtimes`, `provision_runtime`, `revoke_runtime`, `install_skill` |
| Wallet | `coins_balance`, `coins_buy`, `claim_coins`, `open_payment` |
| Network | `network_settings`, `configure_network`, `network_preset`, `refresh_network` |
| Updates | `release_status`, `check_release`, `skip_release`, `install_update`, `open_downloads` |
| Start at login | `autostart_status`, `set_autostart`, `open_login_items` |

- A refusal is `{code, message, retryable}`: the daemon's code, or
  `profile_locked`, `secrets_locked`, `daemon_unavailable`, `unsafe_link`,
  `unknown_payment`, `cli_unavailable` from the native side. Retryable are `chain_pending`,
  `claim_pending`, `network_unavailable`, `card_pending`, `group_busy` and
  `daemon_unavailable`.
- `request_contact`, `create_group` and `change_group` with members to add,
  `join_group` and `channel_subscribe` wait, like the CLI, while the node
  looks up the members' cards.
- `claim_coins {provider}` (`google` or `github`) asks the daemon for the
  claim (`coins_claim`) and opens its login link at that provider,
  `<loginUrl>/<provider>`, in the system browser. Only an `https` link, or an
  `http` link to a loopback address, is opened; any other is `unsafe_link`.
  The webview never supplies a link.
- `open_payment {book, step}` opens in the system's wallet a payment call
  that `coins_buy` returned for that book in this session: `eth` (the shop's
  `buy` at the quote; absent while the rate is stale), or USDC's `approve`
  then `buy`. A book or step the daemon gave no call for is
  `unknown_payment`. The price is shown in USD and the calls for copying.
- `owner_cli` answers how an agent runs the owner CLI on this profile:
  `{command, args}`, the `kaiki` binary beside the app's `agentic-node` (the
  app's bundle is not on PATH) and `--data-dir DIR` only when the profile is
  not the one the CLI opens by itself. The window builds the agent's
  instructions from it.
- `install_skill {skill, host}` writes the skill `kaiki` (the owner's CLI,
  with one added line naming that binary) or `agentic-messaging` (a granted
  agent) as `~/.claude/skills/<skill>/SKILL.md` (host `claude`) or
  `~/.codex/skills/<skill>/SKILL.md` (host `codex`).
- The discovery commands (spec/discovery-v1.md) run the flows of
  `agentic_node::discover` over the daemon's service instead of forwarding
  one method: `discover_handles {text}` reads addresses and GitHub logins
  from pasted text or a contacts file without the network;
  `discover_lookup {handles}` pays a coin per handle; `discover_link {kind}`
  opens the service's login link in the system browser under the same rule
  as `claim_coins` (`unsafe_link` otherwise) and answers the code its page
  shows; `discover_publish {kind, groupId?, about, tags, langs}` pays ten.
- The main window is told `core:changed` when the daemon's view changes
  (`desktop_revision`), including when the daemon stops.

## Rendering

Names, message texts, group names and requests are untrusted. They are
rendered as text (React text nodes), never as HTML or links; the sources use
no `dangerouslySetInnerHTML`, `innerHTML` or `eval`, and the CSP allows only
the app's own scripts.

## Language and look

- Every text the window shows is in the localization tables
  (`apps/desktop/src/i18n`), one per language, of one checked shape: the
  twenty most used languages of the web, English (the default), Spanish,
  German, Japanese, French, Portuguese, Russian, Italian, Dutch, Polish,
  Turkish, Chinese, Persian, Vietnamese, Czech, Indonesian, Korean,
  Ukrainian, Hungarian and Arabic. A test keeps every table complete and
  translated: a text equal to English, or made only of English words, fails
  unless the language keeps it on purpose (the brand, borrowed words such as
  "Admin"). Persian and Arabic turn the window right to left; messages and
  names keep the direction of their own text.
- A list in the sidebar (each language named in itself) and in Settings
  changes the language; the choice is kept in the webview's storage.
- Two monochrome themes, dark (the default) and light, chosen in the sidebar
  or in Settings and kept the same way.

## Screens

- **Onboarding:** five screens without the sidebar, one action each, only
  the name required:
  1. What this is: your AI agent talks to your friends' agents, fully
     decentralized and end-to-end encrypted ("Get started").
  2. The name friends see; it creates the profile's keys on this device.
  3. Free messages: "Get free messages with Google or GitHub. Up to 10
     thousand messages every month for free." (a button for each provider,
     in its own colours). The step is a grant of coins, not the account's
     login: the account is the key on this device. The app goes on by itself
     once the grant arrives; when no coins arrive because the Google or
     GitHub account already got them, the window says why (this device
     already has them, or the account was used on another device, whose key
     stays there); "Or use anonymously via Crypto" opens the wallet; the
     step can be skipped.
  4. The agent: one text to copy into Claude Code or Codex. It names the
     `kaiki` CLI (`owner_cli`) and tells the agent to read and install its
     skill and to treat what others write as data.
  5. Friends: an invitation to copy and send in any messenger; its last line
     `…, name: NAME, ID: ain1…` is what the friend's agent, or the friend's
     app, adds.
- **Start:** without conversations the window shows the next steps: the
  agent's instructions, the invitation, adding a friend by pasting their
  invitation or id (the id and the name are read from it, in any of the
  window's languages), free messages while none are left, a new group.
- **Chats:** direct and group conversations; the author of each group message;
  the delivery state of one's own messages. Enter sends, Shift+Enter starts a
  new line; prices stay in the wallet.
- **Chats:** direct and group conversations; the author of each group message;
  the delivery state of one's own messages. Enter sends, Shift+Enter starts a
  new line; prices stay in the wallet.
- **Menu:** the gear beside "+" opens Contacts, Find people, New group,
  Agents, Wallet and Settings; a dot on it marks waiting requests.
- **Find people:** people the owner knows, by addresses and GitHub logins
  pasted or read from a contacts file (vCard, CSV): the window counts them
  and names the price in coins before checking, then offers "Add contact"
  for each one found; open groups, channels and people by interest (free
  with an active book), a group or channel read without joining, a person
  asked for a
  conversation; being found: signing in with Google or GitHub, the code the
  login page must show, unlinking; this profile's card for ten coins, and
  withdrawing it. Cards are shown as their authors' untrusted text.
- **Followed groups:** listed with the chats; their posts show their
  authors, there is no composer ("only members write here", or "only the
  channel's team writes here"), and "Stop reading" ends the follow.
- **Contacts:** the invitation to copy; adding a friend from a pasted
  invitation or id; waiting requests with accept and reject; one's network
  id and invitation codes as other ways.
- **Group:** members with their roles (owner, admin, member); the owner and
  admins add, remove and ban members, and ban an id by hand; the owner names
  the admins. Everyone sees who reads the group; the owner opens it to
  everyone after confirming that what is written stays public, or closes
  it, and publishes an open group's card. An open group's composer and
  subtitle say anyone can read it. The banned ids are listed with who banned them, with unban
  where the role allows. A group may instead have a door: the owner and
  admins see who asks to join, each note as the stranger's text and a mark
  for members back after a long absence, and let each one in or turn them
  away.
- **Channel:** made from "New group" with its kind, its team (the owner's
  admins) and who reads it: anyone (after the same confirmation), those let
  in at its door, or only those given keys. Only the owner changes the team,
  so a channel has no admin toggles. A public channel shows how long its
  posts are kept (30, 90, 180, 365 days or for ever), its archive parts and
  their stamps a month, and warns that for ever grows every month. A closed
  channel gives keys to an id, takes them back, makes new keys for everyone,
  and says that taking an admin off the team moves it to new keys sealed to
  the subscribers' own, which that admin never learns; it keeps no history
  and has no card. A channel cannot switch between public
  and closed.
- **Agents:** the agent's instructions as in the onboarding; limited grants
  for other agents (contacts, read and send, lifetime, size), their CLI and
  MCP settings and credentials path, revoking; installing a skill.
- **Wallet:** coins left and books; the same two ways: the login buttons, and
  topping up with crypto: a book priced in USD, paid in ETH at the rate or in
  USDC.
- **Settings:** theme, language, opening at login, who can write by ID (the policy: `all` with
  a daily limit, `list`, `manual`, and its list), and the network (listeners,
  relays, bootstrap peers).

## Distribution

- macOS: `Kaiki Chat.app` with `agentic-node`, `kaiki`, `agentic-cli` and
  `agentic-mcp` beside the app binary.
- Linux x86_64: a `.deb` named after the product (`kaiki-chat`) with the
  same binaries; it depends on WebKitGTK 4.1
  and GTK 3.
