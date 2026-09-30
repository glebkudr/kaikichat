# V1 phase B — the GUI (V1-GF01, V1-GF02), 2026-09-27

The desktop app over the profile's daemon ([spec](../../../spec/desktop-gui-v1.md)),
checked on branch `feature/v1-gui-20260927` at 7a3b816, after merging
implementation/v1 (books priced in USD, paid in ETH at the rate or in USDC).

## V1-GF01: the GUI over the daemon

`apps/desktop/tests/native-owner.mjs` drives three hidden WKWebViews of the
e2e build (isolated vault, no Keychain) on a local anvil chain with ten
bonded holders (`af08_host_network`) and our identity server behind stand-in
Google and GitHub (`gui_host_identity_server`). Everything is done in the
window. Passed in 240 s ([result](macos-gf01/result.json)):

| Step | Seconds from start |
|---|---|
| network and identity server ready | 52 |
| Alice logs in with Google, Carol with GitHub; monthly coins granted | 68 |
| Bob tops up with crypto: the app opens the daemon's ETH payment call, paid with `cast`, a book of 1,000 coins | 99 |
| Alice asks Bob by id under his manual policy | 117 |
| Bob accepts Alice, rejects Carol; direct chat with markup; delivery status | 183 |
| Alice makes a group of Bob and Carol by ids | 185 |
| Bob (her contact) joins at once, Carol accepts under her manual policy; group message with authors | 210 |
| Alice names Bob admin; Bob removes Carol, cannot remove the owner; Carol reads nothing new | 240 |
| grant with CLI/MCP settings and a private credentials file, skill for Codex, revoke; wallet; English → Russian | 240 |

- Each window started its daemon with the flags saved in `daemon.json`
  (chain, bootstrap, identity server), the same way the CLI does.
- Untrusted markup in a profile name, a contact request, a message and a
  group name stayed text: no element, no script (`window.__pwned` unset).
- Screenshots: [macos-gf01](macos-gf01/).

The fast suites behind it:
- Tauri commands over live daemons (18): every command refused to other
  windows and origins; coded refusals; policy, requests, groups; wallet
  without a chain; buying with a stand-in JSON-RPC chain (ETH at the quote,
  USDC in two steps, USDC only while the rate is stale); login links opened
  only as the daemon gave them, at the chosen provider; skills; one profile
  for the window and the CLI (a daemon the CLI started, one stopped from the
  CLI and started again by the owner, a new password-sealed profile, a
  keychain profile).
- Identity server (19): GitHub besides Google, a verified primary email,
  one grant per GitHub account id, logins kept per provider, TLS for GitHub.
- Core: groups in the desktop list and history (108 conversation tests).
- Frontend (62): screens, roles, wallet, gates, localization tables and the
  language and theme switches, markup rendered as text, no HTML sinks.

## V1-GF02: installable builds, production smoke, screenshots

| | macOS arm64 | Linux x86_64 |
|---|---|---|
| Build | `scripts/macos-release.sh`: `Agentic Internet.app`, ad-hoc signed, `codesign --verify --deep --strict`, no automation plugin | `scripts/linux-x86_64/build.sh` in `ain-v1-amd64-base` (Ubuntu 24.04 amd64, Rosetta under OrbStack): `agentic-internet_0.1.0_amd64.deb`, depends on `libwebkit2gtk-4.1-0`, `libgtk-3-0` ([package](linux-deb-smoke/package.txt)) |
| Production smoke | `scripts/macos-smoke.sh`: the release app with a temporary password-sealed profile starts the daemon; the bundled `agentic` uses the same profile (`init`, `contacts policy`); the daemon outlives the window ([outputs](macos-release-smoke/)) | `scripts/linux-x86_64/smoke-deb.sh` in a clean `ubuntu:24.04`: `apt install` of the .deb, the app under Xvfb starts the daemon, the packaged CLI initialises the same profile and the window shows it ([outputs](linux-deb-smoke/)) |
| Native GUI case | `native-e2e.mjs --case chat` on WKWebView ([result](macos-chat/result.json)) | the same runner on WebKitGTK under Xvfb, `scripts/linux-x86_64/check-gui.sh` ([result](linux-chat/result.json)) |
| Screenshots | [macos-gf01](macos-gf01/), [macos-chat](macos-chat/) | [linux-chat](linux-chat/), [linux-deb-smoke](linux-deb-smoke/) |

The macOS release window was not captured: this session has no
screen-recording permission. Its screens are the e2e build's (the same
frontend and WKWebView, without the automation plugin).

## Found and fixed on the way

- `agentic daemon start` with flags on a fresh machine made the profile
  directory 0755 while saving `daemon.json`, and the daemon then refused it.
- The desktop list and history did not include groups: a new group was
  missing from the window.
- A contact's group invitation joins at once even under a manual policy
  (as core intends); the runner had expected a request.
- A message sent before the sender's node has applied a removal is sent in
  the old epoch and is still readable by the removed member; the runner now
  writes after the sender knows the removal (the protocol's behaviour).
- The fullwidth "＋" has no glyph in common Linux fonts.
- Under Rosetta `ps` repeats the program path; the webview's storage
  (language, theme) outlives a run on macOS — the harness resets it.
