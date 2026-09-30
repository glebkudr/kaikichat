# Same-pass page preparation reuse

The ordinary sender passes its already verified retained page and exact paid index entry to a private shared queue helper within the same synchronous call. It avoids repeating storage/signature preparation and clones page bytes only for a genuinely new transport job. Live connection, wire kind, exact commitment/peer/position, request conflict, four-stream capacity, real Noise ACK and persisted paid-page ledger remain unchanged. No prepared trust or page context survives a Runtime turn.

The standalone preparation adapter now exists only in tests; it uses real storage getters followed by the actual production queue/response/store helper. Independent test-coverage and implementation review ACCEPT preceded the change. Existing tests preceded this behavior-preserving extraction. Actual targeted checks:17 backend and51 frontend, production Clippy and Rust format all exit0. Logs and exact hashes are in history-page-preparation-checks.json; review in history-page-preparation-review.json.

Native130 R9 tested the prior coalescing revision, and failed during recovery despite all130 genuine originals being published in2522seconds. This additional extraction has no native130 acceptance or demonstrated whole-run timing yet. Source changes began only after terminal R9 collection,876 unchanged inputs and clean teardown. This is not full V1 acceptance.
