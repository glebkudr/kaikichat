# AR3 reference-bound atomic import acceptance

Accepted Core prerequisite: each declared operation has independent import
progress under the current exact live pointer and directory. A later successful
message cannot hide an earlier missing object. Progress reports imported, pending
and expired-unimported operations, without declaring complete conversation history.

The [contract](../../../spec/custody-history-import-v1.md) reuses existing target-bound
capabilities, full descriptor/ciphertext verifiers, custody packet opening and the
ordinary atomic receive transaction. Message identity, MLS state, dedup and the
operation record commit together. A failed progress INSERT/UPDATE or message
INSERT rolls back all state. Direct-delivered originals need only progress;
overlapping exact completion performs no SQL write or second MLS advance.

Eight tests received separate context-free **REVISE → ACCEPT** before production
([critic](critic.md)). The revision isolates a same-anchor manifest update and
real existing wrong-scope/direction message references in cold corrupt progress.
[Baseline-2](baseline-2.json) against `16e264a` has one E0432 and 61 E0599 missing
API/type errors; no runtime assertion ran in that RED. Earlier [baseline-1](baseline-1.json)
is preserved. [Candidate-1](candidate-1.json) passes all eight accepted tests.

The final [affected gate](checks.json) passes **57 backend /21 frontend**,
production Clippy and fmt, with **604 unchanged inputs**: 41 Core custody tests,
six Core mailbox, one custody-node and nine mailbox-node regressions. These are
targeted checks for this Core change and the shared helpers. Unchanged crypto and
paid-store groups were not repeated. No new native manifest process gate is
claimed. Source maps and log hashes record the actual tested inputs; raw logs
remain in `output/ar3-history-import/`.

Coverage includes genuine MLS/SQLCipher, three distinct candidate indexes,
out-of-order originals, cold gaps, expired remote capabilities with still-live
local fetch tokens, malformed/wrong descriptor and ciphertext, target fences,
retention/time boundaries, real MLS epoch transition and corrupt stored references.
A newer pointer with no matching loaded directory fails unresolved; accepting
that directory preserves unchanged completed imports but invalidates older read
and fetch tokens. Fresh simultaneously prepared distinct fetches both complete.
Cold reads check original incoming message existence, conversation/direction/
author and reference metadata. Read/planning never heal corrupt state.

This does not authenticate remote paid index/holder responses, prove native
multi-book retrieval or replace the runtime's live-job index-roster intersection.
[Next](NEXT.md): bounded native manifest put/read and exact paid anchor response,
ordinary sender/recipient use, funded disjoint books, Welcome/control, successful
retirement, attachments and autonomous index/data R10. AR-R03 and V1 remain open;
preserve 67 cards /22 E2E /three platforms. No push.
