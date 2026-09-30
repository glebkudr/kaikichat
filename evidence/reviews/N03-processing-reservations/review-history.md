# Independent backend test review

Agent `/root/processing_test_critic` was spawned with `fork_turns="none"` and read the
backend-test-critic skill and repository AGENTS.md. The parent awaited FINAL verdicts
before implementation and after each test change.

R1: REVISE. Missing real worker lifetime after successful decode while waiting for the
application response; no observed overlap inside blocking finalization waits. Reviewed
6 test/spec,128 unchanged production,494 unchanged helper hashes on f3f6844.

R2: ACCEPT before production. New real TCP/Noise bootstrap worker test retains four
completed requests' response channels, observes shared charge and fifth refusal, then
response completion/reuse, cancellation and actual swarm-drop drain. Integration now
samples actual capacity, carrier wire counters and effect counts during queueing and
convergence; independent QC verification remains. Distinct-peer early refusal and
fresh selected-peer rate/slot controls added; signed deadline made two seconds.
All7 test/spec,128 production,494 helper hashes verified unchanged before implementation.
Nonblocking limitations: 500ms carrier statistics can miss very fast certification;
selected global slot test also coincides with total64; mixed reclassification and
outbound largest-response lifetime can receive further focused coverage.

R2 baseline RED was run against f3f6844 with exact reviewed test/spec, production and
helper inputs: absent processing API (compile) and actual TCP load failed the early
refusal condition before any new processingCapacity diagnostic assertion. Cleanup[];
full old connection, replay and effect controls reached the new processing phase.
R2 implementation focused15 tests passed, including real decoded worker lifetime.

R3: ACCEPT after the full gate found Clippy expect_used in the new lifecycle helper.
Sole test revision: timeout(...).await.expect(message) changed to
unwrap_or_else(|_| panic!(message)), keeping the3s timeout, return value and failure.
Reversing the replacement reproduces exactR2 hash; other6 files unchanged. Critic
verified7 test/spec and130 current implementation hashes, bothRED log hashes and
confirmedR2 RED and acceptance remain valid. Immediately before this test change,
production poll_close received the same original-reservation fence as read/write/flush.
No further production edits during review. This review is not itself GREEN or fullV1.

R4: ACCEPT. The firstGREEN attempt passed596Rust/40frontend, held16 ordinary readers
and independently certified effect13, then completed traffic did not reach the rate
ceiling (404wire completions,370unverified responses,capacityRejected3903/rateRejected0).
Its fulltrace is retained. Inspection identified expensive profile reconciliation before
transport rejection and synchronized200ms carrier bursts that exhaust concurrency first.
Production moved reconciliation after actual transport validation; an unchanged unknown
scope control caught changedresponse semantics, retained in a secondfailuretrace. A cheap
scope/running/base check restoresUnavailable before the transport check; fullCoreauthority
validation remains before engine admission. Before furtherproduction, R4 changes only the
completed flood to staggered20ms requests, keepingoneoutstanding/peer and allacceptance
assertions; held/idle path unchanged. One latest diagnostic snapshot is added. Critic
verified7tests/spec+130implementation hashes; reversed narrowedits reproduceR3, confirmed
R2RED remains valid because its heldphase precedes the newflood branch. FreshGREEN required.
