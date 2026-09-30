# 02. Storage and the cost of a long life

History growth does not trigger rewriting of the entire profile and does not break compatibility after an upgrade.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R13, AR-R14, AR-R16, AR-R20, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-S01** (atomic forward migrations): F1 online schema change (VLDB 2013); SQLite atomic commit/WAL docs; Pillai et al. (OSDI 2014).
- **V1-S02** (CAS dedup + refcount GC): Venti (FAST 2002) content-addressed GC; Plainfossé–Shapiro distributed GC survey; Cassandra tombstones/`gc_grace` as an anti-pattern for lease-aware GC.
- **V1-S03** (incremental MLS persistence): OpenMLS storage provider API; RFC 9420 §8 key schedule / §9 secret tree; FS-GAEAD (Alwen et al., CCS 2021).
- **V1-S04** (bounded verification outside the Core writer): SEDA (SOSP 2001) bounded admission; go-libp2p resource manager (scope budgets); commit-time revalidation as a TOCTOU guard.

<a id="v1-s01"></a>
## V1-S01. Introduce a single profile version and atomic forward migrations

**Type:** backend. **Source cards:** F04. **Position in dependency order:** 17.
**After:** [V1-H11](01-tasks.md#v1-h11).

**Change boundary / entry points:** `crates/store/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/historical_postage_context.rs`, `crates/postage-spend/src/custody`, `crates/node/src/processing.rs`, `crates/node/src/processing_budget.rs`.

**Implementation plan:**

1. Inventory the existing schemas and versioned state namespaces without renaming all modules.
2. Add a common schema version and an explicit supported-read/write envelope; provide for a verifiable backup before an irreversible migration.
3. Apply migrations in a single transaction/in stages with a resumable marker; forbid silent reset and running an old writer on the new schema.

**Verifiable scenarios:**

- An old profile upgrades with the same identity, outbox, MLS, and grants.
- Kill/disk-full/wrong-key do not create an empty new profile; rollback restores the consistent old state.

**Checks:** STORE HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There are production upgrade/reopen tests for the whole profile, not just a single JSON namespace.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-s02"></a>
## V1-S02. Eliminate duplicate storage of immutable proof bundles

**Type:** backend. **Source cards:** D02, P05, F04. **Position in dependency order:** 18.
**After:** [V1-S01](02-tasks.md#v1-s01).

**Implementation handoff — 2026-09-15:** the current S02 was continued in the shared workspace;
the first store tests were preserved unchanged. Per the user's instruction, no new tests, test or
build runs, or separate review were performed in this continuation. The card is not marked GREEN
and not closed; S03 is not part of this bounded pass.

- Incoming rows read the previous inline format and the first format with a single
  `proof_digest`. The new format stores the authority snapshot, operator presentation,
  shared postage context, and the exact receipt bytes separately by hash. The operation is
  stored separately from the shared context; wire and public types were not changed.
- Loading a single new object uses up to four point proof lookups with hash verification
  and a shared 4 MiB limit. Presence/deserialization of bytes does not replace the
  previous authentication. A missing or corrupted linked proof does not switch loading
  into inline mode.
- Legacy archive hashes and counters are computed from the original inline representation;
  reading and an exact retry of an unchanged legacy original do not migrate it. Changing
  the representation of an individual row saves body, metadata, quota, and proof references
  in one transaction, including inspection. The logical evidence quota is preserved.
- Incoming cleanup releases references together with the row; the subsequent GC removes
  up to 32 unreferenced bundles after retention. digest/retention indexes were added;
  unchanged retention is not rewritten. Opening an S01 profile adds only the proof tables
  and does not recreate missing identity/message/state tables.

**Remaining:** the saved custody regression fixtures that build a v1 archive by copying a
raw row, or that look for the whole proof inside `states`, require adaptation to external
bundles (`tests/support/incoming_rows.rs`, `portable_obligation.rs`). This must not be fixed
by accepting a linked row without its proof bytes. Test files were not changed in this pass.
The new implementation has not been built/run yet; measuring real bytes before/after and
confirming the bounded lookup remain acceptance items. The previously saved Full130 H11
blocker on the indexed original atomic message/progress transaction and the unproven
Diagnostic32 read reduction remain separate open outcomes; this pass did not re-verify
them.

**Change boundary / entry points:** `crates/store/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/historical_postage_context.rs`, `crates/postage-spend/src/custody`, `crates/node/src/processing.rs`, `crates/node/src/processing_budget.rs`.

**Implementation plan:**

1. Measure the duplicated paid proof bytes by book/authority/QC in real rows.
2. Reuse the content-addressed immutable proof record and references from obligations where verified bytes are already available.
3. Migrate without changing the signed wire; GC deletes a proof only after the last live reference and admissible retention.

**Verifiable scenarios:**

- Different operations of one book are read after cold restart from the original proof bytes.
- A missing/substituted bundle does not count as verified; an SQL fault leaves no dangling reference.

**Checks:** STORE HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Bytes before/after and the bounded lookup are measured; historical retrieval does not depend on an active wallet or a new checkpoint.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-s03"></a>
## V1-S03. Move to incremental transactional MLS persistence

**Type:** backend. **Source cards:** F04, I05. **Position in dependency order:** 19.
**After:** [V1-S01](02-tasks.md#v1-s01).

**Implementation handoff — 2026-09-15:** the implementation was done in the shared workspace
after S02. Four first tests were added before production code: three Core scenarios in
`crates/core/tests/support/mls_persistence.rs` and one crypto scenario in
`crates/crypto/tests/support/incremental_persistence.rs`. Per the current user instruction,
tests, builds, and review were not run; only formatting of the changed Rust files via the
build-storage wrapper was done. S03 is not marked GREEN.

- `state_records` stores provider records separately, under the CAS of the existing `mls`
  row. Changes to records, the MLS prefix, app/progress state, message, and outbox go into
  the previous Core/SQL transaction. A retry does not apply the prepared MLS delta.
- Crypto reuses unchanged encoded records between prepared states; it encodes the changed
  ones and also passes deletions. The full CBOR snapshot is assembled lazily on an explicit
  compatibility read. The snapshot v1 format and wire are preserved; intermediate
  create/add/activate/join pass a ready `SecretState`.
- `ProfileStore::state("mls")` still returns the full compatible snapshot. Core reads
  records without assembling this buffer. A legacy snapshot opens without migration; the
  first successful MLS transaction moves it into records. An ordinary write of the full
  snapshot atomically replaces the fragmented state, preserving the previous adapters.
- The live client's state does not change during preparation: the isolated provider and the
  prepared delta are discarded on an SQL error. The next operation reads only committed
  records. Staged SQL record bytes are cleared on drop.
- For later profiling, `PersistenceStats` (snapshot bytes, written bytes, changed/deleted
  records) and an SQL audit of the actual writes were added in the first tests. They cover
  several dialogs, retry, a late send/receive SQL error, cold reopen, migration rollback,
  and deletion of a used KeyPackage.

**Unverified items:** compilation and execution of new/existing backend and frontend
scenarios, real bytes/latency before and after. There are no numeric results from this pass.
The existing OpenMLS memory provider remains: reading and copying the whole set of records
before an isolated operation still has linear cost. The SQL write and re-encoding are
separated from this residual read/copy cost; moving to a lazy provider will require
profiling results. The previous Full130 acceptance was not re-confirmed by this pass.

**Change boundary / entry points:** `crates/store/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/historical_postage_context.rs`, `crates/postage-spend/src/custody`, `crates/node/src/processing.rs`, `crates/node/src/processing_budget.rs`.

**Implementation plan:**

1. Profile the current MLS store snapshot on a typical send/receive; select the real changed keys/rows.
2. Embed the incremental provider into the existing Core transaction instead of independent writers.
3. Preserve in-memory MLS rollback together with the SQL rollback and cold reading of the old profile.

**Verifiable scenarios:**

- Send/receive/retry across several dialogs change only the necessary records and preserve an identical transcript.
- An error in the last SQL write does not save the new ratchet separately from the message/outbox.

**Checks:** STORE HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The number of rewritten bytes is bounded by the change; the atomic model and recovery are verified, and the measurement is compared with the baseline.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-s04"></a>
## V1-S04. Bound expensive immutable verification outside the Core writer

**Type:** backend. **Source cards:** F04, F05, N01. **Position in dependency order:** 20.
**After:** [V1-S02](02-tasks.md#v1-s02), [V1-S03](02-tasks.md#v1-s03).

**Change boundary / entry points:** `crates/store/src/lib.rs`, `crates/core/src/lib.rs`, `crates/core/src/historical_postage_context.rs`, `crates/postage-spend/src/custody`, `crates/node/src/processing.rs`, `crates/node/src/processing_budget.rs`.

**Implementation plan:**

1. From the trace, isolate the pure checks that actually block, preserving the already existing opaque validated tokens.
2. Wire bounded execution slots/completion events into the existing processing budget; do not create independent mutable Cores.
3. At commit, re-verify the current grant/trust/time/connection and the CAS of the original bytes; cancellation frees the slot.

**Verifiable scenarios:**

- A heavy honest/hostile peer does not block owner IPC or another dialog beyond the stated budget.
- Revocation, pointer/epoch change, and disconnect before the worker finishes reject a stale result; memory is bounded.

**Checks:** STORE HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The improvement on the real bottleneck is measured and the security fences are preserved. If the baseline already fits, a documented no-change with this evidence is acceptable.

**Implementation handoff (2026-09-15, bounded S04):** For ordinary paid `RangeFetch / ObligationPage`, an immutable worker is wired in: the Core stores the trusted clock once and issues a shared read-only snapshot of the installed profiles. Authentication of the original ancestry/registry and the public/legacy postage proof runs via `spawn_blocking`, without Core/SQL/signer. An opaque completion is returned to the previous writer and the shared atomic prefetch bodies/progress path; the original grant lease, the exact connection/network revision, time, mailbox head/epoch, the original request/descriptor bytes and the revision/bytes of registry/finalizer policy, as well as the original checkpoint profile/issuer/certificate bytes are re-verified. Monotonic advancement of the checkpoint clock by itself does not invalidate a proof.

Execution admission uses the existing processing budget: 1 ordinary + 3 selected physical slots, at most one per peer, no waiting queue; a fixed 16 MiB reservation is held until the completion is consumed. Cancellation/revocation stop the subsequent page checks; an already running non-cancellable cryptographic call holds the slot until it exits. Panic/closed channel/swarm change do not release the limit early and do not accept a stale result. The writer processes at most one completion per tick. `execution*` counters were added to processing capacity and the trace target `ain_verification` with prepare/worker/commit durations.

First tests: `processing_worker_tests.rs` (3), `support/immutable_obligation.rs` (2), `immutable_range_completion_keeps_the_real_queue_pointer_connection_and_request_bytes_fences` (1, genuine paid range fixture). Due to the direct restriction of this task, tests, builds, and critic/review were **not run**; no new GREEN/evidence is claimed. Remaining: QC/operator/receipt checks keep the synchronous Core path, individual non-range handlers were not moved; latency/peak-memory and the correctness of the new fences require a subsequent permitted targeted run. S04 is not closed by the measurement criterion. The S03 memory-provider full read/copy and the Full130 H11 atomic original message/progress blocker are not part of this change.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
