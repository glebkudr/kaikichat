# Start at login (2026-09-30)

The owner asked that the desktop app and the CLI be in autostart: done
directly where the system allows it, and otherwise asked of the user through
the system's own dialogs. Contracts: [spec/owner-cli-v1.md](../../../spec/owner-cli-v1.md)
("Start at login", `autostart`), [spec/desktop-gui-v1.md](../../../spec/desktop-gui-v1.md)
("Start at login"). Code: `crates/node/src/autostart.rs`.

## Design

- **Directly, with files only.** A login item is a file of the system's
  login manager: a launchd agent in `~/Library/LaunchAgents` (macOS), a
  systemd user unit enabled for `default.target` or an XDG autostart entry
  (Linux). None of them needs a permission, so nothing asks. Nothing is
  loaded now (`launchctl`/`systemctl` are not called): the next login runs
  it, and a test or a start never runs a second window or daemon.
- **What starts.** The installed app (a bundle in `/Applications` or
  `~/Applications`, the package's binary under `/usr` or `/opt`) opens its
  window; an install of `kaiki` from install.sh starts the daemon of the
  profile it opens by itself (`kaiki daemon start`). Both jobs leave the
  daemon running when their process ends (`AbandonProcessGroup`; systemd
  `Type=oneshot` + `RemainAfterExit=yes`, not `forking`, whose `ExecStop`
  would stop a daemon restarted from a terminal).
- **The system's dialog.** macOS may keep a launch agent from running until
  the owner allows it (turned off in System Settings → Login Items). The
  state comes from ServiceManagement's `SMAppService.statusForLegacyURL`
  (`RequiresApproval`), and the owner is asked by
  `SMAppService.openSystemSettingsLoginItems()`: once at a start, and again
  whenever the owner turns autostart on or presses "Open login items". The
  workspace forbids unsafe code, so both calls go through `osascript`'s
  JavaScript bridge (about 70 ms); `launchctl print-disabled` does not show
  this state (on this Mac it lists an agent the system reports enabled and
  misses one it blocks).
- **The owner decides.** `autostart.json` in the profile keeps the window's
  and the daemon's choices apart. The owner's off (the setting,
  `kaiki autostart off`, or the job taken out in the system, e.g.
  `systemctl --user disable kaiki`) holds at later starts.
- **Secrets.** The daemon's job carries `AGENTIC_SECRETS` and
  `AGENTIC_PASSWORD_FILE` (and the preset's variables), never a password: a
  password only in `AGENTIC_PASSWORD` makes `autostart on`
  `password_file_required` and the automatic start leaves the job out.

## Tests first

Written before the code and reviewed by an independent critic: REVISE
(the systemd unit's type was not fixed, so a unit that kills the daemon
right after login would pass; the password check could not fail; which app
copy registers was untested, risking the developer's real login items),
then ACCEPT after the fixes.

- `crates/node/src/autostart_tests.rs`: launchd jobs read back by `plutil`
  (labels, arguments, environment, `RunAtLoad`, `AbandonProcessGroup`,
  one job per profile); the systemd unit and the XDG entries; the owner's
  off, here and in the system; a job naming an old place written anew and
  the same job left untouched; a blocked job asked for once and again when
  turned on, a look asking nobody; the window's and the daemon's choices
  apart; only the installed app on its own profile opens at login (not
  `target/`, a disk image, App Translocation, Downloads, `AGENTIC_DATA_DIR`);
  the real macOS login items do not block a job they never saw.
- `crates/node/tests/support/owner_cli.rs`,
  `an_installed_kaiki_starts_the_daemon_at_login_until_the_owner_turns_it_off`:
  real processes in a home of their own; a build of kaiki, a named profile
  and a password only in `AGENTIC_PASSWORD` put nothing in; the install's
  first command (`--secrets file init`) does, with the file and not the
  password; off holds across a restart; on puts it back.
- `apps/desktop/src-tauri/tests/support/profile.rs`,
  `the_app_opens_at_login_until_the_owner_turns_it_off`: the window's
  commands over a real daemon; off holds when the app opens again; blocked is
  shown and `open_login_items` asks; a build has no setting.
- `apps/desktop/tests/autostart.test.tsx`: the setting, the blocked notice
  and its button, no setting where the app cannot open at login.

## Runs

- macOS arm64: `agentic-node` lib `autostart` 8/8, `processes owner_cli::`
  19/19 (7 s), `agentic-desktop` 25/25, frontend 126/126 with the i18n
  check of the twenty tables, clippy `-D warnings` and rustfmt clean.
- Real launchd, on a temporary profile and home: `kaiki --data-dir …
  autostart on` wrote `com.kaikichat.kaiki.<hash>.plist` (`plutil -lint`
  OK); `launchctl bootstrap gui/$UID` ran it: `runs = 1`, `last exit code =
  0`, kaiki gone and the daemon alive and answering `daemon status`
  afterwards. Booted out, the daemon stopped and the files removed.
- Linux aarch64 (container `ain-v1-arm`, `--profile portable-linux`, no
  systemd there, so the CLI test took the XDG entry): `autostart` 6/6 (the
  two macOS tests are not built), `owner_cli::an_installed_kaiki…` and the
  skill test 2/2.
- Visual fixture (`tests/visual.html?autostart=on|off|blocked|none`):
  Settings reviewed in English, light, blocked (checkbox, notice, "Open
  login items") and in Arabic, dark, off (right to left).
