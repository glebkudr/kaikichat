# Platform boundary of the local daemon

Source snapshot of September 13, 2026. This is a map of the unfinished U06/U07,
E01/E24 implementation, not a Windows build result. The full Full130 runs separately
on macOS; its success does not close this part of V1.

## Confirmed change sites

| Area | Current behavior | Required outcome |
| --- | --- | --- |
| `crates/node/src/lib.rs`, `main.rs`, `bin/agentic-{cli,mcp}.rs` | Runtime and adapters are gated behind `cfg(unix)`; Windows gets a launch refusal | One runtime and the same CLI/MCP on all three OSes |
| `crates/node/src/ipc.rs` | UnixListener/UnixStream mixed with framing, owner-token, and signed-agent admission; the guard removes the socket file | Platform bind/connect/cleanup under shared framing and authorization; a Windows local-only pipe |
| `crates/node/src/runtime_client.rs` | Private file check via mode/dev/ino, then agent seed loading | An equivalent check of access and open-file identity on Windows; a publicly accessible file is not accepted |
| `crates/node/src/mcp.rs::materialize` | Unix 0700/0600, temporary file, rename, and directory fsync; suffix-less CLI/MCP names | Private atomic key materialization and the real paths of the deployed executables on each OS |
| `crates/desktop-host/src/lib.rs` | Connect/save_listeners are Unix-only; 100-byte socket path limit; Unix modes and directory fsync | Private profile, one bootstrap lock, a stable endpoint, reconnection, and atomic settings on Windows |
| `crates/node/src/postage_jobs.rs` | Unix executable mode and suffix-less sidecar names | Verification of the actually deployed files on the target OS; ordinary public sending stays independent of the legacy prover |
| `scripts/build-desktop.mjs` | macOS storage guard, bash helper, sidecars without a Windows suffix, Tauri bundle `app` | Target installers and resources; local macOS builds keep using the current storage wrapper |
| `crates/node/tests/processes.rs`, `ipc_tests.rs`, desktop-host tests | Real Unix processes/sockets and Unix permissions | Shared scenarios over the platform transport plus real Windows ACL/pipe checks |

The change cannot be limited to removing `cfg(unix)` or adding a connect branch:
private agent credentials and daemon bootstrap are part of the same boundary.
Do not replace local IPC with a network server for the sake of a platform port.

## Implementation and acceptance order

1. Tests of the shared boundary first: a valid owner call, a real signed agent,
   a wrong owner token, the current grant/revoke, an oversized frame, a slow client,
   cancellation before dispatch, and two clients at once. Reuse the existing
   tests and framing; the platform adapter must not duplicate the broker or Core.
2. For Windows, separately verify the privacy of the endpoint/profile/key, refusal of
   a connection from an unauthorized local user, pipe locality,
   a second-daemon conflict, and a relaunch after a stop. Real
   user isolation requires verification on Windows; a mock ACL does not prove it.
3. Verify writing and reading agent credentials together: the exact seed after
   a cold restart, refusal of a public/substituted file, atomic replacement on
   re-materialization, and the absence of a corrupted key after an error.
   The new process test must launch the CLI/MCP commands from the actual
   provisioning response, including paths with spaces and executable suffixes.
4. A separate backend-test-critic without inherited context accepts the tests before
   production changes. After implementation — targeted backend and frontend checks,
   then a real Windows daemon/CLI/MCP/desktop smoke test and cold restart.
5. A native installer, keystore, and voluntary update remain separate
   conditions of U06/E01/E24. Debug smoke, cross-compilation, and the macOS native gate do not
   replace a clean install and launch on every target OS.

Automated checks must not request the system keystore password.
The current E2eFileStore is limited to Unix debug/e2e; its Windows equivalent must
keep the temporary profile isolated and be deleted after the processes stop.
The ordinary keystore and the prohibition on including the E2E store in a release are preserved.
