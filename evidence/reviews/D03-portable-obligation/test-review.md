# Independent backend test review

Reviewer: /root/portable_custody_test_critic, initially spawned without inherited context.
All passes completed before production changes. Exact reviewed tests are retained
in tests-r1/r2/r3 manifests and diffs.

R1 FINAL REVISE: owner RPC test nonce used u64 where the existing helper requires
u8. Storage compilation RED had only expected missing APIs; node compilation had
a test defect, so did not establish behavioral RED.

R2 FINAL ACCEPT: corrected nonce; real daemon scenario now failed with
custody_obligation unknown_method. Reviewer accepted genuine proof retention,
restart/cold reconstruction, SQL failures, expiry/legacy behavior and owner access.
Suggested stronger advanced-head and immutable retry coverage.

R3 FINAL ACCEPT: all six test files matched the frozen manifest. Added original
carrier assertions after 80 checkpoint advances, restart, independent historical
Core reconstruction, actual alternate operation and a renewed-binding retry with
complete bundle equality. No blocking issues or additional required scenarios.
Core RED: five expected missing evidence() getter errors. No current-copy or
repair authorization follows from historical replay.

No reviewer production edits or builds. Reviewed R3 tests were unchanged after
implementation; source/test hash verification is part of the checked evidence.
Production's first targeted build failed because a mechanical re-export edit
misspelled RetainedPostageContextInput; it was corrected without test changes.
The next targeted paid-store run passed all six tests. Core advanced-head and
actual daemon owner-query scenarios also passed.
