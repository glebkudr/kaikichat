# 01. R14: resumable reading and a single pending-body store

An ordinary paid receiver survives Work rotation and cold restart, then passes the unchanged Diagnostic32 and Full130.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R03, AR-R13, AR-R14, AR-R19, AR-R22, AR-R23.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-H01–H07, V1-H10** (resumable fetch, durable cursor, pending-body dedup): RFC 9162 §2.1.4 consistency proofs; Merkle Mountain Ranges (Todd; Grin MMR repo); Meyer, "Range-Based Set Reconciliation" (SRDS 2023) + negentropy; IBLT (Goodrich–Mitzenmacher) and Yang et al., "Practical Rateless Set Reconciliation" (SIGCOMM 2024); keyset pagination (Winand); Kafka KIP-98 idempotent producer (sequence-based dedup).
- **V1-H08** (gap retry based on MLS state): RFC 9750 §7 (buffering of undecryptable messages); skipped-message keys — Alwen–Coretti–Dodis, "The Double Ratchet" (EUROCRYPT 2019).
- **V1-H07** (migrations): F1 online schema change (VLDB 2013); crash consistency — Pillai et al., "All File Systems Are Not Created Equal" (OSDI 2014).

<a id="v1-h01"></a>
## V1-H01. Close two gaps in Node range continuation tests

**Type:** tests. **Source cards:** D02, D05. **Position in dependency order:** 6.
**After:** [V1-C02](00-tasks.md#v1-c02).

**Change boundary / entry points:** `crates/node/src/custody_range_runtime_tests.rs`, `crates/node/src/custody_history_anchor_fixture.rs`, `crates/node/src/custody_prefix_test_support.rs`, `crates/node/src/custody_prefix_runtime_tests.rs`, `crates/node/examples/support/custody_loss.rs`.

**Implementation plan:**

1. Keep the six runtime tests already written and the real paid Noise fixtures; do not change production yet.
2. Add an empty advancing page: 33 originals, the first 32 expired, the last one alive; the actual after=0 response contains next=32 with no bodies present.
3. Add complete=true below the selected missing: the holder has originals 1–2 left, the current history requires 4; after a cold Core the request must be exact after=3.
4. Verify the durable cursor, MLS immutability before import, and the actual next request; obtain an independent final ACCEPT of the tests.

**Verifiable scenarios:**

- Removing next_sequence from the result breaks the first scenario after a cold open.
- Ignoring complete breaks exact repair; one holder's complete does not close the whole history.
- Fixtures keep the ordinary request limit, real receipts, the current root/epoch, and a real loss of SQL rows.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Both previously blocking REVISE remarks are closed by discriminating tests; compile/behavioral RED and a separate ACCEPT are preserved. Compilation passing alone does not count as behavioral RED.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h02"></a>
## V1-H02. Preserve the boundaries of the verified paid page

**Type:** backend. **Source cards:** D02, D05. **Position in dependency order:** 7.
**After:** [V1-H01](01-tasks.md#v1-h01).

**Change boundary / entry points:** `crates/node/src/custody_obligation_read.rs`, `crates/node/src/custody_fetch_binding_tests.rs`, `crates/node/src/custody_obligation_receiver_tests.rs`, `crates/node/src/custody_fetch_prefix_tests.rs`.

**Implementation plan:**

1. First adapt the existing obligation/prefix/binding tests to the common range contract and run the critic gate.
2. Return the verified envelopes, next_sequence, and complete from the whole-page verifier; keep verification of every later proof before any staging.
3. Allow after below the selected sequence with the previous binding to operation/index/epoch; forbid swapping holder, order, or budget.

**Verifiable scenarios:**

- A valid first body and a corrupted second one save nothing.
- An empty range with skipped expired slots saves next; foreign index/epoch, next going backwards, and exceeding the proof budget are rejected.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The verifier produces a single verified page result without losing continuation; wire/admission limits and historical paid checks are preserved.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h03"></a>
## V1-H03. Wire the durable range into the ordinary Runtime

**Type:** backend. **Source cards:** D02, D05, F04. **Position in dependency order:** 8.
**After:** [V1-H02](01-tasks.md#v1-h02).

**Change boundary / entry points:** `crates/node/src/custody_history_anchor.rs`, `crates/node/src/custody_index_sync.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/custody_range_runtime_tests.rs`.

**Implementation plan:**

1. Select after via Core custody_prefetch_after immediately before the real request and store the exact current head in Pending.
2. Apply the response through the atomic Core range API; re-verify the current head/time/authority on apply.
3. Continue with the same holder only when staging is complete, the page is unfinished, and the next range is needed; on capacity, move on without repeating an unchanged page.

**Verifiable scenarios:**

- A real Work expires, Core is opened from disk, the new request continues after=2 without repeating the prefix.
- A pointer change under the same root and a marker SQL failure save bodies/cursor/MLS together or change nothing.
- With 127 cached bodies, a response with two new ones does not advance the cursor and does not loop the holder.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The new H01 and the existing six runtime scenarios are green through the ordinary queue, Noise, paid verification, and Core; import goes through the common receive boundary.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h04"></a>
## V1-H04. Remove the temporary prefix-ledger and preserve read compatibility

**Type:** backend. **Source cards:** D02, D05. **Position in dependency order:** 9.
**After:** [V1-H03](01-tasks.md#v1-h03).

**Change boundary / entry points:** `crates/node/src/custody_fetch.rs`, `crates/node/src/custody_history_sync.rs`, `crates/node/src/custody_fetch_policy_tests.rs`, `crates/node/src/custody_fetch_prefix_policy_tests.rs`, `tests/evm/public_history_read_trace.py`.

**Implementation plan:**

1. Reduce Prefix/Obligation to a single range mode; remove per-Work PrefixReads and duplicate checker branches.
2. Preserve the route class order; if priority for unread holders is needed, take it from the durable Core, not from a new temporary map.
3. Keep legacy for the existing non-history locator; a generic failure does not return the legacy fallback. Align the trace kind with the parser.

**Verifiable scenarios:**

- A new Work does not make a previously read holder new.
- Capacity/unavailable/rejected/transport failures move to the next holder; the old non-history locator reads the original ciphertext.
- The 129th holder does not evict durable progress and does not disappear from the candidate list.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A single range path, the previous negative/legacy controls, and the parser are green; there is no separate diagnostic RPC.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h05"></a>
## V1-H05. Carry an unfinished root proof across Work

**Type:** backend. **Source cards:** D02, D05, F04. **Position in dependency order:** 10.
**After:** [V1-H04](01-tasks.md#v1-h04).

**Change boundary / entry points:** `crates/node/src/custody_history_sync.rs`, `crates/node/src/custody_sync.rs`, `crates/core/src/custody_history_page_import.rs`, `crates/core/tests/support`.

**Implementation plan:**

1. Find the part of Flow that loses the unfinished traversal and use the existing durable graph cursor/cache.
2. Persist only bounded verified pages and the exact head/root/source; do not serialize network streams or the whole Runtime.
3. On cold resume, re-verify the mutable authority/expiry/head and continue the missing branches.

**Verifiable scenarios:**

- A kill after receiving part of a branch proof does not force already saved pages to be read from scratch.
- A new pointer/epoch and an SQL failure do not join incompatible proof fragments; a result completed after the deadline is verified before Work is released.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A bounded incomplete proof survives Work/cold restart without a false complete and without increasing the 120 seconds/16 visits.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h06"></a>
## V1-H06. Specify a compatible unified pending-body schema

**Type:** tests. **Source cards:** D02, D05, F04. **Position in dependency order:** 11.
**After:** [V1-H05](01-tasks.md#v1-h05).

**Change boundary / entry points:** `crates/core/src/custody_prefetch.rs`, `crates/core/src/custody_history_deferred.rs`, `crates/core/tests/support`, `spec/custody-prefetch-v1.md`, `spec/custody-history-page-import-v2.md`.

**Implementation plan:**

1. Describe the operation key with its binding to conversation/index/epoch, the single body, and the reasons for waiting for import.
2. Write fixtures for both previous prefetch/deferred formats, their intersection, and cold migration.
3. Preserve the combined capacity of the two old stores: 128+128 entries and 4+4 MiB of bodies; account for metadata/proofs separately.

**Verifiable scenarios:**

- One ciphertext present in both old caches becomes a single entry without losing the fresher valid context.
- Conflicting bytes for one operation are rejected; two differently filled old caches are not truncated to a single limit of 128.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The format contract, SQL fault tests, compatibility fixtures, and ACCEPT exist before the production migration.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h07"></a>
## V1-H07. Unify storage and migration of pending bodies

**Type:** backend. **Source cards:** D02, D05, F04. **Position in dependency order:** 12.
**After:** [V1-H06](01-tasks.md#v1-h06).

**Change boundary / entry points:** `crates/core/src/custody_prefetch.rs`, `crates/core/src/custody_history_deferred.rs`, `crates/core/src/custody_history_page_import.rs`, `crates/store/src/lib.rs`.

**Implementation plan:**

1. Introduce a single persisted representation in the current ProfileStore/Core without a third cache.
2. Move the prefetch/deferred producers to shared admission with dedup and the previous combined budget.
3. Perform the forward migration atomically; the reader of old rows remains until a successful commit and cold verification.

**Verifiable scenarios:**

- A kill/SQL fault in the middle of migration leaves either the old readable state or the new one in full.
- Repeating an already imported original does not return it to pending; exceeding capacity does not advance the scan marker.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A body is stored once, the previous pending originals are available after restart, quotas and original signatures are preserved.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h08"></a>
## V1-H08. Make gap retry depend on real progress

**Type:** backend. **Source cards:** D05, F04, I05. **Position in dependency order:** 13.
**After:** [V1-H07](01-tasks.md#v1-h07).

**Change boundary / entry points:** `crates/core/src/custody_history_deferred.rs`, `crates/core/src/custody_history_page_import.rs`, `crates/core/src/lib.rs`, `crates/node/src/custody_index_sync.rs`.

**Implementation plan:**

1. Tie the decrypt/import retry to a change in relevant MLS state, the root, or the exact body.
2. Order attempts by the authenticated sender stream/epoch, not by the global group sequence.
3. Keep collecting subsequent bodies during a gap; put an SQL failure on its own retry rather than waiting for an MLS change.

**Verifiable scenarios:**

- An unchanged ratchet gap does not trigger repeated SQL/crypto attempts on every tick.
- The arrival of the missing message triggers consecutive imports; another sender is not blocked.
- After the SQL trigger is lifted, the same ciphertext is imported without a new MLS event.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is no infinite gap rotation; message/MLS/import record/pending removal/recovery progress are committed in a single receive_verified_with_states transaction.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h09"></a>
## V1-H09. Run the targeted R14 integration cluster

**Type:** verification. **Source cards:** D02, D05, F04, U01. **Position in dependency order:** 14.
**After:** [V1-H08](01-tasks.md#v1-h08).

**Change boundary / entry points:** `crates/node/src/custody_fetch.rs`, `crates/node/src/custody_obligation_read.rs`, `crates/node/src/custody_index_sync.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/custody_history_sync.rs`, `crates/core/src/custody_prefetch.rs`, `crates/core/src/custody_history_deferred.rs`, `crates/core/tests/support`, `tests/evm`.

**Implementation plan:**

1. Run the changed range/import/worker/legacy/admission/backend clusters and the related frontend recovery views.
2. Verify the cold upgrade of the previous profile and the exact rollback/import IDs; finish Clippy/fmt for the affected crates.
3. Save critic verdicts, commands, duration, failed candidates, source/config hashes, and the conclusion about the result's boundaries.

**Verifiable scenarios:**

- The targeted reversed-130 import confirms all the real messages, not a cache union.
- A zero test selection, stale binaries, or a failed negative control forbid ACCEPT.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a current component checkpoint for H01–H08; it is not directly called a native Full130 pass.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h10"></a>
## V1-H10. Compare Diagnostic32 against the unchanged baseline

**Type:** verification. **Source cards:** D02, D05, F03. **Position in dependency order:** 15.
**After:** [V1-H09](01-tasks.md#v1-h09).

**Change boundary / entry points:** `crates/node/src/custody_fetch.rs`, `crates/node/src/custody_obligation_read.rs`, `crates/node/src/custody_index_sync.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/custody_history_sync.rs`, `crates/core/src/custody_prefetch.rs`, `crates/core/src/custody_history_deferred.rs`, `crates/core/tests/support`, `tests/evm`.

**Implementation plan:**

1. Use the ordinary CLI/Core and a single trace schema reference→request→response→staging→import/rollback.
2. Execute 32 publications, real 288+288 losses, sender absent, trust refusal, both SQL faults, partial/full/cold at the previous deadlines.
3. Compare leaf occupancy, new/repeated reads, the number of admitted requests, and wall time against output/hrt32-r1; distinguish causes by the facts of the trace.

**Verifiable scenarios:**

- The SQL fault actually fires on the expected original; the imported count and the plaintext oracle match.
- A reduction in reads is not accepted at the cost of losing bodies, late expiry, or raising allowances.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The full Diagnostic32 either passes or yields a localized blocker; halving reads is a verifiable efficiency hypothesis, not a substitute for correctness.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-h11"></a>
## V1-H11. Close the unchanged Full130 and Native20 control

**Type:** verification. **Source cards:** D02, D05, U01. **Position in dependency order:** 16.
**After:** [V1-H10](01-tasks.md#v1-h10).

**Change boundary / entry points:** `crates/node/src/custody_fetch.rs`, `crates/node/src/custody_obligation_read.rs`, `crates/node/src/custody_index_sync.rs`, `crates/node/src/custody_sync.rs`, `crates/node/src/custody_history_sync.rs`, `crates/core/src/custody_prefetch.rs`, `crates/core/src/custody_history_deferred.rs`, `crates/core/tests/support`, `tests/evm`.

**Implementation plan:**

1. Build the exact candidate binaries and pin the initial gates before the run.
2. Verify 130 paid originals, 390 signatures, 1300 data/1300 index and 13000 location ACKs; delete the real 1170+1170 copies and remove the sender.
3. Pass missing trust, the first rollback, partial129, the last rollback, full130, and the cold full graph up to the original earliest expiry; compare against the Native20 R10 control.

**Verifiable scenarios:**

- All 130 exact messages are available regardless of temporary caches and the current sender.
- Neither TTL 3600, deadlines, nor read/CPU/storage limits are changed for the sake of PASS; all processes are stopped and temporary keys deleted.

**Checks:** HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Full native evidence exists on unchanged inputs/binaries; R13/R14 failures are saved. Otherwise D05 remains open with the cause measured.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
