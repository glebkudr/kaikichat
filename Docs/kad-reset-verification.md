# Verifying Kad-substream release after Reset

Status: the fix and regression tests are prepared. In this task, tests, builds,
and native scenarios were **not run** per the user's instruction. No result
below is claimed as PASS yet.

## What was fixed

The local patch of `vendor/libp2p-kad` keeps version 0.48.0 and is wired
through the root `[patch.crates-io]`. On close, the `WaitingBehaviour` handler
moves the stream to `Closing` and then calls the saved waker. This lets
`SelectAll` finish closing and release the inbound slot after `HandlerIn::Reset`.

The wire protocol, query/substream timeout, parallelism, admission, and the
32-stream limit are unchanged. There is no new pacing in this patch: the
slot-leak fix is verified first. Previous changes in the working tree are
preserved.

The regression test `kad_resets_release_slots_and_same_connection_recovers_after_admission_window`
uses real TCP/Noise/Yamux connections, signed service records, and ordinary
admission. A test observer delays Reset until `Poll::Pending` so that the
inbound stream has already saved its waker. It verifies 128 successful reads,
33 denials, another client's access, and read/PUT recovery on the same
connection after the admission window refreshes. Controlled clocks advance only
the project's clock seam; libp2p timers and system time are not faked.

## 1. Storage and dependency selection

Run all commands from the main checkout. If the result directory already
exists, pick a new attempt suffix; do not overwrite previous evidence.

```sh
cd /Users/glebk/Code/chat
python3 scripts/build-storage.py check
python3 scripts/build-storage.py run cargo tree --locked -p agentic-node -i libp2p-kad
```

The tree must show `libp2p-kad v0.48.0` with the path
`/Users/glebk/Code/chat/vendor/libp2p-kad`. If Cargo demands a lockfile change,
stop and inspect the local patch/lock; do not update the whole dependency
graph. If DiskManagement is denied inside the sandbox, retry the wrapper
through the standard access escalation. Do not bypass the wrapper and do not
move target to the internal disk.

## 2. Targeted tests first

```sh
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib kad_resets_release_slots_and_same_connection_recovers_after_admission_window -- --nocapture
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib query_error -- --nocapture
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib service_query_tests::
python3 scripts/build-storage.py run npm --prefix apps/desktop test -- tests/network-panel.test.tsx tests/wallet-network-panel.test.tsx tests/desktop-api.test.ts
```

Verify that the regression test filter ran **one** test, not zero. The test must
pass without a reconnect: preserving the ConnectionId is what is checked. One
seed confirms the PUT but still does not satisfy the two-copy quorum. The
`query_error` tests check the TimedOut/UnexpectedEof classification and error
propagation into the inner query without losing pending-request resolution.

To check regression sensitivity, in a separate checkout you may temporarily
remove only the `waker.wake()` call from the new `WaitingBehaviour` branch and
rerun the first command: FAIL is expected. Then restore the call and get a
PASS. Do this check before preparing native artifacts; do not touch the main
checkout while other tasks work in parallel. Place the verification worktree
only in `.local/verification-workspaces` per AGENTS.md.

## 3. Rebuild and run the full A04

Prepare a fresh set of binaries after all source changes. Old prepared
manifests contain different source identity and do not fit. Do not change
sources during preparation and the native run.

```sh
python3 scripts/build-storage.py run python3 tests/evm/native_prepared.py prepare --output output/kad-reset-prepare-r1
python3 scripts/build-storage.py run env -u AIN_NATIVE_PREPARE_ONLY AIN_NATIVE_PREPARED_MANIFEST=/Users/glebk/Code/chat/output/kad-reset-prepare-r1/release/prepared-artifacts.json python3 tests/evm/public_sender_capacity_a04.py
```

Preparation by itself is not acceptance. A04 must end with a regular successful
`check.json` in a new `output/a04-capacity/runs/` directory: two series of 129
and 258 completed originals and recovery of all 258 without the sender. Check
send and recovery; `resPutRecordRes` growth recovering on its own is not
enough. Do not change production timeouts, message counts, lease, or
parallelism.

For the same full A04 with external `node_info` polling instead of the last
command, the existing diagnostic wrapper can be used:

```sh
python3 scripts/build-storage.py run mkdir -p output/a04-capacity/diag
python3 scripts/build-storage.py run env -u AIN_NATIVE_PREPARE_ONLY AIN_NATIVE_PREPARED_MANIFEST=/Users/glebk/Code/chat/output/kad-reset-prepare-r1/release/prepared-artifacts.json python3 tests/evm/diag_a04_instrumented.py
```

The wrapper appends to `output/a04-capacity/diag/a04-probe.jsonl`; it does not
separate attempts. Analyze only the time interval of the new `runs/<id>`,
cross-checking node PeerIds. Do not mix counters from different processes.
`node_info` uses local Unix IPC, not Kad/yamux.

Watch `routing.kadEv.qErrKinds`, `qErrTopPeers`, `sentPutRecord`,
`resPutRecordRes`, `inFlightReq`, `outboundHeld`, `heldDropped`, and
`admitDropped` on alice. On both seeds watch `kadEv.getRecordReq/putRecordReq`,
**`lookupRejections`**, `recordRejections`, and `repliesDropped`.
`getRecord.admission` is stored precisely in `lookupRejections`. The value 128
in the admission snapshot may be stale: the window refreshes only on the next
`allow()`.

Zero errors are not required: an admission denial remains acceptable. The fix
criterion is that denials do not cause the handler to stop forever on 32 Resets
— once the budget frees up, replies resume and the scenario completes. If the
new A04 never exceeded 32 denials per connection at all, it confirms
application success, but the Reset stress specifically is what the target
regression test covers.

## 4. H11 and finishing verification

After a successful A04, run the unchanged Full130:

```sh
python3 scripts/build-storage.py run env -u AIN_NATIVE_PREPARE_ONLY AIN_NATIVE_PREPARED_MANIFEST=/Users/glebk/Code/chat/output/kad-reset-prepare-r1/release/prepared-artifacts.json python3 tests/evm/public_history_range.py
```

Regular PASS results, 130 simultaneously live originals, and full cold recovery
after the prescribed losses are needed. Evidence is in `output/history-range/runs/`.
If sources or artifacts changed after preparation, prepare a new manifest.

Only at the end of the overall plan, run the full repository-prescribed set:

```sh
python3 scripts/build-storage.py run scripts/check.sh
```

## If the problem persists

1. The regression fails: check the chosen vendor path and the first test error.
   Do not raise the limit of 32 or weaken assertions for a green result.
2. The regression passes but A04 stalls again: save the new check.json,
   trace.json, runtime-logs, and the corresponding probe interval; correlate
   qErrKinds and peer. `UnexpectedEof` is consistent with a stream close/reset
   but does not prove the source. `ConnectionRefused` requires checking
   negotiation/protocol support; `TimedOut` requires distinguishing queueing,
   stream opening, and waiting for a reply.
3. The handler recovers but a retry storm persists: in a separate change, add
   retry pacing per logical operation, preserving next_attempt_at across
   QueryIds, with bounded backoff/jitter and a fair GET+PUT budget per seed.
   Check that reads do not crowd out publications and renewals. Do not replace
   the slot-release fix with this.
4. After the regression and full A04/H11, prepare an upstream bug report/patch
   with a minimal reproducer. Send it separately, as agreed. Remove the local
   vendor patch only after moving to a verified release with the fix and
   re-passing the same scenarios.
