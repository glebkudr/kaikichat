# Independent backend test review

Reviewer: the separately created, context-free `paid_custody_fixture_critic`, reused
for the subsequent test revisions. Decisions below are transcribed from its final
messages. No reviewer modified sources or ran builds.

1. **R1 REVISE**, manifest `efc04b9fed8000e5e1186085ad4d5466399c3da0d5c34c35df86441bdabe91c0`.
   Generic cumulative failure/read counters did not isolate SQLCipher failure.
   Required an unavailable first endpoint and a real multi-page Noise/MLS control.
2. **R2 REVISE**, manifest `1201855c165e6c4dfaf9f87b98a80cb66f0a1f5b4f77f559cd70c111172c8a42`.
   Prior blockers resolved. The independent page-request oracle still needed to
   bind the request signer to the custody index and check Resource/authority epoch.
3. **R3 ACCEPT**, manifest `c44d8aec87b58651a1dbaf0ea2dd9740dd7f233b2e74a83284954d905e2d6df3`.
   Independent SHA256 index derivation and document kind/epoch checks resolved the
   remaining blocker. **Production implementation began only after this final ACCEPT.**
4. **R4 ACCEPT**, manifest `dc904e312bfc914f62aaa7fe6c9cbbff6d15e0d0e7630d8a130a6299e4e43321`.
   The pages fixture originally provided one cache despite the existing two-peer
   publication quorum. Added the second actual cache and both bootstrap checks,
   preserved all retrieval assertions, and added guaranteed diagnostic cleanup.
   The old Runtime test constructor gained the real sync field. Production was
   frozen while these test corrections were reviewed.
5. **R5 ACCEPT**, manifest `5f80ebc60252edc915bc0b005b0045f65084f19727e545b54c108b0d33288518`.
   Message import can precede scheduler accounting by one pump tick. A bounded
   wait preserves the five-completion threshold. The manual referral-budget test
   now waits for initial background work and measures the next owner's actual
   request delta: exactly 32 target-key requests, at most 96 total. Restart cold
   state is checked before launch; automatic discovery may legally resolve it
   during the subsequent bootstrap handshake. Production remained unchanged.
6. **R6 ACCEPT**, manifest `36ffaad953966da17be7b92f99b76444c1b7dca78164df7bf7a74a04f8747624`.
   The live paid gate hit the same import/accounting timing boundary at one
   completed read. Both gates now share the same five-second helper, preserving
   their thresholds and strictly increasing request counts. The live gate retains
   public proof bytes and asserts their hash against the independently verified
   prover output; it saves protocol diagnostics before subsequent fault controls.
   Production remained unchanged. The first paid run stays failed evidence.

The R1/R2 RED runs establish missing Core/daemon APIs, not a completed behavioral
pagination failure. R3 failed before retrieval because of its one-cache fixture.
R4 imported all 33 messages and checked real cursors `[0,8,16,24,32,0,8]`, then failed
its premature counter check. These runs remain failed evidence. R5 passes the full
pagination scenario and all five existing mailbox process tests.

The first paid run independently verified a fresh proof and three QC signatures,
observed the typed SQLCipher failure, removed it and found the exact recipient
message, then failed the premature completion-counter assertion. The raw failure
and its proof hash remain retained; that failed run did not retain full proof bytes.

This review accepts tests for the bounded retrieval increment. It does not certify
the whole V1 release. R7 subsequently demonstrates a controlled owner/background DHT overlap. Sustained
four-slot custody saturation and explicit priority-join cases remain outside these
new automatic retrieval gates.


7. **R7 ACCEPT**, manifest `bba6d3d285e486038f6b7e99b8c457a7b15a66348daf234e42ddc2a4195e004b`.
   The full backend run found a real regression: automatic mailbox discovery used
   both shared routing slots, refusing the first owner lookup of a moved contact.
   A new process test holds an actual independently counted GET_RECORD unanswered,
   checks two queued MLS conversations against one occupied slot, then requires
   the first owner lookup to be admitted and verified without retry/release.
   Background work must resume after the hold. Exactly that test failed before
   the fix at two occupied slots versus one. Production changes started only
   after final ACCEPT. Explicit timing around owner admission and priority-join
   cases were non-blocking follow-up suggestions.
