# Group bans (2026-09-28)

The owner and admins of a group ban network ids (spec/groups-v1.md, Bans).

- The bans ride in the MLS group context (private-use extension `0xF1A0`,
  required of every member once set), changed only by commits: every member
  and every newcomer agrees on them. Readers check each commit with
  `agentic_protocol::group::check_bans`.
- A ban removes a member in the same commit; nobody adds a banned id until
  it is unbanned (`banned`); only the owner lifts the owner's bans. Clients
  built before bans are refused as invitees of a group with bans, and a
  group with such a member takes none (`member_outdated`); groups made
  before this build take none.
- Closed with it: a removed member could go on writing into the epochs it
  still knew, and the others read it until the next commit. Now nothing more
  of a removed member's is taken once its removal is applied (what was read
  before stays; a message it wrote just before, not yet delivered, is lost).

Tests (tests first, then an independent backend-test-critic: REVISE, then
ACCEPT):
- core: `the_owner_bans_an_id_and_nobody_adds_it_back_until_unbanned`,
  `an_admin_bans_plain_members_and_the_owner_bans_anyone_but_itself`,
  `a_commit_changes_the_bans_only_within_its_committers_role` (the reader's
  rule table), and the removal test's refused late message of the removed
  member; the core suite 111 passed.
- crypto: `group_data_is_agreed_by_commits_and_refuses_clients_that_cannot_carry_it`;
  28 passed.
- rig: `a_banned_member_is_not_added_back_until_the_owner_unbans_it` (owner
  IPC through the swarm and the notaries); node lib 154 passed.
- CLI and MCP (`groups ban|unban`, `groups_ban`, `groups_unban`): owner CLI
  processes 11 passed.
- desktop: 87 passed (`tsc` clean), all twenty languages translated.
- Not run: the native desktop gate and a native swarm run (they need the
  main checkout); workspace clippy is clean for the touched crates, and the
  `crates/node/src/host.rs` test-module lint and the Tauri macro in a
  worktree without a built frontend fail as before this change.

Screenshots from the component fixture (`tests/visual.html`, headless Chrome,
1280×800): [dark, Russian](group-dark-ru.png),
[light, English](group-light-en.png), [light, Arabic (RTL)](group-light-ar.png).
