# Continue AR3 without losing the AR1 lifecycle dependency

1. Persist authenticated holder claims with paid index entries. Admit only
   matching descriptor/operation/QC and selected data positions, preserve original
   receipts and limits, and make updates atomic across SQL failure/restart.
   A copied location must retain its original-primary evidence and finite consent.
2. Add bounded network publication/read on the real custodian transport and
   recipient discovery through the stable book-index roster. Use actual fetched
   ciphertext to validate locations; index signatures alone prove no availability.
3. Exercise genuinely disjoint data rosters with sender absent and recipient
   cache lost, then multiple books/epochs, unavailable index nodes and corrupt/gap
   pages. Do not infer complete history from an empty local page.
4. Only then retire successful sender jobs while preserving old-history discovery.
   Full AR1 still needs its durable stage ledger, authority renewal/outage closure
   and recovery of expired pending inputs. R10 needs autonomous data/index repair.

For every new backend transition: tests first, independent context-free critic
and ACCEPT, then production and targeted backend/frontend clusters. The complete
suite remains deferred to the end of the full V1 plan.
