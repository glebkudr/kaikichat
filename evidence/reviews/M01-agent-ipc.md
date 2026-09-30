# Proof-authenticated IPC review

Tests-first additions in crates/node/tests/processes.rs cover real child daemons, signed calls without owner credentials, offline delivery, sender restart/idempotency/replay, actual reply reading, revoke/restart and continuing owner authority. Independent raw IPC frames cover strict disjoint envelopes, wrong runtime, read-only scope escalation and malformed/oversize proof. RED: call_agent did not exist; owner grant/revoke dispatch and agent request branch were absent.

Separate node_test_critic returned REVISE because individually supplied owner fields did not exercise a complete mixed owner envelope. Added valid owner token+send method+request alongside a valid read-only proof; after rejection, history/outbox remain empty and the same pure proof succeeds. FINAL ACCEPT before production. No test assertions were weakened during implementation.

The integrated process test then revealed an existing network fault: a second rapid sender SIGKILL/restart repeatedly failed to reconnect to the living recipient, although the new owner message was durable. Three reproductions showed receiver history2 vs sender history3/pending1; temporary diagnostics identified TCP AddrInUse (macOS code48) while request-response reused the listening port. Ordinary delivery now explicitly dials with allocate_new_port, preserving advertised listeners/identity. Libp2p ignores the redundant request-response dial while that connection is pending. The unchanged ten-process-test suite then passed, including TCP/QUIC/fallback. Temporary diagnostics were removed.

This establishes the IPC stage only. Official MCP stdio, cancellation, inbox leases and native runtime provisioning remain required work.
