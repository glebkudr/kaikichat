# Ordinary graph sender — 2026-09-12

The ordinary paid sender now appends through Core's immutable graph API and
publishes typed leaves and branches before their parents. Root ACKs enter the
sender completion projection only after the walk reaches the root; pointer
publication follows its ten exact paid ACKs. Missing live bodies, incomplete
ACKs, SQL errors and capacity failures leave work pending.

The walk holds signed links on a bounded stack and one active page, yielding to
the existing shared publication queue. It skips only authenticated expired links
or exact peaks covered by Core's successfully retired published-root checkpoint.
Core checks the live-prefix extension before those peaks can be reused. A new
root resets the current walk while earlier exact body ACKs remain durable. No
whole-history carrier scan, replacement payment or larger retention allowance
was introduced. Live v1 history is wrapped unchanged by the existing Core API.

## Verification

- [Post-discovery regression](graph-sender-routing-r1.json) passed on 823 unchanged
  inputs after the shared range helpers and routing optimization: two originals,
  six independently verified signatures, five pages/50 ACKs, all SQL/cold/loss
  assertions and successful cleanup. [Targeted checks](routing-hints-checks.json)
  passed: 31 backend /31 frontend, Clippy/fmt.

- [Checks](graph-sender-checks.json): **30 targeted backend /31 frontend**,
  all-target Node Clippy, formatting and diff checks passed.
- [Native C1](graph-sender-native-c1.json): ordinary paid CLI funding and two
  genuine MLS originals, **six independently verified finality signatures**,
  **five signed pages /50 durable ACK events**. Independent CBOR/signature and
  SQL audit checks require child10 before the first parent ACK.
- The last child ACK hits an actual SQL commit failure; warm and cold senders
  expose neither a completed root nor successful retirement. Existing progress
  and both retirement transaction faults remain checked. Subsequent ordinary
  publication extends history after the first job has retired.
- The sender is stopped before genuine data/index loss. The recipient recovers
  both original IDs, authors and text through surviving paid providers, with
  missing-trust rejection, original commit rollback and cold dedup. Per-operation
  and sequence-claim rows are checked together; traversal attempts are separate.
- C1 exited **0** in **424.8 s** on [822 unchanged inputs](graph-sender-inputs-c1.json).
  Backend/frontend checks use the same final production/tests; later status and
  evidence edits are documentation only. No GUI bundle was rebuilt by this slice.

[Independent test critic](graph-sender-critic.md) returned FINAL ACCEPT before
production. R1 was a test helper compilation failure; R2 was behavioral RED at
the root-kind assertion against the old flat publisher. Exploratory G1 completed
all behavioral assertions but failed the unchanged-source guard because sources
were edited during the run. It is not counted as a passed gate; C1 is the clean
rerun. An initial empty backend test filter and wrong npm workspace invocation
are also excluded from the counts; corrected targeted commands passed. Raw
traces and credential-bearing test profiles are not published as evidence.

## Remaining acceptance

This verifies the ordinary sender integration for two paid originals. It does
**not** prove recovery of more than 128 simultaneously live paid originals.
Extend actual wallet funding and scoped CLI budget for that separate gate, send
in bounded batches within the existing active-job limit, independently audit the
whole signed graph and recover every original with the sender absent. Keep the
existing retention, paid replica and per-anchor allowance limits. Preserve the
two-original fault/retry gate as a separate regression.

The older standalone `public_index_sender.py` scenario still contains flat-only
expectations and legacy read audits; migrate that test to typed graph publication
and retain explicit legacy transport coverage. That standalone scenario was not
run or claimed by C1. Current `public_history_graph.py` uses its shared setup and
status helpers and passes the stronger ordinary CLI graph/loss recovery path.

Product gaps/rejoin, first offline Welcome, control/epochs, groups/devices/backup,
independent repair, E11 external-host/NAT and the full **67 tasks /22 E2E /three
platforms** remain open. `product_validated` stays false. Automated desktop E2E
uses the isolated file vault; production builds retain the system Keychain.
