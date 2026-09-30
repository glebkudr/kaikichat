# Graph completion checkpoint and cooperative publication

Core retains the last graph publication completed by the trusted host for each
outgoing conversation/MLS epoch. The checkpoint, completed sender job and active
queue removal commit together. Completion binds the exact current root, actual
membership of the job's original under its authenticated leaf/branch path and
the exact mailbox sequence/batch. Local root/pointer preparation and partial
progress cannot create it. Existing membership, extension and CAS helpers are
reused. Exact retry writes nothing; a successor preserves the old live prefix.
Expiry retains a historical boundary, not current availability or recipient ACK.

Four real MLS/SQL tests cover partial/complete publication, undeclared originals,
independently stale roots and wrong pointer sequence/size, INSERT/UPDATE rollback,
cold exact retry and a genuine MLS epoch transition. Old epoch bytes survive;
a new epoch needs its own completed publication. An older corruption fixture
was repaired to address the actual retained sequence row; all five corruptions,
cold refusal/no-healing and exact restoration remain. Independent test reviews
accepted the tests and repair before implementation.

Index and history publishers now share one cooperative preparation queue. Both
yield after expensive successful or failed preparation, using the existing 20ms
budget and four-attempt cap. Rotation, pending/completed suppression, peer backoff,
capacity halt and outer fatal errors are preserved. History still consumes
responses before new work, resets the queue when the exact page changes, retains
its request counter and global transport cap4, and uses the same paid-page sender.
Five independently reviewed tests cover the queue with deterministic time and
opaque request IDs, using both index and history-position keys. They test work
between callbacks; actual paid admission and ACKs are checked separately.

[Checks](published-graph-checks.json): **117 backend /31 frontend**, Core/node
all-target Clippy and fmt pass. Counts combine 98 unchanged Core tests from C2
and 19 Node tests from C8, including Core130 and Runtime fair retry regressions.
Repeated passes are excluded; counts overlap earlier reports. Inputs remain
stable within every candidate: 806 for C1–C5, 807 for C6/C7, 808 for C8.

Preserved failures explain the fixes. C1 found the stale corruption fixture.
C2 native linking exhausted the managed build image; growth 250→350GiB kept its
UUID/config/links and 13 protected artifact hashes intact without deletion.
C3/C4 hit a 5s owner IPC timeout. Diagnostic C5 passed but exposed expensive index
work. The index yield fix let normal C6 cross all three sender SQL-fault stages,
but a later IPC read timed out while the second message published history.
Diagnostic C7 isolated a second loop in history publication/send_history_page.
After sharing the queue, **normal C8 passes with the original deadlines and
assertions**. Diagnostic runs remain separately labelled. One callback and work
before the queue are still synchronous; no universal latency bound is claimed.

[Normal paid CLI compatibility](published-graph-native-c8.json) verifies two
ordinary sends, exact retry and budget refusal, shared status, all sender SQL
faults, cold page ACKs, retirement and recovery after real data/index loss with
the sender absent. Six QC signatures are verified. Owner retrieval/sender-work
calls are zero; teardown is clean. Setup remains in a separate conversation.
Only sanitized evidence is exported; raw native traces and secrets are excluded.
The C8 launcher reused the C6 output directory; after clean teardown it was moved
to C8. Original C6 log/manifests/failure note remain; its overwritten report is
explicitly marked as a recovered partial summary. Runner paths are recorded.

The Core checkpoint records trusted-host observations. Paid child ACK ordering
and ordinary v2 traversal remain open: the native flow above still uses flat v1.
Next integrate child-before-parent/root/pointer publication and bounded graph
reads with durable leaf/reference attempts, then prove recovery of more than 128
simultaneously live paid originals. Full 67-card/22-E2E/three-platform V1, product
gap/rejoin UX, Welcome/control/epochs, independent repair and E11 remain open.
