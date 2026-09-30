# New reservations between collector quanta

Date: 2026-09-14. Continuation of step 1 of the [R14 plan](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md)
after [placement priority](PLACEMENT_PRIORITY.md).

The former Core fence compared the entirely saved BOOK and POLICY. An ordinary new
send changed allocated_tickets and reserved, so the opaque authorization of the
finished batch's participants became stale. With one expensive optional visit per
quantum and a new reservation between each scheduling, the group infinitely
returned to preparation. The regression failed after 24 passes without a root.
A separate Core test confirmed the rejection of the old token after a real new
ticket allocation, although its immutable grounds were preserved.

Core now separates immutable grounds and mutable counters:

- BOOK: exact version, commitment and funding_proof; allocated_tickets not below
  the value at full preparation and not above the verified ticket_count; the current
  funding expiry. The snapshot is bound to the just-verified book and stamp.
- POLICY: exact version, config, authority and current_authority. Every previously
  saved cumulative reserved can only grow; the current sponsorship is checked by the
  existing predicate.
- INTENTS, own reservation and message preparation: the former exact hashes of the
  actual bytes. Identity, active job, original message, grant, trust, time
  and the canonical envelope continue to be checked on the current call.

The full funding-proof check is not repeated in the final section. It allocates no
ticket, signs no stamp and creates no new permission from a diagnostic view.
Changed immutable inputs still require a new bounded full visit.

## Checks

The independent critic demanded guards before changing Core. Real Core tests
check the allowed growth, rollback of allocations, exceeding ticket_count,
rollback of cumulative reservations, exceeding sponsorship and a real owner pause.
Allowed growth together with a change of funding/commitment/config/authority is
also rejected. For the policy rollback a third real job first expires: active
becomes 2, cumulative reserved stays 3. A rollback to 2 passes the queue check,
but must be rejected by the old history token.

SQL mutations keep the revision, the fence changes no rows, and the exact
restoration returns the same token's validity. The real advancement of the
checkpoint clock after expiry is done in setup before comparing rows. The first
failure due to this clock write was a setup error and does not count as a
behavioral RED.

The node test uses four real paid originals, different ready callers, the current
full Core queue and real send_message/prepare_public_sender_job between each
expensive scheduling. It checks the growth of allocated_tickets, at most two full
authorization/paid checks per pass, the exact signed root 4/1 and the
preservation of all original and later queued IDs without mailbox publication.

The former code gave a node RED and one expected Core RED with three passing
guards. The tests and implementation received an independent FINAL ACCEPT. Final
results: [checks](arrival-fence-checks.json), [review](arrival-fence-test-review.json).
80 unique backend tests: 4 new +43 former Core, 1 new +32 former Node;
63 frontend in six files, Core/Node Clippy all-targets and fmt pass.

The first node regression: 31 PASS /1 FAIL. The successor publisher did not get a
route: the fixture created different legacy records of one author in one second and
ignored the legitimate refusal of the anti-rollback cache. The helper now uses the
existing publish_node_record with a durable sequence and checks the exact peer+wire
in the cache. The critic accepted the fix; the targeted rerun of the four publisher
tests passed. The total counts unique tests; the original unsuccessful run is
kept separately.

## Boundaries

The collector is confirmed with new reservations in the existing book. Changing
INTENTS, funding, policy configuration or authority requires a new full
check; liveness under continuous replacement of these grounds is not claimed here.
The verified bound of full calls and the cooperative clock do not measure the real
execution within 20 ms. This is not a full-maintenance benchmark, not a native
mailbox quorum and not a new Diagnostic32/Full130. The release was not rebuilt;
native E2E and Keychain were not run. The next parts of the plan are reading
continuation between Works and a single pending-body store with the former limits
and atomicity, then the unchanged native gates.
