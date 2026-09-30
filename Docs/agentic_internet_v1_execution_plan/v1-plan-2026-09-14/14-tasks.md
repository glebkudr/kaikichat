# 14. Local IPC and delivery on three OSes

A single Rust runtime, CLI/MCP, and Tauri work on macOS ARM64, Windows x64, and Linux x64 with private local IPC.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R11, AR-R12, AR-R17, AR-R20, AR-R23.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-P05** (voluntary updates, anti-rollback): TUF — Samuel et al., "Survivable Key Compromise in Software Update Systems" (CCS 2010) + theupdateframework spec; Uptane; Mercury (ATC 2017 — bandwidth-efficient rollback defense).

<a id="v1-p01"></a>
## V1-P01. Extract a transport-only IPC adapter

**Type:** backend. **Source cards:** U07, M01. **Position in dependency order:** 90.
**After:** [V1-M03](09-tasks.md#v1-m03), [V1-S01](02-tasks.md#v1-s01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Split Unix bind/connect/cleanup from the shared framing/owner-token/signed-agent checks.
2. Keep a single dispatcher/Core writer; remove cfg(unix) only after an equivalent platform implementation appears.
3. First verify concurrent clients, oversized frames, cancel/shutdown, and the second-daemon conflict on the existing Unix path.

**Verifiable scenarios:**

- The owner and a real signed agent pass through the shared broker after the refactor.
- An invalid token, revoked grant, and stale session do not work; cancellation admits no late side effect.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The Unix regression is green; the transport is not replaced with HTTP and does not duplicate application rules.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-p02"></a>
## V1-P02. Add private Windows named pipes

**Type:** backend. **Source cards:** U07. **Position in dependency order:** 91.
**After:** [V1-P01](14-tasks.md#v1-p01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Implement a local-only named pipe with an ACL for the right user and a stable profile endpoint.
2. Preserve single-instance lock, shutdown, endpoint cleanup, and reconnect semantics.
3. Run real cross-user/remote rejection tests on Windows; do not count a fake ACL as proof.

**Verifiable scenarios:**

- Two admissible clients talk to one daemon; a cold restart frees the endpoint.
- Another local user, a remote pipe, and a second daemon get no access.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A real Windows runtime/IPC works with verified ACLs and shared auth tests.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A Windows x64 runner with separate test users.

<a id="v1-p03"></a>
## V1-P03. Port the safe materialization of credentials and sidecars

**Type:** backend. **Source cards:** U07, M06, I01. **Position in dependency order:** 92.
**After:** [V1-P02](14-tasks.md#v1-p02).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Implement private atomic credential creation/read with an opened-file identity check on Windows.
2. Reconcile the .exe suffix, paths with spaces, bundled sidecars, and exact provisioning commands.
3. Check cold reads, replacements, and symlink/reparse risks through the existing private-handle policy.

**Verifiable scenarios:**

- CLI/MCP from an actual provisioning response start and use the correct seed after restart.
- A public or substituted file is rejected; an atomic replace error does not leave half of the credentials.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Windows CLI/MCP/desktop bootstrap passes without Unix-only path assumptions.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A Windows x64 runner; ordinary system keystores are verified in an isolated test session.

<a id="v1-p04"></a>
## V1-P04. Build debug/e2e and shipping artifacts per platform

**Type:** backend. **Source cards:** U06, I01, F06. **Position in dependency order:** 93.
**After:** [V1-P03](14-tasks.md#v1-p03), [V1-I06](07-tasks.md#v1-i06), [V1-U05](13-tasks.md#v1-u05), [V1-O10](11-tasks.md#v1-o10), [V1-N05](12-tasks.md#v1-n05).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Split platform-specific build/package steps in the existing scripts/build-desktop.mjs and CI adapters.
2. For native automation, provide a temporary debug/e2e keystore on Windows/Linux and its absence in shipping features.
3. Build the macOS app, Windows installer, and Linux package with real CLI/MCP/skill resources and pinned lockfiles.

**Verifiable scenarios:**

- A clean install on each OS contains working sidecars and a compatible profile.
- The release dependency graph contains no test plugin, dev server, OAuth secret, or test crypto.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The three installer artifacts have hashes, feature manifests, and noninteractive test-key cleanup.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** Three native runners and available signing/notarization credentials for the appropriate release stage.

<a id="v1-p05"></a>
## V1-P05. Implement voluntary updates and upgrade compatibility

**Type:** backend. **Source cards:** U06, F04, F02. **Position in dependency order:** 94.
**After:** [V1-P04](14-tasks.md#v1-p04), [V1-S01](02-tasks.md#v1-s01), [V1-I05](07-tasks.md#v1-i05).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Verify the update manifest signature/version and the user's explicit consent to install.
2. Keep the old working artifact/profile until a successful admissible upgrade.
3. A missing release CDN does not block the installed app; an unsupported downgrade does not write the new schema.

**Verifiable scenarios:**

- A previous-compatible client upgrades with identity/history/inbox intact.
- A substituted update, user refusal, and power loss do not turn the profile empty and do not require a mandatory update.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The voluntary update/rollback policy is confirmed on three OSes and does not give the vendor the right to switch off the chat.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-p06"></a>
## V1-P06. Verify clean install, key store, and native production smoke

**Type:** verification. **Source cards:** U06, U07, I01, X06. **Position in dependency order:** 95.
**After:** [V1-P05](14-tasks.md#v1-p05).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan/PLATFORM_IPC_FOLLOWUP.md`, `crates/node/src/ipc.rs`, `crates/node/src/runtime_client.rs`, `crates/node/src/mcp.rs`, `crates/node/src/lib.rs`, `crates/desktop-host/src/lib.rs`, `crates/desktop-host/src/e2e_secrets.rs`, `scripts/build-desktop.mjs`.

**Implementation plan:**

1. Install each shipping artifact on a clean OS, create an identity without OAuth, send/receive a message, restart.
2. Separately check the instrumented real bridge and the shipping artifact without instrumentation.
3. Inspect screenshots, verify the privacy of files/keystore, and delete test profiles after stopping the processes.

**Verifiable scenarios:**

- Identity and history persist; bundled CLI/MCP start from the installed path.
- Missing permissions/key access give an explicit error without a plaintext fallback; automation does not trigger a login Keychain prompt.

**Checks:** PLATFORM AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** All three OSes have their own actual evidence; cross-compilation and a macOS smoke do not cover Windows.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
