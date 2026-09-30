# Authenticated cumulative spent-history import

This implements the evidence-transfer layer needed by AR1. Full V1 remains 67
mandatory cards / 22 E2E / three platforms. Current epoch > 1 SpendSession and
client admission guards remain in place: network bootstrap, successor proposal
checks and native multi-epoch spending are subsequent required work.

Public surface under test:

- `SpentHistorySource::open(core, store, closing, now)` authenticates an original
  ClosingRecord and a complete terminal-bound HistoryIndex with the exact closing
  target. `page(store, before, limit)` returns at most four `SpentHistoryEntry`
  values with public fields `entry: Vec<u8>` and `record: Option<SpendRecord>`.
  The terminal has no SpendRecord; each ordinary entry requires its original
  historically verified record and exact entry binding.
- `SpentHistoryImport::begin(core, store, closing, now)` recovers the original
  closing evidence, binds progress to the chosen successor and store owner, and
  resumes exact history. `remaining()` counts unimported entries including the
  terminal. `push(core, store, page, now)` commits verified records and progress in
  the same transaction as history rows; returns true only on the last page.
  Completed local history is reused without resetting its QC or rows. A completed
  begin is idempotent, with zero remaining. A different closing target conflicts.
- `SpentContinuity::open(core, store, successor_snapshot, now)` performs a cold
  bounded-memory scan of every predecessor epoch and every original SpendRecord.
  It returns an opaque owner-bound capability only for a complete chain rooted in
  registry epoch 1. `committee_id()` identifies its unique target; `check(store)`
  checks owner and the retained completion state. This capability does not itself
  create a live signer or disable current-authority fencing.

Later imports require complete cumulative continuity for their source. A 2→3
closing alone cannot replace missing 1→2 history. Skipped epochs (1→3) are valid
when the genuine old committee chose that target. No lifetime epoch/history array
cap is introduced; cold validation may be O(total history), with bounded memory.

Keep original issuer-global spent keys and original QC/time on retries. Every
record must equal the authenticated archive entry. Missing, swapped or corrupt
records cannot become negative membership. Malformed, oversized, stale or foreign
pages leave page/history/records/progress unchanged. Final-page SQL failure and a
cold restart cannot expose completion. Cold completion cannot trust a ready flag
when an original record or history row is missing.

Fixtures: actual funded EVM purchases of 160 public tickets; authenticated old,
second and third registries at the same real checkpoint; independently encoded
journals/transition payloads. Rust fixture QCs use genuine P-256 signatures and
canonical finality proofs. They are historical evidence tests, not a claim of
actual network consensus. The old closing fixture must stay unchanged.

Tests cover 130 original paid spends plus terminal, bounded partial/cold transfer,
exact records, both receiver and existing-validator stores, wrong evidence,
activation SQL rollback, cumulative 1→2→3, skipped epochs and cold corruption.
Independent backend-test-critic must accept before production code for this layer.
