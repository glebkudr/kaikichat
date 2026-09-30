# Runner follow-up: A04 history-commit pipeline (2026-09-19)

Scope: `A04` iteration on top of `3fc9b00` (bounded stale-member retries).
This round fixed the node-side history-batch machinery itself; the previous
round fixed custody transport. H11 stays out of scope.

Evidence branch: `fix/v1-custody-history-bounce-20260919`
(`3fc9b00` bounded misses → `a7a4ce2` commit-verified-subset).
Base for the next patch: `implementation/v1` once `a7a4ce2` lands.

## What this round changed (keep)

`prepare_public_sender_history` used to requeue every member that missed the
commit fence and refused to sign while `batch.candidates` was non-empty. Under
sustained input churn (fresh INTENTS digests, renewals, receipt writes) the
drain set never stayed simultaneously current: verified members were held
hostage by stale peers, went stale themselves next visit, and the set rotated
forever. `3fc9b00` bounded the retries (drop after 4 misses) but converted the
wait into destroy-and-recreate — the native run committed **zero** leaves.

`a7a4ce2` removes the gating entirely: the fence commits exactly the members
verified at that visit (`public_sender_history_jobs_current` +
`paid_sender_history_current` + still-active), one atomic
`prepare_custody_history_batch`. Stale members stay queued and a later batch
collects them — churn now rotates membership instead of postponing every
commit. The collector batch also outlives scheduling cleanup: an expired
cohort sweep and `reconcile_placements` membership shrink used to delete
`history_batches` mid-collection; both now keep it (admission + the commit
fence re-verify every member against live Core state; `reconcile()` still
drops batches with no active members).

Unit coverage (RED→GREEN, all reproduced the defects before the fix):
- `members_current_at_the_fence_commit_instead_of_waiting_for_stale_peers`
  — rotating staleness freeze; verified subset commits, stale peers rejoin
  a later leaf.
- `an_expired_cohort_sweep_keeps_the_inflight_batch_for_its_members`.
- `a_member_leaving_the_active_set_keeps_the_batch_for_the_survivors`.
- Existing `members_stale_between_visits_are_dropped…`, `cached_readiness…`,
  13-job bound/SQL-retry tests stay green — **48/48 public_sender, fmt+clippy
  clean**.

## Native A04 — still RED, failure moved downstream

| Metric | pre-patch `183441Z` | `3fc9b00` `231457Z` | `a7a4ce2` macOS `001504Z` | `a7a4ce2` Linux `003707Z` |
|---|---|---|---|---|
| history leaves | 15/43 | **0/43** | **10/43** | 1/43 |
| leaf ACKs (per leaf) | 4,4,5,9,9,10×10 | — | 0,0,2,5,5,7,8,8,8,8 | 0 |
| pointer | 10 | 0 | 0 | 0 |
| stored | 8 | 0 | 0 | 0 |
| delivered | 0 | 0 | 0 | 0 |
| max history revision | 19 | 0 | 17 | — |

- The commit-subset fix restored leaf commits under churn (0 → 10 macOS,
  0 → 1 Linux); the rotating-staleness freeze is gone from the signature.
- Every committed leaf stalls **below 10/10 acknowledgments** (macOS max 8;
  the single Linux leaf 0). `has_terminal_publication` requires
  `acknowledgments.len()==10`, so no pointer is ever published and no job
  completes. Note the pre-patch trace already showed the same ACK wall
  (5 of 15 leaves <10 ACKs) — the new fix exposed it by making commits reach
  the ACK stage at all.
- `checkpoint_rejected` sightings: 16 (macOS) / 3 (Linux), concentrated in
  the terminal sample — renewal races still cost visits but no longer freeze
  the cohort.
- `custody_capacity` remains the dominant error (561 / 505 sightings) —
  the unread-slot backpressure from the previous round, still worth the
  architect's attention as a self-reinforcing stall candidate.

### Residual mechanism (bounded for the architect)

After `prepare_custody_history_batch` commits a leaf, the sender pushes
`IndexHistoryPagePut` to each of the 10 anchor positions
(`custody_history_sender.rs` `publish` loop: ≤4 attempts/pass, 5 s
`retry_after` per position, deferred when `custody_peer_allowed` fails). The
custodian stores the page via `retain_index_history_page`, which calls
`verify_index_obligation` + `index.require_unexpired` at the **custodian's**
`now`. Under renewal churn the obligation view expires → the PUT is rejected
→ the position cycles `retry_after` forever and the leaf never reaches
10 ACKs. A permanent `Err` inside `queue_history_page`'s `prepare` also
aborts the whole publish pass for that visit.

## Narrow ask

1. **Leaf-ACK convergence.** A committed leaf must reach 10/10 ACKs: either
   the custodian accepts the page against a *historical* obligation proof
   (the page is immutable, pinned to `anchor_operation`), or the sender
   re-arms rejected positions instead of cycling `retry_after` on a
   permanently-stale obligation. Surfaced: which positions reject and why.
2. **Pointer publication.** Once ACKs complete, the pointer stage must run;
   verify it is not gated on the same stale snapshot class.
3. Keep: commit-verified-subset, batch-survives-sweep/reconcile, bounded
   passes, atomic `commit_states`, no stale-member signing.
4. H11 untouched this iteration.

---

## Round 2: `3015de9` — publish-pass starvation fix (same day)

Evidence branch: `fix/v1-history-ack-starvation-20260919` → merged into
`implementation/v1`. **A04 remains RED.**

Correction to the round-1 table: `stage.info` (the `history` field in job
views) is populated **only for `Root` pages**
(`custody_history_sender.rs`, `if page.kind == Root`). Jobs showing
`acknowledgments` had already converged every leaf to 10/10 — the `0–8`
sightings were **root-page** ACKs, not leaf ACKs.

### What this round changed (keep)

`publish_history_page` used to `return Err` on the **first** non-success
pending job in its cleanup loop. That aborted the whole visit: sibling
terminal jobs stayed in `pending`, the publish pass never ran, and one
failing position serialized every other position into 5 s `blocked` cycles.
`3015de9` drains all terminal jobs per visit, defers each failure into
`retry_after`, still runs the publish pass for eligible positions, and only
then returns the first collected error. Coverage:
`paid_batch_history_put_failure_does_not_starve_sibling_positions`
(RED→GREEN; critic ACCEPT). 150/150 `paid_custody`, 48/48 `public_sender`,
fmt+clippy clean. Linux `--all-targets`: 97 suites, 0 failures.

### Native A04 — still RED, wall unchanged

| Metric | `a7a4ce2` macOS `001504Z` | `a7a4ce2` Linux `003707Z` | `3015de9` macOS `054610Z` | `3015de9` Linux `055314Z` |
|---|---|---|---|---|
| jobs reaching root stage | 10/43 | 1/43 | 1/43 | 1/43 |
| root-page ACKs (max) | 8/10 | 0 | 0 | 2/10 |
| max history revision | 17 | — | 1 | 14 |
| index leg at end (10,100) | partial | partial | 43/43 | 41/43 |
| blocked errors at end | ~15 (checkpoint_rejected, custody_unavailable, capacity) | mixed | 1 (checkpoint_rejected) | 3 |
| pointer / stored / delivered | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |

- **No regression**: revision spread across platforms (1 vs 14) mirrors the
  baseline's own variance (17 vs 1) — churn timing dominates commit counts,
  not the platform or the patch.
- **Improvement**: error churn collapsed — jobs stay `publishing` instead of
  cycling `blocked` (`checkpoint_rejected` 15 → 1 on macOS). The index leg
  (`indexReplicas`/`indexLocations`) now converges fully for every job,
  faster than before.
- **Wall unchanged**: no run converged a root page to 10/10 ACKs inside
  600 s; `has_terminal_publication` never fires, so no pointer is published
  and the scenario times out identically
  ("ordinary sender did not publish the complete paid batch").

### Residual mechanism (narrower now)

The starvation fix removed the *sender-side* serialization: every visit now
queues all eligible positions instead of stopping at the first failure.
Remaining wall is genuine ACK convergence — leaf or root pages stall below
10/10 under churn. Suspects, in order:

1. `custody_peer_allowed` deferrals (live swarm + allowed-operator
   connection required per position; churn keeps positions in `retry_after`).
2. Custodian-side `retain_index_history_page` rejects
   (`verify_index_obligation`/`require_unexpired` evaluated at the
   custodian's `now`; obligation expiry under renewal churn rejects PUTs).
3. `PASS_BUDGET` (20 ms) + 5 s `retry_after` pacing limiting attempts/sec
   against a churning peer set.

The narrow ask from round 1 stands verbatim — convergence must be made
against a historical obligation proof or rejected positions re-armed; which
positions reject and why still needs surfacing (node logs are empty in the
captured runs; a trace flag for per-position reject reasons would settle it).

Traces: `cb-evidence/a04-{macos,linux}-3015de9/` (this volume),
macOS run `20260919T054610Z`, Linux run `20260919T055314Z`
(`dfc6710`, cherry-picked equivalent of `3015de9`).

## Round 4 — per-position diagnostics land (`ae15087` + `4efa68a`)

`Stage.diagnostics` now publishes `{pageKind, revision, anchorOperation,
manifestHash, acknowledgments, positions}` into `view["historyDiagnostics"]`
on every `publish_history_page` visit — leaf and branch pages included.
`view["history"]` keeps its root-only `SenderHistoryObservation` shape
(deny_unknown_fields contract verified by a key-set pin in
`a_successor_root_does_not_inherit_old_root_acks_or_pointer_progress`), so
typed progress, terminal detection and the runner are untouched; the raw
`public_sender_status` view carries the diagnostics into `lastSender`.

First instrumented macOS run `20260919T102042Z` (`ae15087`, still masked
`due`): 43 jobs — 40 stuck on **branch** pages at mostly 8/10 ACKs, 2 leaf,
2 fully acked. Position outcomes aggregate: `acked 326, due 64, pending 16,
queued 9, rejected:rejected 3, custody_capacity 2`. The stuck positions are
not random: **positions 8 and 9 account for 37/43 unacked positions**, and
they are the same two custodian transport keys across every job
(`df68b5a8…`, `f622f7b5…`). Index receipts carry `expiresAt` ~3600s with
~50 min of margin at the 600s cap, so obligation expiry is ruled out; one
authority renewal (block 66) landed mid-run. Because leaf pages for the
same jobs completed 10/10 earlier, the same two custodians did serve leaf
PUTs — the wall is specific to the later branch/root stage.

`4efa68a` makes `positions` persistent: backoff positions now keep their
last outcome instead of collapsing to `due`, so the next run shows whether
8/9 churn `peer_unavailable`, custodian `rejected:*` or
`failed:transport_failure`.

Tests: `stage_info_reports_per_position_outcomes_while_a_leaf_is_publishing`
pins the leaf shape (pageKind, revision, manifestHash, acknowledgments,
10-position map with pending/failed:*/rejected:*/peer_unavailable outcomes);
`paid_custody` 151/151, `public_sender` 48/48, fmt+clippy clean.

## Round 5 — split history-put reject codes (`fc5448f`)

Instrumented runs on `4efa68a` showed the residual wall is not starvation
and not discovery: positions repeatedly end `rejected:rejected` while the
index leg converges 10/10. But `rejected` was overloaded — wire
`Response::Rejected`, the 10-second freshness gate, a lost/unauthorized
operator connection, a mismatched peer and a commitment mismatch all
surfaced the same string.

`fc5448f` splits the codes **only** for `Context::HistoryPut` jobs
(backend-test-critic advisory: `HistoryFlow::fallback`, `custody_sync`
and the Put/IndexPut error records key on the literal `"rejected"`, so a
global rename would change read/fallback semantics — deliberately out of
scope):

| condition                              | history-put code        |
|----------------------------------------|-------------------------|
| wire `Response::Rejected`              | `refused`               |
| wire `Unavailable` / `Capacity`        | `unavailable`/`capacity`|
| wire `StorageError`                    | `storage_error`         |
| wrong response variant                 | `unexpected_response`   |
| response from wrong peer               | `peer_mismatch`         |
| connection closed/re-authed mid-flight | `connection_lost`       |
| response older than 10s freshness gate | `stale_response`        |
| history commitment mismatch            | `commitment_mismatch`   |

Note on `stale_response`: `job.created` is stamped at wire-send (after
`send_request`), not at position queueing — so the 10s window measures
real response latency, not slot starvation in the ≤4 in-flight pool.

Results — `fc5448f` runs (both failed at the same 600s wall, still RED):

- **macOS** run `20260919T114325Z-2d54433e870c`: 43 jobs — 39 leaf, 2
  branch, 2 root pages; ACK counts 8-9 dominant (15 jobs at 8/10, 24 at
  9/10). Outcomes: `acked 346, rejected:refused 51, pending 8, queued 10,
  due 11, custody_capacity 4`. Position 8 is unacked on **all 43 jobs**,
  position 9 on 19 — same two-custodian pattern as the `4efa68a` run.
- **Linux** run `20260919T114329Z-80fb9b67936c`: 37 jobs reached **root**
  pages (revision 15), 36 stuck at **9/10** with position 0 =
  `rejected:refused` on every one; one job converged 10/10.

**The residual wall is a custodian-side wire `Response::Rejected`** — not
discovery, not connection churn, not the 10s freshness gate
(`stale_response`, `connection_lost`, `peer_mismatch` all absent), not
capacity (`custody_capacity` marginal). What `Rejected` erases is *which*
`SpendError` the custodian hit inside `retain_index_history_page`
(obligation verification vs anchor-binding vs conflict) and whether it
came from the handler or the outer admission gate — that is the next
instrument (`14d79f0` adds `HistoryPageRejected{reason}`).

Results — `14d79f0`/`28005f04` runs (typed refusal reason on the wire):

- **macOS** run `20260919T125352Z-472aa751055c`: 43 jobs — 11 leaf, 32
  branch; 8 jobs reached 10/10. Outcomes: `acked 359,
  rejected:refused 24, pending 23, due 10, queued 8, custody_capacity 6`.
- **Linux** run `20260919T125406Z-b77a6f0c3b82`: 43 jobs — 21 leaf, 22
  branch; 8 jobs at 10/10. Outcomes: `acked 257, pending 63, due 51,
  queued 30, custody_capacity 22, rejected:refused 7`.

**Every refusal on both platforms is bare `refused` — zero
`refused:<reason>`.** Since `14d79f0` routes every handler-level
`SpendError` from `serve_index_history_page_put` to a typed
`HistoryPageRejected` (or `unavailable`/`capacity`/`storage_error`), a
bare `Rejected` for a page PUT can only come from the admission gate in
`serve_authorized_paid_custody`: `operator_connection_allowed` +
`admission.allow`. Connections were stable all run (no
`connection_lost`/`peer_mismatch` outcomes), so the dominant mechanism is
`paid_custody.admission` = `Admission<32, 16>` — **32 requests/minute
total across all peers, 16/minute per peer, 60s fixed window, shared by
every request type** (index puts, locations, page puts, reads, copies).
A04 demand is orders of magnitude above that (~43 jobs x 10 positions x
leaf/branch/root x retries), so whichever positions are attempted after
the window fills are refused — matching the rotating unacked position
(8/9 on macOS, 0 then spread on Linux).

Fix: `IndexHistoryPagePut` gets a dedicated admission budget
(`history_admission: Admission<128, 64>`) inside the same gate —
replication writes no longer compete with interactive traffic and remain
bounded. `operator_connection_allowed` still runs first for every
request; the shared `Admission<32,16>` bucket is unchanged for all other
request kinds. Unit pins:
`history_page_puts_use_a_separate_bounded_admission_budget` (put admitted
after shared exhaustion, 64-put dedicated cap, conn-check ordering,
legacy `index_history_put` cannot borrow the budget) plus updated
sixteen-request-window pins in `page_requests_...` and
`wire_path_cannot_bypass_...` (puts no longer consume the shared cap).

Tests: `paid_custody` 151/151, `public_sender` 48/48, full agentic-node
suite green (333+101+4), fmt+clippy clean.
suite green (333+101+4), fmt+clippy clean.

## Round 6 — sender-side data-put lane + publish lane (`7e96ddd`, `d278f2a`)

Typed codes on `Context::Put` (`d93969f`, merged `769c6d8`) made the next
straggler legible. Linux run `20260919T181206Z-e5d16721b4aa` (cut early
with SIGINT; trace.json intact): series 1 completed, series 2 stalled on
its first job (`seq=130`, `2cdf33…`) — **9/10 replicas, only position 7
(ordinal 12) missing**. Resolution worked: `queriesSent: 1`, a live offer
for peer `12D3KooWCnxeSxc…` (index 13, validUntil 1789843851 <
expiry 1789846777). The job then cycled ~35 min:
`custody_capacity` → `refused`/`rejected` → `put_after` 5s → re-resolve.

Root cause split: `custody_capacity` is sender-side queue pressure
(4 in-flight + 16 retained jobs shared by every request kind);
`refused`/`rejected` are wire `Response::Rejected` — the receiver-side
shared `Admission<32,16>` gate still covered data `Request::Put`
(history page puts already had `history_admission`).

Two production changes landed:

- `7e96ddd` — sender-side `Lane::Put`: data puts get their own in-flight
  window (16) and retained-job cap (64) beside interactive reads (4/16)
  instead of competing for the shared pool.
- `d278f2a` — sender-side publish lane: `history_put`, `index_put`,
  `index_location` jobs share a dedicated 16/64 lane; `existing_request`
  became lane-aware; the A04 in-scenario invariant was updated for the
  two-lane model (`11f433b`).

## Round 7 — receiver-side data-put admission (`7d0e486`, merged `d2dee41`)

RED test `data_puts_use_a_separate_bounded_admission_budget`
(custody_history_page_tests.rs): after 16 interactive reads exhaust the
shared per-peer budget, a real `Request::Put` is still gate-refused —
the server has the fixture authority configured, so a gate-passed put
reaches `selected_custodian_proof` and answers `unavailable`, proving
the rejection came from admission, not the handler. Critic round 1
flagged the vacuous-red version (no authority configured → handler
rejects identically to the gate); the revised test configures
`postage_client` through the `a03_configure` pattern and asserts the
distinct `unavailable` status. Critic round 2: **ACCEPT**.

Production: `put_admission: Admission<128,64>` beside
`history_admission` in `Service`; `serve_authorized_paid_custody` routes
`Request::Put` to it. `operator_connection_allowed` still runs before
admission for every request kind; interactive (16/peer), history-page
(64/peer) and data-put (64/peer) budgets cannot borrow each other.

Tests: focused history_pages 81/81, full agentic-node lib 348/348,
fmt+clippy clean.

Native A04 rerun on `d2dee41` (Linux `20260920T000328Z-a4305016d915`):
failed earlier than the straggler site — the first authority renewal
did not reach the ordinary wallet within the scenario's 45s window
(`peerAcquiredRenewedAuthority: false`, `authorityRenewals: 0`, first
series still in flight: 8 finalizing / 2 storing / 16 custody_capacity /
17 postage_limit). The acquisition path runs on the separate
`postage_authority` protocol with its own admission, untouched by this
change; V1-C05 diffs on that path are clock-seam swaps only. Verdict:
inconclusive for A04 — rerun pending to distinguish a deterministic
regression from a load flake. **A04 remains RED.**

## Round 8 — V1-C05 event-driven wakeup starvation (`eb80303`, merged `e59ed68`)

Two native A04 attempts on `d2dee41` (`a4305016d915`, `b54f51a2b5f1`)
proved the receiver-side fix and exposed the next wall independently:

- **Zero** `refused`/`rejected` across every sample — the dedicated
  `put_admission<128,64>` ended receiver-side data-put starvation
  (baseline `e5d16721b4aa` showed up to 10 `rejected` and 2 `refused`
  jobs by sample ~5).
- But both runs froze deterministically at `rep=3` ~30 s into the first
  batch: 2–8 jobs in flight, `processingCapacity` pinned at
  `ordinaryActive: 12 / peak 16`, `executionActive: 0`, no new receipts
  for hundreds of seconds. Sender stayed `connected`; not a crash.

Root cause (two stacked defects, both in the V1-C05 event-driven
wakeup path, not the admission change):

1. `public_sender::Jobs::next_due` returned `clock::instant()` whenever
   the `ready` ring was non-empty — and `ready` is a rotation ring that
   never empties while jobs exist. Fixed in `eb69c4e` (merged
   `8c86960`): `next_due` reports only real deadlines
   (`w.due`/`reconcile_at`/`gc_at`).
2. That fix was necessary but not sufficient: other lanes legitimately
   hold a *lapsed* `next_deadline()` while work is in flight or blocked
   (outbox retries, custody sync due windows, blocked sender entries).
   The event-driven wakeup had no memory, so it re-pumped after every
   swarm event (~10 ms floor) on the same instant — full `maintain`
   passes starved the loop, responses arrived past the 10 s staleness
   window, retries collapsed the pipeline. Fixed in `eb80303` (merged
   `e59ed68`): a lapsed deadline earns exactly one out-of-tick pump per
   distinct instant (`event_wakeup_due` + `serviced` dedup); blocked
   work keeps the ordinary 100 ms tick cadence.

Test-side fallout: V1-C05 had added `cursor`/`dueInMs`/`work` to the
`custodySync` node_info surface without updating the strict key
whitelist in `tests/evm/paid_ciphertext_delivery.py::sync` — previously
unreachable because every run died earlier. Fixed in the same commit.

Coverage: `event_wakeup_fires_once_per_lapsed_deadline` (new,
critic-polished: boundary `due==now`, forward-only refire, stale
serviced record) + existing `next_deadline_reports_only_real_scheduled_work`.
macOS lib 350/350, fmt+clippy clean.

Native sender-diagnostic on Linux, exact committed source `e59ed68`:
`20260920T044536Z-c37f88db6733` — **PASS**: both originals complete
(10/10 replicas, 100 indexLocations) and cold recovery finishes; the
earlier equivalent run on the pre-helper build `bd31b4887e06` passed
identically. Compare: post-`8c86960` run `80a9d51e9c2a` failed at
`send_message` before the first original reached `progress` in 120 s.

**Status: diagnostic GREEN; full A04 (2×129) runtime verification is
running on `e59ed68` (run dir pending). A04 is not declared GREEN until
the full scenario passes.**

## Round 9 — receiver-side index-write admission (`990ce18`)

Full A04 on `e59ed68` (`20260920T045522Z-10a82aa68d9d`, prepared manifest
`4f1225dda3e8`, source `fe9cbcb6`): two complete 43-job batches finished
in 458 s each — faster than the pre-V1-C05 baseline (547/572 s) — with
**every job reaching 10/10 data replicas and 10/10 index replicas**. The
third batch died on its deadline with 3840/~4300 indexLocations: 15 jobs
`publishing`, 8 `stored`, 20 `blocked`; terminal outcomes were
`rejected`×16, `checkpoint_rejected`, `capacity`.

Analysis: `IndexPut`, `IndexLocationsPut`, `IndexLocation` and
`IndexHistoryPut` still drew the shared interactive `Admission<32,16>`
(32 requests/min globally, 16/peer). Location publication is ~100
requests per job (~4300/batch) — the dominant wire volume — so the
interactive gate starved the publish tail. `serve_index_location*` only
returns typed errors, never bare `Rejected`, so `rejected` on location
jobs is a gate verdict, not a handler refusal.

RED test `index_writes_use_a_separate_bounded_admission_budget`
(custody_history_page_tests.rs): invalid-connection rejection before
admission, interactive flood exhausts `Admission<32,16>`, `IndexPut`
then reaches the handler (`unavailable` — gate passed) instead of
`rejected`; `IndexHistoryPut` shares the same index lane; the lane is
bounded at 64/peer (65th → `rejected`); data-put and history-page lanes
are unaffected by index flood. Critic round 1 REVISE — vacuous
`IndexLocationsPut` probe removed (`operation` as hex string cannot
decode `[u8;32]`; `receipts:[]` always yields `Invalid`), `descriptor`
fixed to a JSON array, boundedness off-by-N corrected (2 tokens already
spent before the flood loop). Critic round 2: **ACCEPT**.

Production: `index_admission: Admission<128,64>` in `Service`;
`serve_authorized_paid_custody` routes `IndexPut`, `IndexLocationsPut`,
`IndexLocation`, `IndexHistoryPut` to it. `IndexHistoryPagePut` stays on
`history_admission`, `Put` on `put_admission`, everything else on the
interactive `admission`. Two existing pins legitimately flipped to
`history_stored` (the handler now stores the manifest instead of the
gate refusing it).

Tests: focused admission tests 4/4, history_pages module 82/82, full
agentic-node lib 351/351, fmt+clippy clean. Merged+pushed `990ce18`.

**Status: A04 remains RED pending a full native rerun on `990ce18`.**
