# Pending sender fence: tests-first contract

Task: `V1-publication-pending-fence-2026-09-17`. Source acceptance cases:
`A04`, `H11`, and the open `R13`/`R14` publication failures. Regression cases:
`A03`, `H10`. `A05` is outside this change.

Product base: `29c683030751c9b44922e6417593e49ac58e8e3a` on
`implementation/v1`. Its only change after the requested `7989ba9` base is the
user's evidence confirming the full Node lib suite at 308/308. The behavioral
production files still match `684dbb5`.
The rewritten placement test from `d021e24` is already merged and externally
confirmed GREEN in `../native-acceptance-2026-09-17/red-test-green-check.json`.
Do not reopen its scalar error-ordering contract: an already terminal job is
invalid input, and restoring sponsorship must not revive it.

## Evidence and bounded claim

The immutable trace source is commit
`1daf8fbbab25b8b798757f9a28582ee94fa3234e` on
`evidence/native-traces-20260917`. `trace-analysis.md` and
`publication-trace-analysis.json` preserve the findings and input hashes.
H11 completed the first 48 originals before failing the fourth batch (49–64).
A04 Linux also lost three unprepared originals to the existing reservation
lease. These are native failures on the original product, not executions of
the new regression tests.

The confirmed source defect is repeated validation of the complete sender
queue for each pending sender operation. `Runtime::command` invokes
`Client::fence_pending` both before and after command dispatch, including the
`custody_storage` and `public_sender_status` paths. One healthy fence currently
does one maintenance pass and two complete queue validations per pending
sender, plus one point job read per pending sender. With `N` active jobs and
`P` pending sender operations, that is `(1 + 2P)N + P` job state API reads.
This is a source-derived prediction until the RED measurement is returned.

The local regression contract bounds actual synchronous `ProfileStore::state`
reads in that shared fence. It does not impose a wall-clock unit-test threshold
or claim to count physical SQLite I/O. A native rerun must still establish
the 5-second IPC requirement and completion of the whole paid batch. The
trace correlation does not prove the shared fence is the sole cause of H11.

## Required behavior

1. With eight genuine, verified pending sender payments and 32, 33, 43 or 128
   active jobs, one fence validates each unrelated active job exactly once.
   Total job state reads are at most `N + 2P`; this permits bounded point reads
   for pending operations but forbids revalidating unrelated jobs per pending
   operation. All eight outgoing permits remain usable, and durable payment
   records remain unchanged on this healthy path.
2. Every call rechecks current persisted queue, jobs, sponsorship, signed agent
   runtime grant, retained envelope/operation and current postage authority in
   the Core writer. No verified queue, policy or authorization may survive the
   synchronous call. No caller-supplied snapshot or job substitutes for Core
   validation.
3. Pausing owner sponsorship stops all owner requests while preserving an
   independently authorized agent request. Owner pause is resumable; it must
   not cancel those jobs. Agent revocation remains terminal. Result association
   must remain correct when the revoked agent lies in the middle of the pending
   vector, and a wrong paid operation must deny only its own request.
4. An invalid unrelated active job, a queue integrity failure or failed Core
   maintenance transaction stops every previously issued pending transport
   permit. Failed SQL must not partially retire jobs, refund reservations, or
   change durable client records. A retry after removing the fault must use
   freshly validated state.
5. A previously revoked `Arc<AtomicBool>` transport permit stays false after
   state restoration or retry. Existing verified/reconcile rules govern the
   creation of any new permit. Never set an old revoked permit back to true.
6. Keep existing canonical terminal projections. Canceled jobs stop the client
   record as `Canceled/Unauthorized`; expired jobs use the existing
   `Expired/EnvelopeExpired` mapping. Missing, completed, compacted, malformed,
   or unauthorized work cannot be emitted. Per-operation denial must not
   suppress unrelated authorized work; an invalid shared queue must suppress
   all work, including pending requests that are not sender jobs.
7. Preserve hard envelope expiry, authority-only recovery, request expiry,
   maintenance clock rollback checks, original ancestry, sponsorship debit and
   refund rules, single-writer transaction boundaries and all current limits.
   Specifically, do not change the IPC 5-second deadline, the 60-second
   reservation lease, the 600-second native publication wait, job/custody
   limits, TTLs, allowances or batching.
8. At the exact persisted reservation lease, maintenance can expire an
   unprepared ninth job and reduce the owner's reserved count from nine to
   eight. With owner limit eight, the same fence must authorize the eight
   still-live prepared payments using the committed policy. It must not carry
   the pre-maintenance pause into authorization. The reservation refund changes
   neither exposed allocations nor paid request identity.

## Proposed production shape, after critic ACCEPT

Add one synchronous Core entry point taking ordered pairs of message ID and
paid operation, and returning ordered per-operation decisions. Core owns one
validated, maintained queue for the entire call. An outer error represents
shared queue/maintenance failure; item decisions distinguish authorized,
canceled, expired and denied requests.

Reuse the existing maintenance algorithm and its atomic commit. Extend its
private return value to retain the checked queue and remaining jobs; adapt the
two existing public wrappers without changing their results. Share the current
canonical terminal lookup with the owner API. Reuse scalar authorization
checks without calling the scalar full-queue API once per item.

Use a fresh policy cache after successful maintenance: a reservation refund in
that same call can change whether a remaining owner request is sponsored.
Keep the existing maintenance-time pause decisions and reservation accounting
unchanged. Perform exact envelope/operation/time/context checks for each item.
Do not change the scalar `prepare_public_sender_job` error precedence.

Node collects the pending sender inputs once and applies Core decisions to the
original pending indexes while retaining the current reverse removal order,
authority permit check, deadlines, verified flags and old-permit revocation.
The command API and `custody_storage` response schema do not need to change:
the custody aggregate already uses bounded cleanup and stored usage counters.
No index-cache edit is part of this first patch.

## Helpers and production entry points for the independent critic

Changed tests: `crates/node/src/postage_client_fence_tests.rs`, registered only
under `cfg(test)` in `crates/node/src/postage_client.rs`.

Existing realistic helpers:

- `crates/node/src/custody_history_paid_batch_tests.rs`: `Fixture::queued`,
  `Fixture::activate`, `Fixture::core_sql`, and the real payment request portion
  of `finalize_sender_payments`. The fixture starts eight paid requests; extra
  active jobs use ordinary `AppCore::send_message`.
- `crates/node/src/public_sender_batch_tests.rs`: existing runtime fixture and
  sender batch behavior.
- `crates/node/src/postage_client_maintenance_tests.rs` and
  `crates/node/src/postage_client_renewal_tests.rs`: retained funding/finality,
  expiry and authority recovery behavior.
- `crates/store/src/test_probes.rs`: synchronous thread-local read observer,
  enabled by Node's dev-dependency feature only. It counts actual API accesses,
  including failed/missing reads, and changes no data or authorization. The
  observation ends before fixture/oracle reads; it cannot cross an await.

Production entry points:

- `crates/node/src/runtime.rs`: `Runtime::command`, dispatch fence placement.
- `crates/node/src/postage_client_lifecycle.rs`: `Client::fence_pending`,
  `Client::stop_pending`, `Client::request_deadline`.
- `crates/node/src/postage_client.rs`: `Client::request`, `outgoing`, pending
  verified state, immutable retained request and transport permit.
- `crates/core/src/public_sender.rs`: `sender_queue`, `load_sender_job`,
  `public_sender_job`, `maintain_sender_queue`, sponsorship and retirement.
- `crates/core/src/public_sender_execution.rs`: scalar sender authorization,
  retained envelope binding and current postage authority.
- `crates/node/src/paid_custody.rs` and
  `crates/postage-spend/src/custody.rs`: existing bounded custody summary.

## Execution and review boundary

The architect supplies static analysis, test source, independent criticism and
patches. The user's existing macOS native / Linux arm64 portable runners execute
Rust and native cases. No host access is required or requested.

First publish the tests-only revision and run its exact nonzero test selector
through `scripts/build-storage.py`. Preserve its command log, `check.json`,
test counts, exit code and emitted read measurements. An assertion failure
in the work-bound test is the expected RED; a compile failure, fixture failure
or empty selector is not RED evidence for this defect. The other permit/fault
tests should pass on the baseline.

Give the independent critic (`fork_turns: none`) these requirements, the tests,
helpers, production entry points, native failure evidence and the returned
new RED result. Final ACCEPT is required before behavioral production edits.
Then run the identical tests for GREEN, required formatter/Clippy/frontend
checks and the full Rust suite, followed by A03/A04/H10/H11 on the agreed native
matrix. Keep fresh prepared artifacts and source identities for that revision.
H10's lost r1 trace remains `null/blocked-by-data-loss`; measure the new run's
semantic reads, admitted work and wall time from its own trace.
