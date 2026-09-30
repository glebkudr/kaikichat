# Bounded group placement before history collection

Date: 2026-09-14. Component of step 1 of the [R14 plan](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).

The first original with real data/index receipts fixes at most 12 already
enqueued sends for its `(conversation, index_id, epoch)`. Work with history_member
set, knowingly paused and expired Work are not included in the new group. Within
its former positions in the ready queue, the unfinished placement is advanced
first. The other sends keep their positions; ordinary rotation, at most eight
advancements and the 20 ms budget check remain in maintenance.

Collection starts when all participants are ready or the no-progress window equal
to the existing BLOCKED_RETRY (5 seconds) has expired. Highwater is tracked
separately for the data receipts, index receipts and location ACK of each
participant. Only an increase of these counters can extend a still-open window.
New sends, repeated observations and retries do not extend it; an expired window
does not open again. This is a no-progress limit, not the overall term of the
whole send while receipts keep arriving.

A finished singleton waits only the former WORK_INTERVAL (500 ms). After the
window expires, a newly permitted sender can replace the group without a ready
caller. Reconciliation removes inactive members. Replacement and composition
change reset only the corresponding collector; the shared graph/pointer
publication is preserved. A successful atomic commit of the new leaf frees the
group; a call from an already included history_member does not.

Placement contains exclusively disposable scheduling hints. The optional
collector is bounded by its IDs but re-checks authority, the original,
index/epoch, actual paid rows and deadlines before the Core commit. ACK, once-spend,
paid obligations, individual observations and retirement were not changed. There
are no changes in the durable data format and the wire protocol.

## Checks

Six ordinary maintenance tests use real signed QC through
Client.request/receive/read and real data/index receipts in CustodyStore.
Only the historical fixture domain, the wall clock and the cooperative clock are
substituted. The ordinary entrypoint passes NETWORK_DOMAIN, the current now and
Instant::now.

Before the behavior change the critic demanded actual leaf sizes, canonical
initial progress, dialog isolation and replacement on a sponsorship change. After
adding these checks, FINAL ACCEPT was obtained. The former scheduler failed in
three premature-singleton scenarios. The single send, fully ready 13 → 12+1 and
the change of the permitted sender passed as behavior-preservation controls. The
first 13-case run with zero advanced Work was a clock adapter error and does not
count as a behavioral RED; after fixing Work.due it passed.

42 backend tests passed (6 new, 21 former sender, 5 index scheduling,
10 TCP/Noise) and 63 frontend tests in six files, Node all-targets Clippy and fmt.
The independent implementation audit ended with FINAL ACCEPT.

Final results and sources: [checks](placement-priority-checks.json),
[independent review](placement-priority-test-review.json).

## Boundaries of evidence

The test with new sends confirms that they do not expand Placement and do not
update its deadline; through ordinary maintenance the ready original and the
later recovered participant sign. The cooperative clock is constant within this
pass. The test does not prove the liveness of the expensive collector under
continuous new reservations between its quanta: hashes of the shared
POLICY/BOOK/INTENTS can repeatedly invalidate opaque authorizations. This check
remains open.

The 500 ms and 20 ms in the tests are scheduling rules, not the measured execution
time of signatures and SQL on a real device. The new tests do not prove a native
mailbox quorum, an improved Diagnostic32 or a Full130 recovery. The release
application was not rebuilt, native E2E and Keychain were not run. Next needed are
the check of the slow collector under new reservations, resumable reading and the
shared pending-body store, then the unchanged native gates. V1 remains open.
