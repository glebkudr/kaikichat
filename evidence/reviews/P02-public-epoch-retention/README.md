# Local public epoch retention

Core retains an authenticated public epoch independently of sender wallet state.
After eighty genuine successor checkpoints, ordinary archive eviction and restart,
it can derive a fresh common context from retained original evidence and a current
registry proof. Original evidence is immutable; a shared chain cursor advances
atomically with checkpoint and archive. Both public and paid lineage changes share
the existing SQL transaction boundary.

The cache has 32 reusable slots (at most 2 MiB each) and one index (at most 16 KiB).
A 33rd live epoch cannot evict a live policy. Only epoch admission expiry permits
reuse, even if an older checkpoint lease expired much earlier. The real capacity
fixture contains 33 contract-created epochs and 99 independent registry checks.
No wallet keys, tickets or receipts are stored by the new cache.

Stored certificates and registry evidence are reauthenticated through the existing
public-policy derivation. Syntactically valid evidence from another chain/root,
missing or orphaned rows and failed SQL writes cannot release new authority or
silently repair the cache. Semantic retries preserve original evidence. Existing
non-retaining authentication and owned-ticket preparation remain compatible; the
immutable epoch comparison is shared with the paid assignment refresh path.

Eight tests preceded implementation and received separate no-context critic ACCEPT
following REVISE. The final RED contained only absent-interface compiler errors.
All eight then passed in debug and release. Full ordinary validation passed 501
Rust tests, seven models, nine oracle tests, formatting/workspace Clippy and forty
frontend tests with TypeScript/Vite.

One additional existing Core process test generated a genuine 584683-byte proof;
the full scenario took 331478 ms. The fixed image, independent complete-journal
oracle, foreign receiver and refreshed authority all passed. This uses historical
fixture clocks. A prior genuine receipt also passes the new local and bundled
verifiers with every result field unchanged. No current-time admission is claimed.
The repeated release cases are not added to the unique count: 501 ordinary plus
one real-process Rust scenario.

Five hidden packaged WKWebView scenarios passed. Four result/reference screenshot
pairs were visually inspected: chat, agent access, network settings and restored
checkpoint. Layout remains unchanged; times, runtime IDs and ports differ. The
subsequent ordinary release bundle passed deep/strict ad-hoc codesign and its
default dependency graph excludes the automation driver. It is not notarized.
All 399 source inputs, seven accepted inputs and five native inputs stayed frozen.
The owned proof worker/parent and isolated native profile were cleaned up.

This increment is a local Rust API. Daemon/MCP use, cold-peer epoch-history
availability, issuer-global canonical spend and paid custody/repair remain open,
along with full V1 acceptance. The full EVM aggregate, Linux matrix and remaining
real-proof process suites were not repeated. Prior packaged current-time proving
is preserved separately in P02-daemon-common-proving; this increment does not claim
a new run of that scenario.

See validation.json for exact outcomes, hashes, raw-log locations and limits;
review.md and accepted-inputs.json record the preimplementation test review.
The fixture generator is crates/core/tests/fixtures/generate_public_epochs.py.
Build/test commands always run through python3 scripts/build-storage.py run.
