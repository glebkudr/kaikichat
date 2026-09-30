# Durable selected transport binding floors

Pre-implementation test contract extending compact discovery and service discovery.
This module protects authenticated selected transports across process restart. It
does not yet persist service addresses or complete V1.

Both Core `verify_finalizer_binding` and `verify_finalizer_peer` must use the same
durable version barrier before returning `CheckedFinalizerPeer`. The existing
current checkpoint, full selected committee, expected P256 key, actual Ed25519
transport, signature and original finite lifetime checks remain mandatory.

For a given network, base transport committee, epoch and selected public key:

- Strictly newer binding issuance supersedes the previous binding.
- An exact binding duplicate remains valid within its original lifetime and
  does not rewrite the binding-floor state.
- Older issuance and equal-issued different binding bytes are refused, including
  across compact/full-presentation API boundaries and cold restart.
- Malformed signatures, an incorrect expected key or actual transport, and a
  presentation that fails current selection cannot advance the barrier.
- A refused claim may advance the existing durable observed clock, as before;
  it cannot replace a valid binding floor.

Reuse the existing encrypted SQLCipher `states` table, CAS and checkpoint
`save_checkpoint_states` transaction. Store at most 64 live signed floors in
`l2/finalizer/transport-floors`, with a bounded serialized representation. Store
the original signed compact binding and enough checked metadata to validate it
again at its signed issuance when loading. No new database, owner IPC mutation,
signature format, raw public-key authority constructor or libp2p dependency in
Core is needed.

Checkpoint clock and floor changes commit atomically before verified transport
escapes. A real storage failure overrides success and leaves both states
unchanged. A duplicate at the same observed time writes neither state. Current
head/membership checks are repeated even when a retained binding matches.

Expiry is not permission to forget a newer binding while an older signature can
still be live. Retain its barrier until `issued_at + 60` (the existing maximum
transport binding lifetime), using checked arithmetic and the durable monotonic
checkpoint clock. Prune only beyond that bound. Refuse live overflow rather
than evicting a barrier. Original signature expiry still controls route use;
retaining a barrier never extends authority or permits voting.

Tests first, using existing two-chain canonical registry fixtures, encrypted
profiles and public-test registrar keys:

1. Observe a rotated transport, cold-reopen and refuse an older still-valid
   signature through either API. Preserve exact duplicates, refuse conflicting
   equal issuance and accept a strictly newer positive control.
2. Wrong key/transport and damaged signature do not poison the valid floor;
   recovery succeeds after cold reopen and correct authentication.
3. Actual SQL INSERT/UPDATE failures at the floor and checkpoint boundaries
   release no verified transport and preserve all persisted `l2/%` states. After
   fault removal and reopen, a valid update succeeds and old replay is refused.
4. A genuinely signed newer short-lived binding expires before an older one;
   cold reopen cannot revive the older transport. Clock rollback is refused,
   while a fresh binding at a valid later time succeeds.
5. A real renewed checkpoint and freshly proven roster preserve the same base
   committee/epoch floor across cold restart. The full old/new presentations both
   carry the refreshed roster, so stale checkpoint rejection cannot mask a lost
   binding floor. The latest binding remains usable without extending its lease.

The existing full-presentation deadline test first checks that a refreshed roster
preserves the old binding's original expiry, before observing a newer binding.
It then requires old-version refusal and a successful exact newer retry. This
updates its previous old-after-new success expectation to the new version rule.

The no-context backend critic must accept the tests before production changes.
Then preserve actual RED, focused GREEN, full backend/frontend results and the
existing real TCP/QUIC selected-service gate. Rebuild and verify the ordinary
Tauri package from the same sources. Address-cache persistence, committee
capacity, DHT roles, private ciphertext custody/repair and full V1 remain open.
