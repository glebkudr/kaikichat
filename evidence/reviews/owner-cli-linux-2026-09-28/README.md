# Owner CLI on Linux — 2026-09-28

The owner CLI `agentic` ([spec](../../../spec/owner-cli-v1.md)) built and
checked on Linux at commit cadcb74.

Environment: the OrbStack container `ain-v1-arm`, Ubuntu 24.04.5 arm64,
rustc 1.91.0, the `portable-linux` build profile. Linux x86_64 is not covered
by this run.

## Process tests

`python3 scripts/build-storage.py --profile portable-linux run cargo test --locked -p agentic-node --test processes owner_cli::`
passed all 8 tests in 5.5 s:
- start, stop and restart from the sealed secrets file;
- kept daemon flags;
- concurrent commands start one daemon;
- two profiles talk;
- JSON refusals and exit codes;
- the inbox with poll, ack, watch and restart;
- grants, including a real `agentic-cli` call before and after the revoke;
- the Linux-only `without_a_secret_service_linux_defaults_to_the_password_file`:
  with no session bus the default is the password file (`password_required`
  without a password).

## A live Secret Service

[`keyring-check.sh`](keyring-check.sh), run under `dbus-run-session` with a
private `gnome-keyring-daemon` 46.1 (secrets component, unlocked, temporary
`HOME` and `XDG_RUNTIME_DIR`), with no `AGENTIC_SECRETS` or password set:

| Step | Result |
|---|---|
| `daemon start`, `init --name Linux` | exit 0, identity created |
| `daemon stop`, then `contacts list` | exit 0: the daemon restarted with the secret read back |
| `daemon status` | the same peer id and network id |
| `secrets.json` in the data directory | absent |
| keyring items for `net.agenticinternet.desktop` | 1 |

So on Linux the default is the Secret Service when one answers, and the
password-sealed file otherwise. The check is manual; the automated suite
never touches a keyring.
