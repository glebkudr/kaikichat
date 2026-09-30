# 13. Everyday desktop

The user can use chat, search, notifications, and recovery through an ordinary interface.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R09, AR-R10, AR-R19, AR-R20.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-U02** (encrypted local search / index): crash-safe index↔data consistency — Pillai et al. (OSDI 2014); F1 online schema change as an analogy for index migration. The remaining tasks of the chapter are UI plumbing.

<a id="v1-u01"></a>
## V1-U01. Show history, gaps, and recovery without false completeness

**Type:** frontend. **Source cards:** U01, D05. **Position in dependency order:** 85.
**After:** [V1-H11](01-tasks.md#v1-h11), [V1-G05](08-tasks.md#v1-g05), [V1-M01](09-tasks.md#v1-m01).

**Change boundary / entry points:** `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/useMessageHistory.ts`, `apps/desktop/src/HistoryRecovery.tsx`, `apps/desktop/src/desktop-api.ts`, `apps/desktop/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/desktop_history.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Continue the existing pagination, distinguishing loaded/local/retained/complete from unavailable/expired/rejoin.
2. On a new pointer/epoch, update the projection without erasing already imported history.
3. Route retries into the shared Core recovery; do not make UI requests bypassing paid admission.

**Verifiable scenarios:**

- History of more than 1051 records scrolls without losing position; a cold reopen shows the same messages.
- A partial graph/MLS gap is not displayed as an empty completed chat.

**Checks:** FRONTEND DESKTOP FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A person sees the reason for incompleteness and the admissible action; a native flow confirms the backend state.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-u02"></a>
## V1-U02. Add local search over the protected history

**Type:** backend. **Source cards:** U05, F04. **Position in dependency order:** 86.
**After:** [V1-S01](02-tasks.md#v1-s01), [V1-U01](13-tasks.md#v1-u01).

**Change boundary / entry points:** `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/useMessageHistory.ts`, `apps/desktop/src/HistoryRecovery.tsx`, `apps/desktop/src/desktop-api.ts`, `apps/desktop/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/desktop_history.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Index only the admissible local decrypted history inside the encrypted profile.
2. Update the index atomically with import/delete and request bounded pages.
3. After lock/recovery/version upgrade, synchronize the index without plaintext in external caches.

**Verifiable scenarios:**

- Search finds exact messages/contacts after imports and restart.
- Disk-full/rollback leaves no search result without a message; a locked profile does not disclose content.

**Checks:** FRONTEND DESKTOP FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Search works in the ordinary UI and does not move plaintext into an unprotected DB or telemetry.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-u03"></a>
## V1-U03. Wire up notifications and app lock

**Type:** backend. **Source cards:** U05, I01. **Position in dependency order:** 87.
**After:** [V1-U01](13-tasks.md#v1-u01), [V1-I06](07-tasks.md#v1-i06).

**Change boundary / entry points:** `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/useMessageHistory.ts`, `apps/desktop/src/HistoryRecovery.tsx`, `apps/desktop/src/desktop-api.ts`, `apps/desktop/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/desktop_history.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Tie native notifications to the durable inbox, user setting, and lock state.
2. On lock, close renderer access to sensitive views/keys and revoke the transient UI session while keeping an explicit daemon policy.
3. Control text preview, sound, and navigation to the right chat without redelivery.

**Verifiable scenarios:**

- One message does not create a duplicate notification after restart; the action opens the right conversation.
- The locked preview does not show secret text; an OS permission refusal does not break delivery.

**Checks:** FRONTEND DESKTOP FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Notifications/lock are verified on each target OS; screenshots and OS settings are not substituted with UI mocks.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-u04"></a>
## V1-U04. Add backup/recovery and clear system errors

**Type:** frontend. **Source cards:** U05, I06. **Position in dependency order:** 88.
**After:** [V1-I05](07-tasks.md#v1-i05), [V1-U01](13-tasks.md#v1-u01).

**Change boundary / entry points:** `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/useMessageHistory.ts`, `apps/desktop/src/HistoryRecovery.tsx`, `apps/desktop/src/desktop-api.ts`, `apps/desktop/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/desktop_history.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Build export/import/version warnings into the current profile flow.
2. Show disk-full, wrong key, failed migration, missing authority, and rejoin with a specific safe action.
3. Rule out silent reset and offering profile deletion as the regular way to restore sending.

**Verifiable scenarios:**

- The user transfers the profile to a clean install and sees the saved history.
- An import error preserves the old profile; an incorrect recovery does not restore a revoked device.

**Checks:** FRONTEND DESKTOP FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Recovery is available without raw RPC and internal scripts; errors match actual backend results.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-u05"></a>
## V1-U05. Verify XSS, malicious content, and UI accessibility

**Type:** verification. **Source cards:** U01, U03, U04, U05, X05. **Position in dependency order:** 89.
**After:** [V1-G10](08-tasks.md#v1-g10), [V1-M04](09-tasks.md#v1-m04), [V1-B05](06-tasks.md#v1-b05), [V1-U02](13-tasks.md#v1-u02), [V1-U03](13-tasks.md#v1-u03), [V1-U04](13-tasks.md#v1-u04).

**Change boundary / entry points:** `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/useMessageHistory.ts`, `apps/desktop/src/HistoryRecovery.tsx`, `apps/desktop/src/desktop-api.ts`, `apps/desktop/src/types.ts`, `apps/desktop/src-tauri/src/lib.rs`, `crates/core/src/desktop_history.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Treat message/markdown/attachment previews as untrusted data in the current renderer.
2. Check CSP, external links, keyboard focus, labels, and error announcements in the main flows.
3. Take and inspect headless screenshots of all new states; the native bridge without mocks passes a limited malicious-input scenario.

**Verifiable scenarios:**

- Text instructing to disclose a key or perform a send remains message content.
- XSS/unsafe URI/path does not run code/privileged calls; changing parameters requires a new valid action context.

**Checks:** FRONTEND DESKTOP FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Basic user flows are accessible, contain no secret-bearing logs/previews, and pass a visual check.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
