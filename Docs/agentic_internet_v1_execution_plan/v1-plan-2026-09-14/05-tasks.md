# 05. Autonomous recovery of ten replicas

Operators recover data and indexes 10→7→10 with clients turned off and a finite budget.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R04, AR-R08, AR-R14, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-R01** (obligations + probes): SWIM (DSN 2002); φ accrual failure detector (SRDS 2004); Lifeguard (DSN-W 2018). PoR/PDP — Juels–Kaliski (CCS 2007), Ateniese et al., Shacham–Waters (ASIACRYPT 2008); MR-PDP (Curtmola et al., ICDCS 2008 — t distinct replicas). Important: probes ≠ proof of possession — see research §3 for the boundaries.
- **V1-R02** (replacement + fencing): fencing token (Kleppmann); Chubby (OSDI 2006); Gray–Cheriton leases (SOSP 1989); Zab epochs; replacement sortition — Algorand committee sampling.
- **V1-R03** (crash-safe copy+receipt commit): multi-stage commit via WAL/transactional outbox; Dynamo hinted handoff.
- **V1-R04** (repair storm/backpressure): Carbonite (NSDI 2006 — repair only on confirmed permanent loss); Blake–Rodrigues (HotOS 2003); Amazon Builders' Library backoff+jitter; SEDA.
- **V1-R05**: the 10→7→10 gate — simulation guidelines from research §7.

<a id="v1-r01"></a>
## V1-R01. Persist real repair obligations and probes

**Type:** backend. **Source cards:** D04, N06. **Position in dependency order:** 31.
**After:** [V1-C04](00-tasks.md#v1-c04), [V1-H11](01-tasks.md#v1-h11).

**Change boundary / entry points:** `crates/node/src/paid_custody.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/registry_selection.rs`, `crates/core/src/retained_custody.rs`, `crates/postage-spend/src/custody`.

**Implementation plan:**

1. Derive repair obligations from existing paid receipts, placement, and retention; do not create a global repair consensus.
2. In one bounded scheduler for data/index, keep the probe due time, the actual result, the attempt, and the final term.
3. Verify the presence of ciphertext/hash, not just peer availability or the number of signatures.

**Verifiable scenarios:**

- A missing assigned copy becomes degraded after the confirmed probes policy.
- A brief timeout is not treated as proof of malice; duplicate probes do not multiply jobs.

**Checks:** CUSTODY NETWORK FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The repair queue survives a daemon restart and is bounded by paid obligations/resources.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-r02"></a>
## V1-R02. Select and authorize a replacement holder

**Type:** backend. **Source cards:** D04, N05, P05. **Position in dependency order:** 32.
**After:** [V1-R01](05-tasks.md#v1-r01).

**Change boundary / entry points:** `crates/node/src/paid_custody.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/registry_selection.rs`, `crates/core/src/retained_custody.rs`, `crates/postage-spend/src/custody`.

**Implementation plan:**

1. Reuse the authenticated registry snapshot, placement proofs, and the already paid copy allowance.
2. Fix a bounded repair responsibility/lease with fencing so that several operators cannot spend one quota indefinitely.
3. Select valid spare nodes accounting for capacity and the end of the original lease; updating the composition does not extend the TTL.

**Verifiable scenarios:**

- Two operators notice the same loss simultaneously: the outcome is consistent and the costs are finite.
- A foreign holder, a stale assignment, a replayed lease, and the absence of a reserve do not yield a paid copy.

**Checks:** CUSTODY NETWORK FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Every replacement copy has a verifiable basis and does not require the sender's signer.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-r03"></a>
## V1-R03. Copy ciphertext and update discoverability atomically

**Type:** backend. **Source cards:** D03, D04. **Position in dependency order:** 33.
**After:** [V1-R02](05-tasks.md#v1-r02).

**Change boundary / entry points:** `crates/node/src/paid_custody.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/registry_selection.rs`, `crates/core/src/retained_custody.rs`, `crates/postage-spend/src/custody`.

**Implementation plan:**

1. Fetch the original/copy from a surviving holder with historical proof validation.
2. Issue a new receipt only after the bytes are durably committed on the spare.
3. Through the existing index/location pipeline, publish new locations and keep stage progress until the repair job's retirement.

**Verifiable scenarios:**

- Loss of a process between data copy, receipt, and location ACK is recovered without a false R10.
- A substituted blob, an incomplete page, or an SQL failure does not update the number of available copies.

**Checks:** CUSTODY NETWORK FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** An independent audit client finds and reads the new bytes through the index; a shared scheduler serves both roles.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-r04"></a>
## V1-R04. Bound the storm, exhausted capacity, and repair GC

**Type:** backend. **Source cards:** D04, N06, P05. **Position in dependency order:** 34.
**After:** [V1-R03](05-tasks.md#v1-r03).

**Change boundary / entry points:** `crates/node/src/paid_custody.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/registry_selection.rs`, `crates/core/src/retained_custody.rs`, `crates/postage-spend/src/custody`.

**Implementation plan:**

1. Build per-peer/global backoff and fair scheduling into the existing admission/processing budgets.
2. When spare/funds are absent, keep degraded state with a verifiable reason; do not re-subscribe a promise without a resource.
3. Finish repair on expiry, keeping evidence until settlement and deleting only admissible transient rows.

**Verifiable scenarios:**

- Mass node loss does not evict an honest DM from the bounded queues.
- A repeated restart does not reset budget/backoff; an expired obligation is neither repaired nor extended.

**Checks:** CUSTODY NETWORK FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** CPU/network/disk and costs are measured under burst; the actual durability is shown to the user.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-r05"></a>
## V1-R05. Prove autonomous data/index 10→7→10

**Type:** verification. **Source cards:** D03, D04, N05, N06. **Position in dependency order:** 35.
**After:** [V1-R04](05-tasks.md#v1-r04).

**Change boundary / entry points:** `crates/node/src/paid_custody.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/registry_selection.rs`, `crates/core/src/retained_custody.rs`, `crates/postage-spend/src/custody`.

**Implementation plan:**

1. Bring up 10 assigned and spare identities, send known objects, and verify the original copies.
2. Turn off both clients and three assigned storage profiles; confirm the processes and inventory loss.
3. Wait for operator-only repair, read the new ciphertext/index from a separate auditor, and bring the recipient back. Repeat the branch without a spare.

**Verifiable scenarios:**

- New copies have the original hashes, valid receipts and locations, Bob receives the original message.
- Without a spare, the UI stays degraded rather than showing ten restored.

**Checks:** CUSTODY NETWORK FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The native repair vertical is closed with finite cost; the counts of identities/processes/machines/operators are listed separately.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
