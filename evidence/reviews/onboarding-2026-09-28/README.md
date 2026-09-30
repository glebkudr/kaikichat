# Onboarding for agents — 2026-09-28

Branch `feature/v1-onboarding-20260928` over implementation/v1 f9637a8
([spec](../../../spec/desktop-gui-v1.md), screens "Onboarding" and "Start").

## What changed

- The first run is five screens without the sidebar, one action each: what
  Kaiki Chat is; the name; free messages through Google or GitHub (the app
  goes on by itself when the grant arrives; crypto stays the anonymous way);
  one text to copy into Claude Code or Codex; an invitation for friends.
- The agent's text names the owner CLI `kaiki` (renamed from `agentic`) by
  its absolute path from the window command `owner_cli`; the installed skill
  carries the same path, because the macOS app bundle is not on PATH.
- Invitations are texts: the friend's agent adds the last line
  (`…, name: NAME, ID: ain1…`), or the friend pastes the whole message into
  the app, which reads the id and the name in any of the twenty languages.

## V1-GF01 on the branch

`apps/desktop/tests/native-owner.mjs` on the branch after the review fixes
(the harness also looks for the wizard's root, `.app-shell,.wizard`): passed
in 240 s ([result](macos-gf01/result.json)). Alice (Google) and Carol (GitHub) went
through the wizard; the grant moved them on by itself to the agent's
instructions, which named `target/debug/kaiki --data-dir <profile>` (the e2e
profile is not the default one). Screenshots:
[choice of login](macos-gf01/alice-onboarding-choice.png),
[agent's instructions](macos-gf01/alice-onboarding-agent.png).

Fast suites: vitest 81/81 (7 files, including every table's invitation read
back by the parser and the translation checks for twenty languages); Rust:
`owner_cli::` 11/11, `skill_tests` 1/1, agentic-desktop lib 1/1 and
`commands` 20/20 (an agent that runs exactly the window's command without
AGENTIC_DATA_DIR reaches the window's own identity; opening the window,
still locked, points an installed `kaiki` skill that names a gone bundle at
the CLI it ships).

## After an independent review

- The installed skill names the command with `--data-dir` for a profile the
  CLI would not open by itself, and the window rewrites installed `kaiki`
  skills with its own CLI when it opens (a moved or renamed app, a first run
  from a disk image).
- The agent's screen asks to move the app to Applications first when macOS
  runs it from a disk image or an AppTranslocation copy.
- The invitation parser needs `, name:` before the name and an id that is not
  part of a longer word, drops direction marks, and keeps up to 80 characters
  (not UTF-16 units); a profile made meanwhile by the CLI skips the name step.
- Open: the Keychain may ask the owner once when `kaiki` first reads the
  profile secret the app created (another binary); not checked on a signed
  build. A `~/.claude/skills/agentic` left by phase-B builds is not removed.
