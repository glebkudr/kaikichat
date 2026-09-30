# 00. Execution contract and reproducible checks

Before any code changes, the real commands, resource limits, and evidence format are defined.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R13, AR-R21, AR-R22, AR-R23, AR-R24.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-C03** (fault manifest, seeded replay): FoundationDB deterministic simulation — Wilson, "Testing and Debugging of a Distributed Database" (Strange Loop 2014) and the FDB paper (SIGMOD 2021); TigerBeetle VOPR; Jepsen fault histories. `commonware-runtime` deterministic already exists in the code — build on top of it.
- **V1-C04** (committee/resource risk model): Gilad et al., Algorand (SOSP 2017) committee sampling; Hafid et al., hypergeometric committee bounds (IEEE Access 2019); Serfling sampling-without-replacement bounds. Book of Swarm incentives as economic context.
- **V1-C05** (controlled time and reproducer): the same deterministic-simulation stack as V1-C03 — FoundationDB simulation (Wilson, Strange Loop 2014; FDB SIGMOD 2021: seeded RNG, BUGGIFY, virtual time), TigerBeetle VOPR (seed+commit replay), Antithesis (deterministic hypervisor as reference architecture); the existing in-repo example is `commonware_runtime::deterministic` + `commonware_p2p::simulated` in the finalizer engine tests.

<a id="v1-c01"></a>
## V1-C01. Tie card acceptance to executable checks

**Type:** tooling. **Source cards:** F01, F06, X06. **Position in dependency order:** 1.
**After:** no outstanding dependencies of this plan.

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan`, `scripts/check.sh`, `scripts/check-native.mjs`, `tests/models`, `tests/evm`, `tools/risk-simulator`, `spec`.

**Implementation plan:**

1. Reconcile this plan with the release scope and the current capability matrix; separate historical PASS from the current one.
2. For each future check, fix a task/E2E ID, test symbol, fixture, non-zero expected count, and evidence owner.
3. Extend the existing report with a missing/failed/skipped-required check and the exact revision; do not create a second backlog.

**Verifiable scenarios:**

- A missing mandatory card or a mismatched binary hash forbid product_validated.
- An old successful report remains available but does not close the new revision.

**Checks:** DOC MODEL profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** All 67/22/3 and 11 suites have a path to a mandatory verifiable outcome; no V2 card has become a V1 dependency.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-c02"></a>
## V1-C02. Separate targeted native checks from the full release runner

**Type:** tooling. **Source cards:** F06. **Position in dependency order:** 2.
**After:** [V1-C01](00-tasks.md#v1-c01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan`, `scripts/check.sh`, `scripts/check-native.mjs`, `tests/models`, `tests/evm`, `tools/risk-simulator`, `spec`.

**Implementation plan:**

1. Analyze scripts/check-native.mjs: it currently invokes all of scripts/check.sh.
2. Extract selection of an existing targeted scenario without an implicit full suite; keep the full mode for the end of the plan.
3. In all modes keep build-storage, fingerprints, exit status, and cleanup.

**Verifiable scenarios:**

- The selected native case runs exactly once and does not start the workspace suite.
- An empty set, an unknown case, and an erroneous child exit do not become PASS.

**Checks:** DOC MODEL profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There are exact documented commands for targeted native cases; the full runner still collects all mandatory outcomes.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-c03"></a>
## V1-C03. Pin down a reproducible fault manifest

**Type:** tooling. **Source cards:** F03, F06. **Position in dependency order:** 3.
**After:** [V1-C01](00-tasks.md#v1-c01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan`, `scripts/check.sh`, `scripts/check-native.mjs`, `tests/models`, `tests/evm`, `tools/risk-simulator`, `spec`.

**Implementation plan:**

1. Reuse the existing clock/RNG and process fixtures; describe the seed, topology, ports, profiles, and kill/SQL fault points.
2. Persist the faults that actually fired and the source message hashes as a separate oracle.
3. Add replay for one real offline/reorder/crash scenario without an LLM dependency.

**Verifiable scenarios:**

- A single seed yields the same set of fault transitions and final MessageIDs.
- A kill/SQL trigger that did not fire or a lost log makes the result incomplete.

**Checks:** DOC MODEL profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Replay verifies production reducers and a durable result; virtual time is not passed off as elapsed native time.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Recorded in `evidence/reviews/V1-C03-fault-manifest/evidence.json`: 26 checks GREEN, critic ACCEPT (3 rounds), the `spec/models/fault-manifest-v2.md` contract, native fixture from `output/hrt32-r2/trace.json`.

<a id="v1-c04"></a>
## V1-C04. Fix the resource and trust model of the V1 testnet

**Type:** design. **Source cards:** F05, L01, L02, P05. **Position in dependency order:** 4.
**After:** [V1-C01](00-tasks.md#v1-c01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_execution_plan`, `scripts/check.sh`, `scripts/check-native.mjs`, `tests/models`, `tests/evm`, `tools/risk-simulator`, `spec`.

**Implementation plan:**

1. Consolidate the effective byte/entry/CPU/read/retention/queue limits and the source of each parameter.
2. Tie paid classes, R=10, minimum copies, repair reserve, subsidy, and royalty to the conservation model.
3. Describe the chosen L2/checkpoint profile, quorum assumptions, and failure domains; leave undetermined monetary values as explicit decisions until the economic implementation.

**Verifiable scenarios:**

- An example of one send, index, read, and repair fits within the paid resource without a second postage stamp for the index.
- A shortage of fund/capacity or a hard lease expiry yields a bounded refusal, not a new resource.

**Checks:** DOC MODEL profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A versioned manifest distinguishes protocol limits, rig parameters, and not-yet-adopted monetary decisions; there is no hidden TTL increase.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

**External prerequisite:** Confirmed testnet, subsidy budget, fee, and payout parameters; until they are chosen, the monetary gate cannot be declared fulfilled.

<a id="v1-c05"></a>
## V1-C05. Build a fast diagnostic loop and a reproducer with controlled time

**Type:** backend. **Source cards:** F03, F06. **Position in dependency order:** 5.
**After:** [V1-C01](00-tasks.md#v1-c01).

**Change boundary / entry points:** `tests/evm` (harness and `live_wallet_authority.py`), `crates/node/src` (`runtime.rs`, `public_sender.rs`, `paid_custody.rs`, `custody_history_sender.rs`), `crates/finalizer/src/service`, F03 ownership — `crates/test-runtime` and `tests/simulation`, `tools/risk-simulator`, `spec`. The time seam and waking ready work intersect the active A04 cycle: proceed on top of the accepted lane fixes, not in parallel on the same lines.

**Implementation plan:**

1. In the native-run harness, add selection of the target diagnostic phase and a no-useful-progress budget (initial local budget on the order of 90–120 s; this is not a new liveness bound) with an early stop. On stop, save: for publication — identity pages, revision, manifest hash, the set of ACKs, data/index receipts, and capacity/error counter deltas; for the late H11 phase — exact inbox MessageIDs, recipient cursor/deferred state, `custodySync`, and the exact SQL fault event; show the "import of previous originals" and "last exact fault" conditions as separate.
2. Extend `historyDiagnostics` with the retry schedule and a series of snapshots: a single unchanged snapshot does not prove a stall; after the sender is turned off in H11, show the main expected work.
3. Introduce a "clocks + timers + events" seam: distinguish protocol time (`issued_at`, lease, expiry, verification of signed documents), scheduler monotonic time (`due`, backoff, admission windows), chain time (block timestamp, proofs/checkpoints), and real execution time (costs, watchdog, real IPC deadline). In the simulation profile the first two loops are driven by a shared controller while keeping distinct types and semantics; the interface covers creating a deadline, checking that it has arrived, and waking a task. Complete the existing partial `_at` seams: `maintain_public_sender_at` (direct `Instant::now()` remains below), `paid_custody` (`created.elapsed()` next to the passed instant), the embedded finalizer service, which separately reads `SystemTime`.
4. Build one fast sender/custody/history reproducer: several logical nodes in a single test process with production transitions and controlled message delivery; real SQLCipher (real transactions, rollback, and reopen for H11); controlled RNG and order of work completions. Advance to the nearest event: virtual time jumps to the next `due` only after accounting for all events that must happen earlier; an unfinished request to the real network or SQL cannot be treated as absent.
5. Connect Anvil via a separate adapter: set timestamp → emit a block → get the result → continue the scenario; `Source.maintain` currently ties block time to `time.time()` and waits until `issuedAt` is no longer in the future.
6. In parallel, check for extra pauses after dependencies complete: wake ready work on an event without weakening the mandatory backoff/admission limits; speeding up only retries while keeping the previous admission windows can increase failures and does not count as diagnostics.
7. Run the full unchanged A04/H11 on a fix candidate as the only acceptance evidence. To diagnose renewal, initiate it with the existing `maintain(force=True)` in the needed phase; do not scale `authority_lease`/`registry_snapshot_lease`/the number of messages — that removes the 128-boundary check, the full cold-walk branch below 129, and the consistency of `Source` with 1800/3600.

**Verifiable scenarios:**

- An early stop returns "needed observation captured" or "suspected stall" with a full snapshot and is not passed off as a PASS of the full scenario.
- A single seed yields the same event trace and reproducer digest; hidden use of the wall clock, a nondeterministic RNG, or the real network is caught by a negative check.
- The reproducer reproduces a real failure class (for example, a straggler whose index leg never starts) in minutes, preserving meaningful load and real SQL fault boundaries.
- Unchanged native A04/H11 on the candidate confirm the fix; simulation elapsed time in the evidence is explicitly separated from native elapsed time.

**Checks:** DOC MODEL CUSTODY HISTORY POSTAGE profiles from [RUNBOOK](RUNBOOK.md) for the affected parts. Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only parts production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A diagnostic iteration of the selected phase completes in minutes with a saved snapshot instead of waiting for the 600/1300 s ceilings; the reproducer drives production sender/custody/history transitions under shared controlled time; no acceptance parameter was changed for speed.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

**External prerequisite:** None. QEMU `icount` with `sleep=off` is a separate experiment outside this task. Full F03 (partition/loss/Byzantine orchestration, `simulate --seed` for an external agent) remains with V1-V03; this task builds the time seam and the first reproducer without closing the whole card.
