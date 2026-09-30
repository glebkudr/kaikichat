# Published native failures: trace extraction

Source: private development archive (published source: `glebkudr/kaikichat`), trace revision `1daf8fbbab25b8b798757f9a28582ee94fa3234e`, product base `684dbb5a3a800fa4c380db221b26b2b8722e4260`. All three trace blobs and all three check blobs were fetched and verified against Git blob SHA-1; SHA-256 and byte lengths are retained in `publication-trace-analysis.json`.

## H11 is the fourth batch, not the first

`h11-macos-failed/trace.json#/completedBatches` contains successful completion of 16 originals at `1789669948`, 32 at `1789670202`, and 48 at `1789670641`. Its final 16 envelopes have sequences **49–64**. `check.json` names the reported run `20260917T182847Z-c08b82eb371c`. Therefore this is the same requested run and it reached 48 stored originals before the failed wait.

The sampled batch boundaries below are inferred from replica counters returning to zero after each completion. The last three completion records and final envelope sequence range independently establish the batch count.

| Batch | Sample indexes, inclusive | Maximum duration of 16 status calls | First sampled nonzero data replicas, seconds after that batch's first sample | Last sampled data / index / location totals |
|---|---|---:|---:|---|
| 1, originals 1–16 | 0–18 | 9.164 | 23.945 | 160 / 160 / 1600 |
| 2, originals 17–32 | 19–40 | 17.881 | 45.493 | 160 / 160 / 1600 |
| 3, originals 33–48 | 41–68 | 31.462 | 230.144 | 160 / 160 / 1600 |
| 4, originals 49–64 | 69–99 | 30.987 | 464.165 | 160 / 149 / 680 |

`tests/evm/public_index_sender.py::views` measures `statusSeconds` around a sequential sweep across the batch. It is **not one IPC latency**. `samples[].at` is the beginning of that sweep; the network snapshot is taken afterward. These are not atomic simultaneous snapshots.

In batch four, samples 69–87 have zero data replicas. Most retain eight `finalizing` jobs, with `postage_limit`, `custody_capacity`, `storing`, and `resolving` on the others. The last such sample takes 5.833 seconds for its 16 status calls. Sample 88 first observes 24 data replicas and takes only 0.208 seconds. The data count then reaches 160. The final snapshot has all 16 payments finalized, 160 data replicas, 149 index replicas and 680 location ACKs; none of this batch is `stored`. This is severely delayed progress followed by incomplete index publication within the fixed 600-second wait, rather than a permanent zero-replica state across the whole H11 run.

At final failure `postageClient.pending=0`, `networkPending=0`, `sent=173`, `failedResultCommits=0`, `rejectedResults=0`. These are final snapshots, not pending counts for previous samples. Authority synchronization is current after three accepted renewals, with zero failed or rejected renewals. Each final plan has `validUntil=1789672952`, whereas final durability observations range from `1789671194` to `1789671212`; envelope expirations range from `1789674242` to `1789674255`. There is no sampled `unauthorized`, cancellation, or expiry in H11. The trace therefore does not support an owner-pause or expired-authority explanation for the final failed batch.

The sender remains connected to 18 peers in every H11 sample. At failure there are 3,480 custody-resolution transport failures and 3,517 processing rate rejections. The failed run has **22 runtime log files, all zero bytes**. These counters do not provide the failed request IDs, response dispositions, transport permit histories, SQL counts or fence timings required for stronger causal attribution.

## A04 includes reservation loss, not only a slow read

The Linux run has four sweeps of 43 jobs taking 123.095, 129.811, 120.773 and 109.928 seconds. Every sample has zero data/index/location counts. Its final funding projection has 13 finalized, 27 allocated and **three reserved originals already terminal `expired/reservation_expired`**. Those three have `prepared=null`, no paid operation and `funding.refundable=true`. The first 40 prepared envelopes span `issuedAt=1789658939` through `1789659019`; the remaining jobs did not prepare before the existing reservation lease. `RESERVATION_LEASE_SECONDS` is 60 in `crates/core/src/public_sender.rs`.

The Linux final sender has eight pending payments; its terminal check failure is `daemon closed response early`. The macOS A04 check fails with `TimeoutError: timed out`; its trace has no samples or lastSender, but the failure snapshot has eight pending payments and three network-pending requests. Neither failed A04 check includes a per-command timing or command name, so the supplied user observation identifying `custody_storage` is distinct from what these JSON snapshots alone establish. Both runs have 22 empty runtime logs.

## What this supports, and a test that can falsify it

`Runtime::command` runs `fence_pending` before and after dispatch. Current `fence_pending` performs queue maintenance, then uses `public_sender_job` and `authorize_public_sender_operation` separately per pending sender. Both validate the entire active queue again. This static repeated work and the trace's abrupt status-cost collapse when finalizing work drains make the shared Core fence the first bounded change to test. The trace has no profiler spans, so it does not establish what fraction of the wait is in that code or guarantee the proposed patch closes H11.

Do not attribute H11 to the first 129th index carrier. Its successful first batch already accumulates 160 distinct index promises. The whole-cache-clear policy can still amplify repeated validation, but the new failure begins in a phase with no fourth-batch index publication. A cache edit by itself is not justified by these traces.

The strongest short falsification run is the same native H11 with only the approved single-pass fence change, unchanged batching, allowances, TTL and deadlines. Compare the fourth batch's zero-data interval and `statusSeconds` with the preserved baseline. The same run also shows whether a residual index-publication delay survives after pending payment work becomes cheap. A04 must additionally demonstrate all 43 originals prepare before the unchanged 60-second reservation lease; otherwise a later recovery of network progress cannot restore the already expired original jobs.

For deterministic RED coverage, combine a 32/33/43-active-job boundary with eight real pending sender operations and a same-conversation sequence retaining 48 completed originals before another 16. Count actual SQL work around the ordinary read/fence entry point; preserve stop-on-SQL-error, cancellation, grant and authority fences. Test the 60-second preparation boundary and unchanged 5-second real IPC separately on the runner; the sampled aggregate sweeps are not substitutes for either assertion.

Reproduce this static extraction:

```sh
python3 extract_publication_traces.py /path/to/materialized/native-traces --output publication-trace-analysis.json
```

This analysis changed no product or test source and executed no Rust/native acceptance test.
