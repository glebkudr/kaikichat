# Continue the full V1 goal

Do not mark AR1 or V1 complete on historical receipt recovery. Preserve 67 required
cards, 22 E2E and macOS arm64 / Windows x86_64 / Linux x86_64.

1. Implement authenticated epoch closing, unique successor authority and full
   issuer spent-state continuity. Retain the epoch !=1 refusal until real
   multi-epoch crash, partition and double-spend tests pass. Refresh authority
   must extend service availability without replacing or resetting spent history.
2. Reconcile expired/unverifiable retained candidates and pending work. Known
   finalized facts must survive expiry; exposed tickets never become refundable.
   This recovery path requires a live selected service and does not itself allow
   a node to synchronize while its authority is expired.
3. Finish successful sender retirement and fair ready scheduling. Continue paid
   index transport/history discovery and automated R10 repair, ordinary user/agent
   entry points, groups/device recovery and independent release gates. See the
   architecture followup graph for the remaining required work.

Use /Users/glebk/Code/chat, the build-storage wrapper and tests before production.
Run a separate backend-test-critic without inherited context after test changes.
Treat test-specific carrier failures as fixture evidence, not production failures.
Keep the previous lifetime gate's 144-spend evidence separate from this receipt
recovery gate; two recovered/continued spends do not replace the larger lifetime
regression or prove cross-epoch behavior. Unrelated user media and maintenance
notes must remain untouched.
