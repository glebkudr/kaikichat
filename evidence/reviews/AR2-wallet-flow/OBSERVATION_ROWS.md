# Retained inspection observations and ordinary Runtime maintenance

Inspection observations now use the common operation-addressed custody row engine.
Each body retains the exact signed read, challenge, holder receipt, metadata and
original timestamps. The small head accounts for occupied bodies and bytes; the
separate expiry queue follows the paid object's retention deadline. The short
freshness deadline still describes when the actual read occurred. Reopening,
migration and unrelated reads cannot renew it or establish physical independence.

Observations have no paid nullifier or index/sequence claims. The shared engine
now represents conflict keys explicitly as optional; paid data, index and outgoing
adapters supply their original genuine keys and retain the same serialized v2
metadata. An observation uses only its operation, digest, counters and expiry.
Its byte quota counts the complete serialized body, including signed evidence.
The compatibility default is 128 operations /16 MiB, independent of the old
ciphertext constructor limit. The daemon sets a separate 4096-operation /64 MiB
admission cap for observations,
alongside its independent data/index/outgoing budgets. The separate evidence cap
also applies. Lowering admission limits does not evict existing observations.

Inspection loads only the addressed operation in incoming, outgoing and
observation stores, then commits every resulting change in one transaction.
Background expiry runs separately, so three complete cleanup batches cannot
overflow the Store's 16-change transaction bound. Quota refusal and SQL failure
cannot leak a new outgoing receipt without its observation. Current receipt,
payment, finality, peer and attestation checks remain in the inspection path.

Version-1 observations use a strict compatibility decoder and one exact bounded
archive. Same-time reads/retries do not migrate them. The first change installs
the new head/archive/rows atomically. At most 128 legacy entries are represented
in the head; all new operations get independent bodies. Legacy expiry keeps the
shared bounded compatibility behavior. New-row GC reclaims at most two bodies
per call. Body/entry/queue tombstones preserve CAS revisions; the retained archive
and small metadata mean this is not a constant-total-database-size guarantee.

The ordinary Runtime attempts observation expiry as its fourth independent
transaction, at most once per second, including when jobs are empty. An error in
another store does not prevent its progress. Failed attempts retain the normal
backoff, and idle maintenance does not create heads in fresh empty stores.

## Tests and evidence

The context-free backend-test-critic accepted all five changed test files before
production. RED compiled the real fixtures but stopped at the three missing
observation APIs; it does not establish a pre-change runtime assertion failure.
The tests use genuine paid commitments and separately signed inspection responses,
with the existing independent attestation oracle.

- 134 actual remote reads survive cold reopen with exact manifests and original
  freshness. Appending changes only one observation body and bounded metadata.
  A 135th operation is refused without leaking its outgoing receipt. Reaping two
  then one expired body physically releases capacity before any survivor reads.
  Surviving body bytes/revisions are exact; stale reads keep the original time.
- Both orientations of real legacy observations/outgoing evidence preserve exact
  manifests and archive bytes. Eight SQL failure points per orientation leave
  all SQL unchanged. A byte cap one below the genuine encoded total refuses the
  operation atomically; the exact sum admits it. Smaller later quotas preserve
  reads. Exact retries perform no writes.
- Addressed attestation damage fails after cold reopen while an unrelated
  observation remains readable. The corruption helper repairs local digest and
  byte-accounting links, so those checks cannot mask the signature failure.
- The existing real Runtime test now accepts 134 incoming data/index commitments
  and signed remote observations, preserves complete cold manifests, and checks
  all four inventories through isolated SQL failures, scheduled retries, restart,
  actual no-job pump calls and backward-clock rejection.

The first regression run found a real compatibility bug: tying observation
bytes to the ciphertext constructor limit rejected valid inspections of small
messages. Restoring the independent compatibility budget fixes that unchanged
ten-receipt scenario. The initial migration helper also selected a metadata row
and seeded `null` instead of actual observations. Requiring the observation map
fixes extraction without changing migration or quota assertions; the critic
independently accepted this exact correction. Both failed runs remain recorded.
An initial incorrect test-name filter ran zero tests and supplies no acceptance.

[Final validation](observation-rows-checks.json) passes: 98 distinct backend
scenarios (54 paid-custody, 42 paid-index and two Runtime), 31 frontend tests,
postage-spend/node all-target Clippy, formatting and whitespace. The 59 focused
source hashes were verified after the checks. Custody's 54 cases were split into
12 inspection and 42 other cases without overlap. This is not a full application
input manifest and does not renew native acceptance. Regressions overlap prior
reports and are not additional independent product coverage.

## Remaining acceptance

This is storage and ordinary Runtime wiring in the signed fixture network. It
does not populate Runtime Core with current daemon-network payment authority or
renew a native acceptance report. Signed network history pages/roots and explicit
worker continuations remain required. The full native gate must retain over 128
simultaneous paid originals, restart/remove the sender, lose real data/index nodes
and recover the complete declared range through ordinary recipient work. MLS
epochs/Welcome, autonomous repair, full E11 and the full 67-card /22-E2E /three-
platform V1 scope remain open.
