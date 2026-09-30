# Independent test review

Separate backend-test-critic `/root/native_handover_test_critic`, started without
inherited context, returned REVISE for two evidence gaps: a SQL trigger masked
revoked page commits, and failed-page canonical rows were not checked independently.
Both were corrected before production. The second review returned ACCEPT with no
remaining blocking or non-blocking findings. Probe revision, cold conflict result
and expired-source isolation improvements were also accepted.

Acceptance concerns the test contract only. The observed native RED is successor
configuration rejection; the complete handover scenario has not yet passed.

The subsequent mechanical addition of `postage_history: Traffic::new()` to the
existing Runtime test constructor was separately reviewed and ACCEPTED. No
assertions changed.

After full1 stopped on an expired intermediate checkpoint, the fixture timing
correction received separate ACCEPT: all certificates remain within the existing
600-second profile, renewal uses a real later mined state and original-source
expiry/isolation assertions remain intact. No production change was needed.

The full2 source-isolation correction first received REVISE: checking expiry
only before connection did not exclude renewal before serving. The revised
scenario received ACCEPT. A test-only certificate-replacement SQL fault leaves
clock writes, history reads and native checkpoint gossip enabled. The test
requires the same expired head and no services after all 130 records, plus an
actually rejected renewal response. The database parameter preserves the existing
fault helper default. Full execution is still required.

Full3 passed the first closing and all partial-import/revocation checks, then
observed one successor result before a still-importing selected member correctly
refused lookup. The polling correction received ACCEPT: wait while handoverPending
without swallowing other errors or extending the deadline.

A source audit identified separate configuration/bootstrap commits. The new
public_epoch_configuration.py test received REVISE for an operator-key-dependent
journal assertion on an ordinary client. Its corrected log-wide absence check
received ACCEPT. The gate uses real selected/ordinary profiles, narrow SQL aborts,
full before/after snapshots, live/cold revision retries and original paid input
preservation. The actual RED changed only postage/service while in-memory revision remained 3.
The accepted test detected a partial configuration commit before production was
changed. Both callers now use the existing state-batch transaction for config
and bootstrap; native GREEN passed with 263 owner calls and no cleanup errors. Focused
node/postage Clippy and 59 frontend tests/TypeScript/Vite also pass; full native
handover and backend regressions remain in progress.

Before full4 reached expired-source service, source inspection showed that a
fresh recipient retaining only the latest checkpoint cannot serve a suffix from
an unknown old ID. The run was intentionally interrupted (exit 130, cleanup
errors empty), not reported as a product failure. The corrected fixture retains
a genuine live old public checkpoint on the otherwise empty future recipient
before renewal publication, then accepts its direct successor later. Empty postage
rows, no configuration/services and both expired-source/failed-renewal assertions
remain required. The independent critic returned ACCEPT.

Full5 reached the original archive with its checkpoint actually expired, then
timed out waiting for first-history transfer. The cause remains unconfirmed.
Additional assertions require the cold source to return its closing and exact
first/last original records before connection. Failure handling retains public
node/checkpoint/operator info and state hashes/cursors before profile cleanup,
then reraises the original error. The independent critic ACCEPTED this stronger
diagnostic contract without any deadline or success-criterion relaxation.

Full6 retained decisive public diagnostics: the old source served 32 pages, the
recipient committed 127 original records and still needed the oldest three. Its
connection and current target authority were live; the source remained expired
with eight failed checkpoint renewal responses. The existing admission policy
allows 16 requests per peer per 60-second window, so a 33-page transfer cannot
finish before 120 seconds. The independent critic ACCEPTED a 160-second transfer
deadline, separate from unchanged 50/60-second client/consensus deadlines, and a
genuinely mined renewal at +501 seconds to leave enough current authority for two
cold transfers. No production limit or original-history requirement changed.
The seventh full run passed (133 spends, two closings, 1605 verified signatures).
Full regression also passed: 871 Rust tests, 59 frontend tests, 19 model tests,
TypeScript/Vite and workspace Clippy/fmt; 582 source inputs stayed unchanged.
The optional suggestion to assert the exact
501-second expiry gap was not required for acceptance; the fixture constructs it
explicitly and retains all original checkpoint/expiry checks.
