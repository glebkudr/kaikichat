# N03 — private mailbox codec and durable Core state

Historical codec/Core milestone. Subsequent actual DHT pointer publication/retrieval is recorded
in [N03-private-mailbox-network.md](N03-private-mailbox-network.md). Retained indexes, paid
admission, operator registry, R=10 custody and autonomous repair remain unimplemented. Neither
milestone closes N03, D05 or the full E01–E26 product acceptance.

## Wire and key contract

`MlsClient::mailbox_secret(group, domain, recipient)` uses the committed RFC9420 MLS exporter:

- Label: `AgenticInternet/mailbox-export/v1`.
- Context: canonical CBOR `[domain: bytes32, group: bytes32, recipient: credential bytes]`.
- Output:32bytes, only for an active local member, an actual recipient member and no pending commit.
- Same epoch/context exports the same capability through application traffic and restart. Removal
  changes the epoch/capability. The adapter does not change the MLS snapshot or application ratchet.

The capability and epoch result have no Debug/Serialize implementation. Secret intermediates use
zeroizing storage or OpenMLS secret buffers. Both/all current group members can derive a given
recipient capability; possession is a shared mailbox permission, not a unique sender signature.
The signed MLS application envelope must independently authenticate retrieved message authors.

For daily slot `floor(unix_seconds/86400)`:

```
PRK = HKDF-Extract-SHA256("AgenticInternet/mailbox/v1\0" || domain32, seed32)
sign_seed = HKDF-Expand-SHA256(PRK, "signing\0" || slot_be_u64, 32)
enc_key   = HKDF-Expand-SHA256(PRK, "encryption\0" || slot_be_u64, 16)
author    = Ed25519-public(sign_seed)
key       = SHA256("AgenticInternet/mailbox-address/v1\0" || domain32 || author)
```

The existing canonical `SignedDocument` wrapper is reused: network domain, version1, kind9
(`Resource`), authority_epoch0, issued_at, mandatory expires_at and empty extensions. Body:

```
["ain-mailbox-pointer-v1", slot, positive_sequence, nonce: bytes12, ciphertext: bytes2064]
```

AES128-GCM uses a fresh random nonce. Its AAD is canonical CBOR:
`["ain-mailbox-aad-v1", domain, slot, sequence, issued_at, effective_expires_at]`.
The locator is canonical CBOR `[1, index_id: bytes32, endpoints: array(text)]`, prefixed with its
u16 big-endian length and zero-padded to exactly2048bytes before encryption. There are1–4 nonempty
UTF-8 endpoints of at most256bytes each. Core also rejects control characters; transport adapters
must validate actual dial protocols/addresses before using them. This is not a multiaddr verifier.

Wire is bounded at4096bytes before generic parsing/signature work. Public admission verifies
network/signature/time, exact purpose/schema/canonical encoding, positive sequence, nonce/ciphertext
lengths, slot bounds and the key derived from the document author. Only the secret opener checks
the AEAD tag, canonical locator and all-zero padding. A valid outer signature alone is insufficient.

## Rotation, retention and rollback

Retention must be positive and no longer than30days. Publication includes every slot from
`issued/day` to `(retention_expiry-1)/day`, at most31 records. Each expires at
`min(retention_expiry, (slot+1)*day+30seconds)`. Public nodes can admit finite future records at
issuance. Readers derive only previous/current/next keys (two at Unix slot0), including after29days
offline; knowing a future key cannot bypass the read window.

A checkpoint retains the highest accepted sequence and SHA256 of the complete canonical locator,
including endpoints. Lower sequences and conflicting equal-sequence heads are rejected. A higher
sequence can replace a locator. Equal logical heads may be reencrypted across slots. Core refreshes
the authenticated expiry when such a new slot is accepted; a still-valid older slot cannot shorten
that expiry. Expired locators are hidden from route consumers while their checkpoints remain.

Thirty-one rotating pointers are finite resource obligations, not free metadata. A later network
publisher/admission layer must account for their storage, replication, lease duration and repair
budget along with the referenced index/manifest and ciphertext. The codec neither checks that an
index exists nor proves a storage operator independent or entitled to payment.

Visible metadata includes slot/sequence, publication and expiry times, record size, lookup/traffic
timing and batch correlation. Fixed padding hides locator length within this bounded class. There
is no claim of global-observer anonymity or secrecy from other holders of the same MLS capability.

## Durable Core integration

Trusted Core methods are additive and are not routed through the scoped agent broker:

- `prepare_mailbox_publication`: derive the remote recipient capability, create a finite batch,
  and save exact wire bytes, locator and sequence in one encrypted CAS state before returning.
- `mailbox_publication`: retrieve the latest current-epoch, unexpired batch after restart. Individual
  expired daily records remain in that finite batch; the publisher must check each at actual time.
- `mailbox_read_keys`: derive only the local recipient's current bounded window.
- `accept_mailbox_pointer`: verify/decrypt against the current MLS context and durable checkpoint,
  then commit the locator, expiry, sequence and hash atomically before releasing the result.
- `mailbox_head`: expose only a current-epoch, unexpired accepted locator.

`expected_sequence` is CAS. An exact lost-response retry requires `expected+1 == latest.sequence`
and unchanged locator/retention expiry; it returns the original stored random ciphertexts. Different
stale requests conflict. One versioned `mailbox/publish/<conversation>` state and one
`mailbox/read/<conversation>` state retain only the latest values, each bounded at512KiB. Epochs
come from real MLS state, not caller input. Current exports do not solve delivery of a future group
epoch to a removed/offline device; replicated group control and epoch recovery remain required.

These operations do not consume message outbox entries, advance MLS ratchets, create chats,
change delivery/replica counters, or reveal capabilities through UI snapshots/MCP. Durable SQL
failures return errors before any publication/head escapes. Read checkpoint/locator share one
state row and transaction. Application messages still use their existing signed receipt path.

## Test-first evidence

Crypto: new tests failed on missing mailbox/exporter APIs. Separate context-free
`/root/mls_test_critic` required exact read-window/expiry bounds, endpoint equivocation and
independently signed schema violations. Corrections, padding and maximum-locator checks received
FINAL ACCEPT before crypto production code. Six mailbox tests and eighteen OpenMLS tests pass.

The fixture generator uses independent Python hashlib/HMAC and `cryptography`, not the Rust
producer. All9 vectors reproduce exactly: valid, bad AEAD, nonzero padding, noncanonical body,
wrong kind/purpose, zero sequence, slot-expiry overflow and a correctly encrypted invalid locator.
Generic-valid signed fixtures distinguish public validation from secret decryption. An independent
raw OpenMLS group also exports the same context and agrees with adapter-derived addresses.

Core: six new tests failed on missing APIs. Separate context-free `/root/core_test_critic` required
daily equal-sequence expiry renewal and bounded durable storage. The revised contract received
FINAL ACCEPT before implementation. A later test-only `(u64,u64)`→`(i64,i64)` correction for
SQLite `FromSql` received separate ACCEPT without changing assertions.

Core checks actual independent profiles, MLS, SQLCipher and INSERT/UPDATE failure triggers:
month-offline bounded resolution after reopening; original batch retry; unchanged ratchet/history/
queue; real message decryption/receipt after lookup; checkpoint rollback/equivocation; positive daily
rotation; unchanged expiry on old-slot replay;18 successful generations with bounded row count and
bytes; latest state after reopen; wrong direction/conversation/key/signature; denied agent methods.
The month-offline test hands the existing MLS packet directly between Cores after pointer lookup;
it deliberately does not present that handoff as network storage or keeper retrieval.

Existing workspace versions of Ed25519, serde and serde_json were reused; no new package/version
was installed. No application message, invitation, profile SQL schema or public IPC contract changes.

## Full regression gates

Both full runners exited0 on2026-09-05:229 Rust tests,30 frontend tests, formatting/Clippy,
TypeScript/Vite, four actual hidden WKWebView flows, normal macOS release bundle, deep/strict
ad-hoc signatures and no WebDriver in the normal dependency graph. Apple notarization remains
unconfigured. Native log: `N03-mailbox-native-green.log`.

All7 current-source Linux network outcomes passed; sourceHash
`607bee0c8f691152c8bcc60ba0ef1c36af00ffa70c8a62d308a3ca2edb303c75`, runId
`ain-nat-56f8925b`, cleanupErrors`[]`. Exact run-label checks found no remaining containers or
networks. Report: `output/network-e2e/result.json`; log: `N03-mailbox-network-green.log`.
Own vision compared the current packaged native network/restored screenshot with the previous
verified result: controls, layout and relay policy presentation remain intact. The network/native
scenarios establish regression compatibility; the new mailbox APIs have not yet been wired into
network storage or native product flows. Full V1/E01–E26 acceptance remains open.
