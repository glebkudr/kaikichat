# Graceful peer retirement regression

Real operator EVM restart acceptance failed twice. Reciprocal diagnostics show the
QUIC consumer retaining the old connection while the restarted provider has no
connection to that consumer. The operator query fails after about ten seconds,
beyond the unchanged eight-second acceptance. Isolated real-daemon diagnosis shows
TCP retirement in 3 ms but QUIC still advertised two seconds after clean exit.
Both use the same actual Node binary; no network thresholds were changed.

Before any network production change, add the two Rust-process tests in shutdown.rs
as crates/node/tests/support/shutdown.rs and a path module in processes.rs. They
reuse Node, IPC, MLS and persistence helpers. Genuine SIGTERM must exit successfully
within five seconds, without fallback to SIGKILL. The surviving localhost peer must
retire the connection within two seconds after that process exits. Rebind the exact
same endpoint and persistent identity; verify fresh MLS messages/receipts and one
copy of every message across three cycles. TCP is a working positive control and
QUIC is the observed failing topology. Full actual operator EVM acceptance remains
required after a fix; this focused test alone does not close that integration gate.

A bounded graceful transport drain is the suspected production correction. Do not
change protocol authority, request deadlines, QUIC idle settings, payload checks or
EVM test timing to obtain GREEN. Existing abrupt-death/outbox tests and actual
packaged prover SIGKILL/shutdown cancellation must retain their behavior. These tests
cover normal local shutdown; they make no reliable-close claim for a crashed remote
machine or a lossy network.

The first single-peer baseline passed on unmodified production; that is recorded
as GREEN, not RED. The revised topology uses three real clients: all TCP in the
positive case, and one TCP plus two QUIC clients against a mixed listener in the
QUIC case. All peers share the same two-second retirement deadline. Every retained
message ID on sender and receiver must match across three cycles, in addition to
new plaintext, exactly-one-copy and delivery-receipt assertions. This models the
multiple live connections present in the observed EVM failure.
