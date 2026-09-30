# Cross-log daemon replay: independent test review

Baseline: 0d5596b5a0331c3c36ffbad8b445158c0db3e754. No production source changed.
The existing selected daemon service already passed its two-transport fixture gate;
this delta adds a regression against existing scope isolation and wires that runner
into scripts/check-evm.sh.

Independent backend-test-critic /root/scope_replay_test_critic, spawned without any
inherited conversation: **FINAL ACCEPT**. The parent awaited its final verdict before
running the live scenario or making further changes. The critic also ran the existing
six oracle tests successfully. Its review found no blocking issue in the tests.

Reviewed files: tests/evm/finalizer_scope_replay.py, the delta in finalizer_service.py,
crates/node/examples/finalizer_replay_peer.rs, scripts/check-evm.sh and the contract
spec/finalizer/daemon-scope-replay.md. Core/daemon/engine production behavior was read
to confirm the actual admission and scope-local engine rejection boundaries.

The critic confirmed that a genuine independently verified QC, actual donor transport
key, Core-issued proof, both initial enqueued responses, scope-local subsequent
rejection, unchanged route/generations/running state, unchanged effects and later
independently certified work exclude shallow unknown-scope or authority-loss controls.
Baseline GREEN is appropriate for this regression of existing behavior.

Execution advice retained: restrict the compiling mutation to channel-1 finalization
certificates so it reaches the replay assertion rather than failing initial setup;
verify where RED occurs. The carrier's 32-probe bound may fire before a four-second
negative deadline; distinguish that outcome from rejection. Cleanup can replace an
earlier failure; preserve logs when evaluating a failure. No reviewed tests were
weakened in response to these non-blocking observations.

At the initial review checkpoint, live positive runs and the compiling mutation had
not finished. FINAL ACCEPT is a design verdict, not evidence of successful execution.

The first live run reached replay but failed before sending any probe: a retained
old route satisfied a peer-only wait before the independent carrier served its
fresh Core proof. The report was failed, cleanup empty and probes empty. Preserved
run output: output/evm-e2e/P01-scope-replay-first-failed/.

The helper now waits for servedProof:true and a different proof connection ID than
the pre-stop route, preserving all authority/response/effect assertions. It also
fails explicitly on missing engine rejection before the carrier's request bound.
The same independent critic reviewed only this correction and returned another
**FINAL ACCEPT**; it confirmed the preserved failure and no weakened assertion.
The exact reviewed files are pinned in P01-daemon-scope-replay-test-manifest.json.

## Subsequent review and execution

The critic returned separate **FINAL ACCEPT** verdicts for read-only runtime snapshots
before cleanup, dev-only Commonware tracing with bounded 64 KiB log tails, and a stronger
withholding scenario. Each final verdict was awaited before continuing. The strengthened
scenario keeps one process stopped for another six seconds after the honest quorum has
finalized, continuously checks the two durable prefixes, then retains the original
35-second recovery deadline. Elapsed time exceeds the five-second wire timeout but does
not by itself prove that a particular stream timed out. A final cargo fmt edit only wraps
the fixed tracing-filter string. No production Rust behavior or assertion was weakened.

The isolated compiling mutation redirects only channel-1 finalization packets to the
first log. Build exited 0; the real TCP test exited 1 at the intended new assertion,
`engine did not reject a real certificate from another log`, after 24 correlated
`enqueued` responses. Setup and genuine Core proof exchange completed; cleanup errors
are empty. The mutation ran before later diagnostic additions and the longer withholding
phase, which follows the failing assertion. Its diff, exact public packet evidence and
raw-output hashes are in `P01-daemon-scope-replay/dispatch-mutation.{patch,json}`.

Two complete runner attempts passed TCP and the QUIC replay/independent-other-log effect,
then failed QUIC recovery: one after cold replay at effect 12, the other after withholding
at effect 5. Runtime snapshots from the latter show live services, authority and routes;
they do not establish the cause. After tracing was added, one QUIC-only diagnostic passed;
the stronger six-second-pause QUIC diagnostic also passed. Logging can affect scheduling.
Neither success explains those failures or constitutes full product acceptance.

`P01-daemon-scope-replay/execution-history.json` preserves each result and raw-artifact
hashes. The aggregate first stopped on formatting; after that correction it passed
earlier gates and TCP service acceptance, then failed QUIC recovery at effect 12
(target 11, other three 12). This is retained in output/evm-e2e/P01-pre-fix-aggregate;
it is not a successful aggregate. Frontend verification separately passed 40 tests.
Two actual embedded-service regressions now establish quiet-tip liveness defects:
lost finality for an already certified tip, and a notarized tip with too few final
votes after two honest timeouts. The candidate and honest regression history are
recorded in quiet-notarization-recovery.json and spec/finalizer/quiet-tip-recovery.md.
Full V1 and production application admission remain incomplete.


## Current clean aggregate

After test-first quiet-tip recovery, the complete main check.sh run passed: 458 Rust,
29 Solidity, 7 model, 9 oracle and 40 frontend tests, fmt/Clippy, TypeScript/Vite and
all EVM gates. Actual TCP and QUIC each complete 13 effects on four selected profiles,
including cross-log replay rejection and later independent OTHER-log work. The run
independently verifies 159 quorum signatures and makes 922 owner calls; cleanup is
empty. Source hash: 0bb8c447a57929fd48b7c38645c5883647574bd218e721a4225ae1270018bd07.
The fixture remains a trusted local signed-work application; it does not authorize
production spend or group actions. Native/Linux/release checks are separate.

The original compiling mutation remains a negative control for the replay assertion.
The two quiet-tip regressions have independent baseline RED and candidate GREEN.
These establish real corrected liveness faults, without proving the exact packet
causality of every historical QUIC timeout. execution-history.json retains both.

The subsequent five hidden native flows, seven Linux outcomes and four paired visual
comparisons passed. A new ad-hoc-signed release bundle passed strict deep signature
verification; no automation plugin appears in its default dependency graph. Detailed
hashes and retained PNGs are in quiet-notarization-recovery.json. The full V1 goal
remains incomplete.
