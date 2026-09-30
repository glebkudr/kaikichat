# Operator-network restart investigation

Status: observed graceful-restart correction accepted after bounded transport
drain and bootstrap reconnect. Original EVM criteria remain unchanged. This does
not establish immediate recovery after a crashed machine or a lossy network.

The retained-wallet full regression run on 2026-09-07 passed 479 ordinary Rust,
nine genuine proof/CLI, two Core process, 29 contract and the initial checkpoint
EVM checks. `tests/evm/operator_network.py` then failed in its offline hook after
the actual provider restart. Five remote roles had verified; the next operator
job remained `pending` beyond the unchanged eight-second test deadline. The
aggregate `scripts/check.sh` exited 1. This is not an aggregate GREEN.

The same source and daemon binary passed two subsequent complete operator-network
runs: 16 verified roles, both TCP/QUIC chains, copied binding/wrong index/oversized
wire rejection, pending capacity, restart, expiry and live head changes. Its source
hash was identical (`6fe26aab03159c54fbaae9f5759b6786fc4c466a2e33721d757ccb03a3a47abb`).
The diagnostic wrapper only observes additional state after an assertion failure
and then rethrows that failure; it changes neither the original deadline nor the
success predicate. Neither run reached that diagnostic path. Passing retries do not establish the
cause or remove the open issue.

Local dependency inspection found that request-response 0.29.0 configures its
worker streams with our five-second timeout, but creates outbound
`SubstreamProtocol::new` without overriding swarm 0.47.1's ten-second negotiation
timeout. Therefore the test's eight seconds do not cover every transport phase.
This is a diagnostic lead, not a demonstrated cause of the failed request or a
reason to silently increase deadlines. A future reproduction must observe actual
job terminal state, reciprocal live connections and provider request handling
before distinguishing a deadline mismatch from a connection-lifecycle defect.

Raw records are under `output/postage-proof/retained-wallet/`:

- `shared-lineage-full-gate.log`: original failure and preceding successful checks.
- `full-gate-failed-operator-{network,identity}.json`: failed reports preserved before retry.
- `diagnose-operator-network.py`, `operator-network-rerun.log`: unmodified scenario
  plus failure-only state observation.
- `rerun-passed-operator-{network,identity}.json`: successful retry reports.
- `operator-network-investigation.json`: exact scope and daemon hash.
- `operator-network-diagnostic-2.log`: second unchanged complete diagnostic run, exit 0;
  no failure-only diagnostic was produced.

The retained-wallet remainder passed and was committed at `f9a507e`; its component
evidence is retained separately. Full V1 readiness, including resolution or
justified handling of this intermittent network result, remains open.

## Current-source reproduction during actual proving

The unchanged scenario failed again while the genuine proof suite was running.
The failure-only observer captured `consumer-quic` still advertising a QUIC
connection to the restarted provider, while the provider listed only the TCP
consumer (with TCP and QUIC routes), and no connection to `consumer-quic`. All
three checkpoints were within lease. The provider had served two post-restart
checks; the QUIC request remained pending after the original eight-second limit
and became `failed` roughly two seconds later. This is evidence of stale one-sided
connection state after restart. The ten-second terminal timing is consistent with
the negotiation/idle lead, but does not alone identify which timer fired.

The original assertion was rethrown and the diagnostic run exited 1. Failed EVM
reports and reciprocal states are retained in
`evidence/reviews/P02-daemon-jobs/operator-restart/`. Raw log:
`output/postage-proof/daemon-jobs/operator-network-under-proof-load.log`. No request
was retried or reclassified as success by the diagnostic observer. The next change
must first reproduce the connection lifecycle in a focused real-process test and
receive independent backend-test-critic acceptance before production changes.

## Graceful-drain candidate and bootstrap recovery

The separate critic accepted realistic multi-peer lifecycle tests despite honestly
GREEN focused baselines, using the actual EVM RED above as the correction basis.
The daemon now allows a bounded 500-ms transport drain before Tokio teardown.
All 482 ordinary Rust tests and frontend checks passed. A fresh release app passed
actual proving, cancellation, clean shutdown, parent SIGKILL, wallet recovery and
independent receipt verification. Its proof took 416191 ms and was 585106 bytes.

During that actual proving run, the unchanged operator scenario failed again, now
waiting for a connection after restart rather than observing an old connection.
Six roles had verified; no cleanup errors occurred. A second unchanged complete
run passed all 16 roles on both chains. This is not resolution. Both reports,
the genuine receipt and unchanged 329-source manifest are retained under
evidence/reviews/network-graceful-shutdown/.

The bootstrap scheduler retains its 30-second successful-refresh delay when the
last live verified connection closes. Reciprocal dialing or a message outbox can
mask this delay. New TCP/QUIC real-process tests use an outgoing client with no
verified public address; no message, invitation, settings change or explicit dial
can initiate recovery. Both fail at the original eight-second reconnect bound
on unchanged bootstrap production. A deterministic scheduler test constrains the
500-ms first reconnect, subsequent one-second failure backoff, duplicate closes,
active-slot ownership and unaffected healthy refresh. The new test package
was submitted for separate critic acceptance before bootstrap production changes.

## Accepted final correction

The first critic revision required proof that disconnect cannot release an active
slot to a fifth waiting peer. That pressure test and authenticated rootId retention
were added; the repeated TCP/QUIC behavioral baselines failed at the expected
eight-second reconnect deadline. The critic then returned FINAL ACCEPT. Production
now advances only inactive healthy-refresh candidates to the existing 500-ms retry
after loss of their last verified connection. Duplicate closes cannot reset
backoff, create hints or release active work. The next failed retry waits one
second; normal 30-second refresh and resource bounds remain unchanged.

Both focused TCP/QUIC recovery tests and the previous multi-peer shutdown tests
pass. All 485 ordinary Rust tests, seven models, nine oracles and 40 frontend tests
passed. The unchanged operator EVM gate then passed twice on identical source,
with 16 verified roles and two chains per run while actual prover CPU advanced.
The final release app passed the full daemon proof lifecycle with a real
408315-ms, 584913-byte receipt. All seven isolated Linux network outcomes and five
hidden native WKWebView flows passed; cleanup completed and native screenshots
were visually compared. Release daemon/worker bytes and accepted inputs stayed
unchanged. Evidence: evidence/reviews/network-bootstrap-reconnect/.

This closes the observed graceful-restart regression under the existing acceptance
criteria. Earlier failures remain preserved, and the exact ten-second timer behind
the original stale-connection symptom is not claimed to have been individually
identified. Full V1 and a fresh all-component aggregate remain separate work.
