# Native desktop tests and packaged WKWebView evidence

Separate backend-test-critic `/root/native_test_critic`, originally spawned with fork_turns=none. No production implementation delegation.

- Native command tests: RED missing NativeBridge/configure exports; ACCEPT. Fixture initially used non-private TempDir itself and correctly failed host directory validation. Changed to a new private child directory, added immediate child RAII and snapshot read denial for secondary/foreign origin; separate final ACCEPT. Three tests passed with real daemon processes and Tauri MockRuntime.
- Event test: RED missing start_updates. Separate final ACCEPT, then implementation and four tests passed. Real remote MLS message invalidates the main window, payload is only null, stable state does not emit continuously, daemon failure emits one transition and snapshot fails.
- Native E2E: tests written while main was empty; RED missing e2e feature. Reviewer REVISE identified a possible stale delivery receipt and leaked daemon on partial startup. Exact message/receipt pairing, confirmed exited UI, own-profile process discovery and awaited cleanup fixed both. FINAL ACCEPT before native main implementation.
- Actual native E2E passed with two hidden WKWebViews, then test changed to require daemon beside the selected desktop executable. RED missing old test-only node path env; separate FINAL ACCEPT. Main now uses the same sibling lookup in all builds. Packaged debug .app E2E passed using its own bundled daemon.

Observed product flow: both users create profiles using UI; Bob creates a real invitation; Alice adds him; actual libp2p/OpenMLS contact, message/reply and signed receipts automatically render without refresh. Alice UI exits, her independent daemon acknowledges a new message, reopened UI has the exact same NetworkID and three exact messages. Two distinct daemon processes are required; no mocked frontend state or protocol. Test uses OS Keychain with isolated entries and cleans up all its resources.

Screenshots `output/native-e2e/alice-chat.png`, `bob-chat.png`, `alice-reopened.png` are actual WKWebView snapshots (2400×1600 pixels for 1200×800 logical window), visually inspected against the earlier component-chat reference. Layout, text, messages, network count and delivered badges are legible with no overlap. Read/unread state remains a future feature and is not inferred from these images.

Full backend/frontend check: 105 Rust integration tests and 15 frontend tests pass, fmt/strict Clippy/TypeScript/Vite pass. Tauri2.11.5, tauri-build2.6.3, CLI2.11.4 and optional tauri-plugin-wdio-webdriver1.3.0 were verified as current compatible versions before installation. Official references: https://v2.tauri.app/develop/tests/webdriver/ and https://v2.tauri.app/develop/sidecar/.

This closes a native integration slice only. All complete upstream task/E01–E26 gates, broker/MCP, R=10 and other required V1 features remain tracked as unfinished. Production release contains no automation dependency; packaged E2E uses the explicit debug feature, not a notarized production deployment.
