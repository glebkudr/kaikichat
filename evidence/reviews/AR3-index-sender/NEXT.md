# Historical continuation at sender publication acceptance

Recipient integration and pointer migration from items 1–2 are now accepted for
one book. The current remaining work is in
[the recipient continuation](../AR3-index-recipient/NEXT.md). The list below is
retained as the original sender-stage plan, not current completion status.

1. Connect recipient index-page → holder-location → authenticated endpoint
   resolution → descriptor-bound ciphertext fetch → atomic import/bookmark.
   Then change the latest pointer to confirmed book-index endpoints. The current
   data-holder intersection remains an open architectural defect (AR-R03).
2. Prove complete recovery across disjoint surviving message-holder sets and
   multiple books/epochs, including cold indexes, corrupted/gap pages and sender
   absence. A 16-provider/R10 fixture cannot produce disjoint full primary draws;
   do not label disjoint surviving sets as disjoint original rosters.
3. Add explicit finite continuity/completeness evidence, durable first Welcome,
   and safe successful sender retirement only after discoverability is durable.
4. Connect index/data inspection and repair to one bounded operator scheduler;
   run real 10→7→10 with both clients offline and actual disk/data loss.
5. Continue AR1 renewal/lifecycle and AR2–AR5 wallet/CLI/groups/recovery/platform
   work. Preserve 67 mandatory cards, 22 E2E and all three required platforms.

Targeted checks during implementation; full suite only at the end of the plan.
