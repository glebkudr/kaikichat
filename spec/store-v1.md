# Encrypted profile store (V1)

`ProfileStore` (`crates/store`) keeps one local profile in SQLCipher: its
root key, versioned state, message history and outgoing work. The caller
supplies the 32-byte master key (the daemon gets it on stdin,
[node-runtime-v1.md](node-runtime-v1.md); the host keeps it in the keychain
or a password-sealed file, [desktop-host-v1.md](desktop-host-v1.md)). Domain
cryptography and authorization belong to its callers.

## Opening

- SQLCipher is required and the key is checked by reading the existing pages
  before any schema or journal write, so a wrong key never resets data. WAL,
  `synchronous=FULL`, foreign keys, temporary pages in memory, and an
  exclusive OS lock on a lock file beside the database for the store's
  lifetime (a second writer gets `ProfileInUse`). No plaintext copy of the
  key is saved.
- The schema is version 2 (`user_version` and the `profile_schema` row:
  version, supported read and write versions, migration step). A version-1
  file is first copied to `<db>.migration-v1.bak` (after a WAL checkpoint,
  still encrypted), then migrated in one transaction; any other version is
  `InvalidProfile`.
- On first open the owner's Ed25519 seed is drawn from the OS generator and
  stored inside the same transaction. The public identity is the key and its
  network id, `ain1` and the hex SHA-256 of the key. Signing is an internal
  operation of the core, never an exported tool; key material has no
  `Debug`.

## Tables and commits

- `states` (namespace, revision, bytes) with compare-and-swap revisions (a
  missing state is revision 0, the first commit makes it 1);
  `state_records` (fragments of a state keyed under its namespace, used for
  MLS records, [mls-adapter-v1.md](mls-adapter-v1.md)); `messages` (one local
  sequence for incoming and outgoing records, no network-wide order);
  `operations` (operation id, request hash, message); `outbox` (message,
  destination, wire).
- **Outgoing commit:** operation id, the core's request hash, the local
  message, destination and wire, and 0–16 state changes, all in one
  transaction; if any write fails nothing is visible. Duplicate namespaces in
  one batch are refused. The same operation id with the same hash returns
  the original message (its wire stays as queued) and applies only the
  caller's retry states (the agent broker's fence), not the new message's
  state, even if the retry prepared other ciphertext; another hash is
  `IdempotencyConflict`.
  Acknowledging removes the outbox entry and keeps history and the
  operation; retrying an acknowledged operation does not queue it again.
- **Incoming commit:** a verified message and state changes. The same id with
  the same content returns the first sequence and applies nothing; the same
  id with other content is refused. Received messages never enter the
  outbox.
- Two expression indexes over the JSON event kind (`messages_by_event`,
  `messages_by_event_sequence`) serve recent-text and unread queries; they
  are created on open without rewriting any record, and opaque non-JSON
  content stays valid.

## Limits

Message content ≤ 49 152 bytes, wire ≤ 1 MiB, each state value ≤ 32 MiB,
≤ 16 state changes per commit, identifiers and namespaces 1–256 bytes, list
pages of 1–1000. State records: ≤ 100 000 per state, keys ≤ 4096 bytes,
≤ 32 MiB in all. Bounds are checked before writing; errors are typed, not
panics.

## Tests

`crates/store/tests` use test-only SQLite triggers that abort each write in
turn: fault injection through the real database, checked through the public
API. They do not claim crash or fsync safety of a whole process.
