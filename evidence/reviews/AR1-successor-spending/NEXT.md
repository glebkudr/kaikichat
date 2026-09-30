> Native integration and its full funded gate now pass: 133 spends, two closings
> and 1605 independently verified signatures. Full backend/frontend regression passes.
> Follow [AR1-native-handover](../AR1-native-handover/) and its
> [remaining work](../AR1-native-handover/NEXT.md). The plan below is historical;
> statements about "current" source describe the earlier library-only phase.

# Historical plan: connect application policy to actual peers

Full V1 remains 67 mandatory cards / 22 E2E / macOS arm64, Windows and Linux
x86_64. This earlier source enabled successor application policy and ordinary-client
public lineage. That library gate alone did not prove native multi-epoch consensus
or close AR1; the later native evidence is linked above.

Completed API prerequisites:

- new_with_continuity requires current selected Core authority and complete
  owner-bound state for the exact scope. Store-aware proposal/verification checks
  global spent keys; old storeless methods reject successor sessions.
- finalize rejects another epoch or another position for an already finalized
  ticket; exact entry retries retain first QC/time. lookup_continuous authenticates
  an original record only within the chosen lineage. The parser is only a hint.
- EpochLineage import/open checks original choices back to epoch 1, without
  spent records or an operator key. Public client preparation reuses current
  Core/receipt checks. Partial/full old handover metadata works without migration.
- Cold source/continuity scans remain bounded in memory but O(total history).
  Warm global negative membership uses one record lookup; chosen historical
  membership follows point-read links bounded by actual cold-verified depth.
- Incomplete overlapping history now extends to the authenticated old closing.
  Existing rows and original SpendRecord QC/time remain immutable. A pending head
  disables negative membership, including old handles, and pages can cross the
  old/new boundary atomically. Complete closed indexes retain their local path.
  See ../AR1-overlap-history/ for four test-first regressions and checks. The
  native gate must exercise this real missed-tail case, not force every overlapping
  validator to catch up under its old authority before reconfiguration.

Required next work, with tests and a separate critic before production:

1. Actual node lifecycle. postage_spend::configure currently calls legacy new
   before persisting Saved and cannot retain a pending successor. Persist the
   selected successor snapshot/configuration while admission remains unavailable.
   Keep unresolved paid inputs. Services::install_postage already stops/joins the
   prior signer; retain that ordering and its revocation checks. State::prepare
   must acquire complete continuity before constructing the successor session or
   opening its terminal-bound index and launching the signer. Cache validated
   history per lifecycle, not on every retry/tick.
2. Wire every ordinary/closing proposal through store-aware APIs. Candidate in
   postage_closing.rs currently has no store argument. The finalized-record
   fast path in State::effect compares only journal; bind the original full Entry
   to the current delivery before history/retirement/ACK, including sequence,
   parent and committee. Use continuous lookup for old original results.
   submit_checked also constructs legacy SpendSession on every submission; share
   a current-session helper that reuses cached complete continuity instead of
   scanning every predecessor again for each receipt.
3. Bounded ordinary-peer transfer. Existing postage_submissions Archive recovery
   requires current old selected membership and a running old service. Genuinely
   new selected members cannot use it. Introduce a narrow historical read path
   independent of old live signing, reusing authenticated connections, discovery,
   processing/admission limits and bounded wire decoding. The source must remain
   available after its old selected lease ends; it serves only authenticated
   original public evidence. A hostile/missing page never advances application
   progress. Fetch cumulative ancestors before newer epochs, resume after crash,
   and activate only after current Core authority and complete state coincide.
   Bound serialized bytes as well as page entries: four maximal original JSON
   records can exceed a bounded response. Reduce the page length when necessary
   while retaining the existing four-entry and sixteen-state upper limits.
4. Ordinary client integration. Submit::prepare_client currently calls legacy
   prepare_client, and postage_client::Configure::authority independently rejects
   every epoch other than 1 before Client can retain pending configuration.
   Public chosen-lineage discovery/import must use EpochLineage;
   it never substitutes for full validator state. Avoid O(all epochs) rescans on
   every tick; current authority and retained link revisions remain revocable.
   A cold role-disabled node has no SpendSession, and current read/response code
   uses one historical committee. Reuse/extract the authenticated chosen-history
   reader so cold old-record reads and client conflict results work without a
   selected session. Do not restore a live signer to read historical evidence.
5. Extend tests/evm/public_epoch_closing.py's genuine funded ordinary-node setup.
   It already authors 18 daemon-generated old/new/third registry keys including
   a post-freeze entrant. Add fresh successor spending, old-ticket refusal, exact
   old records, overlapping/new members, partial transfer SQL failure and cold
   restart, preserved pending paid input, partition and a second transition.
   No owner RPC page copying, fixture signer injection or manually installed QC
   substitutes for this native gate. Pin binary/source and independently verify
   every real QC. Reuse the existing helpers rather than duplicating EVM setup.
   Prefer extending the same funded issuer beyond 128 spends before its two
   transitions; earlier separate 144-spend and closing gates remain historical
   evidence. public_spend_lifetime.PublicAuthor supports 160 funded tickets and
   the shared paid fixture. public_spend_recovery.Author currently hardcodes two
   operations, so do not use it unchanged for the longer scenario.
6. Preserve the closed service's configured scope with an empty candidate list;
   removing the scope previously disabled its historical record server. Separate
   historical availability from active candidate/signing state.
7. Still required: closure after old-authority outage without reviving ordinary
   old spends, full ordinary queue closing admission, malformed mixed candidates
   and hostile closing carriers through selected transport. Then automatic
   sender/client authority/context renewal, reconciliation, terminal retirement
   and fair scheduling, network history/R10, UI/CLI/MCP, groups/recovery,
   independent operators and the three-platform current-revision release gates.

Concrete next native fixture plan (not an executed or accepted test):

- Reuse postage_spend_node's dynamic_successor_keys setup: all 18 validator
  keys belong to actual daemons, including the post-freeze registry entrant.
  Author 160 funded public tickets and use 130 in epoch 1; adapt PublicAuthor's
  receipt authoring instead of inheriting its hardcoded lifetime-test positions.
- Choose real later sealed epochs with an old/new overlap plus new members, and
  a third committee with a member outside both earlier selected sets. The four
  selections out of sixteen are random: assert these set relations or capture
  further genuine epochs under the existing admission window. Never assume an
  overlap, alter registry proofs or inject private fixture signer keys.
- Keep an old-only member as the archive with its operator role disabled. Stop
  the overlapping member with a genuine paid input still pending and without
  the final old tail/closing. Close with the other three old selected daemons.
- Start only two successor members initially. Give the fresh one an SQL trigger
  that allows one four-entry page and then aborts progress updates. Assert no
  successor signer or consumer ACK before complete state. Revoke, crash, reopen,
  verify the exact saved page/cursor and pending input, remove the trigger and
  resume from the ordinary historical peer. No owner-provided pages or QCs.
- Add the remaining successor members and an ordinary client with no operator
  key. Its public lineage must arrive over the network; then send a fresh paid
  receipt, reject a different operation using an old ticket, and return the
  original old QC for an exact lookup. An overlapping node's first original QC
  can differ from the source's QC and must not be overwritten.
- Close again and let a genuinely fresh third-epoch member recover both old
  histories. Renew the current target's real checkpoint long enough to show
  the old archive still serves after its original closing lease expires. Use
  actual wall time, not a rewound product clock. Preserve independently derived
  per-epoch committee inputs, every original QC and all canonical entries.

Source integration points: runtime.rs/network_settings.rs for the bounded peer
protocol and replacement cancellation; finalizer_frames::maintain_finalizer_services
for upkeep; postage_spend State for pending config, activation and historical
serving; postage_client for public lineage. operator_connection_allowed binds
the authenticated PeerId/connection and relay policy without requiring a live
old selected role. Reuse processing::Behaviour/Budget and Admission limits.
Existing selected submission Permit requires a live route, so do not accidentally
make historical availability depend on that route. Current requester revocation
and connection replacement must still stop pending download writes.

For cumulative bootstrap, query the original choice by issuer plus target
committee, and pages by issuer plus source committee/cursor. Materialize the
public target-to-source choice when the actual terminal effect is committed,
including retained-closing replay, before retirement/ACK or configuration
replacement. A source may retain a legacy closing before this marker exists;
its configured closed snapshot can supply the migration path.

Walk unknown choices backward to epoch 1, authenticating each exact target and
retaining original ClosingRecords with one durable ancestor cursor. Then import
forward: the retained closing keyed by the just-completed committee supplies the
next transition, until the configured desired target is reached. Avoid an
in-memory array or recursion proportional to all epochs. Public clients import
only chosen links; selected validators additionally import all original records.
Cache a checked source index across its bounded pages rather than cold-opening
it on every request. Reuse the chosen-record authentication in a role-free reader
for cold node reads and public-client old-record/conflict results.

All commands/builds use /Users/glebk/Code/chat and build-storage.py. Preserve the
user's untracked Docs/maintenance/build-storage-cleanup.md and media/.
