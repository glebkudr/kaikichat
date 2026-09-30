# Ordinary client public epoch lineage

EpochLineage retains authenticated closing choices back to epoch 1 without
validator spent records or an operator key. The new client entry point requires
that lineage and the same current Core authority/context/receipt checks as the
first-epoch path. Public lineage never becomes voting authority.

Five tests passed: two-link cold restart with no spent state; corruption of the
earlier QC while the last remains valid; direct epoch skips and competing choices;
wrong owner/target, changed link, damaged signature, replaced roster and expiry;
existing partial/full handover compatibility; and a failed choice-marker write
whose first successful retry carries an alternate QC/time but retains the first
original ClosingRecord. Old handover progress supports public reads without
creating a migration or granting admission to an incomplete validator.

`test-contract.md` and `test-review.md` record independent REVISE/ACCEPT before
production. The shared final validation is in
[successor-spending checks](../AR1-successor-spending/checks.json), with its exact
source manifest. Failed and successful logs remain under output/ar1-client-lineage.

No native client network lineage or peer bootstrap is claimed. The actual daemon
still uses the first-epoch path. Continue with
[the integration handoff](../AR1-successor-spending/NEXT.md); AR1 and full V1 remain
open with unchanged 67 cards / 22 E2E / three platforms.
