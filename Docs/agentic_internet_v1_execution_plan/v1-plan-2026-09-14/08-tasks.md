# 08. Groups and epoch-aware recovery

Real groups have roles, sequential encrypted control, safe rejoin, and two paid privacy profiles.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R18, AR-R19, AR-R06, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage. The chapter contains V1's heaviest hotspot (G03+G04).

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-G01** (intents toward the final parent epoch): fork-consistency — SUNDR (Li et al., OSDI 2004), fork-linearizable histories; MLS epoch binding (RFC 9420 §8 epoch authenticator).
- **V1-G02** (payload before finality): Narwhal (availability certificate before ordering); DispersedLedger VID; an analogy with data-availability sampling (LazyLedger/Celestia).
- **V1-G03** (BFT ordering): Simplex — Chan & Pass (TCC 2023, ePrint 2023/463) + commonware docs; HotStuff (PODC 2019); PBFT (OSDI 1999) for checkpoint/state-transfer patterns.
- **V1-G04** (concurrent commits — the key task): DCGKA (Weidner et al., CCS 2021); CoCoA (Alwen et al., EUROCRYPT 2022); Bienstock et al., "On the Price of Concurrency in Group Ratcheting Protocols" (TCC 2020); SAIK (CCS 2022); insider security of MLS (Alwen–Jost–Mularczyk, CRYPTO 2022); TreeSync (USENIX Security 2023); Matrix dMLS — an engineering precedent for fork-resolution. There is no formal model of "MLS over total-order BFT log" — this is an original construction, see research §1.
- **V1-G05** (catch-up/key retention): RFC 9750 deletion guidance; Quarantined-TreeKEM (CCS 2024); FS-GAEAD (CCS 2021); Double Ratchet skipped-message keys (EUROCRYPT 2019).
- **V1-G06** (rejoin + history share): DOGM (S&P 2024); MLS Welcome semantics.
- **V1-G07/V1-G08** (privacy profiles, secret pointers): Signal Private Group System (Chase et al., CCS 2020); Keyhive; sealed sender + limits (NDSS 2024 deanonymization); Tor prop224; OMR — oblivious message retrieval (Liu et al., CRYPTO 2022).
- **V1-G09** (delegated budget): capability attenuation (spec capability-grants); atomic debit CAS.

<a id="v1-g01"></a>
## V1-G01. Add group intents and a role matrix

**Type:** backend. **Source cards:** G01, I02. **Position in dependency order:** 47.
**After:** [V1-I02](07-tasks.md#v1-i02), [V1-W03](04-tasks.md#v1-w03).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Introduce domain commands create/invite/join/leave/remove with a human/device distinction.
2. Bind the signed intent to group_id, the final parent epoch, and the current role grant.
3. Use the MLS adapter for keys, the shared broker for policy, and an atomic outbox for control.

**Verifiable scenarios:**

- Owner/admin/member have explicitly different permitted operations.
- A former admin, a foreign group invite, and an unconfirmed device do not change the roster.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A group is created by the ordinary application service; no endpoint bypasses role checks.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g02"></a>
## V1-G02. Persist encrypted control payload until finality

**Type:** backend. **Source cards:** G02, D01, D03. **Position in dependency order:** 48.
**After:** [V1-G01](08-tasks.md#v1-g01), [V1-R03](05-tasks.md#v1-r03).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Publish control ciphertext through the shared paid custody/index pipeline with retention and receipts.
2. Reach consensus only on commitment/order metadata; do not pass MLS secrets to the finalizer.
3. Do not allow finalization before sufficiently verified payload persistence under the accepted contract.

**Verifiable scenarios:**

- With a participant offline, retained control can be fetched from other holders.
- A withheld payload or a receipt without saved bytes does not become a final control step.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The final control entry references an accessible verified ciphertext; the work does not require a single administrator to be online.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g03"></a>
## V1-G03. Wire BFT ordering into the group control log

**Type:** backend. **Source cards:** G02, P01. **Position in dependency order:** 49.
**After:** [V1-G02](08-tasks.md#v1-g02), [V1-A02](03-tasks.md#v1-a02).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Reuse the generic finalizer for control, not for every application message.
2. Separate the ordering of proposals from the recipients' verification of MLS semantics.
3. Apply a valid final entry in a single transaction with the epoch/control cursor; preserve safety under partition.

**Verifiable scenarios:**

- A 4/3 committee under a 2/2 partition does not finalize new state; after recovery there is a common prefix.
- A modified parent/QC or two conflicting final entries are not accepted.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Real processes converge to a single control prefix without lowering the quorum.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g04"></a>
## V1-G04. Handle concurrent MLS commits and deterministic reject

**Type:** backend. **Source cards:** G03, I05. **Position in dependency order:** 50.
**After:** [V1-G03](08-tasks.md#v1-g03).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Build a commit only from an accepted final parent; re-evaluate losing intents on the new epoch.
2. Define a unified reject/no-op for a finalized ciphertext with invalid MLS semantics.
3. After a locally accepted Remove, do not encrypt new sensitive messages to the old epoch; do not roll back used ratchets.

**Verifiable scenarios:**

- Add+Remove, Remove+Update, and two device joins yield the same roster on all honest clients.
- A malicious steward and an invalid MLS commit do not create a fork or key reuse; a removed leaf cannot read the new epoch.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Safety and decryptability are verified with real OpenMLS on conflicting traces, not only by consensus state.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g05"></a>
## V1-G05. Recover control and application across epochs

**Type:** backend. **Source cards:** G04, G03, I05, D05. **Position in dependency order:** 51.
**After:** [V1-G04](08-tasks.md#v1-g04), [V1-H08](01-tasks.md#v1-h08), [V1-I05](07-tasks.md#v1-i05).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Read retained control and its related application messages in an order that preserves the valid key window.
2. Process an epoch's messages before deleting the needed keys; do not do unbounded key retention.
3. On an incomplete legacy/control range, show an explicit gap/rejoin-required and continue permitted work.

**Verifiable scenarios:**

- Being offline longer than three past epochs recovers within the retained data or yields the exact boundary of what is unavailable.
- Applying all new commits does not silently destroy the only keys to still-accessible messages; a forged snapshot is rejected.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a real multi-epoch catch-up, an explicit partial result, and a safe transition past the retention window.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g06"></a>
## V1-G06. Perform permitted device rejoin and history sharing

**Type:** backend. **Source cards:** G04, I03, I06. **Position in dependency order:** 52.
**After:** [V1-G05](08-tasks.md#v1-g05).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Request rejoin under current membership/recovery grants and create new device/MLS secrets.
2. Recover only permitted retained messages; make transferring old history to a new member a separate authorized action.
3. Persist rejoin progress/Welcome consumption atomically and retry after an SQL failure.

**Verifiable scenarios:**

- A lost device is replaced and the remaining participants continue the chat.
- A revoked device and a former member do not use backup/rejoin to regain rights; a new member does not get old secrets by default.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Rejoin does not clone an active ratchet and does not hide the loss of inaccessible history.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g07"></a>
## V1-G07. Enable the shared-group-storage profile

**Type:** backend. **Source cards:** G05, D06, P05. **Position in dependency order:** 53.
**After:** [V1-G04](08-tasks.md#v1-g04), [V1-B04](06-tasks.md#v1-b04).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Store one encrypted application blob with valid group read/repair rights.
2. Account for the fanout of several members/devices in the price/resource manifest.
3. Tie membership changes to future read capabilities without promising to erase previously received content.

**Verifiable scenarios:**

- Permitted members read the same ciphertext through the shared store.
- Quota/excluded recipient and stale capability do not yield new paid reads.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The profile and the exposed storage metadata are documented; the actual cost is verified.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g08"></a>
## V1-G08. Enable the private-recipient-pointers profile

**Type:** backend. **Source cards:** G05, N03. **Position in dependency order:** 54.
**After:** [V1-G07](08-tasks.md#v1-g07).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Use the same E2EE bodies with separate secret recipient pointers and bounded discovery.
2. Do not include an open roster in pointers; account for the cost of each permitted fanout.
3. Provide pointer/manifest repair without widening access.

**Verifiable scenarios:**

- A lost portion of pointers is recovered for permitted recipients.
- A foreign recipient does not get a lookup capability; the linkage of the same blob/time is not claimed to be hidden from a global observer.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Both profiles pass the same content-secrecy check and a separate metadata boundary check.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g09"></a>
## V1-G09. Add an organizational group and a shared messaging budget

**Type:** backend. **Source cards:** G06, I02, P05. **Position in dependency order:** 55.
**After:** [V1-G08](08-tasks.md#v1-g08), [V1-I01](07-tasks.md#v1-i01).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Tie a persistent org group to a membership policy and a shared finite postage budget.
2. Delegate individual runtime limits without transferring the treasury/master key; do not include V2 jobs/order rooms.
3. On revoke/archive, keep the history and the already spent budget while stopping new actions.

**Verifiable scenarios:**

- Two agents simultaneously spend their limits from the shared fund without overspend.
- A guest of one group does not read other org conversations; revoke/restart does not restore spending.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Organizational messaging works through shared grant/budget primitives, without a new job engine.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-g10"></a>
## V1-G10. Add UI for groups, devices, and privacy selection

**Type:** frontend. **Source cards:** U03, G01, G04, G05, G06. **Position in dependency order:** 56.
**After:** [V1-G06](08-tasks.md#v1-g06), [V1-G09](08-tasks.md#v1-g09).

**Change boundary / entry points:** `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/broker.rs`, `crates/finalizer/src/engine`, `crates/node/src/finalizer_network.rs`, `crates/node/src/custody_history_sync.rs`, `apps/desktop/src/ChatShell.tsx`.

**Implementation plan:**

1. Add create/invite/roles/device list/remove/rejoin and explicit privacy selection to the current shell.
2. Show pending membership before finality, the real metadata/cost differences between profiles, and the boundaries of history sharing.
3. Check keyboard flows and headless screenshots; native actions must call the shared Rust service.

**Verifiable scenarios:**

- The user creates a group, adds a second device, removes a member, and recovers after offline.
- A quorum/storage error does not look like a completed removal; dangerous history expansion requires a specific user action.

**Checks:** GROUP IDENTITY HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a usable UI for the whole group vertical with actual verification of keys/roster, not just screenshots.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
