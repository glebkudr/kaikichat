# Signed application document, V1

Status: F02 first implementation slice, tests before code. This defines the shared signature wrapper, not all domain schemas or full F02 completion.

The application document is a definite CBOR array `[unsigned_bytes, signature_bytes]`. `unsigned_bytes` is the exact deterministic CBOR encoding of a nine-element array:

1. Network/genesis domain: 32-byte byte string.
2. Protocol version: unsigned integer, 1.
3. Kind: unsigned integer (Identity=1, Invitation=2, Message=3, GroupControl=4, Credential=5, ServiceCard=6, Job=7, Review=8, Resource=9, GroupRoster=10, GroupCommit=11, PublicPost=12, PublicRoster=13, Directory=14, Part=15, GroupDoor=16, GroupMember=17, ChannelKeys=18, ChannelSubscriber=19).
4. Ed25519 author public key: 32-byte byte string.
5. Authority epoch: unsigned u64.
6. Issued-at: Unix seconds, unsigned u64.
7. Expires-at: unsigned u64 or null. When present, strictly greater than issued-at.
8. Application body: bytes, at most 49,152 bytes (a `Part`'s at most 65,344 bytes, so that it fills its envelope). Its own domain schema is a later layer.
9. Extensions: definite map from unsigned u16 to bytes. Keys are strictly increasing, unique. IDs 0..32767 are optional opaque extensions, preserved exactly. IDs 32768..65535 are critical, none are defined in this version and all are rejected. At most 16 extensions, each value at most 1024 bytes.

All arrays/maps are definite; integer/length representations use the shortest RFC 8949 encoding. No trailing bytes, CBOR tags or alternate representations. The full document is at most 65,536 bytes. The decoder checks limits before allocation/expensive cryptography. A signed but noncanonical representation is rejected, not silently rewritten.

Signature: pure Ed25519 over the concatenation of ASCII `AgenticInternet/signed-document/v1` followed by one NUL byte and `unsigned_bytes`. Verification uses strict Ed25519 verification. Object ID is SHA-256 of the complete canonical wire bytes, including signature. This is a signature and identity primitive, not E2EE or a proof of author honesty.

Verification requires the expected network domain and trusted local `now` from the caller. At `now == expires_at` a live document is expired. An issued-at value more than 30 seconds ahead of `now` is rejected; comparison must avoid u64 overflow. Optional no-expiry documents remain subject to application authorization/revocation/retention rules. Historical evidence verification must supply its own authenticated acceptance context; an untrusted timestamp does not bypass live admission.

`SignedDocument::sign` binds author to the supplied key. Only `VerifiedDocument::decode` exposes a verified parsed document; it verifies canonical encoding, signature, expected domain and time. Altering domain/kind/epoch/body/extensions invalidates the signature. Consumer code must additionally validate the appropriate body schema and actor permissions.

## Documents in parts

A document too big for one envelope (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 2) is signed as usual but may be up to 4,177,920 bytes (`SignedDocument::sign_large`, read with `VerifiedDocument::decode_large`; `decode` keeps refusing anything over 65,536 bytes). It travels as parts: documents of kind `Part` (15) whose body is the CBOR array `[whole, index, count, bytes]` — `whole` the SHA-256 of the whole wire, `index < count ≤ 64`, `bytes` 1–65,280 bytes of the whole wire in order. A whole is named by its hash and its number of parts. A reader keeps the parts of one (part author, whole) together, takes each place once, and releases the whole once, when all parts are there and their bytes hash to `whole`; parts of another author naming the same whole never join that assembly. The whole is then checked like any document; its own signature names its author.

Fixture `fixtures/wire/signed-document-v1.json` is produced independently with a fixed literal unsigned encoding and Node/OpenSSL Ed25519, using the public RFC 8032 example seed. The seed is a public test vector and must never be used for a user identity. The generator does not import the Rust codec or signature implementation.

Sources: [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html), [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html), [Ed25519 Dalek](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/).

