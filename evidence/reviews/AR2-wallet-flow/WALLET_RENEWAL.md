# Ordinary paid originals across a successor checkpoint — in progress

The focused `public_wallet_renewal.py` gate sends two genuine paid originals
through the scoped CLI, with a real linked successor checkpoint between them.
The initial registry snapshots last3600 seconds; checkpoint maximum lease stays
1800, and message retention, purchase lifetime, R10 and network/page limits stay
unchanged. This is the prerequisite for correcting the infeasible fixed-snapshot
[130-original fixture](history-range-fixture-review.md).

`live_wallet_authority.py` mines an actual fresh Anvil block and obtains issuer
and both registry proofs at that state. Providers refresh their existing
publications and finalizer configuration using current revisions. The ordinary
Alice client receives no checkpoint certificate or authority proof from the
helper: its own peer synchronization must select the new head and import its
public authority. The second paid plan must name that exact head; the first must
retain the preceding head and ticket1, with ticket2 used exactly once afterward.
The independent placement oracle rejects every unknown head.

The existing CLI retry/budget, exact wallet purchase/balance, queued delivery
despite R10, sender-absent copy loss, atomic MLS import and cold dedup assertions
remain. The setup sentinel uses its own genuine MLS conversation. The final
balance query uses the current head, followed by ordinary wallet refresh before
the unchanged exact balance oracle.

The independent context-free backend test critic returned FINAL ACCEPT after
three fixture corrections; [review and exact test hashes](wallet-renewal-critic.json)
are retained. No production code changed for this fixture correction.

[Native R1](wallet-renewal-native-r1.json) failed with exit1 on 825 unchanged
inputs and pinned debug node/CLI/MCP binaries; cleanup succeeded. The first
original completed, then the ordinary client demonstrably acquired the real
successor and public authority from peers. The second original stayed blocked
with `checkpoint_rejected` before preparation until the unchanged180-second
publication deadline. Final signature and recovery assertions were not reached.

Core's queued execution still uses the current authority saved in the sender
policy by configuration. Before the following correction, peer acquisition and
ordinary wallet refresh left that policy's node-registry proof unchanged. The
existing Core contract requires owner refresh and its successor test explicitly
reconfigures the sender; R1 exposes the missing ordinary-user transition.

## Ordinary refresh bridge

The [reviewed tests](wallet-sender-refresh-critic.json) now exercise real wallet
RPC contexts/proofs and a linked successor, exposed and queued originals, exact
cold retries, all sponsorship/runtime denials and actual policy UPDATE failure.
Successful refresh changes only the existing policy's current authority; original
ancestry/config/reservations and all other SQL bytes remain. The checkpoint CAS
revision changes with the atomic policy update. Exact retry and failed writes
retain complete row equality.

Core shares verification between configuration and the new narrow refresh. On a
stale-context failure, Runtime uses only its ordinary wallet's verified context
to refresh the matching current policy before retrying existing execution checks.
There is no policy scan, new persisted schema, proof input or sponsorship setting.
Completed diagnostics keep historical obligation verification and use current
authority for live planning/book checks, preserving every original signed byte.
The native oracle obtains original admission heads from retained provider bundles,
compares first completion before/after renewal, and compares real page ACK rows
and completed lifecycle rows after a cold sender restart.

[54 targeted backend /31 frontend, Core/node all-target Clippy and fmt pass](wallet-sender-refresh-checks.json).
[Native R2 passes](wallet-renewal-native-r2.json) on826 unchanged inputs and pinned
debug node/CLI/MCP binaries, with six independently verified QC signatures and
clean teardown. Both originals crossed the genuine peer-acquired successor,
retained their exact tickets and signed evidence after cold sender restart, and
recovered automatically after actual sender/data/index loss. Page ACK and completed
lifecycle rows remained unchanged; owner retrieval calls were zero.

[Full130 release R5](history-range-native-r5.json) used this fixture and bridge,
but failed at the original fifth-batch deadline after64 stored originals and
three real peer successors. All826 inputs stayed unchanged and cleanup passed.
Eight pending finalizations occupied every client slot after the third successor;
no copy-loss/full-range/cold-cycle oracle was reached. The focused R2 above changes
authority between completed originals and does not cover an in-flight spend.
[The next regression](PENDING_POSTAGE_RENEWAL.md) must cover that ordinary case
before production changes. Two originals do not establish >128 or full V1.
