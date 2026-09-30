# Continue into ordinary manifest retrieval

1. Core import is now accepted in [AR3-history-import](../AR3-history-import/README.md).
   It binds bounded progress to each actual manifest reference.
   Exact descriptor/ciphertext, original message/MLS/dedup/progress transaction,
   revision fencing and explicit missing/expired references are covered there.
   Continue with native steps below; no runtime completeness is implied.
2. Add manifest put/read and exact anchor retrieval to the existing bounded Noise
   custody transport. Authenticate the actual paid anchor/operator; keep original
   funding/QC/receipt and shared capacity limits. Publish only after required
   index acknowledgments. No extra spend or owner-only retrieval path.
3. Use the new commitment and bound Core acceptance from ordinary sender/recipient
   paths, replacing live-job index-roster intersection. Prove genuinely funded
   disjoint books, no common index, sender absent, cold recipient/providers,
   missing/corrupt manifests/references, SQL faults and expiration with one frozen
   application source/binary. Legacy compatibility is a separate regression.
4. Real control/epoch and first-Welcome delivery, successful sender retirement,
   attachment coverage, historical-row pruning and autonomous index/data 10→7→10
   repair remain mandatory. Preserve the other AR1–AR5 work, 67 cards /22 E2E /
   three platforms. Full suite only at the end of the complete V1 plan. No push.
