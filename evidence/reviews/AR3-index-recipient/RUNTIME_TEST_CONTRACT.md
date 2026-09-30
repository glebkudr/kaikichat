# AR3 recipient runtime — test-first contract

Pre-implementation application: `d3a3d5e`. The independent context-free critic
returned REVISE for ambiguous empty-page SQL faults and missing public-trust
rejection. Both were corrected, then ACCEPT preceded production. The real
baseline reached publication and data loss, but performed only direct reads.
After production and a transport-priority fix, all four final native gates pass
on the same source/binary. Test-only trust-helper extraction and the shortened
macOS fixture path received another ACCEPT before those final runs.
The new native test is `tests/evm/public_index_recipient.py`. It reuses the
ordinary sender preparation from `public_index_sender.py`, the real funded
public book, native finality signature verification, all ten genuine data and
index receipts, and the existing observer-only retrieval harness.

The recipient initially has only its established MLS conversation and no public
trust, wallet or configured postage client. The sender sends two ordinary messages, automatically
stores ten copies of each, and publishes ten indexes with 100 location ACKs per
message. The sender is stopped before data-loss preparation and never restarted.

The test reads the stopped sender's retained locator only to choose the loss
fixture. It never copies that state, capabilities or provider addresses into the
recipient. All original ciphertext is deleted except one original operation on
each of two distinct selected holders outside the locator endpoints. Every
locator endpoint has zero ciphertext. The test helper acquires the real stopped
custody profile lock, changes only `custody/storage-v1`, preserves complete
surviving objects, and checks all other state rows byte-for-byte. After cold
restart each daemon's real obligation and storage APIs must independently agree
with the intended loss. The index state hashes must survive the loss unchanged.

Only ordinary bootstrap/DHT networking makes provider routes available. On Bob's
return the harness permits only node_info, snapshot and mailbox_head. The exact
original messages must be recovered through the independently discovered latest
head, whose endpoints must be confirmed selected index peers. The first return must make actual network reads without importing either
original while public trust is absent. Only then are public profiles installed.
Real SQLCipher message INSERT faults for the exact two original IDs require a
matching progress cursor already applied inside that same transaction. The first
refusal must preserve MLS/progress state exactly; permitting only the first
original must leave the second absent with no bookmark reaching its sequence.
Separate Core tests retain progress INSERT/UPDATE fault coverage; this native
scenario claims exact-original transaction rollback, not both SQL progress verbs.
Cold retry with the first seed absent must import both original IDs/authors/text;
further cold traversal must preserve exactly one of each. The setup sentinel is
never indexed and must remain absent from the recipient.

This is disjoint **surviving** holder evidence after actual data loss, not disjoint
original R10 draws, which are impossible in the 16-member fixture. This slice
does not claim cross-book/epoch completeness, successful sender retirement,
autonomous repair or complete V1. Existing old-pointer native retrieval remains
an explicit compatibility regression. Core negative tests already exercise
signed foreign/reverse descriptors, bad signatures/ciphertext/cursors, epoch
changes, token/retention expiry and atomic storage failures. Independent paid
index/location verifier tests cover wrong selected authorities and copy proofs.
The final native set includes both the ordinary owner/signed-agent flow and the
legacy direct-data locator flow. See `runtime-checks.json` for current evidence.
