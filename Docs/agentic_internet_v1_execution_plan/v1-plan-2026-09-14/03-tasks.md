# 03. Long-running payment and authority lifecycle

The chat continues paid operations after epoch changes, authority updates, and a long L2 outage.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R01, AR-R02, AR-R07, AR-R21, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-A01** (authority refresh, trust profile): weak subjectivity — Buterin, "Proof of Stake: How I Learned to Love Weak Subjectivity" (2014) + the Ethereum consensus-specs weak-subjectivity guide; Altair light-client sync (committee signature threshold — an analogue of the attestor threshold); PBFT §4.3 checkpoint protocol.
- **V1-A02** (epoch close + spent-set handover): reconfiguration — "From Permissioned to Proof-of-Stake Consensus" (AFT 2025; cited by Commonware as the reshare model), Stoppable Paxos, Raft joint consensus §6, BFT-SMaRt reconfiguration; Narwhal (availability certificate before ordering — an analogue of "receipt before finality"). Key risk: double-spend across the epoch boundary — see research §2.
- **V1-A03** (pending inputs, idempotent lifecycle): the saga pattern; Kafka KIP-98; transactional outbox; commit-time revalidation (TOCTOU).
- **V1-A04** (retention/GC/payment lifecycle): distributed GC survey (Plainfossé–Shapiro); Cassandra tombstones; Gray–Cheriton leases (SOSP 1989).
- **V1-A05** (verifier boundary): RISC Zero proof-system docs (journal/seal boundary); Semaphore scoped nullifiers; Zcash nullifier model; Steel — a reference EIP-1186 inside a zkVM.
- **V1-A06**: a combination of A01+A02 + Jepsen-style partition-safety histories.

<a id="v1-a01"></a>
## V1-A01. Obtain and renew a valid authority automatically

**Type:** backend. **Source cards:** P04, L06. **Position in dependency order:** 21.
**After:** [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Connect the existing checkpoint refresh with ordinary client/finalizer discovery and an explicitly accepted trust profile.
2. Persist the trusted authority and cursor atomically; cut off a false source, a stale epoch, and an incompatible chain lineage.
3. Remove the need for manual owner commands from routine renewal.

**Verifiable scenarios:**

- A clean client with an accepted trust obtains an authority and sends, then continues after renewal/cold restart.
- Collusion of several RPCs without valid checkpoint signatures does not create an authority; an SQL fault leaves the previous consistent state.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Renewal runs in the ordinary flow with bounded retry; sources and trust assumptions are shown in the evidence.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-a02"></a>
## V1-A02. Close an epoch when the old authority expires

**Type:** backend. **Source cards:** P01, P03, P04, L06. **Position in dependency order:** 22.
**After:** [V1-A01](03-tasks.md#v1-a01).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Determine a valid signed closing lineage from the already existing spent history and expiry policy.
2. Separate historical validation of old records from the right to a new signature; do not revive an expired signer.
3. Load the full authenticated prefix before resolving negative membership in the successor.

**Verifiable scenarios:**

- The old signer is offline/expired, but a valid retained closing moves all spent rows into the new composition.
- An incomplete prefix, a modified QC, and a replay of an old postage stamp do not produce a second spend.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Epoch transfer does not reset the spent namespace; exact historical QC/readAt are preserved after kill and cold resume.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-a03"></a>
## V1-A03. Recover pending inputs after expiry and non-delivery

**Type:** backend. **Source cards:** P03, P04, P05. **Position in dependency order:** 23.
**After:** [V1-A02](03-tasks.md#v1-a02).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Separate reserved, exposed stamp, final spend, and refundable in the existing job ledger.
2. For an unconfirmed operation, choose a valid resumption/new preparation with an idempotent operation ID and accounting for the already exposed resource.
3. After an authority change, re-verify the original grant and budget before any network action.

**Verifiable scenarios:**

- A crash between reserve/publish/finality and authority expiry do not create two paid operations.
- An exposed/final stamp is not returned to the balance; a cancelled grant does not come back to life after renewal.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The user sees the exact pending/expired reason and can continue a valid send without an issuer reset.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-a04"></a>
## V1-A04. Pin down retention of the completed-send journal

**Type:** backend. **Source cards:** P05, F04, D03. **Position in dependency order:** 24.
**After:** [V1-H11](01-tasks.md#v1-h11), [V1-A03](03-tasks.md#v1-a03).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Reuse the implemented retirement and message_delivery projections; inventory the proof/evidence rows still held.
2. Delete only transient completed work after durable publication, keeping original/history, claims, and mandatory paid evidence until their term.
3. Separate GC of observation/receipt/evidence from freeing the active queue.

**Verifiable scenarios:**

- Two consecutive series of more than 128 sends free active slots, while an old recipient recovers history without the sender.
- An SQL failure during retirement/GC does not return the spent resource and does not delete the last discoverability link.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The active queue is bounded by concurrency, not lifetime; long-lived evidence has an explicit term and a measured volume.

**Retention clarification of September 17:** compact completed/terminal facts are stored until
the profile is deleted, with a separate measurement of the number and volume of records; no
automatic deletion term is added for these facts. Full criteria for GC, SQL rollback, cold
restart, and measurements are in
[verification-criteria-2026-09-17.md](verification-criteria-2026-09-17.md).
This contract clarification is not an acceptance of A04.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-a05"></a>
## V1-A05. Close the public/legacy verifier boundary

**Type:** backend. **Source cards:** P02, F02, F06. **Position in dependency order:** 25.
**After:** [V1-C01](00-tasks.md#v1-c01).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Check the already working ordinary public build and extract only the genuinely shared neutral funding/authority DTOs from proof-heavy dependencies.
2. Preserve the frozen legacy relation/image IDs and separate verify/prove targets; an ordinary send does not launch the prover.
3. Add cross-format vectors of the shared spent namespace and a strict ban on the malformed public→legacy fallback.

**Verifiable scenarios:**

- An old paid record is verified by the current reader after an upgrade.
- A re-encoded or substituted public stamp does not get a second nullifier namespace and does not trigger the fallback.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The ordinary target is independent of the prover; historical compatibility and the explicit legacy gate are verified without regenerating the frozen proof identity.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-a06"></a>
## V1-A06. Verify the long spend/renewal/outage cycle

**Type:** verification. **Source cards:** P01, P03, P04, P06, L06. **Position in dependency order:** 26.
**After:** [V1-A04](03-tasks.md#v1-a04), [V1-A05](03-tasks.md#v1-a05).

**Change boundary / entry points:** `crates/core/src/checkpoints.rs`, `crates/core/src/checkpoint_history.rs`, `crates/core/src/public_sender.rs`, `crates/core/src/public_sender_progress.rs`, `crates/node/src/postage_authority.rs`, `crates/node/src/postage_closing.rs`, `crates/node/src/checkpoint_schedule.rs`, `crates/finalizer/src/history`, `crates/postage-spend/src`.

**Implementation plan:**

1. Reuse the native handover fixture: more than 128 spends, several real compositions, cold missing history, and client return.
2. Add ordinary automatic renewal, undelivered inputs, and an outage before/after the hard lease.
3. Preserve supply/spent/claim invariants and independent verifier signatures.

**Verifiable scenarios:**

- After a hard expiry, new admissions wait while old live retrieval keeps working.
- A concurrent replay during handover does not create a second final spend; recovery does not require manual QC setup.

**Checks:** POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a complete current-revision AR1 scenario; the local historical 133-spend results do not substitute for the new gate.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
