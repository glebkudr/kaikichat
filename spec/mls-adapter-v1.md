# Staged MLS adapter (V1)

`crates/crypto` wraps OpenMLS 0.9.0 with RustCrypto 0.6.0 (RFC 9420) behind
staged operations, for the trusted Rust core only. One ciphersuite:
`MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519`. No custom network
cryptography, plaintext fallback, crypto or content debug output,
`test-utils` or file persistence features. Dependency notes:
[dependency-decisions.md](dependency-decisions.md#openmls-2026-09-05).

## Staged state

- An `MlsClient` owns an opaque secret snapshot. Every operation that
  changes state returns a `Prepared<T>`: a value and the **next** snapshot;
  reads (`epoch`, `members`, `view_commit`, `ratchet_tree`, the mailbox
  secrets, …) return plain values. The input snapshot never changes, not
  even on errors.
- The caller commits the next snapshot with the state's revision check in the
  same SQLCipher transaction as its history, inbox or outbox
  ([store-v1.md](store-v1.md)), and sends wire bytes only after that commit.
  A failed preparation or commit is discarded.
- The snapshot holds OpenMLS's memory-provider records in bounded CBOR. It is
  persisted as separate records (`state_records`), so one operation loads
  only the profile's own records and those of its group
  (`restore_scope`); a legacy whole snapshot still restores. Key material
  has no content-revealing `Debug` and is zeroized on disposal. Snapshots
  never reach the renderer or agents.

## Operations

- **Keys and groups:** `key_package`, `last_resort_key_package`,
  `create_group`, `join` (a Welcome with the ratchet tree) and
  `join_with_tree` (the tree given apart), `forget_group` (after a removal,
  to join again), `members`, `epoch`, `is_active`, `knows_group`.
- **Commits:** `add_members` and `commit_changes` (adds, removals and the
  group's data in one commit) prepare a **pending** commit; sending while one
  is pending fails with `PendingCommit`. `activate_pending_commit` merges it,
  `clear_pending_commit` drops it when another commit took the epoch,
  `view_commit` shows what another member's commit does and `apply_commit`
  applies it. The adapter proves no ordering, roles or finality: the core
  applies commits in the order the group's notary names
  ([groups-v1.md](groups-v1.md#commits)) and checks roles and bans itself.
  (`remove_member` and `apply_finalized_commit` remain for tests.)
- **Group data:** a private-use GroupContext extension (`0xF1A0`) that every
  member agrees on and only commits change; a client whose leaf cannot carry
  it is `Outdated`.
- **Large groups:** `set_tree_apart` makes later Welcomes carry no ratchet
  tree; newcomers get it from `ratchet_tree` through the group's mailbox
  ([Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md](../Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md)).
- **Mailbox secrets:** `group_mailbox_secret` (one per epoch, for every
  member) and `mailbox_secret` (per direction, for a direct contact) are
  exported from the committed epoch without advancing any ratchet.
- **Messages:** `encrypt`, `decrypt`, `decrypt_contiguous` (below),
  `inspect_application_message(s)` and `ready_application_messages`
  (authenticate sender and generation on a strict copy, without consuming
  the ratchet), `message_epoch`, `welcome_for`, `inspect_key_package`
  (returns the verified credential and key for the core's root binding).

## Checks

- The expected group id and application AAD are mandatory on receipt; the
  AAD binds the network and conversation. Wrong AAD or group, tampered,
  concatenated or truncated TLS frames, replays, a Welcome for someone else
  and unsupported suites fail without changing the committed snapshot.
- Application messages are MLS `PrivateMessage`s. The sender is the verified
  leaf credential. `join` refuses a Welcome for another group and never
  replaces an existing one (`AlreadyJoined`).
- New members cannot read traffic from before they joined; removed members
  cannot read later epochs.

## Bounds

Plaintext ≤ 49 152 bytes, AAD ≤ 1024 bytes, application wire and
KeyPackage ≤ 65 536 bytes, control messages and Welcomes ≤ 1 MiB, a ratchet
tree given apart ≤ 4 MiB, the snapshot ≤ 32 MiB with ≤ 100 000 records and
keys ≤ 4096 bytes, ≤ 1024 adds and removals together per commit. The sender
ratchet tolerates reordering within 128 generations and a forward distance
of 1000; 3 past epochs are kept for late messages. Sizes are checked before
anything is persisted.

## Receive order

Committing a later generation first evicts the keys of skipped ones from the
128-generation window, and stored ciphertext cannot bring them back: 130
live messages of one epoch, received in the order 1..129, 0, lost message 0.
So the core receives in each sender's order.

- `decrypt_contiguous` validates like `decrypt` but prepares a new snapshot
  only when OpenMLS can receive without skipping a generation (a staged copy
  with forward distance 0; the stored configuration is unchanged). A valid
  message from the future is `ReceiveGap`: no plaintext, no state, no side
  effect. Only that refusal is double-checked with an ordinary bounded
  decrypt of the original snapshot, whose result is discarded, so malformed,
  replayed or wrong-context frames stay errors.
- The core's application state is version 2: every direct contact names its
  policy, `contiguous` or `legacy_bounded`. New contacts and new profiles
  are `contiguous` from the first Welcome; group messages are always
  received contiguously. A version-1 profile stays readable; creating its
  first new contact marks the old ones `legacy_bounded` in the same
  transaction. A missing or unknown policy in version 2 is invalid. An old
  ratchet is never certified as contiguous.
- A gap changes nothing: no receipt, no stored message, no MLS advance.
  The mailbox client holds such envelopes per conversation in their sender's
  order (at most 256) and imports them once the predecessor arrives; the
  sender's outbox keeps retrying direct delivery until acknowledged.

Tests: `crates/crypto/tests/mls_flows.rs` (with its
`support/contiguous_receive.rs`, `incremental_persistence.rs`,
`scoped_records.rs`), `tree_apart.rs`, `large_group_measure.rs`, and in
`crates/core/tests` the `receive_admission`, `groups` and `mailbox_swarm`
suites.
