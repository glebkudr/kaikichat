# Compact finalizer bindings

The Rust implementation, existing daemon transport regression and ordinary Tauri
release have passed their checks. Full V1 and the network service-discovery
architecture remain unfinished.

`verify_transport_hint` authenticates the existing short P-256 binding wire and
its finite lease without granting membership. The opaque hint cannot become a
`VerifiedTransport` without the current committee verifier. That verifier reuses
the same signature/canonical parser and preserves local scope, membership, actual
transport key and authority checks. The old OpenSSL vectors remain byte-identical.

Core can now return the bindings for enabled local selected keys in one call.
A receiver verifies compact bindings against its own current fully proven roster.
Both APIs commit the monotonic clock before releasing their result. Changing the
head requires its real roster proof; rechecking a still-live old binding does not
extend its deadline. No new binding wire, crypto scheme or external package was
introduced. The daemon's automatic discovery still uses its existing algorithm.

Tests preceded production changes. The independent no-context reviewer first
rejected a wrong second-chain transport in the new test. The correction preserved
exact independent-wire comparison and cold verification, shared the existing
registrar fixture, and added valid foreign-scope hint cases and a cryptographically
corrupt but well-formed local roster. The second review accepted all five test/spec
inputs with all 89 production files unchanged. Actual compile RED then confirmed
the missing APIs. Six new tests and four existing routing tests passed; see
`test-review.json` and the reviewer records.

All 551 workspace Rust tests across 73 suites passed, with zero failed or ignored;
40 frontend tests, formatting and Clippy passed. The unchanged live selected
service regression passed TCP/Noise and QUIC with 13 effects per selected profile,
66/60 independently checked signatures and 1,082 owner calls. Cleanup errors were
empty. It checks real daemon/Core/network machinery with a local signed fixture
application, not production paid-ciphertext admission. Both traces are retained.

All 468 source and five accepted test/spec inputs stayed fixed through validation
and packaging. The ordinary Tauri app passed deep/strict ad-hoc codesign, test-driver
exclusion and both genuine historical receipt compatibility checks against the
unchanged fixed image. It is not notarized. `release.json`, `compact-release-run.json`
and `validation-summary.json` retain the results. Native UI and Linux matrix were
not rerun here. Next: signed service address records, bounded
announcements, actual unknown-PeerID DHT lookup, persistent rollback protection,
committee capacity and network role integration, then the remaining V1 scenarios.
