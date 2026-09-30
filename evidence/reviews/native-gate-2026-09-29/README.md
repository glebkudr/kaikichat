# Native desktop gate — 2026-09-29

`python3 scripts/build-storage.py run node scripts/check-native.mjs` (the
full gate) in the main checkout `/Users/glebk/Code/chat`, fast-forwarded
to `origin/implementation/v1` and clean; nothing was written to it while
the gate ran.

## First attempt, at ad26ed64: failed in the chat case

`scripts/check.sh` passed; the native chat case stopped after its second
outcome, at `a.button('Agents')`: the sidebar no longer has an Agents
button since the onboarding redesign moved the screens into the gear menu
([result](first-attempt-result.json),
[the window at the failure](first-attempt-alice-failure.png)). The GF01
runner already opens them with `nav('agents')`; the gate's chat case did
not. Fixed in the test (954bf25a); no product change.

## Second attempt, at 954bf25a: passed in 7 min 23 s

- `scripts/check.sh`: rustfmt, workspace Clippy with `-D warnings`, 684
  Rust tests passed (0 failed, 11 ignored — the native and manual runs),
  41 Foundry tests, the site's and the welcome agent's Python tests (6 and
  30), 113 frontend tests, `tsc`, `vite build`.
- The debug `e2e` bundle and `apps/desktop/tests/native-e2e.mjs`, both
  cases, on WKWebView ([result](result.json)):
  - chat: two windows, onboarding, invitation, MLS contact, message, reply
    and signed receipts ([Alice's chat](alice-chat.png)); the window closed
    and reopened with the same identity and history while its daemon kept
    receiving; the Agents screen grants a scoped runtime whose MCP and CLI
    commands launch the bundled binaries, share idempotency and delivery
    state, and a revoke denies both ([agents](alice-agents-active.png));
  - network: the DHT role saved and restored, an independent packaged
    relay/AutoNAT provider configured in Settings, messages and receipts
    through its circuit, preferences restored after a daemon restart
    ([settings](network-alice-settings.png)).
- The release bundle `Kaiki Chat.app`, ad-hoc signed, `codesign --verify
  --deep --strict`, and no automation plugin in the default dependency
  graph.
