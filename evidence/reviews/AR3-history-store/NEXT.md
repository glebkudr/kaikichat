# Continue toward ordinary multi-book history

1. Expose manifest put and recipient read over the existing bounded Noise custody
   transport. Authenticate the actual index peer and its original paid anchor.
   Carry exact signed manifest bytes; no owner-only retrieval path or extra spend.
2. Give the latest authenticated locator an exact manifest/anchor commitment, or
   an equally explicit bounded discovery contract. A first ordinary index entry
   is not necessarily the longest-lived anchor. Cold peers must distinguish a
   missing/stale manifest from an authenticated current directory.
3. Core outgoing exact revisions and incoming durable checkpoints, including
   anchor changes, are now accepted in [AR3-history-core](../AR3-history-core/README.md).
   Live published references survive restart and failed commits. Still tie message
   import/MLS/dedup/progress to the actual manifest reference;
   absent and unavailable entries must not become an empty-history success.
4. Switch ordinary sender publication and recipient traversal away from live-job
   index intersection only after those paths exist. Prove actual funded books
   with no common index, missing/corrupt entries, cold sender-absent recovery,
   read/commit failures, expiry and compatibility on the same application build.
5. Actual MLS control/epoch catch-up and first Welcome, safe successful retirement,
   attachments and autonomous index/data 10→7→10 repair remain mandatory. Preserve
   all remaining AR1–AR5 work, 67 cards, 22 E2E and three platforms. Full suite only
   at the end of the entire V1 plan.
