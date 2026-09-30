# Reference-bound atomic import — tests before implementation

Core exposes read-only progress for a currently loaded manifest matching the
current live mailbox pointer. Progress lists declared operations in manifest
sequence order as imported, pending or expired (unimported). This is the finite
directory's local state, never proof of complete conversation history. Missing
pointer/directory, mismatch or expired anchor is unresolved/error, not an empty
successful list. Expired missing references remain explicit.

prepare_custody_history_read(conversation, operation, target, now) requires a live
listed reference and one of its declared candidate index keys. Reuse the existing
target-bound 30-second recipient capability: after_sequence=reference.sequence-1,
limit=1 and max_bytes=262144. Planning and descriptor acceptance perform no writes.
accept_custody_history_descriptor checks the live capability, current exact
manifest commitment and exact reference-bound signed descriptor. It returns an
opaque local fetch token with 120 seconds from descriptor acceptance; remote
capability expiry after acceptance does not invalidate that local work.

complete_custody_history_fetch rechecks current pointer/manifest/MLS scope, token
time, descriptor and ciphertext. It decrypts the actual original signed packet,
then commits message/MLS/dedup and that operation's completion in one existing
SQLCipher transaction. No empty/missing ciphertext can complete a reference.
Unrelated later imports cannot skip earlier missing references or advance the
legacy direct/index peer bookmarks. Concurrent prepared fetches for distinct
operations merge with the latest saved progress via normal SQL CAS; they need
not re-fetch merely because another reference committed first. Exact completion
retry performs no write. A duplicate previously delivered original can commit
only its progress without advancing MLS or adding a message row.

One bounded versioned state row per conversation/epoch under
custody/history-imports/ contains at most 128 operation records with exact descriptor
commitment, sequence, expiry and original message ID. Current references must
match those records; their original incoming messages must exist in the correct
conversation/direction. Use existing bounded state serialization and atomic
receive helpers. New manifest revisions preserve completed unchanged references;
tokens from older revisions fail. Rows from earlier actual MLS epochs remain
separate and cannot satisfy a fresh epoch's progress. Removed expired references
may be pruned on a later successful import; pruning is not a completeness claim.

Eight tests reuse genuine SQLCipher/MLS/history-pointer fixtures: out-of-order
disjoint candidates/cold gaps, exact descriptor/ciphertext and target guards,
actual first/update progress and message SQL faults, direct-delivery dedup and
overlapping sources, changed manifest with the same anchor/pending directory and concurrent imports,
retention/local-token time, real MLS epoch transition and cold broken message
reference (missing ID, real own/foreign incoming ID, or changed descriptor commitment).
No paid network response is simulated as authentic; actual transport,
operator/funding/QC/holder checks, multi-book recovery, Welcome/control, retirement
and R10 remain necessary. Preserve full 67 cards /22 E2E /three-platform scope.
