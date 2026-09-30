# Native host bootstrap test review

Separate `/root/desktop_host_test_critic`, fork_turns=none, backend-test-critic skill. Initial RED: missing DesktopHost/HostConfig/SecretStore/KeychainStore imports against comment-only new library.

Initial REVISE: listener equality could pass with empty/incomplete sets; missing concurrent first launch; no failure test for occupied saved port/child cleanup. Added concrete TCP/QUIC nonzero port and PeerID assertions, tokio::join concurrent bootstrap with single secret/child, busy TCP failure with UDP released and original identity/ports restored. Also tested malformed keys, canonical aliases, private directory creation/shared-directory rejection and observation of secret-save-before-DB/socket. Reviewer returned ACCEPT, no blocking or missing business scenarios for this slice. Suggested optional helper-level readiness and test-wide timeouts; implementation itself bounds lock/startup/IPC waits.

Implemented actual keyring4.2.0 OS store, daemon anonymous stdin bootstrap, existing bounded authenticated IPC client reuse, private bootstrap lock and synced public port configuration. No plaintext master-key fallback. Tests use narrow vault failure injection plus an actual unique disposable macOS Keychain entry (removed after check). No user credential entries accessed.

Validation: ten host integration tests passed (including occupied-port recovery and OS Keychain); full scripts/check.sh exit0,101 Rust integration tests +15 frontend tests, strict Clippy, formatting, TypeScript and Vite production build. Full native app and remaining V1 goals are not complete.
