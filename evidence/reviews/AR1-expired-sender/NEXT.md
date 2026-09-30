# Continuation after prepared-expiry retirement

Follow Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md and its architecture-followup.json.
The current slice addresses expired prepared jobs only. Do not close AR-R02, AR1,
F04, P01/P03/P04 or any full E2E based on its seven new tests.

1. Design and test the issuer lifetime transition (AR-R01): current full-prefix
   guards in crates/postage-spend/src/lib.rs and finalizer/src/engine/guard.rs
   stop at 128 / epoch 1. Reuse the global issuer spent namespace and existing
   Commonware finalizer. Define indexed durable ticket decisions, authenticated
   bounded snapshots and state transfer before removing any guard. New epochs
   must inherit spent keys; legacy/public formats must share the same namespace.
   A real gate must cross 128 finalized spends under the same issuer and multiple
   authority epochs, with conflict races, crash/replay, partition and lease outage.
2. Complete the remaining sender lifecycle: durable successful completion and
   ready scheduling with bounded work per pump, avoiding rescans of terminal work.
   Do not retire successful jobs by dropping history inputs from the current
   intersected-roster pointer algorithm. Persist the discovery obligation/read
   projection and preserve older routes before removing that work. The stable
   network book-index integration in D05 remains the intended replacement.
3. Define admission timeout/cancellation separately from prepared retention, and
   reservations separately from exposed/finalized tickets. Current cumulative
   sponsorship has no automatic refund; reclaiming queue capacity must not reset
   limits or recycle a released signature. Define retention/GC and antirollback
   migrations for historical terminal/nonce/operation evidence before deleting it.
4. Continue user UI/CLI/read-model/default-build and network index/R10 verticals
   per AR2/AR3, including verified holder locations, disjoint data rosters,
   book/epoch completeness, first Welcome, actual ciphertext and 10→7→10 while
   both clients are offline. Preserve all remaining V1 platform/recovery/economy
   requirements unless the user explicitly changes scope.

Tests first; independent backend-test-critic with no inherited context must
ACCEPT before production changes. Always use the managed build-storage wrapper.
Keep the live 64-validator/R24 gate parked and existing failed evidence intact.
