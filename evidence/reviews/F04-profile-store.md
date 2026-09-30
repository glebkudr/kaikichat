# Encrypted profile storage test review

Scope: `spec/store-v1.md`, `crates/store/tests/profile_store.rs`. Separate agent `/root/store_test_critic`, fork_turns=none. Production file contained only a comment during both reviews. Initial RED: `cargo test -p agentic-store --test profile_store` compiled real SQLCipher but failed on unresolved unimplemented APIs.

First verdict REVISE: no test proved successful updates of an existing state revision; an implementation refusing second-message state advancement could pass. Added send → receive → send with revisions 0→1→2→3, exact records and bytes, reopen, outgoing-only queue and selective ack. Also checked WAL mode and unchanged retry bytes.

Second final verdict ACCEPT for 20 tests in this bounded slice, no blockers. Deferred checks explicitly include process locking/crash harness, keychain integration and network E2EE. Production was implemented only after ACCEPT.

Validation after implementation: 24 wire + 20 real profile-store tests pass. `cargo clippy --workspace --all-targets -- -D warnings` passes. Frontend 13 tests pass, TypeScript check and Vite production build pass. This review does not complete whole F04/I01 or the product.
