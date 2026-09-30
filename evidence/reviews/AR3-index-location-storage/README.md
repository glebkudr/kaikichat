# AR3 — durable compact holder locations

An existing paid index can now retain authenticated original/replacement data
locations and return them through a local Core-checked read. Shared original
funding/QC/descriptor stays stored once; individual records reuse OutgoingReceipt.
The original index receipt and paged index descriptor format stay unchanged.

Five tests were written before production. The independent context-free critic
returned REVISE for a borrow conflict and missing unconfigured-Core refusals;
revised tests received ACCEPT before production. Baseline was missing-method
compilation RED only. The first production candidate needed two private-method
visibility corrections; the next ran all nine location/verifier tests successfully.

The accepted targeted gate passes **17 backend tests (16 paid-index + one native
public historical ciphertext test), 21 frontend chat/history tests, matching
Clippy and fmt**. All **577 input fingerprints stayed unchanged**.
[Results](checks.json) · [Exact inputs](source-inputs.json).

```sh
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-location-storage/verify.py
```

The SQL tests cover first and later location UPDATE failures, cold restart,
read-clock failure, exact retry, immutable-position conflict, byte quota boundary,
whole-entry expiry and corrupted signed evidence. A real disjoint index operator
retains original and opted-in replacement claims, then a cold reader without a
wallet checks them after consent expiry. The returned claims are checked with
their unchanged original index evidence and the actual ciphertext descriptor.

This is local persistence/read acceptance. Network publication, bounded remote
location reads, disjoint-roster recipient discovery, book/epoch linking and R10
are still open. There is no full-workspace or native network gate for this slice.
Raw build/run logs stay in ignored `output/ar3-index-location-storage`; structured
reports retain their hashes and paths. Prior revision test counts are not added
to this gate.

[Contract](../../../spec/index-holder-storage-v1.md) ·
[Test rationale](TEST_CONTRACT.md) · [Next work](NEXT.md).
