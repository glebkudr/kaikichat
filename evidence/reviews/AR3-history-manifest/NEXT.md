# Required continuation

The subsequent [paid anchor store](../AR3-history-store/README.md) accepts step 1
and the local capability/byte-bounded read primitive. Its [continuation](../AR3-history-store/NEXT.md)
now defines the remaining Core, locator and native work.

1. Store a finite manifest only under its exact already-paid anchor index. Reuse
   historical Core trust, original descriptor/QC, the shared index namespace/CAS
   and quota accounting. Count the signed directory bytes. Preserve immutable
   index receipts; no new storage promise before commit, no lease extension and
   no extra spend. Reject stale/equivocating updates and make exact retries and
   cold reads preserve bytes/checkpoints; test real SQL and quota failures.
2. Add bounded manifest put/read on the existing Noise custody transport. Reads
   need a fresh target-bound recipient capability and exact anchor operation.
   Authenticate the actual replying paid index before releasing references.
3. Add durable Core outgoing revisions and incoming checkpoints, with atomic
   import/progress and explicit per-entry failures. Choose a longest-lived paid
   anchor and retain all live published entries; no current-job intersection or
   early retirement may silently remove their routes. A signed finite list is
   sender-declared content, not independently proven full conversation history.
4. Replace ordinary sender pointer selection and recipient traversal only when
   those durable paths are ready. Test actual funded books with empty common
   index intersection, distinct candidate routes, cold absent Alice, partial
   failures and expired shorter-lived entries. Keep the current one-book/direct
   paths covered. Do not claim that disjoint keys in crypto vectors are funded
   R10 rosters or that the crypto checkpoint test exercises persistence.
5. Complete actual MLS control/epoch catch-up and first Welcome, safe successful
   retirement, attachments and autonomous index/data 10→7→10 repair. Preserve
   AR1–AR5, 67 V1 cards, 22 E2E and all three platforms. Full suite at the end of
   the complete V1 plan, not after each prerequisite.
