# Daemon retained public context

The owner can explicitly pin public epoch evidence without a sender wallet or
proof workers. Receipt verification can use the durable pin with fresh current
registry evidence after peer advancement, provider loss and restart. Explicit
history keeps its previous non-retaining behavior. Responses remain admission:false.

The complete packaged actual-time EVM scenario passed. Two genuine proofs took
351499 and 345767 ms and produced 584691 and 584688-byte receipts. The fixed verifier
and independent oracle matched the complete journal, nullifier and resources.
The source EVM chain was stopped before proving. This report covers one combined
run of the retained-context, common-proving, foreign-verifier and legacy suites.

The new receiver followed 80 actual successors over 20 genuine P2P pages, with no
rejected responses, failed requests or rate limiting. It then verified without
history after provider loss and restart. Cold full-history success did not create
a pin, including after restart; explicit pinning enabled cached verification.
Strict owner access and public inputs, real seal/operation rejection, cancellation,
shared worker capacity, unchanged paid custody rows, graceful held-child shutdown,
crash after completed verification, pending/completed head revocation, expiry and
fresh renewal passed.

The first combined attempt remains a failure: it made a valid independently
verified proof, then exceeded the original 30-second peer wait. The separately
reviewed correction uses 60 seconds to cover the existing 30-second idle poll plus
four pages, preserving production pacing and all assertions. Final batch timings
were 8.369, 33.837, 33.825, 33.787 and 33.739 seconds. This supports the timing
explanation; the exact batch state of the initial failure was not captured.
See review.md and failed-peer-sync-1.json. Raw logs and the first receipt remain in
ignored output/postage-retained/failed-peer-sync-1/ with hashes in validation.json.

Validation also passed 502 ordinary Rust tests, 7 models, 9 independent oracle
tests, fmt/Clippy, 40 frontend tests, TypeScript and Vite. Five hidden packaged
WKWebView flows passed. Four native screenshots were viewed beside the preceding
Core-retention screenshots: layout is unchanged; clocks, temporary IDs and ports
differ. The ordinary release bundle passed deep/strict ad-hoc codesign, excludes
the test driver and remained byte-identical during the final EVM run. It is not
notarized. The only source change after ordinary/native gates was the separately
accepted EVM wait/diagnostics correction, recorded in the initial/final manifests.

Files: validation.json (gates and raw-log hashes), combined-evm.json (full outcomes),
receipt.json and common-receipt.json (public genuine receipts), bundle.json,
review.md, accepted-inputs.json, source-inputs*.json, native-inputs.json,
native-e2e.json, native/ and cleanup.json. All test-owned profiles/processes were
cleaned up; no unrelated process or project configuration was changed.

This is not full V1 acceptance. Cold-peer history availability, UI/MCP spending,
issuer-global canonical spend, paid custody and autonomous R=10 repair remain open.
The full EVM remainder, Linux network matrix, separate real-proof process suites
and notarization were not repeated in this increment.

Reproduce the combined packaged scenario from the canonical source checkout:

```sh
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
python3 scripts/build-storage.py run env AIN_POSTAGE_APP="/Users/glebk/Code/chat/target/release/bundle/macos/Agentic Internet.app" python3 tests/evm/postage_retained.py
```
