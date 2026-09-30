# AR3 exact paid manifest server transport acceptance

The existing Noise custody protocol stores signed history under an already paid
anchor and returns that exact original anchor obligation with the manifest.
Recipient lookup addresses an operation, even when another index sequence comes
first. Full compact portable JSON payload accounting precedes the same existing
historical trust verification/read-clock commit. No extra spend or ciphertext is
created, and missing history cannot become an empty successful directory.

The [contract](../../../spec/paid-history-network-v1.md) defines typed put/read,
operation/hash acknowledgment, exact anchor response and unchanged legacy
manifest-only accounting. [Three store tests](../../../crates/postage-spend/tests/support/index_history_bundle.rs)
cover nonfirst anchor with after=0/limit=1, cold historical public trust, exact
size boundary, real SQL rollback, scope/target/expiry and missing manifest.

The separate [critic](critic.md) accepted these tests, the native server gate and
test-only fixture before production. [Store baseline](baseline-store-1.json)
failed compilation with one E0432 and 11 E0599 missing API/type errors.
[Native baseline](baseline-native-1.json) failed at unsupported new typed ingress;
later funding/history assertions did not execute. All inputs were frozen.

[Native candidate-1](candidate-native-1.json) reached signed cold history recovery,
then failed because its five-request guard batch exceeded the independent peer's
four-request limit. The test split it into 3+2 with unchanged assertions, received
separate ACCEPT, and production stayed unchanged. The failed evidence is retained.

[Native candidate-2](candidate-native-2.json) passes in 52.89 seconds including
build time, with **607 unchanged inputs** and binary SHA256
`f0420b50dd688d1a4f87f6703d7adaf1d9018685ec7b2fd4cb980c57934e0270`.
The final [targeted gate](checks-2.json) uses the same input map and passes
**44 backend /21 frontend**, production Clippy/fmt: 34 paid-index, one custody-node,
nine mailbox-node regressions and the chat-shell frontend group. Checks-1 retains
the earlier fixture revision; it is not combined into a larger test count.

The native scenario uses actual funded public postage, three independently checked
finality signatures, a selected index, native MLS ciphertext and genuine holder/
copy receipts. It inherits legacy cold sender-absent retrieval and one-ticket
balance verification, then checks exact manifest/anchor, independent manifest
signature/fields, cold index with sender daemon absent, missing/malformed/oversize
history, wrong target/range, insufficient bundle budget, real put/update/read SQL
faults, same-anchor revision updates and rollback refusal. Cleanup reports no
errors. Public exact history/anchor bytes are retained in
[native-history-evidence.json](native-history-evidence.json), linked to the raw
trace hash; raw traces/logs stay in `output/ar3-history-network/`.

A test-only helper prepares manifests from the stopped sender's genuine Core
profile under its normal exclusive store lock. It does not inject index or inbox
state. Raw independent Noise peers forward the signed bytes and Bob's genuine
recipient capabilities. This is server transport acceptance; it does **not** prove
ordinary automatic manifest publication/traversal or disjoint-book recovery.
The runtime still uses index-roster intersection. Those paths, control/Welcome,
sender retirement and autonomous R10 are [next](NEXT.md). AR-R03 and full V1 remain
open, with 67 cards /22 E2E /three platforms preserved. No push.
