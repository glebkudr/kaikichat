# Profile host (V1)

The host starts or reuses a profile's daemon and hands its owner an
authenticated client. It is `agentic_node::host` (`crates/node/src/host.rs`),
shared by the owner CLI `kaiki` and the desktop app; `crates/desktop-host`
re-exports it for the Tauri shell together with the E2E secret store. What
the owner sees of it is in [owner-cli-v1.md](owner-cli-v1.md) (data and
secrets) and [desktop-gui-v1.md](desktop-gui-v1.md) (one profile, one
daemon).

## The profile directory

Created private (0700); an existing directory with group or other access is
refused, never chmodded. It holds:

| File | Purpose |
|---|---|
| `profile.db`, `profile.db.lock` | The SQLCipher store and its writer lock ([store-v1.md](store-v1.md)) |
| `node.sock` | Owner IPC of the running daemon (the path must fit in 100 bytes) |
| `.bootstrap.lock` | Serializes daemon starts and changes of the network and release files |
| `daemon.json` | The daemon's saved flags (`kaiki daemon start`) |
| `listen-addresses.json` | The listen addresses the daemon actually bound (1–8), reused on restart |
| `network-preset.json`, `release.json` | The signed network presets as served with the last check, and the newest one fetched for the release it names ([Docs/V1_NETWORK_PRESET_2026_09_28_RU.md](../Docs/V1_NETWORK_PRESET_2026_09_28_RU.md)) |
| `autostart.json` | The owner's start-at-login choices |
| `secrets.json` | The password-sealed secret, with the `file` backend |
| `node.log` | The daemon's output (mode 0600) |
| `runtimes/` | Scoped agents' credentials ([agent-grants-v1.md](agent-grants-v1.md#credentials)) |

## Secrets

- One secret per profile, exactly 64 bytes: the SQLCipher master key and the
  owner IPC token. Its account is the hex SHA-256 of the canonical
  `profile.db` path, so aliases of one path share it.
- Backends behind the `SecretStore` trait: the system keychain (`keyring`
  4.2.0: macOS Keychain, Linux Secret Service, service
  `net.agenticinternet.desktop`), the password-sealed file (Argon2id and
  XChaCha20-Poly1305), and for automated tests only the `E2eFileStore` of
  the `e2e` feature, which a release build refuses to compile.
- A new profile's secret is drawn and saved before the database exists or the
  daemon starts. A database without its secret, a secret of the wrong size or
  a store that cannot be read is an error; a replacement key is never made
  up. The existing database is left as it is, so the identity comes back
  once the secret does.

## Starting and reusing

- Under `.bootstrap.lock` (waiting up to 20 seconds for another start), the
  host asks `node_info` over `node.sock`. A running daemon whose listeners
  are ready is reused.
- Otherwise it starts `kaiki-agentic-node serve --profile … --ipc … --secrets-stdin`
  with the saved listen addresses and flags (taking the network from the
  signed preset when the profile has none), writes the secret as one JSON
  line on the child's stdin and removes the password variables from its
  environment. No secret goes into arguments, the environment, logs or
  profile files.
- It waits up to 10 seconds for `node_info` with the configured listeners
  bound, saves the actually assigned addresses and returns the client. A
  child that exits or never gets ready is killed. A saved address that can
  no longer be bound fails the start visibly; no other address is invented.
- Closing or dropping the client leaves the daemon running. Owner calls go
  through the daemon's own IPC client; remote errors stay errors.

## Tests

`crates/desktop-host/tests/bootstrap.rs` starts real daemons: the secret is
saved before any database or socket exists, reopening reuses the running
daemon, a locked or missing secret keeps the database bytes and the identity
returns with access, a crash restart keeps root, peer id and listen ports,
concurrent starts make one secret and one child, a busy saved port fails
without fallback, and invalid owner methods never succeed. They use an
in-memory secret store; the real keychain round trip is a separate test,
ignored unless run on purpose.
