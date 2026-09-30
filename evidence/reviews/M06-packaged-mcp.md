# Packaged native UI to MCP acceptance

Separate /root/native_test_critic reviewed the extended native-e2e.mjs and independent mcp-client.mjs before packaging changes. FINAL ACCEPT. RED: selected previous debug .app lacked sibling agentic-mcp. Production change copies both Rust binaries into Tauri externalBin; no arbitrary shell API added.

Actual scripts/check-native.mjs completed successfully as one command: full144Rust/20frontend + format/Clippy/TypeScript/Vite, debug .app build/signing, hidden WKWebView product E2E, release build/signing, deep/strict verification and exclusion of WebDriver from default dependencies.

E2E config is read from the native panel after explicit name/dialog/send selection. The helper launches exactly that selected bundle's MCP binary; raw legacy JSON-RPC initialization/tools, actual peer send/reply, three exact incoming messages, ack/empty next page and UI revoke denying the same live process all pass. Both final histories match. Screenshots in output/native-e2e were visually inspected against component agent references. Test clients/MCP/daemon/keychain state cleaned up; no test processes remained.

Release artifact: target/release/bundle/macos/Agentic Internet.app, macOS arm64, all three executables. Ad-hoc signed; not notarized. FullV1 goal remains active; bounded scope discovery and other incomplete requirements are recorded in IMPLEMENTATION_STATUS.md.
