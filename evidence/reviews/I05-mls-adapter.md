# Staged MLS adapter test review

Scope: `spec/mls-adapter-v1.md`, `crates/crypto/tests/mls_flows.rs`. Separate critic `/root/mls_test_critic`, fork_turns=none. Production was only a comment. RED compiled upstream OpenMLS and failed on missing MlsClient/Prepared APIs.

First verdict REVISE: no immutability oracle for group operations, Welcome replay was an ambiguous overwrite test, and short reorder tests could accept incorrect upstream defaults. Added discarded group change/state assertions, fresh valid Welcome from another creator with same group ID, 128-message reordered batch and three retained epochs/fourth retired across snapshot restore. Added malformed control/replay and raw-peer signature-key evidence. Tests formatted before re-review.

Second final verdict ACCEPT, no blockers. Nonblocking future additions: unsupported-suite admission, valid oversized wire, stronger explicit counts in SQLCipher integration. No implementation was written before ACCEPT.

Implementation now passes 15 integration tests, including raw OpenMLS peer interoperability and real SQLCipher write failure. Whole backend 59 tests and frontend 13 tests pass, formatting/Clippy/TypeScript/Vite build pass. `cargo tree -p agentic-crypto -e features` contains no OpenMLS test-utils, crypto-debug, content-debug or file-persistence features.

This is not authorization/finalizer/recovery/P2P/native application E2E evidence. Those gates remain open.
