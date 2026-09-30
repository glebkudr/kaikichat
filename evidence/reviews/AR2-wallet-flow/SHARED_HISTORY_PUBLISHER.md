# Shared graph and pointer publication

Date: 2026-09-14. Component of step 1 of the [R14 plan](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).

Node keeps the existing graph stage, its queue of exact page requests and the
pointer retry once per `(conversation, index_id, epoch)`. A separate message
Work keeps the key of this state and its own progress view. The key is set after
the current full authorization/paid admission; the other participants get it only
after a successful atomic Core batch commit. Reconciliation keeps the publisher
while an active Work references it, so the completion of the first participant
does not reset the work of the others.

When the calling message changes, the traversal continues from the same place. An
error returns the state to the shared map together with the traversal and pending
IDs. A new root uses the existing graph/ACK projection reset. The old pointer
input can serve only as an exact idempotent retry: a new commitment or endpoint
will not match it. For stored, every job still passes the current authorization
and its own `record_public_sender_progress`; Core checks membership, root,
pointer and atomically frees the active record.

The durable data format, graph wire, ten ACKs per page, quotas, retention period
and the transport queue were not changed. The ordinary entrypoint passes
NETWORK_DOMAIN and Instant::now. The private clock/domain adapter allows using
the signed historical paid fixture in the same publication code.

## Checks

The [critic](shared-publisher-test-review.json) initially demanded strengthening
the cold retry, the transition from the existing pointer to the successor and the
isolation of two dialogs. After the fixes the tests received FINAL ACCEPT before
the behavior change. The mechanical adapter with the former per-job stages gave
four behavioral REDs: another caller again traversed the leaf and did not reach
the expected root save. After moving the stages all four new scenarios pass; the
final runs and hashes are in [checks](shared-publisher-checks.json).

| Scenario | Observable outcome |
| --- | --- |
| 9/10 leaf ACKs, another caller, revoke and root SQL failure | The root waits for the tenth ACK; the next permitted caller continues the root; the SQL rollback preserves the retry and the active jobs |
| Pointer SQL failure and cold retry | The real signed publication preserves the sequence and bytes; after the restart a new ordinary mailbox attempt appears |
| New root after the prepared pointer1 | The old pointer is kept until all the successor's ACKs; the new one gets sequence2; the old ACKs do not confirm the new root |
| Two alternating dialogs | Each continues its own graph, without borrowing the other's ACKs or pointer |

The paid helper really saves pages in the paid remote CustodyStores, checks
the reading of the saved bytes and only then performs the local ACK record
through the trusted transport adapter boundary. It does not replace the Noise
check. A separate existing cluster checks real TCP/Noise responses, request/
peer/connection matching and coalescing. The historical domain of the new
fixture is rejected by the ordinary live DHT filter: the signed pointer exists,
but the mailbox job has failed, and the messages are not declared stored. A
native mailbox quorum is not proven by this.

## Remaining work

Data/index placement priority in the bounded group is not yet implemented.
A constant inflow of new reservations can invalidate the collector's snapshots;
a new root still starts the traversal of its graph. These conditions need to be
checked in the ordinary scheduler, then reading continuation between Works and
the shared pending-body store. An improved Diagnostic32 and a successful Full130
remain mandatory. This component does not close all of step 1 or V1 and is not a
new native run.
