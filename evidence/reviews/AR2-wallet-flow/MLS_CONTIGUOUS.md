# Staged contiguous MLS receive — prerequisite passes, Core recovery still fails

`MlsClient::decrypt_contiguous` now prepares an authenticated application receive
without skipping a future sender generation. An otherwise valid future frame
returns `ReceiveGap` without exposing plaintext or a prepared snapshot. Corrupt,
replayed, wrong-group and wrong-AAD input retains its rejection semantics.
The caller's snapshot never changes during preparation or failure.

The implementation shares the ordinary decrypt path and the pinned OpenMLS
public configuration API. A staged candidate uses maximum forward distance zero;
successful preparation restores the original canonical group configuration.
An unknown configuration is rejected rather than rebuilt with defaults. The
128-generation window, normal forward bound 1000 and three past epochs remain.
Only the precise future-generation refusal triggers an ordinary bounded decrypt
from the original snapshot to authenticate the deferred frame. That candidate
state is discarded and its plaintext is explicitly zeroized. No secret-tree
serialization parsing, additional secret archive or new wire format was added.

Independent review required a successful contiguous commit after three real
epoch changes, then cold reopen and decryption of the delayed original. This
guards against an upstream configuration change immediately truncating retained
epoch secrets. The fourth-epoch refusal remains. The revised tests were accepted
before production; a later iterator-only Clippy correction was accepted separately.

[Final checks](mls-contiguous-checks.json) record 21 passing MLS scenarios,
including three new tests, raw OpenMLS interoperability, existing reordering and
epoch-retention checks. The new test receives offsets `1..129` as explicit gaps
with unchanged state and a restart, then recovers all 130 exact plaintexts and
senders after offset `0` and retries. It is a crypto prerequisite, not the Core
history test.

The same frozen run passes another 69 backend and 31 frontend scenarios,
Core/crypto all-target Clippy and formatting. All 794 captured inputs remain
unchanged. It also repeats the mandatory Core recovery test, which **still fails
at offset 0**. The aggregate correctly exits 1 and reports `passed: false`, with
only `mlsPrerequisitePassed: true`. Counts overlap earlier reports. No native or
GUI scenario was run here, and automated tests do not access Keychain.

Ordinary Core receive is unchanged. The primitive prevents new skips but cannot
protect previously skipped keys while the head continues to advance. Its Core
integration therefore needs a durable admission/cursor policy and direct-delivery
fencing, with a proven transition from the initial epoch or an explicit recovery
outcome for an older unguarded epoch. Missing/expired originals must remain visible;
signed custody sequence must not be assumed to equal MLS generation. Keep the
original Core arrival order and final all-130 outcome when adding backpressure.
Welcome, epoch/control ordering, rejoin and the ordinary paid graph recovery gate
remain required. Full V1 is not complete.
