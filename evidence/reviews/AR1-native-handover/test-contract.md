# Actual daemon successor startup and cumulative peer transfer

The target is a real funded issuer executing more than 128 spends and two epoch
transitions through ordinary daemons. Library-only imports or owner-installed
QCs/pages are not acceptance. Keep the complete 67-card / 22-E2E / three-platform
V1 objective open until its remaining requirements are separately established.

`public_epoch_handover.py` reuses the existing native EVM/daemon fixtures, real
registrations, public paid receipts and independent P-256 proof verifier. Every
old/new/third validator key belongs to an ordinary daemon. The test captures
actual additional beacon choices if necessary to obtain an overlap and a third
member outside the first two selected sets; it never edits MPT proofs or injects
a signer. One actual book purchase funds 160 tickets.

The `--configuration` probe tests a prerequisite using the same real selections:
retain an unresolved paid input in the old epoch, configure a currently selected
successor before its history is available, then crash/reopen. Configuration must
persist at its returned revision; the original paid input must remain byte-for-
byte unchanged, no successor runtime journal may exist and submission must refuse
admission. A currently selected role alone must not start a new empty history.
This probe makes no spend or full-handover claim.

The default scenario requires all of the following:

1. Finalize 126 paid spends on the original four daemons. Leave one overlapping
   member offline with an additional paid input, while the other three finalize
   four more spends and the direct closing QC. The overlap therefore lacks both
   ordinary tail entries and the closing, with its original prefix retained.
2. A genuinely new successor member obtains the choice/history through an
   authenticated ordinary connection to an old-only source whose operator role
   is disabled. A SQL trigger permits its first four-entry page, then fails the
   next application cursor update. Independently observe the durable remaining
   count, three ordinary original records and exact row hashes/revisions. No
   new runtime journal, admission or consumer ACK may precede full history.
   Missing records must remain unavailable rather than claim authoritative absence.
3. Role revocation stops outgoing requests and all progress after the SQL fault
   is removed, with the historical peer still available. Crash/reopen keeps the
   exact partial page; a second disabled-role interval with a connected source
   and no fault must also remain stable. Reenabling current authority resumes
   the transfer. The imported original records must match the source.
4. The overlapping member imports its missing tail without rewriting any first
   original SpendRecord or its pending receipt. With only two successor members,
   a pending spend cannot finalize. After quorum joins, the original unresolved
   paid input must finalize without owner resubmission or refund.
5. A new ordinary client has no operator registration and receives only public
   chosen lineage over the network. No full history index or validator spent rows
   may appear in its client store. It prepares a fresh successor spend, receives
   the first original result for an exact old-operation request and a conflict
   carrying that same original evidence for a different operation on the old
   ticket. Cold client lookup must preserve those results.
6. Renew the real current checkpoint, close the second committee, then disable
   its historical source. Wait for the first closing's actual lease to expire.
   A fresh third member first obtains the cumulative choices from the second
   archive, while an SQL trigger refuses its first old-history page. Disconnect
   that source and restart the recipient while the fault remains installed,
   discarding buffered responses. Then let only the original daemon with its own current
   checkpoint expired and its operator role disabled supply the first history.
   A test-only SQL trigger rejects replacing its current checkpoint certificate;
   ordinary clock writes and checkpoint gossip remain enabled. After all 130
   records arrive, its exact old checkpoint ID and expired state must still hold,
   and rejected checkpoint responses must show renewal was actually attempted.
   This models historical service while checkpoint renewal cannot commit.
   The successor must remain unavailable with 130 spent records while the second
   history is missing. Reconnect the second archive and complete both histories,
   preserving each serving source's exact originals. Cold reads with the target's
   own role disabled must also succeed. Current third-epoch authority remains live.
7. Complete a fresh ordinary-client spend in the third committee and reject the
   old ticket there too. Independently authenticate all three canonical chains,
   two direct closing QCs, 133 distinct spent nullifiers, original proofs and
   current final consumer positions. An extra hidden spend must not fit the
   observed history or final store count.

The test-only SQL helper is not linked into the daemon. Its added snapshot mode
returns public state hashes/revisions, canonical history heads and handover
progress. First-page assertions require exactly canonical entries 131..128 and
their operation rows at revision 1, the authenticated source byte hashes, three
exact original records, and agreeing head/application cursors at 127. No entry or
operation row from the failed page may survive. Its new trigger
aborts a real transaction. It never inserts application/history rows. Test data,
binary and source fingerprints are preserved by the existing native runner.

Required observable daemon state: postageSpend.handoverPending,
handoverRemaining and failedHandoverCommits; postageHistory.sent/pending; and
postageClient.lineageReady. These must describe actual retained/authorized state,
not substitutes for SQL snapshots, runtime journals or independent QC checks.
Existing configuration/admission RPC shapes otherwise remain compatible.

Implementation needs one bounded historical peer path, pending selected/public
configuration, cumulative backwards discovery/forward import, current-requester
revocation, historical role-free source reads, and store-aware successor policy.
Reuse Core authority, original closing/record authentication, bounded processing
budgets and the existing import APIs. Old live signing authority must not be
required for historical service. Page count and serialized bytes are both bounded.

New/changed tests require independent backend-test-critic ACCEPT before production.
After implementation run the full native scenario and backend/frontend checks.
Hostile/mixed closing ingress, closure after old-authority outage, automatic
client renewal and the other V1 work remain separate required gates; this test
must not be reported as their completion.

The first full execution finalized 126 spends and independently verified 1512
signatures, then encountered an expired intermediate fixture checkpoint during
sequential owner acceptance. The corrected fixture signs all intermediate and
closing checkpoints with the already installed 600-second maximum. Renewal uses
an actual later mined block (initially +301 seconds), still with a 600-second lease; the
product clock is never rewound and expiry assertions remain unchanged. Fresh
profiles accept the latest current signed certificate through the existing Core
trust rule; already anchored profiles follow their actual successor chain. This
checkpoint setup never supplies a postage choice or spent page to the daemon.

Full6 failure snapshots established that a cold source served 32 pages and the
recipient retained 127 original SpendRecords, with the last three still pending
in epoch 1. The source's Admission<64,16> permits 16 requests per peer per
60-second window. A 131-entry chain needs 33 four-entry pages, so its final page
cannot arrive within the original 100-second deadline. The corrected transfer
deadline is 160 seconds (third window at 120 plus scheduling/verification time).
Transfer waits are separate from unchanged 50-second client result and 60-second
consensus deadlines. Renewal is mined at +501 seconds so two cold rate-limited
transfers fit after old-source expiry; the installed lease maximum remains 600.
No source admission limit, clock, scope or exact-history assertion is relaxed.

The second full attempt completed both closings, successor spending, ordinary
client results and partial-transfer/revocation assertions, then the original
archive's current head was not expired at restart. The original archive is now
stopped before renewal is published to the second committee, so ordinary
checkpoint gossip cannot refresh it during the earlier connected phase. The
expired-state and exact old checkpoint ID are recorded and asserted at restart.
The source retains no active operator role throughout historical service.

The independent critic rejected checking source expiry only before connection: normal checkpoint gossip could renew it before serving. The revised gate requires the same expired head after all 130 records, with a narrowly scoped certificate-write fault and an observed rejected renewal response. The fault is removed after stopping the archive; it never modifies a certificate, history row or QC.

The third attempt exposed a polling error after one successor record was already
finalized: another newly activated validator still had handoverPending and its
negative lookup correctly refused authority. The polling loop now waits for that
flag to clear before reading results, with the same deadline and all other errors
preserved. The independent critic accepted this correction.

`public_epoch_configuration.py` separately tests an actual storage-failure window
found during source review. The old selected validator retains an unresolved paid
input; an abort on bootstrap writes must roll back the new configuration too.
All persisted state hashes/revisions must remain unchanged, and retry at the old
revision must work immediately after the fault is removed. Crash/reopen preserves
that successful configuration and exact input without a successor signer. A fresh
ordinary client exercises initial insertion, a cold restart before removing the
fault, retry at revision zero and pending public lineage without any operator,
spent rows or validator history. The test observed the expected RED: only
postage/service changed while the reported in-memory revision stayed at 3. The
configuration and bootstrap writes now use the existing state-batch transaction;
native GREEN passed with 263 owner calls and no cleanup errors.

The future third recipient retains the genuine live old public checkpoint before
renewal is published, with its postage/history rows empty and no configuration or
service. Its later direct-successor acceptance supplies a real checkpoint archive
capable of answering after=oldId. This makes the failed renewal exercise observable;
a fresh profile holding only the latest checkpoint cannot serve that suffix.
No postage choice, QC or page is installed by this public checkpoint preparation.
