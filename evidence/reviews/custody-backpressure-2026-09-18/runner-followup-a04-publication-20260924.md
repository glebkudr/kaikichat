# A04 publication speed follow-up (2026-09-24)

All runs below use the unchanged A04 scenario through a diagnostic wrapper that
only redirected the output directory so two attempts could run concurrently
(now `AIN_A04_OUT`, 95753d0). They are diagnostic samples, not acceptance
evidence. Two attempts ran concurrently in
every pair. Metrics come from each run's `trace.json` (`completedBatches`,
17 s `samples` of sender jobs and `network.custodyResolution`).

## Fix 4 (seed route announce, branch fix/v1-bootstrap-route-announce-20260924)

| Run | Result | first | last | check.json sha256 |
|---|---|---|---|---|
| par1 `20260924T102442Z-358d088d5902` | PASS | 71.4 s | 1052.7 s | `d0bdd88bef3f39f0e79b6c55863e71a3446c495ab7ae37b5811432695fce7fcc` |
| par2 `20260924T102501Z-185fd931db1c` | PASS | 4.2 s | 997.4 s | `b9247689f71fe2c90f4385c3ce3131ea49b7f97df8611f8f24016cf1f174b53b` |

The publication stall seen once on `f23fdcf` did not recur (2/2 publication complete).
With fix 4 in the combined branch (c2r1/c2r2 below) the first stage took 3.6 s and
2.9 s; overall 4/4 PASS with fix 4, first stage 71.4 / 4.2 / 3.6 / 2.9 s (was
61–64 s or a 120 s FAIL).

## Parallel attempts

Four pairs of A04 attempts ran concurrently from separate worktrees (short
`A04_OUT` paths). Each attempt still took ~52–57 min, about the time of a single
sequential run, so two concurrent attempts halve wall time per attempt; per-batch
publication slowed ~5–10%. `AIN_A04_OUT` (95753d0) makes the output directory a
supported option of the unchanged `public_sender_capacity_a04.py`; keep the path
short (node IPC sockets live below it), e.g.
`AIN_A04_OUT=/Volumes/ChatBuild/output/a04-p1`.

## Sender capacity wake (branch fix/v1-sender-capacity-wake-20260924, 49d622c + e24786e, not merged)

| Run | Result | batch deltas (s) | mean | blocked custody_capacity |
|---|---|---|---|---|
| sw1 `20260924T111052Z-a56c26288e89` | PASS (63.3 / 1175.4 s) | 274 292 306 304 328 | 301 | 34% |
| sw2 `20260924T111111Z-4bd4e08276eb` | PASS (70.7 / 1053.9 s) | 276 293 317 300 330 | 303 | 34% |
| baseline par1/par2 (same concurrency) | | | 315 / 297 | 37% / 36% |
| baseline sequential runs 21d1328…f23fdcf | | | 278–332 | 36–39% |

No measurable speedup. The earlier "idle slot" estimate counted jobs in state
`resolving`, but a job whose resolution is pending shows `storing`/`publishing`;
the resolver's own `active` shows both slots busy in 71% of samples with
capacity waiters.

## Resolver time (sw1, same shape in all runs)

~730 resolutions per run; median 0.6 s. 48 resolutions (7%) of 5–60 s hold 64%
of resolver slot-time. Those issue 2075 queries, 78% `declined` (queried peer is
not the operator of that ordinal); fast ones 4%. The slow ones cluster at each
batch start (hint-less linear scans, 94–126 queries) and a few mid-batch.

## Route hints (branch fix/v1-custody-route-hints-20260924, 41fedab + 73ef6d4)

| Run | Result | batch mean | slow (>5 s) | declined |
|---|---|---|---|---|
| rh1 `20260924T123722Z-f3246fa5abc4` | PASS (71.3 / 1052.7 s) | 318 | 47 | 1116 |
| rh2 `20260924T123742Z-fb1ce3f880ba` | FAIL first stage 120.1 s (no fix 4 on this branch) | 313 | 34 | 1611 |

No effect: batch-start resolutions still carry no routing start (0/14).

Diagnostic run rd1 `20260924T133741Z-5acfc3c32cda` (branch
diag/v1-route-hints-probe-20260924, route counters in `custodyResolution`;
FAIL first stage, no fix 4; check.json sha256
`f2bb1093f6f34cda4afa98489ea221e177631465d19ff073dd4b475ba2e36d1f`): routes were
recorded and hit mid-batch (1266 hits), but at every batch start (471/763/1055/1347 s)
28–34 lookups found only routes whose offer `valid_until` had passed during the
idle gap, and after each checkpoint renewal (213/823/1433 s) lookups missed on the
new checkpoint id with the same registry domain/epoch.

Revised contract (97e1f86 + 9bb7ba7): a route is keyed by registry domain, epoch and
ordinal, ignores the old offer's expiry, survives checkpoint renewals; still only a
scan start (fresh offer fetched and verified), ≤64 entries, cleared on network
replacement. Critic ACCEPT after two rounds (expired offer placed well before the
new job; `<=`/`<` expiry variants, checkpoint-in-key and ordinal-only keys fail).

Combined fix 4 + revised routes (integ/v1-route-hints-announce-20260924 = 01d63a6):

| Run | Result | first | last | batch mean | declined | cold scans | check.json sha256 |
|---|---|---|---|---|---|---|---|
| c2r1 `20260924T143100Z-2b26ed6beb04` | PASS | 3.6 s | 1113.0 s | 334 s | 272 | 2 (45 s) | `9ea9c8cd631fc3f477e1c29b6e23ae42d4d6dabd5644292686c52f43c8a7d746` |
| c2r2 `20260924T143120Z-e47a79ad7947` | PASS | 2.9 s | 1044.1 s | 335 s | 264 | 2 (50 s) | `a7bd64ce4bd1ead722c8d9acecc582287da0b05b83902524a6d89ed9b6219f41` |
| rh1 (first route fix) | | | | 318 s | 1116 | 12 (446 s) | |
| sw1 (no routes) | | | | 301 s | 1821 | 21 (566 s) | |

Routes remove ~80% of declined queries and almost all hint-less scans, but resolver
slot-time stays ~1700 s and batches are not faster (c2 runs overlapped a 450 s
test suite and a release build during their first batches). The remaining resolver
time is response latency from correctly addressed custodians, not wasted queries.

## Recovery reads (traced combo = fix 4 + capacity wake, trc1 `20260924T114027Z-eb38394d458b`)

583 recipient reads in 1110 s: range 285 (all accepted), index 170 (102
rejected), locations 69, history_page 59 (31 unavailable). Index reads walk each
reference's declared index holders in a fixed order; which holder still has the
entry shows no locality across references, so reordering by the last accepting
holder saves nothing in this trace.

## Merged

Fix 4 (18474f2 + 600f2d3), revised route hints (41fedab, 73ef6d4, 97e1f86,
9bb7ba7) and `AIN_A04_OUT` (95753d0). The capacity wake is not merged. Next
bottleneck: custodian proof-query response latency during publication (resolution
p90 ~3 s, 5–30 s tail with correctly routed peers).
