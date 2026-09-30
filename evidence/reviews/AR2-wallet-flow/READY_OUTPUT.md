# A finished page at the Work boundary

Date: 2026-09-14. Part of step 2 of [R14](../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).

Defect: in advance_custody_sync the 120-second check stood before the extraction
of the finished index_output. A completed request had already released the
network stream, but the Work was deleted before applying the page. The initial
RED: all four new tests failed on the unapplied ready answer.

The fix contract: first apply the completed answer with the existing handler,
then finish the expired Work before local retries or the next request. An
unfinished real stream keeps its slot. The 120-second limit, the 16 visits and
the shared network admission remain as before.

The tests use a real queued job and Noise to check the request/stream. A positive
signed branch with a leaf under an already accepted root is fed to the existing
boundary after paid validation. This is not a successful paid answer over the
network. Durable cache and cold persistence without message delivery, a failure
after a real successor pointer, mailbox-head expiry during application, a real
INSERT failure of the cache and an exact cold retry are checked. Head expiry is
not separate proof of checking the paid proof's term.

On SQL failure the former logic keeps the retry claim in history-graph-scan.
The cache, messages and MLS must remain unchanged; the retry mark itself must
appear. Fixing this test expectation does not change the production contract.

## Open parts

An unfinished proof of a new root can remain only in the Flow's memory. This fix
does not transfer such a proof or the range cursor between Works. Needed are a
continuation for the exact head/root/index/epoch/holder and cursor advancement
only after the full durable intake of the corresponding bodies; then the single
pending-body store and the unchanged Diagnostic32/Full130 gates. Native
throughput is not measured here; Keychain and native E2E are not run.

## Results

Only the premature deadline-return before the extraction of the finished output
was removed. The current validation handlers and the remaining deadline check
are unchanged. After clarifying the SQL invariants, the repeated RED: 0 PASS /4
FAIL. GREEN: 4 PASS; 56 former receiver tests, 63 frontend and five parser tests
also pass. Node Clippy all-targets and fmt are clean. The tests and the production
fix received an independent FINAL ACCEPT. [Checks](ready-output-checks.json),
[review](ready-output-test-review.json).

The critic demanded supplementing the cold retry with a check of the exact
original 0: re-receiving the same branch by itself does not prove the needed
claim. The check was added before the production code and passes after a real
SQL failure.
