# Independent backend test review

Core/application critic: `/root/epoch_closing_test_critic`, no inherited context.
First verdict REVISE: isolate prepared successor fence revocation; require exact
terminal binding; independently authenticate both historical snapshots; test
recovery conflict and original-QC preservation. All four were added. Repeated
verdict ACCEPT before production. Revised RED: missing APIs only (E0432/E0599,
60 errors). Existing fixture-authority control: two tests passed. Implemented
Core/application focused gate: seven tests passed, none failed or ignored.

Node critic: `/root/epoch_node_test_critic`, no inherited context. Verdict ACCEPT
before node production. Native RED: unknown owner method postage_epoch; zero
cleanup errors. Accepted live test includes genuine funded postage and registry
proofs, ordinary current/future keys, direct QC verified independently, network
closing, SQL-failed durable effect, revocation, cold two-node recovery, immutable
original evidence, and preservation of unrelated pending paid input.

Nonblocking node feedback: await source ACKs, save pending snapshot in trace,
clarify that the count is initial owner proposals (later retries are tested).
Those reporting/timing changes received repeated ACCEPT before node production.

These verdicts accept tests for implementation. They do not certify a passing
node gate, spent-state handover, successor spending, other platforms, or full V1.
