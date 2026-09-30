# Durable continuation of range reading in Core

Date: 2026-09-14. Part of step 2 of [R14](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).

Core now persists the verified reading position of one holder separately from
the short-lived Work. `custody_prefetch_after` returns the continuation for the
exact current incoming head, including pointer, root, index and epoch. A new
head starts a new bounded ledger. A skipped reference at the cursor position or
below, and a holder-local `complete`, allow an addressed read before the needed
sequence. `complete` does not prove the completeness of history.

`cache_custody_prefetch_range` uses the shared staging with the former cache API.
Core checks the current head, the recipient capability and all ciphertext before
writing. Node must pre-check all portable paid obligations and the full
proof-bytes budget; `next_sequence` and `complete` come from that check. Cursor
advancement is allowed only with the durable retention of all returned bodies,
including already saved duplicates. The body row and the marker are in one SQL
transaction. On partial filling the bodies that fit can be saved, but the cursor
stays at the former position. An SQL failure returns an error without a partial
commit.

Positions are stored in `custody/prefetch-scan/{conversation}`: at most 128
holders and 64 KiB of serialized metadata. Overflow does not evict former
holders; a new holder gets an addressed read without a saved position. This
metadata contains no bodies and does not take away the former 128 bodies / 4 MiB
ciphertext allowance. Importing the last body does not delete the position. The
old body row version 1 is read without rewriting; the marker's presence is not
required to open an old profile.

Six new tests use real Core/MLS originals, signed pointers, recipient reads and
SQL triggers. They check cold continuation, addressed repair below the cursor,
partial filling, separate failures of the body UPDATE and the marker INSERT,
a pointer change under the same root, a foreign holder, a corrupted later body,
expiry, a real 129th holder without eviction, the complete emptying of the body
cache by import, and compatible opening of the former format. SQL-state
comparisons in the new tests use hashes without printing ciphertext and secret
bytes.

The tests were accepted by the independent critic before the production code.
The RED used a thin adapter over the former cache API, without keeping the
cursor: 0 PASS / 6 FAIL. On overflow the adapter computed `all_retained` from
the actually saved bodies, including duplicates. The first GREEN build exposed
a borrow conflict in the guard, fixed by computing the capacity before the
mutable lookup; this is a compile failure, not a behavioral RED. The repeated
GREEN: 6 PASS in 4.65 seconds. The critic accepted the implementation.

The whole Core regression finished: 64 PASS in 1302.12 seconds, including
reversed-130 recovery. Together with the former 21 paid-page and 12
ordinary-worker tests this is 97 unique backend checks. Also passed were 73
frontend tests, five parser tests, Core/Node Clippy all-targets and fmt.
[Checks](range-core-checks.json), [independent review](range-core-test-review.json).
The new node range runtime tests of the next stage are not part of these 33 node
checks; the Clippy run also refers to the checkpoint before their addition. The six
Core tests did not change between RED and GREEN.

## Result boundary

The ordinary node receiver still calls the former cache API and loses
`next_sequence`. The new Core API does not yet prove the continuation of a real
network Work: next, the Prefix/Obligation request must be merged, the exact head
kept in Pending and the verified cursor/complete applied through the new atomic
boundary. The incomplete root proof between Works, the single pending-body
store, Diagnostic32 and Full130 also remain open. Native throughput was not
measured here; Keychain is not invoked.
