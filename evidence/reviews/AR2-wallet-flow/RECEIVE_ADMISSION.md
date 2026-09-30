# Core receive admission from conversation creation

New conversations persist contiguous application admission from their initial
MLS state, on both invitation-creator and Welcome-recipient paths. The shared
Core reducer applies it to direct and custody traffic, including text and the
existing job application path. An authenticated future generation returns
`ReceiveGap`; it produces no ACK and advances neither MLS nor message/import
rows. Sender outbox entries remain pending. Reference proofs plus absence of
an import row preserve pending history across restart without a new receiver
ciphertext spool or an archive of MLS snapshots.

The required Core test keeps the actual first arrival order `1..129,0` for
130 simultaneously live originals. All 129 future attempts leave state unchanged,
including a cold reopen and a direct-delivery bypass attempt. After original 0
arrives, fresh reference-bound retries recover every exact ID, text and author.
Each newly imported original writes only its own import and sequence claim;
existing import rows are unchanged. The final 260 rows and all 130 original
messages survive cold dedup. These are real MLS/SQLCipher messages; this Core
test does not exercise 130 paid messages through ordinary network workers.

Profile state version2 requires an explicit per-contact receive policy. A
version1 profile remains readable without writes. The first new contact migrates
old contacts to `legacy_bounded` and protects only the new contact, atomically
with the original contact/MLS/outbox or Welcome transaction. Tests cover SQL
rollback, both initial roles, cold mixed-policy profiles, new legacy out-of-order
traffic after migration, and missing/unknown version2 policy rejection. Existing
contacts cannot certify a previously unguarded ratchet by admitting a page root:
root admission and retained-token use return `HistoryRecoveryRequired` in both
legacy representations, preserving old data and old retrieval.

The changed import tests preserve root/pointer fences, SQL rollback, the original
130-message outcome, cold progress and exact retries. A future index fetch stays
usable after the initiating read request expires once another authorized route
fills its gap. Another real fixture encrypts messages in a different order from
custody preparation, proving that custody sequence is independent of MLS
generation. Expiry of an earlier paid reference does not silently permit later
MLS imports: a still-valid direct original fills that gap before retry succeeds.
The crypto retention bounds remain 128 generations, forward distance 1000 and
three past epochs; no new crypto dependencies or wire format were introduced.

Independent test review first requested changes to prevent a gap from masking
root-fence assertions, prove fresh legacy reordering after migration, and refuse
old root tokens across migration. Those revisions and the shared job/text test
were accepted before production. The two index regression adaptations were
accepted separately. The initial red runs failed to compile on the missing
recovery error; the earlier unchanged 130-original failure is retained in
[the previous report](HISTORY_PAGE_IMPORT.md).

The first ordinary lifecycle replay failed before its recipient SQL-fault gate.
The fixture intentionally creates an unpublished payment sentinel in the same
MLS chain before its two advertised paid originals. With contiguous admission,
that undisclosed earlier original blocks both later imports: the native observer
saw 48 reads, 17 failures and no received messages or storage errors. The run is
retained as failed evidence. It demonstrates the remaining missing-original
outcome; it is not a native recovery pass.

A revised positive fixture establishes a separate real MLS conversation for the
unpublished payment sentinel before the recipient goes offline. Independent test
review accepted this isolated baseline. Only the lifecycle fixture opts into that
separation; ordinary profiles, encryption,
payment, publication and recipient workers remain unchanged. It still checks
that the sentinel never arrives, both paid originals recover after actual loss,
and all budget/signature/SQL/cold assertions hold. Its evidence must explicitly
record that precondition; it cannot claim recovery across the original missing
sentinel gap.

[Checks](receive-admission-checks.json) record **141 backend and 31 frontend
passes**, Core/crypto/Node all-target Clippy and formatting. The 798 captured
inputs stayed unchanged within each run. After the first check, only Node's
large history-read enum variant was boxed; network checks were repeated. After
the second, only the three reviewed native fixture files changed; the native
scenario was repeated. All other passing results carry forward on unchanged
inputs. The reports retain the earlier Clippy and native failures.

[The revised native scenario](receive-admission-native-c3.json) passes in
211.1 seconds, using pinned daemon, CLI and MCP binaries: actual external
Anvil payment, two scoped sends, exact retry, budget refusal, shared status,
three sender lifecycle SQL faults, exact page ACKs after restart, retirement,
later publication, and full recovery of both originals after actual loss of
18 data and 18 index copies. Six QC signatures are independently verified;
owner sender-work and retrieval calls remain zero. Both clients and custodians
use real encrypted profiles. Initial replica rosters can overlap. This is a
CLI regression on the separated setup baseline, not renewed GUI acceptance or
an over-128 paid network pass. Cleanup completed without errors. No Keychain or
visible desktop automation was used. Raw native traces remain local because
they include secrets.

The remaining work is the ordinary graph sender/recipient cutover and explicit
network/product gap outcomes. Runtime currently maps Core receive gaps to its
generic rejection counter and relies on bounded retry; it does not yet expose
a durable graph traversal or a user-visible recovery range. Persist/revalidate
bounded current-root traversal, avoid treating a deferred original as imported,
and confirm every required child body before publishing the root/pointer or
retiring work. Then run the ordinary paid recovery scenario above 128 live
originals. Welcome/first-offline contact, ordered control and epoch recovery,
rejoin beyond three epochs, autonomous repair and the full 67-card/22-E2E/
three-platform release remain open. No complete V1 card is closed here.
