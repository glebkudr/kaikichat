# The signed network preset, 2026-09-28

Design: [Docs/V1_NETWORK_PRESET_2026_09_28_RU.md](../../../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md).
Branch `feature/v1-network-preset-20260928` on top of `implementation/v1`
a10a7dc.

## Tests (written first, then reviewed by an independent critic)

- `crates/node/src/network_preset_tests.rs` (15, 0.24 s) against a local
  HTTP server: a new profile takes the signed preset and keeps it when the
  server fails (non-200 with a valid body, a redirect, no answer, a silent
  server past the timeout); the signature covers `kaiki-network-preset/v1\n`
  and the preset's text as served, unknown fields are passed over; refused:
  another key, a changed preset, no envelope or signature, plain http,
  a host that only starts like loopback, credentials in a URL, routes
  without a peer id, repeated, none or 17, a bad network id, name or
  contract, a bad version, more than 64 KiB with or without a length;
  serials only go forward (the same serial again is current; an older or
  equal one of another network is not offered); the same network updates
  quietly; another network is offered and taken only when asked, as it was
  offered, not refetched; a changed saved offer or accepted preset counts as
  none; a preset for a newer app waits (`0.0.1` and our version are fine);
  each of the 8 network flags makes a profile manual with no fetch, listen
  addresses do not; routes are chosen at random, up to four.
- `crates/node/tests/support/owner_cli.rs`: a new profile's daemon starts
  with the preset's routes and identity server; commands on a running
  daemon never fetch; `network refresh` starts a new daemon (new pid) and
  shows the offer; `network switch` moves; `no_network_offer` is exit 3; a
  profile with network flags is manual and never fetches. Every other CLI
  test runs with the preset off. Owner CLI suite 13/13 in 7 s.
- Tauri: `network_preset` and `refresh_network` over a live daemon (the
  window's switch, the refusal without an offer, the identity kept), and
  both refused to other windows and origins. 21/21 in 7 s.
- Frontend: the settings section (name, source, last check, check again;
  manual without a check), the notice (unavailable with retry and a reload
  of the window, an offer moved only on the owner's click, an update
  without a button, a refusal shown with the offer kept, the notice during
  the first run); 16 new strings in all 20 tables, the anti-stand-in check
  passing. 93/93, `tsc`.
- clippy (`-D warnings`, all targets of the node, desktop and host crates)
  and rustfmt clean.

## Independent security review of the implementation

Found no way to change the network without the built-in key. Fixed from it:

- A signed preset the daemon would refuse (two routes of one peer, a chain
  given in part, no confirmations) broke every start: the preset now passes
  the daemon's own rules before it is signed or taken.
- A new profile took any signed preset: `MIN_SERIAL` is built in (raised at
  a release), the profile keeps the highest serial it saw, the state file is
  read field by field (a newer app's field no longer erases it) and written
  through a uniquely named temporary file, and the preset is resolved under
  the profile's start lock.
- `AGENTIC_NETWORK_PRESET_KEY` works only in debug builds.
- A switch stopped the daemon after taking the offer: it now checks the
  offer, stops the daemon, then takes the offer.
- `connect_profile` no longer joins a daemon before the start lock (the
  profile-directory and listener checks and one secret read stay); the
  fetch timeout is 5 s.
- An offer is replaced only by a newer one; an old preset asking for a
  newer app is just old; moving to another network asks for a confirmation.

Added tests: those refusals, `an_offer_is_replaced_only_by_a_newer_one`,
`an_old_preset_asking_for_a_newer_app_is_just_old`,
`a_saved_state_of_a_newer_app_is_still_read` (18 unit tests, 0.31 s), and the
confirmation in the frontend tests (95). CLI 13, Tauri 21, desktop host 9,
clippy clean after the fixes. Not changed: the window may keep an exited
daemon process unreaped until it closes (as `reconnect` did before).

## The discovery service in the preset

After implementation/v1 brought the discovery service (`--directory`,
`--directory-key`), the preset may name it: `directory` (https, checked
like the identity server) and `directoryKey` (64 hex, only with a
directory). A directory set by hand wins with its own key and does not make
a profile manual (`the_directory_comes_from_the_preset_unless_set_by_hand`;
19 unit tests). `scripts/network-preset.py` takes `--directory` and
`--directory-key`. Suites after the merge: frontend 104, CLI 14, Tauri 22,
desktop host 9, clippy clean. The published preset (serial 1) does not name
a directory yet.

## The real preset

`scripts/network-preset.py` built `deploy/site/network.json` (serial 1,
`kaiki-testnet-base-sepolia`, the ten nodes' routes read from the server's
`/data/published.json`, the chain of `deployments/base-sepolia.json`,
`https://id.kaikichat.com`, `minVersion` 0.1.0), signed with the offline key
(`.local/network-preset/`, public key
`70485f722511aa48711f5ac650eb6b09f90098538c1baee6eb22ef8152f84d2c`), and
`kaiki-preset verify` accepted it with the key built into the app.

Served by the site's nginx config locally: `200`, `application/json`,
`Cache-Control: no-cache`, the same bytes; `/privacy` still `200`.

Live check against the public testnet, with that nginx as the preset URL:
a fresh file-sealed profile ran `kaiki init`; `kaiki network` answered
`current`, `kaiki-testnet-base-sepolia`, serial 1; the daemon ran with four
of the ten routes, the Base Sepolia chain and `--identity-server
https://id.kaikichat.com`; `coins balance` read the chain; a card lookup of a
random id went through the swarm's intro mailboxes and answered
`card_not_found` (not `network_unavailable`). The profile was then stopped
and deleted.

## Screenshots

From the component fixture (`tests/visual.html`, headless Chrome, 1280×800):
[another network offered, dark, English](chat-switch-dark-en.png),
[a newer app needed, light, German](chat-update-light-de.png),
[settings not fetched during the first run, dark, Russian](onboarding-unavailable-dark-ru.png),
[the network in the settings, light, English](settings-network-light-en.png),
[the same with an offer, dark, Arabic (RTL)](settings-network-dark-ar.png),
[the confirmation of a move, light, Russian](chat-switch-confirm-light-ru.png).
