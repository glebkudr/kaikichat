# Finite signed history manifest — tests before production

Preserved pre-implementation contract. See [README](README.md) for subsequent
critic acceptance, current-input RED and implemented checks.

Business prerequisite: a latest locator must eventually route to all retained
messages across funded-book index rosters without requiring one common index.
Use a finite manifest of exact descriptor references and candidate index keys.
It is anchored to the longest-lived retained descriptor; a later shorter lease
must not cut off earlier history. References retain their individual expiry.

This slice adds only the cryptographic format and verifier. It does not claim
funded placement, storage admission, actual holder availability, a complete
conversation, cross-MLS-epoch access, durable checkpoint persistence, automatic
manifest publication or retrieval. Those require subsequent Core/store/native
integration and real funded-book/epoch tests. Candidate keys become usable only
with full paid-index and actual-peer verification in that integration.

The manifest is a domain-separated signed Resource from the existing custody
direction key. Canonical body: [purpose, indexId, epoch, anchorOperation,
revision, references]. Each sorted reference contains SHA256(descriptor),
operation, envelope sequence, original expiry, and 1–4 sorted unique index keys.
The anchor reference must exactly match the supplied signed descriptor. Every
reference was live at manifest issuance and expires no later than the anchor;
the manifest expires exactly with the anchor. It contains 1–128 references and
at most 32 KiB. Exact descriptor verification remains mandatory after discovery.

Tests use independently generated Python Ed25519/AES-GCM/CBOR vectors with real
authenticated envelopes, not a mirrored Rust encoder. Seven Rust tests cover
exact deterministic wire/export/fetch, genuinely signed malformed and hostile
inputs, altered descriptor-reference fields, differing retention, checkpoint
rollback/equivocation across a real anchor change, foreign direction/epoch isolation and the maximum live directory.
Same-revision changed content is rejected, including changed routes. Legacy
descriptor bytes remain unchanged. The four candidate keys match the existing locator/source budget; ten paid index promises remain a separate obligation. Existing 48 KiB body /64 KiB document limits stay unchanged. No production code has been changed yet.

Required: separate context-free backend-test-critic ACCEPT before implementation;
then actual RED baseline, affected crypto/Core/paid-index tests, frontend chat
tests and Clippy/fmt through the managed build wrapper. No new release claim.
