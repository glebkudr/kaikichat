Fixes after the September 16, 2026 audit
=======

The change base is `implementation/v1`, commit
`89878c659533eb40e3da6f09a227704f77639ca6`. Confirmed lifecycle boundaries,
terminal evidence storage and the verification rig are fixed.
The V1 scope per `../release-scope.json` is not expanded. This document and the PR
do not close A03, A04, A05 or H11 without running their checks on the new commit.

When SQL completion fails after publication is confirmed, the Node now compares
the stored ACKs with the current history root and mailbox sequence in Core.
Progress recording uses the same check. On a match the worker repeats only
completion after checking current authority; on a binding change it returns
to the existing publisher. The immutable envelope, allocation and finalized request
are preserved. Commit job, active queue and published root remain atomic.
Files: `crates/node/src/public_sender.rs`,
`crates/core/src/public_sender_progress.rs`.

A failure to refresh current receipt/authority data no longer turns
a durable completed job into `blocked`. `public_sender_status` keeps `stored`,
durable replica counts and `error=null`; the reason for the failed refresh is
reported separately in `evidenceRefreshError`. After GC the expired/canceled status is
also read from the durable delivery projection.

GC keeps the previous `postage/sender/completed/<id>` v1 format. For expired
and canceled jobs `postage/sender/terminal/<id>` is added: state, retention,
effective reservation expiry, observation and the exact `SenderFailure`, including
the original `observed_at`. The new record is limited to 4096 bytes and contains no
renewable authority. A prepared job becomes eligible for compaction after immutable
envelope expiry; an unprepared one after the admission lease. The compact record and
the revision-fenced job deletion are committed in one transaction. Original,
allocation, policy, QC, history and recipient ACK are not deleted. The cursor changes
after a successful commit; a single SQL scan is still limited to 16 jobs.
Compact delivery facts get no new deletion deadline. A limit on the size of a single
record is not proof of bounded lifetime storage.

H11 again waits for the first import SQL fault for at most 120 seconds. The generic
`storageErrors` is replaced with an exact oracle: the test trigger ties the failure to
message ID, operation, sequence and a random nonce. The ordinary Node reports
the last 16 local SQLite import failures and the total counter through the separate
`node_info.custodyImportFailures`. The event includes conversation, number,
SQLite extended code and a SHA-256 of the error; raw SQL/error text is not exposed.
Direct and deferred imports are observed until the error is normalized. A remote
`storage_error` does not create such an event. The diagnostics live only in RAM
and are cleared on cold restart.

Each recovery stage records the timeout, actual elapsed and the headroom to
earliest original expiry, including on failure. The other H11 waits
keep the previous `max(180, count*10)`, the cold pointer pass 90 seconds.
A separate A04 scenario may only reduce the range wait to 1300 seconds;
the TTL of the original messages stays 3600 seconds.

The former A04 Core test is renamed to match its actual scope: two series of
admissions, two completed originals and component recovery. The new
`tests/evm/public_sender_capacity_a04.py` requires two series of 129 real
paid completions through the ordinary CLI, release of the active queue, preservation of
the first immutable payments, an actual loss of 9/10 replicas and sender-absent
recovery of all 258 messages with cold traversal. A read-only fixture
measures the count and size of `states.bytes` across namespaces/states, without
SQLite/WAL/page overhead. The scenario does not claim acceptance of expiry GC throughput
or a full terminal retention policy.

The build runner gets separate evidence attempts, logging of a command before it
runs, structured errors/timeouts and cleanup of the process group owned by the
command. Desktop keeps the existing build sequence,
but the deadline applies per stage instead of a single 1800-second watchdog
over the whole build. The prepare and prepared-runtime modes bind the scenario to the sources,
lockfiles, configuration and artifact contents. Instructions for the modes are in
`tests/build/README-audit.md`. A successful preparation has `prepared=true`,
but is not a PASS of runtime assertions.

Verification status of this change
-------------------------------

Regression tests were written before the corresponding production code and accepted
by a separate backend-test-critic without context inheritance. Static review
does not replace RED/GREEN, compilation or native evidence.

The PR preparation environment has no Rust toolchain and no local configuration file
for managed build storage. The permitted command

```sh
python3 scripts/build-storage.py run python3 -m unittest discover -s tests/build -p 'test_*.py'
```

stopped before running the tests with exit 1:
`Build storage: Configure .../chat-pr/.local/build-storage.json before using managed external storage.`
The wrapper and the build placement policy are preserved. New Rust/Python/native tests,
backend clippy, frontend checks, formatters and the desktop build were not run here.
Historical PASSes from another snapshot must not be transferred to this commit.

On a configured runner, run the affected suites first:

```sh
python3 scripts/build-storage.py run python3 -m unittest discover -s tests/build -p test_build_evidence.py
python3 scripts/build-storage.py run python3 -m unittest discover -s tests/evm -p test_recipient_commit_oracle.py
python3 scripts/build-storage.py run cargo test -p agentic-core --test public_postage_wallet public_message_preparation::public_sender::
python3 scripts/build-storage.py run cargo test -p agentic-node --lib runtime::public_sender::batch_tests::observation_tests::
python3 scripts/build-storage.py run cargo test -p agentic-node --lib import_failure_tests::
python3 scripts/build-storage.py run npm --prefix apps/desktop test
python3 scripts/build-storage.py run npm --prefix apps/desktop run typecheck
```

Then the native lifecycle A03, desktop verifier assertions A05,
`public_history_full130_h11.py` with the restored first deadline and
`public_sender_capacity_a04.py` on the same commit with saved manifests and
results are needed. Before merge, also run the standard backend lint/format checks.
The PR readiness criterion is passing affected checks without changing TTL,
deadlines, limits or disabling assertions for a PASS.

Still open: A06 long spend/renewal/outage, full A03 lifecycle, H11 runtime
acceptance, A04 expired-backlog/retention measurements and GC speed, A05 packaged gate,
release checks on three OSes. The cause of the previous cold rebuild/IPC stalls requires
measurements of features/env, Cargo fingerprints and stage time/I/O; this change
does not declare the disk or Cargo cache to be the established single cause.
