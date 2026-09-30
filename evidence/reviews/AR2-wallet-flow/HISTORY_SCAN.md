# Durable retry of pending history references

The ordinary v1 recipient now commits one constant-size cursor per conversation
before claiming a signed reference. A bounded work holds one reference and at
most 16 visited operations. A cold restart continues after the last selection;
circular retries keep unimported originals eligible. Growing history preserves
position when that operation remains declared. The cursor is an attempt marker,
not an imported message, source bookmark or completeness claim.

The real Core/MLS fixture encrypts 17 originals in order 17,1..16 and declares
custody order 1..17. The first 16 defer without message/MLS/import/ACK writes.
A full Runtime restart with no RAM cursor reaches original 17; subsequent retries
recover all original IDs, authors and text. Repeated worker loads cannot re-claim
an active reference: a real SQL trigger rejects any repeated cursor UPDATE.

Core tests also cover INSERT and UPDATE failures followed by cold retry, live
history growth, stale pointer rejection, expired/imported skipping and an
exhausted visited pass without writes. Only a single small scheduling row changes.
Current signed history and descriptor/ciphertext admission remain authoritative.

ReceiveGap now increments `custodySync.deferred` and releases the entire attempt
before selecting another reference. The shared lifecycle code treats its payload
as opaque: production owns the paid Plan; a test owns a genuine target-bound Core
PreparedHistoryFetch for the same selected original. This proves payload/routing
cleanup without combining incompatible paid and MLS fixtures. Malformed data
keeps candidate fallback; ordinary unavailable/transport failures retain holder
fallback. Paid admission remains covered by separate transport and native tests.

[Checks](history-scan-checks.json): **72 backend and 31 frontend passes**, Core/node
all-target Clippy and formatting. The unchanged 130-live-original Core gate is
included. Each candidate kept 802 captured inputs stable. Candidate 1 stopped at
a test-helper type-complexity lint after all backend passes; independent review
accepted a type alias, and the four affected tests plus remaining checks ran in
candidate 2. Counts overlap preceding reports and exclude duplicate reruns.

[The ordinary paid CLI regression](history-scan-native-c2.json) passes payment,
budget/exact retry, shared CLI/MCP/desktop projections, sender SQL failures,
retirement, exact page ACKs after restart, and both originals recovered after
actual 18 data and 18 index-copy loss with sender absent. Six QC signatures were
verified; owner sender-work/retrieval calls are zero and teardown is clean.
This uses the already-reviewed separate setup conversation. It is not a native
17-reference gap test or proof that an undeclared missing original can recover.
The previous same-chain missing-original native failure remains preserved.

The ordinary publisher and recipient still use flat v1 history. Connect bounded
v2 graph publication and traversal, preserving child ACKs before root/pointer
publication and complete declared-range accounting. Then verify ordinary paid
recovery above 128 simultaneously live originals. Explicit user-visible gaps and
legacy/rejoin recovery, first offline Welcome, control/epochs, independent R10
repair and the full 67-card/22-E2E/three-platform V1 remain open.
