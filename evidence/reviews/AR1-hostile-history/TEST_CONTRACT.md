# Next native history ingress gate — draft

Use a small genuinely funded public book with three distinct spends and two
actual native closings, through ordinary daemon-owned keys and real overlapping
selected rosters. Reuse public_epoch_handover.Scene/Evidence and the shared runner;
only close-at-height differs (3 and 2 instead of 131 and 3). The previous large
handover already covers length, so do not repeat the 130-spend sequence here.

A new member of the third committee starts with no spent history. Stop every
other daemon. An independent raw CBOR carrier reopens one stopped second-epoch
member's real transport profile, establishes its actual Noise connection, and
serves supplied public evidence on /agentic-internet/postage-history/1. It runs
no application engine and never writes spent/history rows to the target.

Required phases, with pre-response SQL snapshots and actual ResponseSent events:
1. For Choice(target3), return authentic closing1(target2). Assert unchanged
   target spent/history/bootstrap rows, no signer, and repeated identical query.
2. Return closing2 with altered final QC signature byte, framing preserved.
   Assert the same state and repeated identical query; authenticate the original
   before mutation with the independent existing QC oracle.
3. Return exact closing2/closing1, hold all pages empty until the old page query
   is observed. Snapshot before releasing any malformed page.
4. Return an old page with its correct canonical entries but a real SpendRecord
   from a different position. CBOR/JSON and real signatures remain well-formed.
   Assert the entire SQL snapshot is unchanged, no accepted cursor advance or
   early signer. At least two completed replies and a subsequent same query
   distinguish application rejection from unavailable transport.
5. Crash/restart the target while keeping only the ordinary peer carrier; retain
   identical state. Restart the carrier against its new authenticated listener.
6. Supply exact original choices and pages. Require all three original records,
   QCs and verifiedAt values byte-for-byte; all immutable canonical rows equal
   the real second-epoch archive; one signer becomes available only after full
   continuity. Disable role, cold restart with all sources off and read all
   original results unchanged. No extra finalized QC or owner resubmission.

Carrier controls must replace the response JSON file atomically. Log public
query, selected phase and response only; never Bootstrap input/masterKey/token.
Track ResponseSent by inbound request ID, not just send_response enqueue.
Guaranteed cleanup for carrier and all shared fixture daemons. Preserve public
failure snapshots and completed carrier events before temporary profile cleanup.

This is added adversarial coverage of existing AR1 integration, not a new claim
of automatic renewal/outage closing, public wallet UI, R10 or full V1 acceptance.
