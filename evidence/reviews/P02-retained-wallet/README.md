# Retained paid-wallet ancestry

Implementation and eight test-first retention scenarios received independent
FINAL ACCEPT. The broader regression remainder exited 0. Full V1 is not
complete, and the initial aggregate check failed in a separate operator-network
scenario; do not interpret this directory as an aggregate GREEN.

Core retains immutable original paid ancestry and one shared current-head index.
The index exactly describes the retained records for existing intents. Both head
APIs atomically update at most three states, and binding at most four, preserving
Store's limit of 16 and wallet capacity of 32. Original funding and anchor bytes
are not rewritten by head advancement or valid retries. Current authority, paid
expiry and monotonic time remain necessary at use; historical verification alone
does not grant authority.

The first implementation updated one cursor per paid owner. Its code acceptance
was revoked after a real 32-owner wallet reproduced Store(InvalidInput). Revised
tests received FINAL ACCEPT before the shared-index correction. They cover full
capacity, 80 signed successors, eviction from the unchanged 64-entry peer archive,
SQLCipher restart, all four journals/nullifiers for two paid owners, independently
selected assignments for all 32 owners, refreshed operator membership, corruption,
atomic first/second bind and advancement faults, and sequential legacy migration.
They preserve rejection of oversized or incomplete untrusted peer histories.

The public corpus contains two real local EVM chains, 32 distinct paid commitments
per chain and 160 new authenticated heads. Its generator passed 248 actual Rust
CLI checks. Public test seeds are explicitly identified; this is not a source of
production keys or current mainnet authority.

The current code has passed 45 Core registry tests (including the eight
retention tests), all 479 ordinary Rust tests, nine genuine proof/CLI and two Core
process tests, seven models, nine oracles, 40 frontend tests, formatting, Clippy,
TypeScript/Vite, 29 contract tests and initial EVM checks. Real proof timings were
391091 ms / 584740 bytes and 442818 ms / 584787 bytes, with the unchanged image
ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de.

The aggregate operator-network failure was an eight-second pending timeout after
provider restart. One complete rerun passed unchanged, including both TCP/QUIC
chains and all 16 remote roles. Its cause remains unestablished. See
`Docs/maintenance/operator-network-timeout-20260907.md`; both failed and successful
reports are preserved in the raw output directory. No test threshold was relaxed.

The remaining EVM runners passed, including two genuine random-owner shared-root
proofs, custody resolution and selected finalizer services on TCP/Noise and QUIC.
The service verified 159 quorum signatures. All 19 fresh EVM reports are preserved
under `evm/`; public shared-root receipts and their independent-verifier evidence
are under `shared-root/`. All six accepted test inputs, 323 source inputs and three
proof binaries remained unchanged through validation.

Accepted contracts/hashes and review decisions are retained here. Raw execution
logs are under `output/postage-proof/retained-wallet`. `evidence.json` distinguishes
the failed aggregate run from subsequent successful component runs. Native,
Linux, release packaging, daemon proof jobs, fresh receipt admission, canonical
spend and the remaining V1 product scenarios are not accepted by this increment.
