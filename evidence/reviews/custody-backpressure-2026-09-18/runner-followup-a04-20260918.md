# Runner follow-up: A04 residual after custody backpressure (2026-09-18)

Scope: next iteration against `A04` only. H11 stays out of scope (publication
130/130 already passes there; its last-original SQL fault is the separate
range-recovery/visibility issue, `custodySync.received=115` vs required 129).

Base for the next patch: `implementation/v1` containing `7e5f5fe`
(custody backpressure: unread-result retention + 500 ms per-peer dispatch
pacing + terminal-invalidation maintenance).

Evidence branch: `evidence/custody-backpressure-runner-20260918`
(traces, checks, command logs, SHA256SUMS).

## Verified improvement (keep)

- Custody transport failures: ~139,896 → 3 sample-visible occurrences (macOS
  trace `network.custodyResolution.transportFailures`), 0 on Linux.
- Stored jobs: 0 → 8/43 on macOS.
- H10 Diagnostic32 macOS r2: PASS, 32/32 originals, 96 signatures, 320+320
  replicas, 3200 location ACKs, guard clean — same acceptance signature as the
  pre-patch baseline.
- H10 r1 failed once on the last-original SQL fault via
  `custody_deferred_retry` starvation (`attempted:0, remaining:5` for ~300 s)
  inside pre-existing `custody_sync` machinery untouched by this patch —
  recorded as flaky; watch it if it recurs.

## Residual defect — A04 still RED on both platforms

Traces (immutable):
- macOS: `a04-macos/native-trace.json`
  (run `output/a04-capacity/runs/20260918T183441Z-c518886cdbda`)
- Linux: `a04-linux/native-trace.json`
  (run `output/a04-capacity/runs/20260918T190553Z-2e17c8357395`)

Context: `be8ec63` already pins the local half — the ordinary worker
re-derives the expected checkpoint for in-flight sender jobs after a mid-run
renewal (no permanent `checkpoint_rejected` locally). The architect's
suspected target was "operator-side registry refresh or sender
resolution-cache invalidation". The traces below confirm the defect lives in
that cross-node custody-resolution phase, and bound it more tightly.

### Observation 1 — 100% of unfinished jobs hold a stale checkpoint

- macOS: one authority renewal at block 66
  (`0x28ca3db9…` → `0xf801f88c…`). All 35 unfinished jobs (9 blocked +
  26 publishing) carry `resolution.assignment.checkpointId = 0x28ca3db9…`
  = `previousHead`. The 8 stored jobs show no live assignment.
- Linux: two renewals at blocks 66–67 (`0x3fbc9263…` → `0xd665b791…` →
  `0x84466f36…`). All 43 unfinished jobs carry
  `resolution.assignment.checkpointId = 0xd665b791…` — the head superseded
  by the second renewal.

No in-flight job ever rebinds to the current head. Once an authority renewal
lands, every job already holding a resolution assignment is frozen on the
superseded checkpoint for the rest of the run.

### Observation 2 — blocked subset: deterministic `checkpoint_rejected`

- macOS: 9 jobs `checkpoint_rejected` (33 per-sample sightings).
- Linux: 5 jobs blocked — 4 `checkpoint_rejected` + 1 `custody_unavailable`
  (20 per-sample sightings).

The peer correctly refuses resolution under a superseded checkpoint; the
sender never retries under the renewed head.

### Observation 3 — publishing subset: custody complete, discovery frozen

- macOS 26 / Linux 38 jobs sit at `state=publishing` with
  `delivery.phase=queued`, `postage.discovery.state=publishing`,
  `history=null`, `pointer=null`.
- All of them already hold complete custody: `receipts=10/10`,
  `indexReceipts=10/10` (data + index replica sets fully collected).
- They are not erroring — they never finalize the history-anchor
  publication, so the batch never completes.

### Observation 4 — new retention limit is exercised hard

The patch's `custody_capacity` error ("Unread custody results occupy all
retained slots") appears in 589 (macOS) / 984 (Linux) per-sample sightings —
by far the dominant runtime error after the fix. Worth checking whether
retained-but-unread results of the stalled jobs occupy the 16-slot cache and
starve later resolutions (self-reinforcing stall candidate).

`custody_unavailable`: 97 (macOS) / 126 (Linux) sightings; `postage_limit`:
46 / 37; `rejected`: 45 / 28.

## Narrow ask

1. **In-flight checkpoint rebind.** When authority renewal lands, jobs whose
   resolution assignment references a superseded head must re-resolve (or
   rebind) under the current head instead of freezing — while preserving the
   new retention/pacing bounds (16 unread slots, 500 ms/peer, 64-entry map,
   TTLs, leases, 5 s deadlines, 2 active jobs, 4 streams, 60 s windows).
2. **Publication completion.** Jobs with complete `receipts`/`indexReceipts`
   stuck in `postage.discovery.state=publishing` must either finish the
   history-anchor write or fail with a surfaced error — not hang silently in
   `queued`.
3. Keep the transport-failure collapse (139,896 → ~0) and stored-job gains.
4. Do not touch H11's range-recovery path in this iteration.
