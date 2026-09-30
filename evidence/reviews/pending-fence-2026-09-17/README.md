# Pending sender fence: implementation and execution evidence

Status: **implemented; GREEN and native acceptance pending on the user runner**.
The tests-only revision `74049f5` executed on Linux arm64 / Rust 1.91.0 with
five passing controls and the intended work-bound RED. Its verified result is
in [red-result.json](red-result.json); [critic-final.json](critic-final.json)
records final independent ACCEPT before production changes began.

The current revision contains the common Core maintenance/authorization pass.
Read [production.md](production.md) for the change and verification limits,
[production-review.json](production-review.json) for independent static
approval, and [runner.md](runner.md) for the exact next GREEN/full/native
commands. The accepted tests and observer remain byte-identical to `74049f5`.
New production compilation and runtime success are not yet claimed.

The remainder of this README preserves the original tests-first packet and
historical RED request. Use `runner.md` for the current production candidate.

Base: `29c683030751c9b44922e6417593e49ac58e8e3a` (`implementation/v1`).
This adds only the user's 308/308 Node lib confirmation to the requested
`7989ba9` base; production behavior is unchanged.
Working branch: `fix/v1-pending-fence-20260917`.
Trace source: `evidence/native-traces-20260917`, exact commit
`1daf8fbbab25b8b798757f9a28582ee94fa3234e`.

Read [requirements.md](requirements.md) for the production contract, helper
inventory, mutable authority boundaries and the mandatory independent critic
gate. [trace-analysis.md](trace-analysis.md) explains the native observations
and their limits. [publication-trace-analysis.json](publication-trace-analysis.json)
contains the six verified input hashes, all 104 sampled observations and exact
binary/source identities from the original failed runs. [index.json](index.json)
records this packet's status, expected assertions and file hashes.

The current native record establishes that H11 completed originals 1–48 and
failed the fourth batch, 49–64. The first nonzero data replica sample in that
batch arrived about 464 seconds after its first sample. Final progress was
160 data replicas, 149 index replicas and 680 location ACKs, with zero stored
jobs in that batch. The status sweep cost falls sharply when finalizing work
drains. A04 Linux additionally lost three unprepared jobs to the existing
reservation lease. No TTL, deadline, allowance, limit or batching change is
proposed.

## Historical RED runner request, executed on 74049f5

Run at this tests-only revision through the existing wrapper. One suite
selector executes **six tests**. On the unchanged production baseline the
intended result is **one work-bound assertion failure and five passing
permit/fault controls**, with all four measurement records printed before the
work-bound assertion. A compile or fixture failure is not the requested RED.

Linux arm64 portable:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-red-20260917-r1 run cargo +1.91.0 test -p agentic-node --lib runtime::postage_client::fence_tests:: --locked -- --nocapture --test-threads=1
```

macOS native:

```sh
python3 scripts/build-storage.py --output output/pending-fence-red-20260917-r1 run cargo +1.91.0 test -p agentic-node --lib runtime::postage_client::fence_tests:: --locked -- --nocapture --test-threads=1
```

Use a fresh output directory for a retry. One platform is sufficient for this
deterministic RED gate. Return the executed Git revision, wrapper `check.json`,
complete `command.log` and any compiler diagnostics in an evidence branch.
The four JSON records carry `evidence: "pending_fence_state_reads"`; keep them
with the test summary, rather than reporting only the final exit status.
No native rerun is needed before this gate.

Expected measurements, derived from the current source (not yet measured):

| Active jobs | Pending payments | Predicted baseline job state reads | Required maximum | Every unrelated job |
|---:|---:|---:|---:|---:|
| 32 | 8 | 552 | 48 | 1 read |
| 33 | 8 | 569 | 49 | 1 read |
| 43 | 8 | 739 | 59 | 1 read |
| 128 | 8 | 2184 | 144 | 1 read |

The baseline is expected to read each unrelated job 17 times. The test does
not assert the exact baseline total: it asserts the allowed work and real
permit/durable-state behavior. The observer measures actual
`ProfileStore::state` API reads, including missing/error reads; it does not
measure physical SQLite I/O or IPC wall time.

The test symbols share `runtime::postage_client::fence_tests::`:

- `pending_fence_reads_each_nonpending_job_once_at_32_33_43_and_128_jobs`
- `pending_fence_pauses_seven_owner_requests_without_stopping_the_agent`
- `pending_fence_keeps_operation_and_revoked_agent_decisions_with_their_request_ids`
- `pending_fence_corrupt_unrelated_job_stops_all_old_transport_and_recovers_with_new_permits`
- `pending_fence_retirement_sql_failure_stops_every_old_permit_and_retry_retires_only_agent`
- `pending_fence_uses_the_committed_refund_policy_at_the_reservation_lease`

The independent preflight returned four concrete REVISE findings, preserved in
[critic-preflight.json](critic-preflight.json). The leader added mixed sender /
nonsender global-failure coverage, plain-fence recovery boundaries, current
policy proof corruption and exact-lease refund coverage. Those corrections
received a fresh independent
[STATIC_READY source review](critic-static-rereview.json). Final ACCEPT still
requires review of the actual returned RED; a static preflight is not that
final gate.

## Original follow-up contract, now at the implementation stage

The independent critic receives the exact tests, requirements, helpers,
production entry points and returned RED evidence. Production is gated on
final ACCEPT. The proposed change is one Core-owned synchronous maintenance
and authorization pass for all pending sender operations, preserving existing
per-request authority and transport revocation semantics.

After that patch, rerun the same six tests for GREEN, existing lifecycle and
renewal controls, formatter/Clippy/frontend checks, and the complete Rust suite.
Run A03/A04/H10/H11 with fresh prepared artifacts for the patched source on
the agreed Linux arm64 / macOS native matrix. Native acceptance must retain
the real 5-second IPC deadline, prepare every A04 original before its own
reservation lease expires, complete every paid batch, and measure H10 semantic
reads/admitted work/wall time from the new trace. Historical r1 trace comparison
remains `null/blocked-by-data-loss`.

The prior test #3 is already GREEN in
[`red-test-green-check.json`](../native-acceptance-2026-09-17/red-test-green-check.json).
Prior A03/H10 PASS evidence stays historical; it does not certify the next
production revision. A04/H11 remain open until the new native runs pass.

## Reproducing the static trace extraction

Materialize the six trace/check inputs at the paths specified in
`extract_publication_traces.py`, preserving their exact published bytes. The
extractor verifies each Git blob SHA before reading its fields. For Linux:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-trace-extraction-r1 run python3 evidence/reviews/pending-fence-2026-09-17/extract_publication_traces.py /path/to/materialized/native-traces --output output/pending-fence-trace-extraction-r1/reproduced.json
```

The architect executed this extraction successfully; the reproduced JSON
matches the committed extraction byte for byte. This is a static evidence
check, with zero product processes or Rust tests executed.
