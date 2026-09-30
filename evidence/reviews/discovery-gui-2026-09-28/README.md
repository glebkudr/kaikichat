# Discovery in the window, MCP and the directory's key (2026-09-28)

What the owner's window, the owner CLI's MCP server and the daemon gained
for discovery (spec/discovery-v1.md, spec/desktop-gui-v1.md,
spec/owner-cli-v1.md).

- The daemon names the discovery service and the key it signs bindings
  with (`agentic-node serve --directory URL --directory-key HEX`, saved by
  `kaiki daemon start`; the network preset will set them from `directory`
  and `directoryKey`). The CLI and the window ask it (`discover_config`)
  and refuse a service whose policy names another key
  (`directory_key_mismatch`) before anything is paid.
- The discovery flows moved from the CLI binary into `agentic_node::discover`,
  used by the CLI, its MCP tools and the window.
- MCP: `discover_link|status|unlink|lookup|publish|withdraw|search`,
  `groups_access` (`to`, `confirm`), `groups_follow|unfollow|follows`.
- The window: "Find people" (addresses and GitHub logins pasted or read
  from a contacts file, counted and priced before checking; cards by
  interest; signing in with Google or GitHub with the code to compare; the
  profile's card), who reads a group (opening only after the owner
  confirms), the group's card, followed groups listed with the chats and
  read without a composer. Twenty languages.
- The core lists follows in the window's overview and serves their history.

Tests (tests first, then an independent backend-test-critic: REVISE, then
ACCEPT): core `the_owner_inbox_and_the_window_read_a_followed_groups_posts`;
node `discover_tests` (pasted addresses); owner CLI
`a_discovery_service_with_another_key_than_the_named_one_is_refused` and the
MCP test (new tools, `groups_access` confirmed and not, `groups_follows`,
`discover_search` unconfigured); desktop
`the_discovery_screen_opens_a_login_link_reads_addresses_and_follows_a_group`
(an HTTP stand-in for the service; an unsafe login link opens nothing) and
the new commands refused to other windows. Frontend: `tests/discover.test.tsx`
(9) and the translation checks. Results: core 117, node lib, processes 70,
desktop 21 native + 96 frontend, clippy clean for the workspace.

Screenshots from the component fixture (`tests/visual.html`, headless
Chrome, `?open=discover&demo`, `?chat=NAME&members`):
[Find people, light, English](discover-light-en.png),
[dark, Russian](discover-dark-ru.png), [light, Arabic (RTL)](discover-light-ar.png),
[an open group's panel](group-open-light-en.png),
[a followed group](follow-dark-en.png), [the start screen](home-light-en.png).
