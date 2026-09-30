# Staged contiguous MLS application receive

Status: implemented prerequisite after independent test acceptance. All 21 MLS
tests pass, including preservation of three past epochs across a contiguous
commit. [The initial prerequisite run](../evidence/reviews/AR2-wallet-flow/MLS_CONTIGUOUS.md)
retains its then-failing Core gate; the subsequent
[Core admission integration](../evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md)
now recovers all 130 originals for protected new conversations.

R19 is reproducible with 130 live originals in one MLS epoch: committing arrival
order 1..129,0 evicts the key for original 0 from the existing 128-generation
window. Stored ciphertext alone cannot restore it. Preserve that failed Core
gate; do not reorder its inputs or enlarge the key-retention window.

Add an internal staged `MlsClient::decrypt_contiguous` operation. Like `decrypt`,
it validates the exact group, private application frame, AAD, sender credential
and payload limits, and never mutates the caller's snapshot. It prepares a new
snapshot only when OpenMLS can receive without skipping a future generation.
An otherwise valid future application returns a typed `ReceiveGap`, with no
plaintext, prepared state or persisted side effect. A malformed, replayed or
wrong-context frame remains an error, not a successfully authenticated gap.

Use the pinned OpenMLS public group configuration API for a staged candidate,
setting maximum forward distance to zero. Preserve the configured 128-generation
reordering window, three past epochs, and the original stored configuration on
successful preparation. Only the exact future-generation refusal may be checked
by an ordinary bounded decrypt of the original snapshot to distinguish an
authenticated application from invalid data; discard that candidate and its
plaintext. Do not parse or modify serialized secret-tree internals, retain extra
epoch snapshots, or introduce new crypto libraries or wire formats.

Test 130 genuine applications: all 129 future frames are deferred without state
changes, including after restart; receiving original 0 then retrying 1..129
recovers every exact plaintext and sender. Test wrong context, malformed future
frames, replay, a previously retained skipped generation, ordinary reordering
after a committed contiguous receive, and the existing three-epoch bound.

This operation prevents new forward skips. It does not discover or protect older
skipped keys already present in a snapshot, restore evicted keys, or establish a
durable catch-up policy. Core must not automatically activate it midway through
an arbitrary existing ratchet and claim that older gaps are safe. Core now
persists the policy from contact creation and guards direct delivery; legacy
contacts remain explicitly unguarded. Network integration still needs bounded
refetch scheduling and explicit gap/epoch/rejoin outcomes. The prerequisite alone
did not switch ordinary receive; the separate Core admission change does. Paid
graph recovery, Welcome and multi-epoch gates remain open.
