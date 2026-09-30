# 07. Devices, delegation, and safe backup

Profile recovery and device change do not restore revoked rights and do not clone active ratchets.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R06, AR-R17, AR-R19, AR-R20, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-I01** (revoke + fences): fencing tokens (Kleppmann); Chubby (OSDI 2006).
- **V1-I02** (device↔identity↔MLS leaf): RFC 9420 — leaf per device; self_remove — draft-ietf-mls-extensions; proof-of-possession during onboarding.
- **V1-I03–V1-I05** (versioned encrypted backup, anti-rollback): Memoir (S&P 2011 — rollback via counter+continuity chain); ROTE (USENIX Security 2017); Signal SVR + Green's critique; WhatsApp E2EE backup whitepaper; Apple iCloud Keychain escrow; DOGM (S&P 2024 — the conflict of history sharing vs FS/PCS).
- **V1-I04** (atomic export): shadow-validate-commit; Pillai et al. (OSDI 2014).

<a id="v1-i01"></a>
## V1-I01. Finish the device/org grant policy

**Type:** backend. **Source cards:** I02, I03. **Position in dependency order:** 41.
**After:** [V1-C04](00-tasks.md#v1-c04), [V1-S01](02-tasks.md#v1-s01).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Extend the existing capabilities only with the missing device/org constraints, linking owner/device epochs.
2. Check methods, contacts/groups, TTL, budget, and delegation depth in the shared broker before an action and when resuming a job.
3. Make revoke atomic with the change of the corresponding fences; do not update independent UI copies of rights.

**Verifiable scenarios:**

- Valid devices and two runtimes operate within their own limits.
- An old epoch, an extension of a child grant, and a revocation between prepare/commit forbid the action without returning spent budget.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A single authority matrix covers UI/CLI/MCP, devices, and the organizational messaging budget.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-i02"></a>
## V1-I02. Bind a device to the identity and MLS leaf

**Type:** backend. **Source cards:** I01, I03, I05. **Position in dependency order:** 42.
**After:** [V1-I01](07-tasks.md#v1-i01), [V1-W03](04-tasks.md#v1-w03).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Confirm a new device with a bounded owner/recovery challenge and proof of possession.
2. Persist the binding of device key/NetworkID/MLS leaf and the key package lifecycle.
3. Revoking a device forbids future actions and requires a valid MLS epoch transition; do not promise deletion of already received plaintext.

**Verifiable scenarios:**

- A person with two devices receives permitted messages without mixing human identity and leaves.
- A repeated onboarding, a foreign device key, and a stale owner challenge do not connect a device.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a real device join/remove and cold state; future group integration uses this shared service.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-i03"></a>
## V1-I03. Define the versioned encrypted backup

**Type:** tests. **Source cards:** I06, F04. **Position in dependency order:** 43.
**After:** [V1-I02](07-tasks.md#v1-i02), [V1-S01](02-tasks.md#v1-s01).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Produce a single consistency snapshot of identity/contacts/history/MLS/outbox/grants without writing plaintext to temp/logs.
2. Define the envelope version, integrity, password/recovery-key encryption, and compatibility rules.
3. Choose a safe restore: transfer/rejoin with new device state instead of cloning two active ratchets/spend writers.

**Verifiable scenarios:**

- A backup after restart contains a consistent slice of messages and pending operations.
- A wrong password, truncation, and an unsupported critical version are rejected without changing the working profile.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There are format fixtures and an independent test ACCEPT; the antirollback authority policy is written down explicitly.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-i04"></a>
## V1-I04. Implement atomic export and validated import

**Type:** backend. **Source cards:** I06, F04. **Position in dependency order:** 44.
**After:** [V1-I03](07-tasks.md#v1-i03).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Create the encrypted export in a user-chosen location with bounded IO and atomic completion.
2. Validate the import in a separate temporary profile, then switch to the intact result.
3. On an incompatible schema apply S01; do not carry over local active IPC tokens as new authorities.

**Verifiable scenarios:**

- Export/import restores texts, contacts, and valid state on a clean profile.
- Disk-full/kill at the end of export or import preserves the original working profile and does not report a successful backup.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The backup is fit for cold restore; secret temporary files and transient keys are deleted after completion.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-i05"></a>
## V1-I05. Verify current authorities after an old backup

**Type:** backend. **Source cards:** I03, I06, P03. **Position in dependency order:** 45.
**After:** [V1-I04](07-tasks.md#v1-i04), [V1-A02](03-tasks.md#v1-a02).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. On restore, recover current ownership/revocation/checkpoint floors from the valid history.
2. Do not treat old grants/spent snapshots as grounds for new spending or for returning a revoked device.
3. If fresh authority is unavailable, allow safe local reading and show pending/rejoin-required for forbidden actions.

**Verifiable scenarios:**

- An old backup after a revoke does not restore rights; a replay of an old postage stamp is not spent.
- No authoritative fresh data: the profile does not automatically start signing with new copies of the old MLS/spend state.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Antirollback is verified with a real old backup and a separate live profile.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-i06"></a>
## V1-I06. Verify the keystore and cleanup of temporary test keys

**Type:** verification. **Source cards:** I01, I06, U06. **Position in dependency order:** 46.
**After:** [V1-I04](07-tasks.md#v1-i04).

**Change boundary / entry points:** `crates/capabilities/src/lib.rs`, `crates/capabilities/src/wire.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/runtime_provisioning.rs`, `crates/store/src/lib.rs`, `crates/desktop-host/src/lib.rs`.

**Implementation plan:**

1. Check the ordinary system keystore, profile encryption, and E2eFileStore across supported OSes.
2. For automated native tests use only debug/e2e and a temporary profile; ensure cleanup after normal/error/kill.
3. Confirm the feature boundary: the production artifact does not contain the test file-keystore selection.

**Verifiable scenarios:**

- Repeated native runs do not touch the login Keychain and do not trigger a password prompt.
- A cleanup error is recorded; a wrong production profile key yields a refusal, not a reset.

**Checks:** IDENTITY STORE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Automation is isolated from system keys. The real system_keychain_roundtrip remains a separate explicitly permitted interactive gate.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
