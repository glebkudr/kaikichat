# Cumulative original spent-history import

Current application evidence layer: exact original SpendRecords, four-entry
atomic transfer, cold cumulative continuity, local validator reuse and immutable
original proof preservation. All live epoch > 1 spending guards remain in place.

Full regression: **857 Rust tests**, zero failed/ignored in 57 nonempty suites;
**59 frontend tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt all
pass. All 571 frozen source inputs stayed unchanged during the full workspace run.
See `checks.json` for commands, results and explicit unimplemented integration.

The separate genuine EVM fixture purchases 160 public tickets for 1120 wei and
captures registry epochs 1, 2 and 3 at one actual checkpoint. The main application
test verifies receipts and finalizes 130 spends through the existing indexed
epoch-1 policy, then transfers those original records and a terminal entry.
Other tests cover SQL rollback/cold retry, authentic but wrong archive evidence,
same-progress foreign owner and stale handles, alternate QC/time preservation,
cumulative 1→2→3, genuine skipped epochs, terminal-only closure, missing old
evidence and an authentic conflicting later-epoch QC. Epoch-2 fixture QCs are
historical carriers, not a claim that new nodes ran consensus.

Files:

- `test-contract.md`: bounded feature and explicit remaining integration.
- `test-review.md`: separate critic's REVISE fixes and final ACCEPT before code.
- `fixture-generation.json`: actual paid fixture authoring result and digest.
- `checks.json`, `validated-inputs.json`: actual checks and frozen source inputs.
- `NEXT.md`: successor admission, client lineage, peer transfer and full V1 work.

The generic atomic page prerequisite has its own accepted test contract under
`../AR1-atomic-history-import/`. Raw logs remain in
`output/ar1-spent-history-import/` and `output/ar1-atomic-history-import/`.
No fresh native successor-spending, packaged desktop or other-platform result is
claimed by this evidence. AR1 and the full V1 goal remain open.
