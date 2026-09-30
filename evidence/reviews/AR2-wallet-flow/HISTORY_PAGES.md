# Signed history pages: wire prerequisite

The new codec wraps unchanged v1 history manifests as immutable leaves. Each
append produces only its binary-carry branches and one signed root; previously
published pages are neither loaded nor re-signed. Root and branch bodies reuse
the existing custody epoch key, signed Resource envelope, HistoryCommitment and
32 KiB bound. The root contains at most 64 canonical descending peaks. Actual
child verification binds wire hash, anchor, counts, expiry and transport routes.

The anchor follows the longest linked lifetime, preserving earlier paid leases
when a newer message expires sooner. Publication order may differ from message
sequence. A reference count counts declarations, including repeated operations;
it is not a delivery or unique-message count. A multi-reference old manifest can
be wrapped at a higher initial root revision without changing its bytes.

A signed root requires a separate consistency check against an earlier root.
Bounded branch proofs must preserve every live old peak at its original leaf
offset. Missing proof nodes, changed live prefixes, rollback, same-revision forks
and altered routes are errors. Expired peaks keep their offsets but need no
expired bodies. Exact retries accept the identical commitment. The existing
v1-only verifier rejects v2 roots, so this introduces no silent protocol switch.

## Test evidence

The independent Python generator uses the existing public fixture primitives
for canonical CBOR, Ed25519 and AES-GCM, not the Rust producer. Its main vector
contains 257 genuine encrypted originals/v1 leaves, 255 immutable branches and
every intermediate root. The Rust producer matches all wire bytes exactly; the
test decrypts every original and verifies its actual index binding. Publication
starts with sequence 2 followed by sequence 1. Shorter subsequent leases do not
shorten the root. Cold traversal returns 257 live references initially and two
live references plus 255 explicitly expired leaves after the shorter leases end.

Cold prefix proofs cover several boundaries up to 257 leaves; missing branches
and a genuinely signed fork replacing a live original fail. Twenty-four signed
hostile roots pass generic signature verification and fail the typed root codec.
Further cases bind real wrong leaves, claimed route substitutions, malformed
signed branches, a two-reference v1 leaf wrapped at revision 42 then appended at
43, and an expired leading peak followed by a live peak at offset 2. The last
case checks exact later append bytes and proves the live suffix without loading
expired prefix bodies.

The context-free backend-test-critic first requested the multi-reference legacy,
leaf-binding, malformed-branch and expired-prefix-offset cases. Those were added
without changing the original vectors or assertions, then accepted before any
production implementation. The accepted test/fixture hashes and initial contract
hash are retained in the machine report. RED stopped at missing APIs; it does
not establish pre-implementation runtime assertion failures. The first GREEN
passed all seven new tests and reported one unused import, removed before final
checks without changing tests.

Validation results are recorded in [history-pages-checks.json](history-pages-checks.json).
The six targeted crypto suites contain 28 distinct tests, including seven new
page scenarios; 31 frontend regressions also pass. These counts overlap historical
reports and must not be added as independent product coverage. Crypto all-target Clippy, formatting and whitespace also pass. All 41 focused
source hashes remained unchanged through final checks. This is not a complete
application build manifest.

## Required continuation

This is authenticated wire structure, append and verification only. There is no
claim of funded retention, network publication, recipient import or native paid
acceptance for these pages. The ordinary sender/recipient still use the old flat
manifest and its 128-reference limit.

1. Persist immutable pages, current-root fences and bounded pending publication
   state through the existing Core/Store transactions. Reuse prepared-envelope
   pages, and keep retries/late earlier messages independent of full inventories.
2. Retain/read pages through actual paid index admission and occupied byte quotas.
   An acknowledged live root must not lose required pages to GC. Do not let an
   anchor hide the cost of unlimited retained versions or create unfunded storage.
3. Connect ordinary bounded sender and recipient continuations, cold consistency
   retrieval, and per-reference imports committed atomically with MLS/messages/
   dedup. Missing or corrupt live pages remain visible gaps.
4. Run the complete native gate with over 128 simultaneous paid originals,
   disjoint rosters, shorter later leases, cold restarts, SQL failures and actual
   sender/data/index loss. MLS epochs/Welcome, repair, E11 and all 67 cards /22
   E2E /three platforms remain required.
