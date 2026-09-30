# Continue V1 through authenticated spent-state handover

The full goal remains 67 cards / 22 E2E / three platforms. Closing alone cannot
close AR1, AR-R01 or the full V1. Consult checks.json for the actual gate result;
the test-first contract and implementation do not themselves certify a pass.

1. Authenticate a complete predecessor history ending in the chosen direct
   closing QC, plus the original SpendRecords and their historical authority.
   Reuse HistoryImport's bounded, atomic pages and unavailable-until-complete
   behavior. Verify every original record against its exact authenticated entry;
   retain original QC/time rather than synthesizing replacement spend evidence.
   A closing proof alone cannot establish absence of a nullifier in old history.
2. Persist an opaque continuity checkpoint selecting exactly the authenticated
   successor. A fresh node must not create a SpendSession merely because its new
   scoped committee has a new genesis. Keep epoch != 1 guards until the genuine
   complete transfer and current successor authority jointly authorize startup.
   Activate only after all required state commits; crash/SQL failure cannot expose
   a partial spent set. Preserve issuer-global nullifier keys.
3. Support historical authorities for earlier committees. SpendSession's existing
   lookup currently authenticates records with one committee; this must change
   deliberately before new-epoch spending. Cumulative predecessor lineage is
   required across later closings, including skipped registry epochs. Transfer
   completeness for only the most recent committee cannot prove issuer-wide
   absence. Do not turn a known spent ticket with missing original evidence into
   permission to spend again.
4. Extend the funded native gate to ordinary successor nodes, a new spend after
   complete bootstrap, old-ticket double-spend refusal, pending paid work recovery,
   crash during transfer/activation, partition, and another successor transition.
   The fixture helpers now capture old/new rosters at the same real checkpoint,
   permit an ordinary daemon-generated post-freeze entrant and expose all ordinary
   nodes to the exercise. No fabricated signer or archive is needed.
5. Solve closing after an old-authority outage separately. The present live
   SpendSession closes only while its original selected authority is current.
   Historical verification or a later registry snapshot must never reopen ordinary
   old spending. Any terminal-only recovery authority needs explicit scope, durable
   fencing and genuine outage/double-spend tests before it can release a signer.
6. Add the integration cases noted by the node critic: closing admission with a
   full ordinary queue, malformed/mixed input without mutation, and a hostile
   closing recovery carrier through actual selected-peer transport. Preserve
   unresolved paid inputs; capacity or expiry cannot justify a silent refund.
7. Continue automatic client/sender authority and context renewal, stale-input
   reconciliation, successful sender retirement/fair scheduling, paid network
   history/R10, UI/CLI/MCP, groups/device recovery, independent operators and
   three-platform release acceptance under the unchanged release scope.

One failure exposed a concrete lifecycle coupling: returning no candidate scope
after closure also disabled the historical record server. The corrected path
retains its configured scope with an empty broadcast list, so closed history can
still serve authorized recovery. Preserve this separation during handover work.

All build/check commands use build-storage.py from /Users/glebk/Code/chat. Source
and Git stay internal; managed artifacts and verification workspaces stay on
ChatBuild. Preserve the user's unrelated maintenance document and media directory.
