# Exact manifest commitment in private locators — tests before implementation

Extend encrypted Locator with an optional HistoryCommitment. With None, preserve
the exact v1 CBOR payload and omit the new JSON field; existing saved locators
without the field still deserialize. With Some, encode canonical v2 plaintext:

    [2, indexId, endpoints,
     [epoch, revision, issuedAt, expiresAt, anchorOperation,
      SHA256(anchorDescriptorWire), SHA256(manifestWire)]]

Use the same mailbox signed envelope, key derivation, nonce/AEAD, fixed 2048-byte
padding, daily windows, 4 KiB record bound and checkpoint hash over the complete
canonical plaintext. Do not encode commitments in endpoint strings. A commitment
is derived only from a VerifiedCustodyHistory; public deserialized claims remain
untrusted until exact verify(domain,index,anchor,manifest,now) succeeds. All fields
bind actual verified manifest bytes. Zero revision/hashes/operation, malformed
lengths/versions, noncanonical/trailing payloads fail. Pointer issuance cannot
precede the manifest or extend its expiry, including the complete outgoing batch
retention even when one daily record ends earlier.

Core generic publication may advertise a history commitment only for its exact
currently committed outgoing manifest and current MLS direction/epoch. Incoming
pointer acceptance checks current MLS index/epoch. In one epoch, after a history
locator has been accepted/published, a higher mailbox sequence cannot downgrade
to v1, reduce history revision/issuance, or equivocate within one history revision.
The same exact history may be advertised at new endpoints with a newer pointer
sequence. Legacy peers remain supported before this transition.

accept_custody_history_for_pointer requires the current live committed mailbox
head and its exact commitment before delegating to the existing atomic incoming
history commit. A new pointer invalidates an in-flight older directory; absent,
expired, stale or substituted directory bytes never become empty-history success.
Directory acceptance alone does not import ciphertext or move fetch progress.

On cold load, bind saved plaintext Locator to its actual publication records
(decrypt/reverify current-epoch records at their relevant daily window), and bind
saved read-head Locator to its checkpoint's canonical head_hash. A changed stored
claim must fail closed, without healing on read/retry. This also strengthens the
shared legacy saved-state validation without changing its wire or user flow.

Three new crypto tests use independent Python CBOR/AES-GCM/Ed25519 vectors and
real descriptor/manifest signatures; every hostile locator has a valid outer
signature and AEAD. Four Core tests use actual SQLCipher/MLS and reuse existing
history helpers; they cover cold exact publication, INSERT/UPDATE rollback,
cross-anchor stale fetch, genuinely signed newer-pointer downgrade/equivocation/
earlier issuance, uncommitted outgoing claims, incoming scope and cold corruption.
Existing Locator literal tests receive only history:None; shared exporter helper
is factored without changing signing behavior.

Critic revision adds a genuine same-anchor/same-revision/lifetime alternate
manifest before Bob's first incoming history commit; exact pointer binding must
reject it without writes, then accept the original. A separate cold endpoint-only
mutation leaves outgoing history/commitment/records unchanged and must fail read
and retry, isolating plaintext-to-ciphertext verification. A real MLS transition
accepts fresh history revision one with the advancing mailbox sequence. The
independent legacy vector also checks the canonical Locator head hash.

No native multi-book, provider placement, reference-bound message import, Welcome,
retirement or R10 claim. These still require ordinary transport integration and
the full accepted 67 cards /22 E2E /three-platform scope.
