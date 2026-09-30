# Paid anchor history storage — local acceptance

The existing paid index now retains one exact signed history directory under its
original anchor descriptor. Full historical Core trust and the actual local
operator key authenticate the paid anchor on reads and updates. Original index
receipt/funding/QC and ciphertext are unchanged. The optional directory shares
the existing SQLCipher index state, quota, read clock and original retention.

**PASS: 78 backend /21 frontend tests, production Clippy/fmt and 595 unchanged
application/test inputs.** This includes 24 crypto, 22 Core custody, 31 paid-index
(five new) and one node custody codec test. [Commands/log hashes](checks.json) ·
[Inputs](source-inputs.json) · [Storage contract](../../../spec/index-history-storage-v1.md).

The five tests received independent [REVISE → ACCEPT](critic.md) before production.
The final helper-only DRY change received a separate ACCEPT. [Current-input RED](baseline-3.json)
has 34 absent-API E0599 errors against `8b49cdd`; earlier baselines retain their
own inputs. These are compilation RED baselines, not executed behavior failures.

Verified behavior includes cold historical reading with public trust/no wallet;
an anchor index without the other referenced local index; unchanged original
paid evidence; exact idempotent retry; actual first-manifest/update/read-clock SQL
failures; cold rollback/equivocation rejection; preserved live descriptor hashes
and references with expired pruning; exact quota and later admission accounting;
whole-read byte bounds; wrong anchor/scope/peer, missing trust and expired reads;
and rejection of corrupted signed SQL bytes without silently healing them.

Only the actual anchor admission establishes paid storage here. Candidate keys
and other references are signed discovery declarations, not paid roster evidence.
No new native or full-release acceptance is claimed. Core outgoing/incoming
manifest state, locator commitment, network transport and ordinary runtime use
remain next; multi-book/MLS control history, completeness, Welcome, retirement
and autonomous repair remain open. [Required continuation](NEXT.md).
