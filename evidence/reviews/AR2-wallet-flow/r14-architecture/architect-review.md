## Verdict

**The minimal sensible rework is to make the existing bounded paid page the unit of publication and recovery, and to leave `Work` only as the quantum of execution.** Currently the lifecycle of a message, a network request, a graph traversal and MLS recovery is too tightly coupled to the short-lived scheduler states. Because of this, already-done work is partially repeated, and the ciphertext obtained does not necessarily turn into the next admissible imports.

I would not add one more priority for original 31, a third cache or an additional retry. The main option is **a shared history publisher for the batch, resumable page reading and a single store of not-yet-imported ciphertext**, preserving the existing atomic Core import.

This is a justified direction, **but not a proven fix of the cause of the missing 31**: R14 does not have a sufficient trace of its path. Below I separate the confirmed redundant work from assumptions about its influence on this particular omission.

The evidence boundary is respected: compared to R14 one production file changed — `public_sender.rs`; fixing the observation of an unfinished graph does not confirm an improved native throughput or the elimination of the recipient problem. A new native run was not performed within this review. [Snapshot comparison with R14](sandbox:/mnt/data/agentic_review/review/CURRENT_VS_R14.json).

## Five findings by importance

### 1. Batch publication exists, but R14 effectively creates history almost one message at a time

A repeated structural parse of the archived graph gave:

| Metric                                       |     R14 |
| -------------------------------------------- | ------: |
| Originals                                    |     130 |
| Leaf pages                                   | **117** |
| Of them with one original                    | **106** |
| With two / three originals                   |   9 / 2 |
| Branch pages in the current graph            |     112 |
| Total pages reachable from the current root  | **230** |

The archive also contains 117 **prepared** root versions. This does not mean that all intermediate roots were fully published and confirmed. [Repeated parse results](sandbox:/mnt/data/architecture_review_results/r14_structural_measurements.json).

The existing `prepare_public_sender_history()` already collects ready messages, and `prepare_custody_history_batch()` allows **up to 12 originals in one leaf**. But the collection is invoked from an individual message's work; additional candidates must already have 10 index replicas and 100 location ACKs, and their preparation is bounded by a 20 ms pass. After the first additional attempt this budget can stop the collection. `Work` itself keeps its own `history_stage`, `history_not_before`, `history_member` and the pointer publication state. [Batch collection](sandbox:/mnt/data/agentic_review/project/crates/node/src/public_sender_history.rs), [Core batch](sandbox:/mnt/data/agentic_review/project/crates/core/src/custody_history_batch.rs), [sender Work](sandbox:/mnt/data/agentic_review/project/crates/node/src/public_sender.rs).

**Confirmed:** the effective leaf size in R14 is about 1.11 messages. The same useful history volume is represented by a substantially larger number of signed pages and links than the existing format allows.

**Not confirmed:** how much of the 2407 seconds falls on this fragmentation, and how much on data/index placement, discovery, proof verification and waiting for ACKs.

Two limits of the conclusion matter here:

* `Graph::new()` already reuses the exact live subtrees of the published checkpoint. It would be wrong to call publication an unconditional re-placement of the whole graph at quadratic cost.
* Location updates are already sent in `dataPositions` batches. **13,000 ACK facts are not proof of 13,000 separate RPCs.** Their semantics cannot simply be deleted as «redundant requests». [Graph reuse](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_history_publish.rs), [location publication](sandbox:/mnt/data/agentic_review/project/crates/node/src/public_sender_index.rs).

### 2. Completing a `Work` resets the optimizations that bounded recovery efficiency depends on

Several different boundaries exist simultaneously in recovery:

* `Work` is bounded by 120 seconds;
* the traversal bounds the number of visited references, and the source bounds the number of processed pages at 16; **this is not necessarily 16 imports**;
* automatic reads use the shared `Admission<24,12>`: 24 requests per minute in total, 12 to one peer;
* outgoing paid requests share bounded slots, and the server separately applies `Admission<32,16>`.

At the same time `PrefixReads` lives inside the history `Flow` created for a new `Work`. After recreation, the first request to the same holder again becomes `Prefix`, that is `after_sequence=0`. The durable graph cursor is kept, but the prefix ledger, the current plan and part of the local traversal context are not. [Lifecycle and admission](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_sync.rs), [Flow](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_history_sync.rs), [prefix/exact/legacy](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_fetch.rs).

There is also a specific unpleasant branch: in `advance_custody_sync()` after a pending request completes, the age of the `Work` is checked first, and only then is `index_output` extracted. At the 120-second boundary **a ready result can remain unapplied to the recovery plan**. Already performed durable cache/import records are not necessarily lost then; it is wrong to say that all progress is reset to zero.

The bulk-read contract is even more significant. `checked_obligation_page()` checks the page as a whole, including `next_sequence` and `complete`, but returns only envelopes. Then `FetchedPage` contains `requested`, `envelopes`, `retry_exact`, and the full continuation of the page is not kept. The result is:

> We paid with a network request and checks for a page, saved several bodies, but the controlling path still serves one chosen original.

A new `Work` can again start a prefix from zero and then switch to exact. [Page verification](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_obligation_read.rs), [result intake](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_index_sync.rs).

Finally, `capacity`, `unavailable`, `rejected` and `transport_failure` move Prefix/Obligation to Legacy on the same holder. But the server returns `Rejected` both on an authorization refusal and on a rate limit. **Overload can be mistakenly handled as protocol incompatibility**, producing an extra request into the same bounded budget. This is a confirmed code possibility, not an established cause of the missing 31. [Fallback](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_fetch.rs), [server admission](sandbox:/mnt/data/agentic_review/project/crates/node/src/paid_custody.rs).

### 3. Prefetch and deferred duplicate data, and waiting for MLS turns into repeated checks and SQL writes

The graph order really differs from the message order: the start is `1, 6, 7, 2, 8, 9…`; original 31 is at position 30. Inside a leaf Core sorts by `sequence`, but different leaves are added in publication-readiness order. Therefore the graph ordinal cannot be used as the MLS import order.

The strict contiguous receive itself must not be removed. On a `ReceiveGap` the cryptographic layer additionally checks the message on a separate state candidate, without fixing the gap. The problem is not this protection but the frequency of repetition without new circumstances. [MLS receive](sandbox:/mnt/data/agentic_review/project/crates/crypto/src/lib.rs).

`advance_custody_sync()` makes up to four `retry_deferred_custody_history(..., 1, ...)` calls per pass. On the next `ReceiveGap` the deferred item is moved to the end of the queue, after which the queue is serialized again and written to SQL. **The MLS state may not have changed at all, while the checks and the write happen again.** [Deferred retry, lines 214–292](sandbox:/mnt/data/agentic_review/project/crates/core/src/custody_history_deferred.rs).

The archived snapshot shows the consequences of the two layers crossing:

| Check                                                       |        Result |
| ----------------------------------------------------------- | -------------: |
| Originals simultaneously in deferred and prefetch           |            **8** |
| Already imported originals again found in prefetch          |         **1–24** |
| Unique originals in the two caches                          |               83 |
| Unique originals in the caches **or** imports               | **89**, not 121 |
| Deferred revision at 30 imports                             |             7355 |

This is consistent with repeated re-insertion and frequent rewrites, but the revision cannot be automatically equated to the number of failed decrypts or to a specific number of lost seconds. The snapshot was taken after the stop. [Measurements](sandbox:/mnt/data/architecture_review_results/r14_structural_measurements.json).

The cause of the re-insertion is visible in `cache_custody_prefetch`: on page intake there is no filter by already imported and deferred operations; deduplication is limited to the prefetch itself. [Prefetch](sandbox:/mnt/data/agentic_review/project/crates/core/src/custody_prefetch.rs).

**At the same time, increasing the graph cache does not look like an R14 fix.** All 229 non-root pages of the current graph in the JSON representation used occupy approximately **857,593 bytes**, below the 1 MiB and 512-record limits. This is a full-representation size calculation, not a measured runtime high-water mark, but the version «this graph fundamentally does not fit» is not confirmed.

And the main point: **reordering deferred will not return 31, which is not in deferred or prefetch.** It only frees resources for its search and subsequent import.

### 4. With the original limits, not only the speed of individual operations matters, but also the number of mandatory passes

Publication consumed **2407 of 3600 seconds**, that is about 67% of the first messages' term. The remaining **1193 seconds** must fit loss preparation, trust/SQL checks, discovery, proofs, body retrieval, import and the required cold finish.

To illustrate the scale at the limit of 24 reads per minute:

| Recovery model         | Requests | Time at average throughput |
| ---------------------- | -------: | -------------------------: |
| 3 requests per original |      390 |                       975 s |
| 4 requests per original |      520 |                      1300 s |

This is **not a reconstruction of R14**: bulk can return several originals per request, and part of the requests relates to shared proofs. But the calculation shows a structural boundary: at four requests per original the average cost already exceeds the whole remaining window, before CPU, SQL and discovery. The fixed-window start burst is not a sufficient basis for counting on success.

Therefore increasing parallelism by itself does not solve the problem. There is client admission, server admission, shared pending slots and a shared processing budget. What is needed is **fewer repeated requests and more useful originals per verified page**, not just a faster choice of the next peer. [Admission windows](sandbox:/mnt/data/agentic_review/project/crates/node/src/bootstrap_schedule.rs), [shared processing limits](sandbox:/mnt/data/agentic_review/project/crates/node/src/processing_budget.rs).

The R14 counters do not allow attributing the delay to a specific layer. `bulk reads` and `history-path reads` are categories within reads; they cannot be added to 418 as independent requests.

Separately: original 31 expired **261 seconds later than the first original**. Therefore the firing of the shared earliest-expiry guard does not prove that 31 was missing due to the expiration of its own lease. [R14 focus](sandbox:/mnt/data/agentic_review/review/R14_FOCUS.json).

### 5. The acceptance of «we deleted 9 of 10» is currently narrower than the guarantee «we survive any 9 losses»

`paid_history_route()` puts up to four index transport keys into a reference. When nine indexes are deleted, the fixture selects the surviving index **from these published candidates**, not arbitrarily from all ten. [Advertised route selection](sandbox:/mnt/data/agentic_review/project/crates/node/src/custody_history_sender.rs), [loss fixture](sandbox:/mnt/data/agentic_review/project/tests/evm/public_index_recipient.py).

This is **not an explanation of R14**: its survivors are within the intended set. But it is a boundary of the product guarantee.

If the promise really means any nine losses, a path of discovering the remaining six is needed: for example, through the already existing deterministic placement and a verifiable search, or through a full authenticated coverage of candidates. Four routing hints are not enough to prove such a guarantee. This should be decided explicitly, without changing the current Full130 for a new convenient case.

## One main simplification path

### A. One history publisher per `(conversation, index_id, epoch)`, not a publisher process per message

Keep payment, placement obligations and the delivery ledger individual. **Merge the history graph and pointer coordination.**

Reuse the existing:

`prepare_custody_history_batch()` → a paid page publication queue → canonical ACKs → published-root checkpoint → pointer.

The scheduler must work with a bounded group of **already enqueued** messages: first bring their data/index placement to completion, then assemble the ready batch into a leaf, instead of each message immediately creating an almost singleton root. The limit of 12 is dictated by the current SQL transaction budget; raising it is not required for this decision.

There is no need to wait for future messages to fill the batch. For a single online send, immediate advancement remains; for an existing queue the shared work must be amortized. A stuck group member must not indefinitely block the ready ones — the existing attempt and error boundaries are used.

In the fixture with batches of 16, the reference for `12+4` packing is **17 leaves per 130 originals instead of 117**. This is a target structure, not a promise of an already measured speedup.

**Why not to change the graph format yet:** the existing format already allows a significantly more compact representation. First its actual usage should be fixed, not a new history protocol introduced.

### B. Reading continuation belongs to recovery state, not to the next `Work`

Keep the 120-second and 16-visit limits as fair-scheduling rules. But the end of a quantum must mean **«continue later»**, not «restore the temporary optimizations from zero».

On the existing Core cursor and cache, keep a bounded continuation for the exact head/root and sources. A ready answer should be verified and counted before the quantum is released, with current checks of time, authority and bindings.

Merge `Prefix` and `Obligation` into one range read with an `after_sequence` parameter. Return the verified `next_sequence` from the page check, not only the list of bodies and `retry_exact`.

Three mandatory constraints:

**The cursor advances only after the durable intake of the result.** On local staging overflow, one cannot write «read» and lose the bodies.

**`complete` means the end of this holder's data, not the end of history.** Completeness is still determined by the exact signed graph and committed imports.

**A skipped reference has priority over the cursor.** If after repair a holder has an earlier operation, the monotonic range cursor must not exclude it forever: an addressed read remains a parameter of the same path.

Keep the legacy read of old data, but remove the automatic interpretation of congestion/transport failure as a sign of an old protocol. Compatibility must be determined separately from temporary availability; `Rejected` alone is insufficient.

### C. One ciphertext staging store; deferred is derived state

Merge prefetch and deferred by operation key with an exact binding to index/epoch. Store the body once. Take membership proofs from the shared verified metadata cache; do not duplicate the full proof in yet another layer where a reference to the exact verifiable obligation is enough.

From the staged body and the current MLS state it is determined whether the message can be imported. Repeat a gap attempt when the MLS state or other significant inputs change, **not on every tick**. An SQL fault is a separate reason for repetition; its retry must not be blocked waiting for an MLS change.

The import order is by the authenticated sequence of the corresponding sender/epoch stream, **not by graph ordinal and not by the global sequence of all group members**. Proof traversal and body collection must not stop on one not-yet-importable message.

Accumulating all 130 bodies first is not required. A bounded window and the priority of the missing predecessor are needed. When merging the caches, the former combined budget of the two stores must not be quietly replaced by a single 128 limit and a new deadlock then explained by lack of space.

**The fixpoint stays the same:** message, MLS state, import record, removal of the pending body and the recovery-progress change are one transaction through the shared `receive_verified_with_states`. [Existing atomicity boundary](sandbox:/mnt/data/agentic_review/project/crates/core/src/custody_history_page_import.rs).

### What is merged and what is not deleted

| Now                                                            | After the change                                                       |
| -------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Per-message history/pointer coordination                        | One shared history publisher; individual paid obligations and statuses |
| `PrefixReads` + Prefix/Obligation + `RetryExact`                | One range read and a bounded continuation; addressed requests for holes |
| Prefetch and durable deferred with intersections                | One pending-body store; eligibility determined from MLS                |
| Deferred reordering with an SQL write without progress          | A retry after a change of significant state                             |
| Repeated identical clean checks in one pass                     | Reuse of a verified object within the pass                              |

**Do not delete:** once-spend, signature and authority checks, the original expiry, exact root/epoch/pointer fences, atomicity, the legacy data reader, autonomous repair and the `stored`/`delivered` distinction. An object verified in the past does not become an eternal permission: mutable conditions are checked at the current execution/commit boundary.

## Minimal scope of changes

This is not a local retry patch, but also not a new storage protocol. The changes are limited to the owners of the existing states:

| Area                                       | Main files                                                                                                                     |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Shared batch/history publication           | `public_sender.rs`, `public_sender_history.rs`, `custody_history_sender.rs`; reuse of `custody_publication_queue.rs`             |
| Resumable traversal and page reading       | `custody_sync.rs`, `custody_history_sync.rs`, `custody_fetch.rs`, `custody_obligation_read.rs`, `custody_index_sync.rs`          |
| Continuation and merged staging            | `custody_history_page_scan.rs`, `custody_prefetch.rs`, `custody_history_deferred.rs`                                            |
| Keeping the atomic boundary                | `custody_history_import.rs`, `custody_history_page_import.rs`                                                                   |

I would not rewrite the spend scheme, the cryptographic graph format and the shared transport at this stage. When durable staging changes, compatible reading of the old state will be needed; a cold restart must not lose previously accepted bodies.

## What to minimally measure for original 31

One correlatable trace on the existing path is needed, without additional diagnostic RPCs. Common keys: `work_id`, root/index/epoch, operation, sequence, graph ordinal, peer, request kind and monotonic time.

It must distinguish four transitions:

1. **Reference → request:** when 31 was selected, the proof obtained, the candidate chosen; how long discovery and admission/slot waits took.
2. **Request → response:** the actual enqueue, the response, proof size, the number of new/repeated bodies, the fallback reason; separately — admission spent but the request not enqueued.
3. **Response → staging:** whether 31 was verified, saved, and why not; whether the result remained unapplied at the end of the `Work`.
4. **Staging → import:** MLS revision, gap or SQL fault, commit/rollback.

Compact events are needed for the other requests too: otherwise it is impossible to see **which work consumed the budget while 31 was waiting**.

This will make it possible to distinguish «31 was not selected» from «the index was not found», «the body was not received», «received but not saved» and «could not be imported». The current counters do not distinguish these.

The snapshot has a Diagnostic32 wrapper and a check for the presence of `custody_read_queued` / `custody_read_finished`, but I did not find production emit sites of these events. The presence of a diagnostic file is not yet a ready trace or a run result. [Diagnostic32](sandbox:/mnt/data/agentic_review/project/tests/evm/public_history_read_trace.py).

## Check before the next Full130

### A short refuting native test

Use the existing **Diagnostic32 on the shared ordinary CLI/Core path**: 32 crosses the 16 boundary and includes the problematic 31. Keep the original TTLs, deadlines, allowances, the absence of the sender, the real deletions of **288 data + 288 index copies**, trust-refusal, both SQL fault/rollback stages, partial/full import and the cold cycle.

Compare a frozen baseline and the changed release build with identical tracing. The old R14 does not replace the baseline of the current snapshot.

Proposed criteria before moving to Full130:

* With an unchanged head, a transition between `Work`s **does not produce a repeated prefix from zero merely because the local ledger was lost**. There are no unapplied successful results at the quantum boundary.
* With unchanged MLS/root there are **no SQL writes for rotating the gap queue**; at most one gap check of a body per unchanged significant state. An SQL fault is checked separately.
* There are no duplicates between pending layers; already imported bodies do not return to staging.
* For two regular batches of 16, the target packing is **at most four leaves**. The proposed performance threshold is at least a twofold reduction of admitted recovery reads relative to the baseline, without degrading completeness and rollback checks.

The last threshold is an engineering criterion for testing the hypothesis, not an already obtained result. If the pages became larger but the number of reads barely decreased, the main cost remains elsewhere and going straight into another Full130 is premature.

Check the result at the 120-second boundary, the refusal of durable admission with full staging and a root/epoch change with separate small regressions. **The success of 32 messages by itself proves nothing about the full 130 time budget.**

### Then — unchanged Full130

Without changing the criteria, all stages must be reached: 130 originals, the original signatures and receipts/ACK, real losses of 1170+1170, trust-refusal, the first exact SQL rollback, partial 129, the last exact SQL rollback, full import and the cold full-graph cycle up to the original earliest expiry.

Actual execution hashes and a clean cleanup are needed. An unreached CLI post-check must not be turned into a «hash mismatch». R10 remains a mandatory control; the cause of R13 with missing location ACKs remains a separate open problem. Even a successful Full130 does not replace the agreed 67 tasks / 22 E2E / three OSes.

## When a product owner decision will be needed

After the repetitions are removed, the budget should converge:

$$
T_{\text{publication}}+
T_{\text{loss/setup/faults}}+
T_{\text{discovery/proofs/bodies/import}}+
T_{\text{cold}}
< 3600\ \text{s}.
$$

If the **mandatory**, already amortized work still does not fit with the existing resources and allowances, this is a conflict of product parameters. Then an explicit decision is needed on the admissible volume of offline history, resources/tariff or the conditions of the availability guarantee — not a hidden TTL increase in the test.

For a finite TTL, the guarantee must also assume the reachability of at least one usable copy and a bounded time of network failures. «There is one copy somewhere in the network» by itself does not give a guaranteed recovery time.

**Summary:** the most justified rework is to remove the repeated execution of work when its owner changes: message → shared history, `Work` → recovery continuation, prefetch → deferred. Almost all the necessary primitives already exist for this. But the problem of 31 can be considered closed only after a trace of its full path and an unchanged Full130, not after another green local test.

A repeated parse of the archive is available separately: [JSON measurements](sandbox:/mnt/data/architecture_review_results/r14_structural_measurements.json), [the reproducing Python script](sandbox:/mnt/data/architecture_review_results/analyze_r14.py), [the description and exact references to source areas](sandbox:/mnt/data/architecture_review_results/README.md).
