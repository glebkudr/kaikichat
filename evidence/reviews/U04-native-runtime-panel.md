# Native runtime panel review

Native backend test first, separate /root/native_test_critic (originally no context fork). RED: list_runtimes not found in Tauri. REVISE: denied grant_runtime needed valid arguments, not an empty body. Corrected with real revoked principal, fresh logical IDs, valid dialog/actions/expiry/limit, exact Tauri denial and unchanged registry; added approved metadata checks. FINAL ACCEPT before native handler/build ACL changes.

Frontend tests first: AgentPanel missing import RED; tests cover explicitly selected dialogs/read defaults, exact configuration, unchanged complete retry vs edited operation, failed/successful exact grant revoke, API mapping, chat draft preserved across panel navigation.

Validation: scripts/check.sh PASS (144 Rust +20 frontend, formatting/Clippy/TypeScript/Vite). Headless Playwright CLI form→config→revoke visually checked at1280x840/900x650 against component-chat/contact references. Four component-agents screenshots retained. Only missing favicon404 in fixture console; no app errors. Browser closed and no orphan processes.

Packaged native UI→MCP is the next separate test-first gate. Current release .app still predates this feature. No V1 completion claim.
