# Separate backend test critic

Reviewer: `/root/common_context_test_critic`, spawned with no inherited context.
The six Core scenarios and their contract preceded all production changes.

1. **REVISE:** the new eviction test incorrectly required all eighty successors to
   precede the original 35-second lease deadline. Isolate unexpired revocation on
   the first page, then continue the full tail for eviction/restart. The reviewer
   also suggested a denied recheck clock commit and stale-context owned preparation.
2. **FINAL ACCEPT before production:** the first-page assertion now isolates head
   revocation from expiry; all eighty successors remain. A denied recheck persists
   its newer clock across restart. Owned preparation rejects the old unexpired
   context after a genuine renewal and succeeds with a fresh identical statement,
   preserving nullifiers and paid rows. The remaining unisolated case is a genuine
   paid leaf expiring before the common interval ends; no new coverage claim for it.
3. **ACCEPT for a test compilation correction:** once APIs existed, Rust exposed
   E0502 in the test's `v["history"].push(v["shortRenewed"].clone())`. Cloning the
   renewal before taking the mutable history borrow preserves all assertions.
   The critic independently reversed that correction and obtained the previously
   accepted test hash exactly. This compilation failure is not behavioral RED.
4. **FINAL ACCEPT for deeper real-process verification:** an additional feature
   test runs the genuine prover, fixed verifier and upstream receipt oracle, with
   exact public context/result/journal comparisons. The critic regenerated both
   chains and all eight expected journals using the independent Python CBOR oracle.
   It covers a receiver without paid wallet state, altered interval rejection,
   unexpired old-head revocation and fresh registry/context renewal. It was added
   after the six Core tests and implementation as deeper acceptance coverage, not
   represented as a new behavioral RED run or live network admission.
5. **ACCEPT for shared test helper placement:** feature Clippy detected duplicate
   path loading of the existing CLI helper. Both process suites now import the
   single feature-gated parent module; helper code, assertions and deadlines are
   unchanged. Clippy passes after the correction. The original lint failure was
   before expensive proof execution and is not behavioral test evidence.

The initial and revised baseline commands exited 101 for missing public Core
types/methods. Raw logs and successive accepted input hashes are retained beside
this record. No dependency, guest relation or verifier image change was required.
