# One Keychain identity, and the window explains the dialog (2026-09-30)

The owner saw macOS's "Kaiki Chat wants to use your confidential information
stored in “net.agenticinternet.desktop” in your keychain" password dialog at
the app's first launch, before any window. The profile's key had been saved
by another build; the notarized 0.2.1 app was a stranger to that item.

## Which programs macOS lets read a key silently

`probe.swift` does what the keyring crate does (legacy find/add generic
password), on a new keychain file in a scratch directory, never the login
keychain, with keychain user interaction off: where macOS would show its
dialog, the read is refused instead. `run-probe.sh NEW_DIR` builds it twice
(two different binaries), signs the copies as the release does, and runs
every pair; the user's keychain search list is restored at once.
`probe-2026-09-30.txt` is the run:

| saved by \ read by | app (Developer ID, `net.agenticinternet.desktop`) |
| --- | --- |
| another binary, Developer ID, `net.agenticinternet.desktop` | read |
| Developer ID, `kaiki` (the 0.2.1 bundle) | refused -25293 |
| Developer ID, `net.agenticinternet.kaiki` | refused -25293 |
| ad-hoc (the install.sh CLI up to 0.2.2) | refused -25293 |

and the same both ways when the app saved the key. The keychain trusts a
designated requirement (identifier and team), not a path or a binary: a
different executable with the app's identifier and team reads the app's
items and the app reads its items, without a dialog. With dialogs off a
refusal is errSecAuthFailed (-25293), not errSecInteractionNotAllowed
(-25308), which a locked keychain gives.

So every macOS binary that reads the key is one program for the keychain:
the app's main executable and `kaiki` (in the bundle and in the install.sh
archive) are signed with `--identifier net.agenticinternet.desktop`. That is
`scripts/macos-sign-cli.sh` (branch `claude/cranky-shamir-794327`, the
session that makes `publish-cli.sh` refuse ad-hoc binaries). `agentic-node`
gets the secret on stdin; `agentic-cli` and `agentic-mcp` never read it.

## When macOS still asks, the window says why first

Keys saved by an older signature (the ad-hoc CLI, `kaiki` from the 0.2.1
bundle, a local build) still need the owner's one "Always Allow". Before
this change the app read the key in Tauri's `setup`, so the dialog came
before the window.

- `SecretStore::asks_first` (`crates/node/src/host.rs`): `KeychainStore`
  reads with interaction off (`SecKeychain::disable_user_interaction`, the
  lock dropped right after) and says yes for -25293/-25308, as the keyring
  crate's own `decode_error` passes them on (`keychain_dialog_tests`).
  `asks_before_opening` uses the same account as `connect_with`/`attach`.
- The window (`apps/desktop/src-tauri/src/lib.rs`): such a profile opens in
  the state `keychain`; nothing reads the key or starts a daemon; daemon
  commands, `reconnect` and `refresh_network` answer `keychain_consent`.
  `open_keychain` reads it (macOS asks now, once) and opens the profile; a
  denied dialog answers `keychain_denied` and keeps the explanation.
- The screen (`KeychainScreen`, 20 languages, the button named as the
  localized macOS dialog names it): `keychain-light-en.png`,
  `keychain-dark-ru.png`, `keychain-dark-ar.png` from
  `tests/visual.html?state=keychain`.
- `kaiki` prints a line on stderr before macOS asks, for the agent to pass
  on.

Checks (build wrapper, verification worktree): the desktop scenario
`profile::a_key_another_program_saved_is_read_only_after_the_window_explains_the_dialog`
(explanation with 0 dialogs, snapshot and reconnect refused, no daemon →
Deny → Allow: exactly one dialog each → the next launch explains again →
Always Allow → the launch after opens at once), the owner-command refusal
test, the existing keychain and password profile tests, the node unit test,
frontend `owner-screens`, `desktop-api`, `i18n`, `chat-shell` and `tsc`.
The tests were reviewed by a separate critic before the code (two rounds,
ACCEPT).

Not automated (the login keychain is off limits to tests): the real
`KeychainStore::asks_first` in a signed app against a key another signature
saved. That is an interactive run with the owner.
