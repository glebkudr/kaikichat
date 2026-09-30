# Build evidence and preflight

Python 3.11+ is required for standard-library Cargo TOML parsing. All commands
still run through `scripts/build-storage.py`. mac-apfs remains strict by default;
Linux requires the explicit portable profile described below. Keychain, protocol
deadlines, TTL and queue limits are unchanged.

`scripts/build-desktop.mjs` keeps the existing TypeScript, Vite, `agentic-node`,
sidecar copy and Tauri sequence. Each command writes its own log and structured
running/final status, elapsed time, deadline and failure. Build stages use a
1800-second budget each. A timeout stops the owned command process group,
including descendants. Host-target/copy/seal failures are recorded too. No cache
is deleted.

Every desktop build attempt gets a new output directory under
`output/desktop-build/` unless `--output` names a fresh one. Its `check.json`
and logs are retained. A successful build seals `prepared-artifacts.json` and
reports `prepared: true, passed: false`; the build alone is not runtime
acceptance.

## Native runtime preflight

`doctor --suite native-runtime --phase runtime` checks what a daemon scenario
needs from the host once its binaries exist: Python, Git and local Unix stream
IPC (socket creation, private temporary-path bind, listen, client connect and
accept). Connect and accept each have a one-second timeout; every opened socket
is closed and the temporary socket path/directory are removed. The `unix-ipc`
requirement records each operation's status and errno, including `EPERM` as
`blocked`, plus cleanup results. A cleanup failure reports `failed`. This check
needs no netlink access or privilege. The suite has no build phase: `--phase
build` is `unsupported` (use `native-build`), and other suites do not run the
probe. A tool-version PASS alone cannot certify daemon IPC availability.

## Provenance and limits

The producer records Git HEAD and hashes of tracked/nonignored untracked inputs,
including locks and configuration, before building. A source, lock or ref change
during the build, including a new HEAD with identical files, prevents sealing.
The producer records tool executable identities (resolving Cargo/rustc shims),
environment hashes, host target, effective target directory, Cargo configuration
and vendor directory hashes. The actual Node executable used by the JS producer
is bound. Environment values are hashed because some contain credentials. Only
settings that reach the desktop build are bound: Cargo/Rust, Tauri, Vite, npm,
sccache, C/C++ compiler and linker, pkg-config, macOS SDK, `PATH` and
`NODE_OPTIONS`. Foundry, solc and RISC Zero settings are not part of it.

The sealed manifest is a review record. Nothing in the repository reads it back
to skip a build; `scripts/check-native.mjs` rebuilds the bundle it tests.

The producer uses host artifacts under this checkout's `target`. An effective
`CARGO_TARGET_DIR` resolving elsewhere or an explicit `CARGO_BUILD_TARGET` or
`CARGO_BUILD_TARGET_DIR` is rejected instead of sealing stale files from the
default output directory. Applicable Cargo configuration files and recursive
top-level `include` files are bound, including ignored and external builder
configuration. Include paths are relative to the including file; present
optional files and the absence of missing optional files are recorded. Required
missing files, invalid entries and cycles fail before building. Includes merge
left-to-right before their including config; effective build/source paths retain
the defining file's Cargo path base. Unsupported `build.target` and effective
`build.target-dir` overrides fail before building. The Mac profile retains links
to its configured external volume; portable storage requires real checkout-local
directories. No environment variable is silently overridden.

This is a local build/review provenance mechanism, not a signature against someone
who deliberately forges both an artifact and its manifest. It is not a hermetic
build claim or product acceptance.

## Focused regression checks

```sh
python3 scripts/build-storage.py run python3 -m unittest discover -s tests/build -p 'test_build_evidence.py' -v
```

The 7 standard-library checks cover persistent command failures, independent
timeout supervision of a TERM-resistant child, and the sealed desktop record: its
source, lock, configuration and artifact content, and the refusal to seal after a
source, lock or ref change during the build. They need no application build. On
Linux these checks can now use the explicit portable profile without a Mac
configuration. This tooling change does not itself claim Linux, desktop or native
acceptance; saved runs remain the evidence for those claims.

## Portable Linux and suite preflight

Run from the checkout root. A missing Mac config never selects portable mode.
The profile is rejected on other operating systems; Mac/APFS identity and link
checks remain mandatory for the default profile.

```sh
python3 scripts/build-storage.py --profile portable-linux setup
python3 scripts/build-storage.py --profile portable-linux --output output/python-build-001 run python3 -m unittest discover -s tests/build -p 'test_build_evidence.py' -v
python3 scripts/build-storage.py --profile portable-linux run python3 -m unittest discover -s tests/build -p 'test_portable_build.py' -v
python3 scripts/build-storage.py --profile portable-linux --output output/frontend-preflight-001 doctor --suite frontend-unit
python3 scripts/build-storage.py --profile portable-linux doctor --suite native-runtime --phase runtime
```

The first pattern selects exactly the existing 7 build-tooling checks. The
wrapper owns its subprocess and persists stdout/stderr, command arguments, cwd,
exit status and elapsed time. Existing reports/logs are
not overwritten. Its command success does not declare a broader V1 gate passed.
The portable contract test simulates Linux host selection when run on macOS;
that subprocess fixture is not evidence of execution on an actual Linux host.

Doctor supports `build-tooling`, `frontend-unit`, `backend-unit`,
`native-build`, `desktop-build`, `evm`, `native-runtime`, and `native-macos`.
Missing requirements are `blocked`, incompatible/broken probes are `failed`,
unknown suites, a phase a suite does not have, or incompatible hosts are
`unsupported`, and satisfied preflight is `passed`. Python checks do not need
Cargo; frontend requires Node 26+, npm and its local JS dependencies, without
Tauri/Foundry. `native-runtime` needs neither Cargo nor Foundry. `evm` requires
Forge/Anvil/Cast; `scripts/check-evm.sh` passes an explicit `AIN_SOLC` to Forge,
and doctor neither requires nor probes it. The Mac bundle producer/native gate
still explicitly requires macOS; Linux dependency readiness does not imply that
a Linux installer was built.

Portable commands preflight every storage path before creating directories or
write probes. Storage paths and their parents inside the checkout must be real
directories; healthy and broken symlinks are both rejected and left intact.
An unrelated Cargo target, `CARGO_BUILD_TARGET` or `CARGO_BUILD_TARGET_DIR` is rejected. Doctor returns
`unsupported` for mac-apfs off Darwin before probing tools. Nested stock scripts explicitly forward the
profile chosen by the wrapper. See [the offline toolkit contract](README-linux-toolkit.md)
for dependency provisioning without network access or Docker.
