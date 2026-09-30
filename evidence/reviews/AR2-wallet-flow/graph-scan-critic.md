# Independent backend test review: durable graph reference attempts

Reviewer: context-free `index_holder_test_critic`, reused under the mandatory
backend-test-critic workflow. Production methods were absent until final ACCEPT.

## R1 — REVISE

Two fixture blockers: the mixed-TTL v1 directory used a short-lived anchor, and
the epoch fixture incorrectly reset the mailbox publication CAS. The reviewer
also required a valid signed wrong-parent proof and current-pointer checks on
skip itself, including its idempotent return. Fixed with the longest-lived actual
anchor, the shared real MLS epoch helper and the preserved mailbox CAS. The epoch
fixture now succeeds at a skip before transitioning and rejects its old retry.

## R2 — REVISE

The substituted single branch stopped at a still-live intermediate link, masking
missing parent binding. The reviewer accepted the proposed complete alternate
path as the corrective oracle: without parent binding it reaches the actual
expired leaf and would incorrectly permit the skip.

## R3 — ACCEPT

The complete authentic alternate path is passed to the old-root skip; identical
terminal leaf and exact expiry are independently asserted. Denial leaves SQL
unchanged and the correct old path then succeeds. All eight reviewed hashes
match. No other mandatory gaps remain.

Accepted test SHA-256:
`3a14e63bebf5823097601c21c736d0f1e0bf186231181845c9a851ef0ebef716`.
See [reviewed inputs](graph-scan-reviewed-inputs.json). The spec's status sentence
was changed from tests-first to implemented after acceptance; its contract was
not changed. Production preimage hashes in this manifest are intentionally the
review-time source; the candidate manifest records implemented source.

RED r2/r4 contained only absent API errors. Earlier REDs additionally exposed a
BTreeSet-vs-Vec assertion typo and a nonexistent branch accessor; both were fixed
before the relevant review. These are fixture/compile evidence, not passing
runtime results. Raw logs remain in ignored `output/ar2-wallet-flow/graph-scan-*`.
Final ACCEPT admits seven Core tests; it does not establish ordinary Node/native
graph recovery. The subsequent targeted runtime checks are recorded separately.
