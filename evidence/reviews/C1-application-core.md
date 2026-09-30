# Application core test review

Reviewed new `crates/core/tests/conversations.rs`, two store API tests, and KeyPackage inspection tests. Spec: `spec/application-core-v1.md`. Independent critic `/root/core_test_critic`, fork_turns=none. Production core was only a comment; added store/crypto APIs were absent. Critic independently reran all three targets and confirmed RED for missing APIs.

First verdict REVISE: a successful retry could still decrypt after an incorrectly persisted ratchet advancement; no adversarial authentication test covered initial Welcome. Added raw SQLCipher snapshots of every state namespace/revision/value and operation counts around failure, retries and rejection, including reopen before retry. Added a legitimately signed but falsely attributed Welcome, unchanged persisted invitation/crypto/contact state, then acceptance of the original Welcome. Also added incoming Welcome DB failure, independent receipt author/body checks, correctly signed wrong receipt references, populated DTO fields, repeat onboarding, and exact durable message lookup.

Second final verdict ACCEPT: all blocking issues resolved, no required missing scenarios for this slice. Production implementation followed ACCEPT.

Current full `scripts/check.sh`: **79 backend tests** (17 core + 16 MLS + 24 wire + 22 store), **13 frontend tests**, formatting, strict Clippy, TypeScript, Vite production build all pass. Messages are real signed wire and OpenMLS ciphertext between independent SQLCipher profiles. Tests pump wire explicitly; this is not evidence of network processes, native Tauri, MCP, R=10 repair or transport economics.
