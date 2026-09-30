# Publication triage on 684dbb5 — execution blocked

This is a partial, tests-only result for the requested IPC, publication-liveness
and remaining RED-test work. It is **not** acceptance of A03, A04, H10, H11 or
the full Rust suite. No production file was changed. The previous native
[acceptance evidence](../native-acceptance-2026-09-17/README.md) remains historical.

The source base is `684dbb5a3a800fa4c380db221b26b2b8722e4260`. The current host is
Ubuntu 24.04.3 x86_64, glibc 2.39. The exact base commit/tree and 1,277 build,
test and relevant evidence files were restored through the authorized GitHub
connector and verified against their Git blob hashes; unrelated historical
documents/evidence are sparse. Ordinary HTTPS clone lacked private-repository
credentials. No credentials were read or copied.

## Correct the remaining RED scenario

The user-reported RED at
`runtime::public_sender::batch_tests::placement_tests::a_ready_agent_outside_the_group_progresses_after_owner_sponsorship_is_paused`
is a mismatch in the test scenario, not evidence for reversing the production
authorization checks.

The old test sets the agent budget to zero and runs ordinary maintenance.
`AppCore::maintain_sender_queue` makes a denied agent job terminal `canceled`;
only an owner pause is resumable. A subsequent `prepare_public_sender_job`
therefore correctly returns `InvalidInput` for that canceled job. After raising
the agent budget, the old test also expects that same operation to resume,
contradicting the existing A03 cancellation/no-revival contract. Merely moving
the sponsorship check before the state check does not fix the second assertion.

The changed test instead creates 12 owner jobs and one continuously authorized
agent job. It uses real signed paid fixtures and ordinary maintenance, proves
that the exact 12 owners occupy the placement cohort and that the agent is
outside it, then pauses only the owner. It requires owner `Unauthorized`, agent
authorization, a signed root containing exactly the agent operation within the
existing `2 * BLOCKED_RETRY` bound, 13 still-queued jobs, and no fabricated mailbox
completion. The existing A03 canceled-agent no-revival test is unchanged.

The independent critic verdict is recorded in `critic.json`. Its static review
does not supply a compiler result, executed test count or native acceptance.
Both local focused Cargo attempts failed to spawn because Cargo is absent.

## IPC hot path

`custody_storage` already returns five aggregate fields. Its normal v2 storage
path reads usage counters and performs a bounded expiry pass (`CLEAN_BATCH=2`).
There is no sender-job list to paginate in this response.

`Runtime::command` invokes `postage_client.fence_pending` both before and after
dispatch, including read-only `custody_storage` and `public_sender_status`.
Each fence performs one full active-queue maintenance pass. Each queued pending
sender then performs another full queue validation in `public_sender_job` and
another in `authorize_public_sender_operation`.

For J active jobs and P queued, authorized pending sender payments, the source
call graph permits approximately `(1 + 2P) * J` job-row visits per fence, with
P at most 8. Two command fences at J=43, P=8 yield up to 1,462 visits before
counting the handler. This is an analytical call-path count, **not** a measured
SQL-statement count or a runtime duration. Reconcile/read/prepare paths repeat
the same work in the synchronous runtime that also services the swarm and IPC.
The 20ms sender pass budget does not preempt an individual expensive operation
or bound its surrounding reconciliation/fences.

The candidate to test next is one synchronous Core fence that validates one
current queue and evaluates all pending operation bindings against it. It must
retain current grants, sponsorship, immutable expiry and operation checks;
revoke every denied physical permit even on SQL failure; and retain no granting
snapshot across calls. This candidate has **not** been implemented or accepted.
Required regression coverage includes 33/43/128 jobs, all eight pending permits,
owner pause, agent revoke, SQL refusal, and the actual owner command/Unix IPC
path under continuing publication. Native timing remains necessary.

## Publication-liveness remains unresolved

A04 uses six batches of 43 to complete two series of 129. H11 uses nine
sequential batches of at most 16, like H10's two batches of 16. Thus an active
queue optimization alone cannot be assumed to fix H11.

The committed failed checks do not include the full publication samples or
failed-batch ordinal; their listed runtime log files have zero bytes. The
user's full `output/...` traces are unavailable in this session. No trace-level
root cause for H11 was established, and no physical-slot leak was proven.

A separate source-level candidate is the immutable verification cache in
`crates/postage-spend/src/custody/outgoing/index.rs`: the 129th distinct carrier
clears a 128-entry cache. Repeated validation of more carriers can thrash it.
Its contribution to H11 is unmeasured. The cache bound was not changed.

Next trace inspection must identify the failed batch, current/previous job
states, pending requests and response handling, then measure command/pump/fence
time and queue validations. Current full Rust and native acceptance remains
pending; historical PASS reports are not reused as validation of this test diff.

## Actual environment results

- `native-doctor-check.json`: runtime preflight `blocked`; Anvil/Cast absent;
  `socket(AF_UNIX, SOCK_STREAM)` fails with `EPERM` at socket creation. Cleanup
  of the probe succeeded. This is an environment restriction, not a product RED.
- `red-existing-check.json` and `corrected-test-check.json`: Cargo `spawn_error`;
  no test executable was started, no RED/PASS assertion or nonzero count exists.
- `frontend-doctor-check.json`: Node 24.19.0 fails the required >=26.0.0 check;
  Vitest/jsdom are absent. No frontend result is claimed.
- `diff-check.json`: `git diff --check` completed with exit code 0. This is only
  a diff whitespace check.
- The exact toolkit-252-006 files were found, but four standard file-delivery
  attempts returned HTTP 502. The payload was neither obtained nor executed.
  No existing GitHub Actions workflow/run or accessible remote test host was
  available. `environment.json` preserves the diagnostic facts without secrets.

Run the focused test and existing cancellation guard through the wrapper on
the user's accessible native host before making production changes. On Linux:

```sh
python3 scripts/build-storage.py --profile portable-linux run cargo test --locked -p agentic-node --lib runtime::public_sender::batch_tests::placement_tests::a_ready_agent_outside_the_group_progresses_after_owner_sponsorship_is_paused -- --exact --nocapture
python3 scripts/build-storage.py --profile portable-linux run cargo test --locked -p agentic-node --lib runtime::public_sender::batch_tests::a03_canceled_agent_job_cannot_revive_after_restoration_renewal_or_restart -- --exact --nocapture
```

Check that each selector executes exactly one test. Production regression tests
for the IPC/liveness fixes still require RED evidence and a fresh independent
critic gate before implementation. Rebuild prepared manifests after any source
or HEAD change, then perform the required native scenarios without changing
their deadlines, TTLs, allowances, retention or acceptance oracles. A05 remains
outside this task; R1 trace-level H10 comparisons remain blocked by data loss.
