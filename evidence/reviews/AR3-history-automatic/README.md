# Ordinary signed history publication and retrieval

Ordinary sender/recipient workers now use the signed directory and each original's
own paid index candidates. Durable original inventory replaces live-job roster
intersection. Ten exact anchor ACKs precede private pointer publication; recipient
imports commit message/MLS/dedup/per-operation progress together. Missing history
does not trigger legacy fallback. [Runtime contract](../../../spec/custody-history-automatic-v1.md),
[test contract](TEST_CONTRACT.md), [independent critic decisions](critic.md).

Three native gates pass on one final application source/binary:

| Report | Verified behavior | Seconds |
| --- | --- | ---: |
| [Sender](candidate-sender-3.json) | Both ordinary originals; index/location/manifest/ACK SQL failures; cold retries; unchanged tickets/receipts; exact independent manifest reads from all ten ACK peers | 311.43 |
| [Recipient](candidate-recipient-3.json) | Actual index/data loss; different surviving index and holder per original; no ciphertext at pointer endpoints; sender absent; trust refusal; both original-message SQL failures; partial progress/cold retry/dedup | 187.23 |
| [Legacy compatibility](direct-compat-1.json) | Direct-data locator without paid-index trust profiles; existing copies/inspection; sender absence; storage failure and cold automatic retrieval | 112.88 |

[Final targeted checks](checks-3.json): **76 backend /21 frontend**, production
Core/postage-spend/node Clippy and fmt. Backend count includes six existing
real-node mailbox regressions: hostile DHT, request/job limits, owner-work routing
capacity, sender absence and first-cache loss. All **613 input fingerprints**
match the three native runs and remain unchanged. All runs have clean teardown.
[Final consistency record](acceptance.json) rechecks the inputs, binary, raw
log/trace hashes, test counts and unchanged release scope.
Final binary SHA256:
`99a70c85940dade50f2b573396f256e68cfc8db23c6866bfdaf7a97ba10d4274`.
No full workspace suite or cross-platform release gate was run for this slice.

[Sender evidence](sender-evidence.json) retains exact signed publication/originals,
SQL-failure and cold-completion states, and all ten authenticated remote replies.
Identical paid anchor fields are factored into commonAnchor; each peers[].anchor
contains the remaining fields. Merge the two maps and attach the saved status and
manifest to reconstruct every original response. The collector asserts exact
reconstruction without modifying any value. [Recipient evidence](recipient-evidence.json)
retains exact signed publication/originals, actual loss maps, trust refusal,
rollback snapshots, partial/per-reference progress and the completed cold pass.
Both include full raw-trace SHA256/path provenance. Reports bind fresh commands,
source/test fingerprints, binary and raw trace/log hashes. Raw logs/traces remain
in managed output. [Collector](collect_evidence.py).

Historical results remain recorded:

- baseline-sender-1 was a runner variable collision: trace.json.py was attempted;
  no native test ran. baseline-sender-2 stalled at an existing 9/10 index promise
  prerequisite after restart (258.96s); this is not missing-feature RED.
- baseline-recipient-1 reached actual stored ordinary work, then failed because
  no signed manifest was exposed (110.34s). Later import assertions did not run.
- candidate-recipient-1 passed the full recipient scenario (280.36s) on the first
  implementation, before sender retry optimization and mailbox enum boxing.
- candidate-sender-1 failed after removing the preparation SQL trigger: the test
  immediately asserted a new ACK-stage revision against the previous asynchronous
  error (238.64s). Independent ACCEPT approved waiting for blocked/storage_error
  with a prepared revision, retaining the prohibition on any ACK or stored status.
- candidate-sender-2 passed (269.31s). candidate-recipient-2 on the same earlier
  binary reached ten manifest ACKs for both messages, but timed out with pointer
  sequence 2 still publishing, zero confirmations (230.71s). Status calls took
  up to 4.9s. The current worker delays full proof revalidation while an existing
  pointer network job is pending, leaving time for network events. No validation
  or query/test deadline changed. Diagnostic-only test enrichment also received
  separate ACCEPT. The final three gates above include this correction.
- checks-1 passed 70 backend/21 frontend but failed Clippy on the enlarged mailbox
  work enum. Boxing its input fixed that diagnostic. checks-2 passed the targeted
  groups before the final pointer-wait scheduling change; checks-3 supersedes it.

Reproduce with fresh labels:

```sh
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-history-automatic/run_runtime.py fresh-sender-1 public_index_sender
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-history-automatic/run_runtime.py fresh-recipient-1 public_index_recipient
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-history-automatic/run_runtime.py fresh-direct-1 public_paid_ciphertext
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-history-automatic/run_checks.py fresh-checks-1 candidate
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-history-automatic/collect_evidence.py fresh-sender-1 fresh-recipient-1
```

This fixture has one funded book. Actual loss leaves different surviving indexes
and holders, but does not prove originally disjoint funded books. Welcome/control,
successful sender retirement, attachments/historical lifecycle and autonomous R10
remain required. [Continuation](NEXT.md). AR-R03 and V1 remain open: 67 cards,
22 E2E and three platforms. No push.
