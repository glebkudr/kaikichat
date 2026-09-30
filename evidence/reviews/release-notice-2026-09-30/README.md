# Release notice and self-update (2026-09-30)

The owner asked that the server announce a new version, the client tell of
it in the window and in `kaiki`, and the owner either skip it or have the
client replace itself without downloading anything by hand. Design:
[Docs/V1_NETWORK_PRESET_2026_09_28_RU.md, "Releases and updates"](../../../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md#releases-and-updates);
contracts: [spec/owner-cli-v1.md](../../../spec/owner-cli-v1.md) (`update`,
the `update` field of every answer), [spec/desktop-gui-v1.md](../../../spec/desktop-gui-v1.md)
(Updates).

## Tests first

Backend tests were written before the code and reviewed by an independent
critic: REVISE (two blocking issues — a mechanical replacement left `self`
in two free functions; macOS `tar` put AppleDouble entries into the test
archives, unlike `scripts/publish-cli.sh`; and missing scenarios: a network
that needs a newer app, the 12-hour check between daemon starts through the
CLI, a failed update with the daemon running), then ACCEPT after the fixes.

- `crates/node/src/network_preset_tests.rs`: only valid releases are signed;
  a daemon start learns the release; the newest serial's release wins and an
  older preset cannot bring another back, a newer one without a release
  withdraws it, another network's preset counts; the kept release is
  believed only while its signature holds; checks between starts at most
  every 12 hours, a failed one counts and keeps the release, the network is
  not changed by a check; a profile set by hand asks only when the owner
  does; a skipped version stays quiet until a newer one; a network that
  needs a newer app names the release to update to.
- `crates/node/src/update_tests.rs`: an install is replaced whole by its
  announced build (a hard-linked witness of the old binary is untouched);
  other bytes, a missing file, an archive of something else or of another
  build change nothing and leave nothing behind; only an install from the
  archive updates itself; an app bundle is replaced whole.
- `crates/node/tests/support/owner_cli.rs` (real `kaiki`/`agentic-node`
  processes, an install made of this build's binaries and the marker, a
  published build of scripts that run them): the notice in every answer,
  `update --check`, the 12-hour check alongside a command, `update --skip`,
  a wrong hash with the daemon running (same pid, same files), then
  `update`: new files, the daemon started again from the new build's node,
  the installed skill rewritten; and the refusals `no_update`,
  `unavailable`, `no_build`, `not_updatable` (a cargo build asks nothing),
  an update without a daemon starts none.
- `apps/desktop/src-tauri/tests/support/profile.rs`: the window's
  `release_status` from a preset, no second question within 12 hours,
  `install_update` refused where the app is no bundle, `open_downloads`,
  `skip_release`, `check_release`; `installable` for a bundle.
- `apps/desktop/tests/release.test.tsx`: the notice, update, skip, download,
  a failed update, the hourly look, the first run, the settings.

## Screenshots

Component fixture (`tests/visual.html?release=…`), headless Chrome,
1280×800: [a newer release, dark, English](chat-release-dark-en.png),
[light, Russian](chat-release-light-ru.png),
[an app that cannot replace itself: Download, dark, Arabic (RTL)](chat-release-download-dark-ar.png),
[a refused update, light, German](chat-release-failed-light-de.png),
[the first run](onboarding-release-dark-en.png),
[the settings with a newer release, light, English](settings-release-light-en.png),
[up to date, dark, Japanese](settings-current-dark-ja.png).

## Checks

Through `scripts/build-storage.py --worktree`, macOS arm64, version 0.2.0:
`cargo fmt --check`; `cargo clippy --locked --workspace --all-targets -D
warnings`; `agentic-node --lib` 252 passed (3 ignored, as before);
`processes owner_cli::` 18 passed; `agentic-desktop` and
`agentic-desktop-host` 23 + 9 + 1 passed (1 ignored, as before); vitest 12
files, 122 tests; `tsc --noEmit`; `vite build`.

Not run here: the native desktop gate (`check-native.mjs` needs the main
checkout) and a real replacement of an installed `.app` (the app has no
published build yet; `install_app` is covered by the unit test on a bundle
directory).
