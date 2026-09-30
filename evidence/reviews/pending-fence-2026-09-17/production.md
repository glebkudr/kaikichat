# One synchronous Core fence for pending sender operations

Task: `V1-publication-pending-fence-2026-09-17`; source cases A04/H11 and
R13/R14, regressions A03/H10, A05 excluded.

The implementation follows tests-only commit
`74049f5f85207c3a43d444ce9bd4d655441f1c81`. The user ran that exact revision on
Linux arm64 / Rust 1.91.0: six tests executed, five passed and the intended
work-bound assertion failed. The independent final critic accepted those
tests and the actual RED before behavioral production edits began. See
`red-result.json` and `critic-final.json` for immutable source/evidence hashes,
exact results and provenance limits. The accepted tests and observer remain
byte-identical to the RED revision.

## Problem and resulting behavior

Every pending sender request previously caused two additional validations of
the complete active sender queue. With eight pending payments, the measured
job-state reads in one fence were `17N + 8`: 552, 569, 739 and 2184 at N=32,
33, 43 and 128. `Runtime::command` calls this fence before and after dispatch,
including `custody_storage`. The costly work also runs on the ordinary payment
and publication paths.

Node now collects ordered `(message ID, operation)` inputs and calls
`AppCore::fence_public_sender_operations` once. Core maintains and validates
one queue, then checks every requested operation against its remaining jobs.
The healthy regression fixtures are expected to read each job state once:
32, 33, 43 and 128 reads respectively. These new totals are a source prediction
until the user's GREEN run; no latency or native acceptance is inferred from
them.

The custody response stays unchanged. Its existing bounded cleanup and stored
usage counters remain in use. The IPC deadline is still five seconds; actual
A04 execution must demonstrate that it is met.

## Authority and transaction boundaries

| Boundary | Preserved behavior |
|---|---|
| Shared queue or maintenance failure | Core returns an outer error; Node revokes every pending permit, including ordinary nonsender requests. |
| Individual invalid operation or current proof | Core returns `Denied` for that input; independently authorized requests remain usable. |
| Owner pause and agent revocation | Owner pause remains resumable; revoked agent jobs remain terminal. |
| Reservation refund | Maintenance commits atomically, then the batch loads fresh policies. A refund from nine reservations to eight can authorize the eight remaining payments in the same call. |
| Terminal job lookup | The owner API and batch share canonical lookup. Canceled jobs retain the unauthorized/retirement-time check; expired jobs retain the expiry check against the checked queue's `retired_at`. |
| Scalar preparation and authorization | State/expiry still precedes sponsorship rejection. The scalar path retains its exact full-queue membership check. |
| Dispatch authorization | Every queued item checks sponsorship, signed runtime grant where applicable, current postage context, retained operation, issue/expiry times and `require_postage_context`. |
| Transport recovery | Denial clears `verified` and revokes the existing Arc. A plain fence cannot reverify work. Reconciliation may obtain a new permit; an old revoked Arc is never set true. |

The private maintenance helper additionally returns its checked queue alongside
the existing retired count and active jobs. Its algorithm, clock rollback
check, pause decisions, refunds and SQL transaction are unchanged. Its two
existing public wrappers retain their return values.

The new decisions have no serialization or durable state. Core exposes neither
the queue nor a reusable authorization token. The policy cache is local to the
single synchronous call and begins after maintenance succeeds. Core returns
one decision per input in the same order, including repeated message IDs.
Node checks cardinality and consumes decisions only at sender entries while
walking the original pending vector backward; removals do not reassociate
decisions with other requests. Authority permits and all existing request,
hard-envelope and authority-recovery deadline handling remain in Node.

Behavioral changes are limited to `public_sender.rs`,
`public_sender_execution.rs` and `postage_client_lifecycle.rs`. Five existing
Core export files additionally expose the new enum. There are no new
dependencies, storage schemas, queues, workers, cache lifetimes or product
limits. The index-carrier cache is unchanged.

## Verification and remaining acceptance

The original RED log is preserved at evidence branch commit
`453dbc168eee13734f3cabde4b736f3446905e17`. Its two key SHA-256 hashes are:

- `check.json`: `e9dc5fdc132b7fe01efda4eadf5d345f455633efd30ba8d55992211ebd05036b`
- `command.log`: `af1affaf3d4eecb27107a6d8c7c1f6484adb43ad9f86353c32e0baf90ad45d81`

Unrelated jobs had exactly 17 reads, and pending jobs had 18. The test runtime
was 88.01 seconds, with 133.934583 seconds reported for the wrapper command
including compilation. The machine UTC start was
`2026-09-17T21:24:46.499881+00:00`. Keep those facts separate from the runner
README's rounded duration and local-date heading.

The architect's checks cover source review, immutable test/probe hashes,
unchanged product constants and oracles, the current call graph, evidence
integrity and whitespace. New Rust compilation, GREEN, formatter, Clippy,
frontend and native execution belong to the user's agreed runner workflow and
are not claimed here. `runner.md` contains the exact six-test command and the
subsequent targeted/full/native commands.

A04/H11 remain open until that execution succeeds. The published H11 failure
completed 48 originals before stalling in batch 49–64, and A04 lost three
unprepared originals to the existing reservation lease. Removing the confirmed
repeat-validation cost is the first bounded fix. If a new native trace still
shows delayed index publication after payments complete, diagnose it from that
trace without attributing all H11 behavior to this fence in advance.

The five-second IPC deadline, 60-second reservation lease, 600-second native
publication wait, TTLs, allowances, capacity limits and batching are unchanged.
H10's historical r1 trace-level comparison remains
`null/blocked-by-data-loss`; new semantic reads, admitted work and wall time
must come from the new trace. A05 remains excluded.
