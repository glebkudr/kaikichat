# Continue into ordinary manifest discovery

1. Exact locator/anchor binding is now accepted in
   [AR3-history-pointer](../AR3-history-pointer/README.md), including epoch,
   revision, expiry, cold integrity and current-pointer-bound Core acceptance.
   Ordinary network use remains required: a first index page need not contain the
   longest-lived anchor, and missing/stale data is not an empty-history success.
2. Add bounded Core import progress bound to each actual manifest reference. Check
   the returned descriptor/ciphertext and commit original message, MLS, dedup and
   completion together; missing/expired entries remain explicit. A new directory
   revision must not erase prior successful imports or authorize changed bytes.
3. Serve manifest put/read over the existing bounded authenticated custody swarm,
   using the exact paid anchor and actual Noise peer. Publish only after required
   index acknowledgments; do not introduce a second spend or owner-only retrieval.
4. Replace ordinary live-job index intersection, then run actual funded multi-book
   acceptance with no common index, cold sender-absent recovery, missing/corrupt
   entries, SQL faults and expiration on one frozen application source/binary.
5. Actual MLS control/epoch and first-Welcome delivery, safe successful retirement,
   attachment coverage, historical-row pruning and autonomous index/data 10→7→10
   repair remain open. Preserve AR1–AR5, 67 mandatory cards, 22 E2E and three
   platforms. Full suite only at the end of the entire V1 plan; no push.
