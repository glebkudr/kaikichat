# N01 / F03 direct runtime test review

Separate critic `/root/node_test_critic`, fork_turns=none, backend-test-critic skill.

Initial RED: core integration tests failed compilation because create_node_record/receive_from did not exist. Initial daemon IPC test compiled against an empty entry-point scaffold and failed because the child immediately exited. No runtime production code existed before review.

First verdict REVISE: a malformed token did not test wrong but well-formed secrets; oversized-frame test allowed application-level rejection and therefore did not prove codec bounds. Fixed with an alternate valid 32-byte hex token and mandatory transport failure. Added raw-root-signed invalid NodeRecord lifetime/canonicality, malformed/mixed route rejection before mutation, listener suffix and stdout checks, verified duplicate receipts, and an independent libp2p receiver proving identical wire retries after invalid and lost acknowledgments.

Second verdict ACCEPT. No blocking issues or missing scenarios for this slice. Critic noted possible Welcome timing sensitivity. Actual suite ran successfully. Follow-up ACCEPT for explicit Multiaddr type annotation, then ACCEPT for two Clippy let-chain formatting changes; test assertions unchanged.

Implementation uses actual SQLCipher, staged OpenMLS, libp2p QUIC/TCP+Noise, durable outbox, and independent child processes. No transport/UI success is inferred from documentation tests.

Validation on implementation branch: scripts/check.sh exit0; 91 Rust integration tests (24 protocol,22 store,16 MLS,21 core,8 process/runtime),13 frontend tests, cargo fmt, strict workspace Clippy, TypeScript, Vite production build. Runtime process suite completed in2.71s in the full check. Native UI, agent scopes, R=10 and remaining upstream acceptance are still open.

DTO follow-up: desktop reads `network.state=online`, but the first runtime emitted `connected`. Added a process exchange assertion for the shared DTO, observed RED (`connected` vs `online`), separate critic ACCEPT, changed runtime state to `online`. Full 91 backend/13 frontend checks pass again, with formatting, strict Clippy, TypeScript and UI build.
