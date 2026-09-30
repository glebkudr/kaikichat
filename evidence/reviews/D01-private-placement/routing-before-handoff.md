# Selected finalizer transport binding v1

This is a finite discovery proof connecting a selected public P-256 key to a live
libp2p Ed25519 transport peer. It grants no voting, spend, custody or application
permission. The selected voting service and its application admission policy remain
separate required integrations. Missing routes never shrink the selected committee
or its quorum, and never cause resampling.

## Signed wire

Canonical definite-length CBOR envelope `[body_bytes, signature_bytes]`. The body is:

```
["ain-finalizer-transport-v1", network32, committeeId32, epoch,
 compressedP256Key33, transportEd25519Key32, issuedAt, expiresAt]
```

The signature is the upstream Commonware P-256 standard signature over namespace
`ain-finalizer-route-v1` and the complete body (Commonware union_unique encoding,
SHA-256, ECDSA, normalized low-S). The same encrypted P-256 scalar was enrolled with
the owner-bound on-chain proof of possession. The wire is limited to512 bytes.
No cryptographic implementation or new external dependency is introduced.

The verifier receives the expected selected public key from its local request and
the Ed25519 key from the actual authenticated Peer ID. It requires membership in the
complete verified committee, exact network/committee/epoch/key/transport fields,
valid signature, canonical body and envelope, no trailing data and a live authority.
`issuedAt` must not precede the canonical epoch validity start or exceed local time;
`now < expiresAt <= authorityExpiresAt`, with `expiresAt-issuedAt <=60 seconds`.
Signing uses `expiresAt = min(now+60, authorityExpiresAt)`.

A newer checkpoint can prove the same canonical committee. It may reverify an old
binding before its original expiry; it cannot extend that expiry. A presentation
must carry a roster proof for the receiver's actual current checkpoint. Historical
checkpoint IDs and proofs cannot be used as current authority.

## Core ownership and durability

`publish_finalizer_roster` verifies the full current registry snapshot and every
selected member before persisting one shared roster in encrypted SQLCipher state.
Policy remains owner-selected and immutable. A changed roster requires an exact CAS
revision; an unchanged retry accepts the saved or an older revision. The roster and
monotonic clock commit atomically before acknowledgement. Proof and member limits
are512KiB and16KiB respectively, at most64 members, and the stored record is at most1MiB.
The serialized public roster leaves1024 bytes of room for record metadata.

Every response requires the current head, a matching stored roster, an enabled local
selected key, fresh full verification and a successful durable clock commit. Restart
preserves the roster, key and role intent. Disabling the operator prevents new signed
responses. Already issued public signatures expire naturally; remote disable cannot
revoke a copied signature. Consumers must separately recheck service role/authority
and stop voting on disable, head change, lease expiration or shutdown.

Public carriers are deserializable untrusted data. `CheckedFinalizerPeer`, the checked
committee and verified route have no Deserialize/public constructor. Private scalars
are neither returned by owner IPC nor sent across the network.

## Daemon API and transport

All three commands require the existing owner IPC token; messaging agents cannot
call them. The daemon supplies time. Unknown fields are rejected.

- `publish_finalizer_roster`: same `checkpointId`, `epoch`, `registryProof`, `members`
  as `verify_finalizer_committee`, plus `expectedRevision`. Each member has JSON proof
  text and hex32 `qx`/`qy`. Returns revision, checkpointId, committeeId, epoch, validUntil.
- `check_finalizer_peer`: `peerId`, hex33 `publicKey`, `epoch`. Requires a live allowed
  connection, installed finalizer policy and a live current checkpoint. Returns requestId.
- `finalizer_peer_result`: `requestId`. Returns state, peerId, publicKey, epoch and result.
  A verified result has committeeId, publicKey, transportKey, bindingId, issuedAt,
  expiresAt and quorum. `bindingId` is SHA-256 of the exact signed envelope.

CBOR request-response protocol `/agentic-internet/finalizers/1` carries
`{checkpointId: [u8;32], epoch, publicKey: Vec<u8>}` and
`{presentation: Option<{roster, binding: hex}>}`. It shares libp2p's authenticated
TCP/Noise and QUIC connections and existing relay-only checks. Request and response
limits are512 bytes and2MiB, four concurrent streams, five-second timeout. Incoming
requests use the existing bounded global/per-peer admission mechanism.

The local owner queue allows four pending checks and retains at most16 jobs for up
to120 seconds. A full queue evicts the oldest completed job. Failed transport, denied
response and invalid proof produce `failed`, `unavailable` and `rejected` respectively.
Every read of a verified result repeats Core proof/signature/time verification against
the same still-live connection. Expiry, head change or connection replacement yields
`stale` with null result. Network configuration replacement also invalidates jobs.
`node_info.finalizers` exposes pending, retained, served and rateLimited counters.

## Evidence and remaining work

Tests begin with independent Python/OpenSSL canonical-CBOR vectors and real two-chain
registry proofs. Core scenarios cover enabled selected keys, another receiving Core,
current-head renewal, short expiry, restart, bad signatures/rosters and atomic storage
failure. Actual daemon tests cover owner/agent separation and forbidden overrides.
The fresh EVM network scenario pays for17 actual daemon-generated keys, verifies the
full selected roster, uses actual TCP/Noise and QUIC endpoints, captures a valid proof
and rejects its replay from another Noise peer, proves four received requests and the
absence of a fifth, changes head, restarts, disables/re-enables, and observes both the
short binding expiry with live head and later head expiry using the daemon's own clock.

This module supplies authenticated routes, not a running selected consensus service.
P01 completion still requires selected daemon lifecycle/channel integration and bounded
application log admission; the full V1 tasks and E01–E26 acceptance remain open.
