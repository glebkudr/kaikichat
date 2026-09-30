# Kaiki Chat network preset (2026-09-28)

Status: agreed with the user and implemented 2026-09-28 (branch
`feature/v1-network-preset-20260928`, checks —
[evidence](../evidence/reviews/network-preset-2026-09-28/)). The signing
public key is `PRESET_KEY` in `crates/node/src/network_preset.rs`.

## Why

The application (the window and the `kaiki` CLI) knows a single address:
`https://kaikichat.com/network.json`. Everything needed to operate on the
network — bootstrap nodes, the blockchain with contracts, the login server —
the client takes from this file, the **network preset**. Therefore:

- a fresh install works on the public network immediately, nothing needs
  manual configuration;
- nodes, RPC, contracts, or the login server can be changed on the server,
  without releasing a new version;
- moving from the test network to production is a new preset on the server,
  not a new release, as long as the protocol is compatible (see "Minimum
  version").

The preset determines where the application sends crypto payments (the
`BookShop` address) and which login server it trusts. That is why it is
**signed**: without the offline signing key the network cannot be swapped,
even with control over kaikichat.com, its DNS, TLS certificate, or the
Coolify server.

## What is in the preset

The signed part (payload) is a JSON object:

| Field | Type | Meaning |
|---|---|---|
| `network` | string `[a-z0-9-]{1,64}` | Network identifier. Different values are different networks: coins and data of one do not work in the other. |
| `name` | string up to 80 characters | Human-facing network name, shown as is. |
| `serial` | integer | Release number. One counter for all presets of all networks, only grows; not less than the `MIN_SERIAL` baked into the build. |
| `minVersion` | `"X.Y.Z"`, optional | The oldest application version this preset fits. |
| `bootstrap` | 1–16 addresses | Bootstrap nodes, multiaddr with `/p2p/<peer id>`, one address per node (distinct peer ids — that is how the daemon accepts them). |
| `chainRpc` | URL | Blockchain RPC. |
| `chainId` | integer | Blockchain network id. |
| `bookShop`, `grantIssuer`, `registry` | `0x…` address | Contracts. |
| `chainConfirmations` | integer | How many confirmations to wait for. |
| `identityServer` | URL | Login server via Google/GitHub. |
| `directory`, `directoryKey` | URL, 64 hex; optional | The people-search service and the key it signs bindings with (spec/discovery-v1.md). The key only together with the address. An address set manually (`--directory`) wins together with its key and does not make the profile manual. |
| `welcome` | `{agent, name, lobby, lobbyName}`; optional | Where a newcomer starts: the network's welcome agent (`agent` — its `ain1…`, `name` — a name up to 80 characters) and the lobby — the agent's open group (`lobby` — its `G` reference, 64 lowercase hex, `lobbyName` — a name up to 80 characters). Not passed to the daemon; `kaiki network` shows it, the skill teaches the agent to greet and join. The agent is a script on testnet nodes (`deploy/node/welcome.py`). |
| `release` | `{version, builds}`; optional | The latest app release: version `X.Y.Z` and builds by name (`cli-macos-arm64`, `cli-linux-x86_64`, `app-macos-arm64`), each `{url, sha256}`. Not passed to the daemon; see "Releases and updates". |

The blockchain and login server fields are the same as the `kaiki daemon
start` flags (`--chain-rpc`, `--chain-id`, `--book-shop`, `--grant-issuer`,
`--registry`, `--chain-confirmations`, `--identity-server`, `--bootstrap`).
The client skips unknown fields: new fields do not break old versions, and
incompatibility is expressed via `minVersion`.

URLs (`chainRpc`, `identityServer`) — `https://` only. An exception for
tests: `http://` on loopback (`127.0.0.1`, `[::1]`, `localhost`). Blockchain
fields are set all together or none, and `chainConfirmations` is at least 1 —
the same rules as the daemon's: a preset the daemon would reject cannot be
signed.

Example for the current test network:

```json
{
  "network": "kaiki-testnet-base-sepolia",
  "name": "Kaiki testnet (Base Sepolia)",
  "serial": 1,
  "minVersion": "0.1.0",
  "bootstrap": [
    "/ip4/51.91.126.3/udp/4101/quic-v1/p2p/12D3Koo…",
    "…nine more nodes 4102–4110…"
  ],
  "chainRpc": "https://sepolia.base.org",
  "chainId": 84532,
  "bookShop": "0x2233C293FBcE0EC59F6f9dcCb6e8A492B12CE597",
  "grantIssuer": "0x6710BDeBb92c9D1D4c22f09aAB1305501e5b850d",
  "registry": "0xCb3EFbf760293CE658c44f7D17bAb4Fb5d53CbB1",
  "chainConfirmations": 5,
  "identityServer": "https://id.kaikichat.com"
}
```

## File and signature

`network.json` on the server is an envelope:

```json
{"preset": "<payload: JSON text on a single line>", "signature": "<128 hex>"}
```

- An Ed25519 signature over the bytes `kaiki-network-preset/v1\n` + the UTF-8
  bytes of the `preset` string. The payload is signed as text, so JSON
  canonicalization is not needed.
- The public key is baked into the application (`crates/node`). A preset
  without a signature or with a foreign signature is rejected by the client
  entirely.
- File size — up to 64 KiB, request timeout — 5 s, the response must be 200,
  redirects are not followed.
- The signing secret key is kept offline in
  `.local/network-preset/` (mode 0600, never enters
  git, never printed in output). The owner keeps a copy.
- Losing or compromising the key requires a release with a new key. Later a
  second, backup key can be baked in for loss; it does not help against
  compromise.

## When the client fetches the preset

Only when it **starts the profile's daemon**, in one place for the window and
the CLI (`connect_profile`), under the profile startup lock: two
simultaneous starts do not contend for the profile file. A CLI command or
window connecting to an already running daemon does not fetch the preset:
this does not slow down every agent command.

The daemon outlives the window and can run for a long time, so a fresh
preset is applied on the next daemon start or via the "Check again" button
(`kaiki network refresh`), which restarts the daemon.

### Network source: preset or manual

- If the profile's `daemon.json` has at least one network flag (`bootstrap`,
  blockchain flags, or `identityServer`, saved by
  `kaiki daemon start --…`), the profile is **configured manually**: the
  preset is not fetched and changes nothing.
- Otherwise the profile follows the preset. The `--listen` flag does not
  affect this.
- The variable `AGENTIC_NETWORK_PRESET=off` disables the preset; a profile
  without network flags then runs isolated.

### How the client decides

The profile stores the last accepted preset. The client downloads the
current one and verifies the signature and fields:

| What arrived | What the client does | State |
|---|---|---|
| A valid preset, profile has none yet | Accepts | `current` |
| Same `network`, `serial` not less than the saved one | Accepts (nodes, RPC, and the rest update silently) | `current` |
| Same `network`, `serial` less than the saved one | Keeps the saved one (rollback protection) | `cached` |
| Different `network`, `serial` greater than all seen | Keeps the saved one and **offers** a network switch | `switch` |
| The already offered preset again | The offer stays | `switch` |
| Different `network`, `serial` not greater than seen | Ignores (a stale preset of another network) | `cached` |
| `minVersion` newer than the application (for a preset that would otherwise be accepted or offered) | Does not accept, asks to update | `update` |
| No response, not 200, bad signature or fields | Keeps the saved one | `cached` (with an error) |
| Same, and nothing saved | Starts the daemon without a network | `unavailable` |

The daemon always starts, even in the `unavailable` state: the profile can
be created and the network will connect on the next check.

An offer is replaced only by a preset with a greater `serial`; an accepted
preset of one's own network with a `serial` greater than the offer's removes
the offer.

All fields flow from the preset into the daemon flags. The daemon accepts no
more than 4 bootstrap nodes, so on every start the client takes 4 random ones
from the list, spreading the load across all nodes.

### Bootstrap routes and saved settings

Saved network settings (Settings → Network, `kaiki network lan`) override
the daemon's flags, but the bootstrap routes only when the owner named them:

- The "Nodes to join the network" field is empty by default and shows the
  routes in use in grey. The node then takes the `--bootstrap` flags of
  this start, so after saving LAN discovery, the DHT role or relays a
  profile still gets the routes of a newer preset and of a network it
  switches to.
- Addresses the owner types there replace the preset's routes, across
  restarts and network switches, until the field is cleared. Any save with
  the field empty returns the profile to the network's routes.
- In the node's saved settings, following the flags is the absent
  `bootstrapPeers` key (`null` in a request); `status.bootstrap.routes` of
  `network_settings` shows the routes in use.
- Settings saved by an earlier version keep the routes it saved (it saved
  the routes shown with any setting) until the owner clears the field in
  the window; `kaiki network lan` keeps them as it reads them.

## Network switch

A `network` change never happens silently. The profile is tied to the
network:

- keys, name, and history on the device remain;
- coins and stamp books of the other network (a different blockchain) do not
  work in the new one; the balance starts fresh;
- contacts remaining in the old network are unreachable until they move too.

Therefore the client shows an offer and switches only on the owner's action:

- window: a banner "Network <name> is available. The current network's
  coins do not work there" and a "Switch" button;
- CLI: `kaiki network` shows `state: "switch"` and `offered`;
  `kaiki network switch` switches.

The "Switch" button first shows that there is no going back on your own and
switches only after "Switch now". Switching verifies the offer exists
(otherwise nothing is stopped), stops the daemon, then accepts the offered
preset (already verified and saved in the profile) and starts the daemon with
the new flags: the profile never names a network in which no daemon runs.


## Releases and updates

Added 2026-09-30. The preset names the latest app release, the client tells
the owner about it, and on the owner's command replaces itself with the
release build. Updating is voluntary: there is no forced update and no
kill switch. Network incompatibility is still expressed only by `minVersion`.

### What the client knows

- `release.json` in the profile directory (mode 0600) keeps:
  - `newest` — the envelope of a valid preset with the largest `serial` of
    all downloaded ones, of any network, because the release is shared by
    the whole app;
  - `checkedAt` and `error` — the time and error of the last check;
  - `skipped` — the version the owner skipped.
- The signature of `newest` is verified on every read. A preset with a
  smaller `serial` does not replace it, so replaying an old preset does not
  bring back a different release. A newer preset without `release` withdraws
  the release.
- A release counts as new when its version is greater than the app's
  version. Until the new release is skipped, it is reported.

### When the client checks

- On every daemon start: the same request as for the network, including when
  the preset requires a `minVersion` newer than the app. This way the
  release is known in the `update` state too.
- Between starts, no more often than once in 12 hours:
  - the CLI checks in parallel with the command that works through the
    daemon and waits for the check no longer than 3 s;
  - the window asks every hour and reaches the server under the same rule.
  A failed attempt also counts as a check. A manually configured profile
  does not check on its own, only at the owner's request.
- At the owner's request — at once: `kaiki update`, `kaiki update --check`,
  the "Check for updates" button.
- A check changes only `release.json`: the network is still accepted at
  daemon start.

### What the owner sees

- CLI: every answer except `update` carries an `"update": {"current",
  "latest"}` field. `kaiki update --check` shows `{current, latest,
  available, skipped, checkedAt, error}`; `kaiki update --skip` skips the
  release. The skill asks the agent to tell the owner and to update only
  with the owner's consent.
- Window: a banner "Kaiki Chat X available. You have Y" with the buttons
  "Skip this version" and "Update" (or "Download", when the app cannot
  replace itself). Settings show the version, the last check, "Check for
  updates" and installing a skipped release.

### How the client updates

- A build is a tar.gz archive. The hash from the signed preset protects
  against swapped files on the server the same way the preset itself does.
- The archive is downloaded and unpacked into a temporary directory next to
  the installation, on the same volume. The installation is replaced as a
  whole by two renames, then the temporary directory is removed. On any
  error (`download_failed`, `hash_mismatch`, `bad_archive`) the installation
  stays unchanged.
- The CLI (`kaiki update`) updates only an install.sh installation. Its
  directory contains `install.json` — `{"build": "cli-<platform>"}` — and the
  new build's archive must carry the same. A cargo build and the `kaiki`
  from the app bundle answer `not_updatable`. After the swap the new build
  restarts the daemon if it was running (`kaiki network refresh`) and
  rewrites the installed skills in `~/.claude/skills` and `~/.codex/skills`.
- The window on macOS replaces the `.app` bundle with the `app-macos-arm64`
  build, stops the daemon and restarts itself; the new app starts the daemon
  from its own bundle. A Linux `.deb` package and debug builds do not
  replace themselves: for them the "Download" button opens
  `https://kaikichat.com/#get`.
- On macOS the builds are ad-hoc signed, so after the swap the Keychain asks
  once whether the new build may access the profile secret. Only a
  Developer ID signature can remove that question.

### Releasing

1. Raise the version in `Cargo.toml` (workspace),
   `apps/desktop/package.json` and `tauri.conf.json`.
2. Build and run `scripts/publish-cli.sh macos-arm64 DIR linux-x86_64 DIR
   [app-macos-arm64 APP]`. The script uploads the archives to `downloads/`
   (the latest, for `install.sh`) and to `downloads/<version>/` (for the
   release), and writes `deployments/release.json`.
3. Publish a preset: `scripts/network-preset.py` takes the release from
   `deployments/release.json`, then commit and deploy the site.

## Minimum version

If the new network needs an incompatible protocol or new fields, the preset
specifies `minVersion`. Old versions do not accept it, stay on the saved
preset, and show "Update Kaiki Chat to connect to <name>"
(`state: "update"`, field `required`). This is the only case where a network
switch requires a release.

## What is stored in the profile

`network-preset.json` in the profile directory (same permissions as the
profile):

- `accepted` — the accepted preset envelope as it came from the server (the
  signature is re-verified on every read);
- `offered` — the envelope of an offered preset of another network, if any;
- `highest` — the greatest `serial` the profile has accepted or received in
  an offer;
- `state`, `error`, `required`, `checkedAt` — the result of the last check.

The file is read field by field: a field a version does not understand (for
example, from a newer version) is treated as absent, the rest is preserved.
Writes go through a uniquely named temporary file and an atomic rename.

## States for the window and CLI

`kaiki network` and the window command `network_preset` return:

```json
{"source": "preset", "state": "current",
 "network": "kaiki-testnet-base-sepolia", "name": "Kaiki testnet (Base Sepolia)",
 "serial": 1, "checkedAt": 1790550000,
 "offered": null, "required": null, "error": null,
 "welcome": {"agent": "ain1…", "name": "Kaiki welcome",
             "lobby": "<G, 64 hex>", "lobbyName": "Kaiki Lobby"}}
```

`welcome` comes from the accepted preset (not from the offered network),
`null` if it is absent there.

| `state` | What the user sees |
|---|---|
| `current` | In Settings: "Network: <name>, from kaikichat.com". |
| `cached` | The same plus "could not verify kaikichat.com", a "Check again" button. |
| `unavailable` | Banner "Could not fetch network settings from kaikichat.com" and a "Retry" button. |
| `switch` | Banner with a network switch offer and a "Switch" button. |
| `update` | Banner "Update Kaiki Chat…". |
| `manual` | In Settings: "Network configured manually". |
| `off` | In Settings: "Preset disabled". |

Commands:

- CLI: `kaiki network` — the state; `kaiki network refresh` — restart the
  daemon with a fresh check; `kaiki network switch` — accept the offered
  network and restart (error `no_network_offer` if there is no offer).
- Window: `network_preset` — the state; `refresh_network {switch}` — restart
  with a fresh check, with `switch: true` — switching to the offered network.

## Overrides and tests

- `AGENTIC_NETWORK_PRESET` — `off` or another URL (for staging and tests);
  `AGENTIC_NETWORK_PRESET_KEY` — the public key hex for that URL, in debug
  builds only (tests). The release build always verifies with the baked-in
  key.
- The window E2E build has no preset by default: native runs set the network
  with flags, as now; `AIN_E2E_NETWORK_PRESET` and
  `AIN_E2E_NETWORK_PRESET_KEY` enable it for such a run.
- CLI and window tests that start daemons without flags set
  `AGENTIC_NETWORK_PRESET=off`, and preset tests spin up a local HTTP server
  with a preset signed by the test key.

## Server side

- `deploy/site/network.json` is served by the kaikichat.com site's nginx as
  `application/json` with `Cache-Control: no-cache`. Deployment is the usual
  Coolify from `main`.
- Releasing a preset:
  1. `scripts/network-preset.py --published published.json --seed SEED`
     builds the payload from `deployments/base-sepolia.json` (`nodeFlags`),
     node routes from `/data/published.json` on the `chat-nodes` volume, and
     the login server address, and sets `serial` one higher than in the
     current `deploy/site/network.json`.
  2. `kaiki-preset sign` verifies the payload with the same code as the
     client and signs it with the key from `.local/network-preset/`.
  3. `kaiki-preset verify` verifies the result with the baked-in key, then
     commit and deploy.
- `kaiki-preset keygen` creates the key pair once: the secret into a 0600
  file, the public key into the client code.

Moving to the production network: deploy contracts and nodes, release a
preset with a new `network` and a greater `serial`, deploy. New profiles land
on the production network right away; existing ones get an offer to switch.

## Security and privacy

- Breaking into the server, DNS, or TLS without the signing key yields only
  denial of service: clients stay on the saved preset, new profiles get no
  network.
- Rolling back to an old signed preset, including one of another network,
  does not work because of `serial` for a profile that has a saved preset. A
  new profile (or one that lost the file) accepts any signed preset not below
  `MIN_SERIAL`, so when releasing a version `MIN_SERIAL` is raised to the
  current `serial` to make old presets useless.
- The request to kaikichat.com at daemon start reveals the IP address to the
  server, like any web request. It carries no identifiers, cookies, or
  profile data.

## What is not included now

- Periodic network checking on a running daemon: the network is checked at
  daemon start and via the button (releases are checked more often, see
  "Releases and updates").
- Moving profile data between networks.
- A backup signing key and key revocation.

## Verification plan

- Unit tests of the decision against a local HTTP server with the test key:
  a new profile takes the preset and saves it; a foreign signature, payload
  substitution, and invalid fields are rejected; an old `serial` does not
  replace a new one; another network is only offered and accepted after
  `switch`; a `minVersion` newer than the application yields `update`; manual
  flags — not a single request to the server; without a server — the saved
  preset, and without one saved — `unavailable`; 4 distinct nodes are taken
  from the node list.
- CLI as processes: `init` on an empty profile starts the daemon with the
  preset flags; `kaiki network`, `refresh`, `switch`; after `kaiki network
  lan on` the daemon dials the route of a newer preset, and a route named
  in the window replaces it until it is cleared
  (`owner_cli.rs`,
  `saved_network_settings_keep_the_presets_routes_until_the_owner_names_others`).
- Window: the `network_preset` and `refresh_network` commands on a live
  daemon; banners and the Settings line in all 20 languages, screenshots.
- The real `network.json` on kaikichat.com is verified with
  `kaiki-preset verify` and by the first run of the release build on a clean
  profile.
