# Durable sender observations for immutable history pages

[Targeted validation](outgoing-history-pages-checks.json) passes: 116 backend
scenarios (56 paid-index, 54 paid-custody, six actual TCP/Noise page scenarios)
and 31 frontend tests. Postage-spend/node all-target Clippy and formatting pass;
137 focused source/fixture hashes were verified after the checks. The six new
sender-page cases are included in the paid-index count. These counts overlap
earlier reports and do not add independent product or native coverage.

The [sender contract](../../../spec/outgoing-history-pages-v2.md) now retains exact
leaf/branch/root bodies and separate ACK sets inside the existing outgoing paid
anchor row. Preparation, read and ACK bind the complete commitment, actual paid
index position/transport and current historical trust. Exact retries preserve
prior ACKs. Old roots and distinct signed roots at one revision can coexist; this
store does not select a current root or assert consistency between them.

The unique wires and legacy slot share one 32 KiB allowance. The complete JSON
page map, including keys, kinds and ACKs, occupies local outgoing quota. Duplicate
prepare/ACK retries and reads survive lowered admission limits; new page/ACK bytes
remain subject to quota. A legacy update cannot free pinned versions. Empty maps
are omitted, preserving old encodings. Sender and operator pages now reuse one
typed verifier and wire allowance checker; legacy and page ACK sets reuse the
same sorted-position validation.

Six tests use actual public paid originals and actual index admissions. They
verify independent page ACK sets, late old-root replies, cold historical reads,
legacy changes, data-only refusal, wrong anchors/purpose/signatures/commitments,
unknown and known-but-wrong-position peers, and a real profile without trust.
Body/entry/head SQL failures cover preparation and ACK, plus positive/negative
read-clock release. Exact global map bytes and ACK growth are checked at quota
boundaries. Pinned versions exhaust finite allowance; same-revision roots cannot
borrow ACKs. Signature corruption changes the requested hash and local digest
consistently, while invalid paid positions reach semantic validation. Expiry
rollback and cold reads cannot renew a page or revive its acknowledgments.

Independent review requested the known peer at the wrong paid position; it was
added before production. The critic also accepted a signature-only corruption
check with independent document verification. Existing forest fixtures were shared
through test-only visibility changes. No assertion was removed from those tests.

## Next ordinary publication boundary

These ACKs are local authenticated-transport observations, not new portable
storage proofs or a declaration of fresh availability. The network publisher must
persist them only after an exact response from the matched request/connection,
peer and paid position. Capacity, timeout or local commit failure stays pending.
Cold reads of old ACKs must not create a newer observation time.

Next connect the existing page transport to bounded durable publication and
traversal. Follow each live signed child commitment from a fixed current root;
a root's own ACK cannot cover missing descendants. Preserve old live snapshots
and paid promises while appending or retrying. Publish the pointer only under the
exact current-root fence after the required live graph has paid confirmations.
Recipient per-reference imports must commit with MLS/message/dedup state. Then
run ordinary >128 paid native loss/recovery with disjoint rosters, cold restarts,
short leases and missing/corrupt pages. Ordinary workers remain v1 today; the full
67-card /22-E2E /three-platform goal is not complete.
