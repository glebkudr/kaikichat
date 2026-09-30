# Project storage and builds

## Working paths

- Sources and the standalone Git: `/Users/glebk/Code/chat` on the internal APFS.
  On this macOS machine use this cwd in all existing tasks, even if a task
  remembers an older one.
- Published source: `https://github.com/glebkudr/kaikichat`; the development
  checkout keeps its existing remote and history, working branch `implementation/v1`.
- Build image: `/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle` on the
  external WD4000.
- Mounted APFS volume: `/Volumes/ChatBuild`. The physical data of that volume
  lives in the image on WD4000, not on the internal disk.
- Local parameters of the image, UUIDs and tools: `.local/build-storage.json`.
  This file is not part of Git; its backup lives at
  `/Volumes/WD4000/Code2/chat/.local/storage-migration/build-storage.json`.
- Project Node/Foundry/solc: `.local/toolchains`. The shared Cargo/Rust and
  OrbStack stay in their regular places; do not touch other projects or shared
  storage.

The external checkout `/Volumes/WD4000/Code2/chat` and the local
`.local/pre-migration-*-20260906` are archives; do not develop or build there.
The old path `Library/Caches/agentic-internet/worktree` is a compatibility
symlink to the main repository, but name the main path in new commands.
The container `ain-v1-arm` (OrbStack) mounts
`/Volumes/WD4000/Code2/chat_builds/workspace` as `/workspace`; the Linux
checkout for builds is `/workspace/chat-arm64`, the data stays on WD4000
outside the shared btrfs volume. The previous container with the named volume
is kept as `ain-v1-arm-prev`; do not start new runs on the old volume.

## Commands

Run from `/Users/glebk/Code/chat`. For any build or check use the shared
wrapper; it attaches the image in the background and sets the tool and cache
paths.

```sh
python3 scripts/build-storage.py mount          # Attach the image without starting a build
python3 scripts/build-storage.py check          # Verify the volume and symlinks without attaching anything
python3 scripts/build-storage.py setup          # Restore empty directories and missing symlinks
python3 scripts/build-storage.py install        # Restore npm from the lock file inside the image
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
python3 scripts/build-storage.py run scripts/check.sh
python3 scripts/build-storage.py run node scripts/check-native.mjs
python3 scripts/build-storage.py run node scripts/check-network.mjs
```

The same format applies to individual commands:
`python3 scripts/build-storage.py run COMMAND...`.
`run` attaches the image automatically; there is no need to run `mount`
separately each time. The built release app:
`target/release/bundle/macos/Kaiki Chat.app`.

### Explicit portable profile for Linux

On Linux work in the root of the current checkout and always choose the
profile explicitly:

```sh
python3 scripts/build-storage.py --profile portable-linux setup
python3 scripts/build-storage.py --profile portable-linux doctor --suite build-tooling
python3 scripts/build-storage.py --profile portable-linux run python3 -m unittest discover -s tests/build -p 'test_build_evidence.py' -v
python3 scripts/build-storage.py --profile portable-linux --output output/build-evidence-001 run python3 -m unittest discover -s tests/build -p 'test_build_evidence.py' -v
```

`portable-linux` is available only on Linux. It uses `target`, `output`,
`cache` and `.local/verification-workspaces` of the current checkout and never
calls APFS, `diskutil` or `hdiutil`. These paths and their parents inside the
checkout must be ordinary directories: any symlinks are rejected before
directories are created or anything is written, and are left untouched.
`CARGO_TARGET_DIR` must point at the target of this checkout;
`CARGO_BUILD_TARGET` and `CARGO_BUILD_TARGET_DIR` are forbidden. The profile
mounts no images and installs no dependencies. The `run` command starts the
child process from the absolute root of the checkout. With `--output` it saves
a command log and `check.json`; a new attempt needs a new directory. Without
`--output` a plain exec is recorded.

The default profile is `mac-apfs`: a missing `.local/build-storage.json`,
volume or utilities, or a UUID/APFS/image/symlink mismatch remains an error,
including for `check`. The presence of Linux or the absence of the Mac config
does not enable portable automatically. The wrapper passes the chosen profile
to the nested first-party scripts through `AIN_BUILD_STORAGE_PROFILE`; those
scripts again name `--profile` explicitly. `build-storage.py` itself does not
choose the profile from an environment variable.

Doctor checks only the selected suite and reports `passed`, `failed`,
`blocked` or `unsupported`; it is a preflight, not product acceptance. The
Python suites need no Cargo, frontend-unit needs no Tauri/Foundry,
native-macos is unavailable on Linux. native-runtime has only a runtime phase:
Python, Git and local Unix IPC for daemon scenarios, without Cargo/Foundry.
Doctor does not check `AIN_SOLC`. Doctor does not check the mac-apfs tools
outside Darwin and returns `unsupported`. `build-desktop.mjs` seals into
`output/desktop-build/<run>/prepared-artifacts.json` the sources/ref, locks,
the build configuration (including recursive Cargo `include`) and the
artifacts. It is a record for review: nothing reuses it later. The offline
toolkit and its safe layout are described in
`tests/build/README-linux-toolkit.md`: npm, GTK 3, both lock files and the
full checksum-bound frontend payload are mandatory. Using it requires no
Docker.

### Access to system utilities from a sandbox

- For diagnostics use the regular paths:
  `/usr/sbin/diskutil info -plist /Volumes/ChatBuild` and
  `/usr/bin/hdiutil info -plist`. The error `command not found` is about
  `PATH`; the error `Unable to run because unable to use the DiskManagement
  framework` means the utility ran but got no access to the system framework.
  These are different causes; changing the path or reinstalling the utility
  does not fix the second one.
- In a sandbox a DiskManagement/DiskArbitration error is possible with an
  already attached healthy volume. `/sbin/mount` and `hdiutil info -plist`
  help confirm the attachment and the image path, but they do not replace the
  wrapper's UUID and symlink checks. Do not declare the disk missing or broken
  based on that error alone.
- If the current task and the session policy allow widening access, on that
  error rerun `python3 scripts/build-storage.py check` from
  `/Users/glebk/Code/chat` through `exec_command` with
  `sandbox_permissions: "require_escalated"` and a justification of
  DiskManagement access for verifying the build storage. This is the regular
  way to request execution outside the sandbox, not running via `sudo`; the
  permission mechanism decides. If widening is forbidden or the request is
  declined, report the exact reason for the block and do not work around it.
- After a successful check, run the whole command you need through
  `python3 scripts/build-storage.py run COMMAND...` in the same permitted
  mode. A separate successful `check` outside the sandbox does not grant
  access to a later `run` inside the sandbox: every wrapper run talks to
  `diskutil` again.
- On an access error do not re-attach an already attached image, do not run
  `setup`/`install` to fix it and do not disable the APFS/UUID/image/symlink
  checks. Do not move the build to the internal disk and do not run it
  outside the wrapper.

Before unplugging WD4000, finish builds, tests and apps launched from the
image, run `python3 scripts/build-storage.py unmount`, then eject WD4000 the
regular way. The unmount must not be forced. When the disk is missing or the
APFS/UUID/image/symlinks do not match, stop and restore the attachment; do not
bypass the check. Sources and Git are available without WD4000. After a sudden
disconnect the build should be restarted; if damaged, the build data is what
gets recreated, keeping the sources.

## Testnet deployment

Until the mainnet launch, deploy the testnet yourself without asking the
owner: merge the finished `implementation/v1` into the development `main`,
then publish its reviewed tracked files as a new commit to
`glebkudr/kaikichat` on `main`, without importing the development Git history.
Coolify's existing `chat-production` application builds from
`git@github.com:glebkudr/kaikichat.git`, branch `main`. Push the production
commit and start its deployment with the regular Coolify means (API over
SSH), then verify the services (`/v1/policy` of identity and the directory,
`kaikichat.com`, nodes healthy). Keep the existing application, volumes and
runtime secrets. CLI installers and updates continue to use the signed
network preset and archives at `https://kaikichat.com/downloads`; GitHub
Releases are not the artifact source. The deployment restarts all application
containers, including the nodes. Spending on the blockchain (bonds,
purchases), firewall changes and other projects on the server still require
agreement. After the mainnet launch, revisit this rule.

## Placement rules

- Rust target, npm node_modules, frontend dist, Tauri binaries/gen/permissions,
  Foundry out/cache/broadcast and the shared cache/output use symlinks into
  the image.
- Do not replace these symlinks with real directories. Do not run a plain
  `npm ci` in the sources: it may replace the node_modules symlink; use
  `install` above.
- Do not create heavy targets or verification checkouts in Library/Caches or
  a permanent internal `/tmp` copy. Place temporary worktrees in
  `.local/verification-workspaces`, which points into the image. Run `mount`
  first. Run commands for such a worktree from the main checkout with an
  explicit `--worktree` before `run`, for example
  `python3 scripts/build-storage.py --worktree .local/verification-workspaces/NAME run cargo test --workspace`.
  After the regular check of the volume, UUID, image and symlinks, the wrapper
  requires the path to be the root of a Git checkout strictly inside
  `.local/verification-workspaces`, with its `target` not leading outside the
  worktree. The command then runs from that worktree with
  `CARGO_TARGET_DIR=<worktree>/target`, including with `--output`; the main
  repository's build is not touched. The relative path is resolved from the
  current directory. Without `--worktree`, `run` always executes in the main
  checkout, so running from a cwd inside `.local/verification-workspaces`
  without this flag is rejected. `--worktree` is supported only for `run` in
  `mac-apfs`.
- Keep unique fixtures and verified evidence in the sources and commit them.
  Do not add builds, run logs, images, secrets or local configs to Git.
- Do not delete `.local/build-storage.json` to bypass the protection. When
  restoring the environment, restore the config first, then use `setup` and
  `install`.

## Fast diagnostic test loop (V1-C05)

Offline message storage and delivery is a swarm of recipient mailboxes
([Docs/V1_STORAGE_REDESIGN_2026_09_24.md](Docs/V1_STORAGE_REDESIGN_2026_09_24.md),
plan and phase status in
[Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md](Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md)).
The former custody/history path, A04/H11 and their rigs are removed.

For bug-hunting and reproduction iterations use the accelerated loops instead
of waiting for the ceilings of native runs:

1. First the single-process reproducer with managed time:
   `cargo test --locked -p agentic-node --lib reproducer_tests::` (swarm
   scenarios: `reproducer_tests::mailbox_swarm::`), the `runtime::clock` and
   `mailbox_holder` tests take seconds. Reproduce a new failure class by
   extending the existing rig (`reproducer_tests.rs`,
   `reproducer_mailbox_swarm_tests.rs`), not by a separate rig.
2. The load acceptance spike on the same rig:
   `cargo test --locked -p agentic-node --lib -- --ignored acceptance_spike --nocapture`;
   `AIN_SPIKE_MESSAGES=<n>` reduces the message count. Method and results:
   `evidence/reviews/mailbox-swarm-spike-2026-09-26/`. Keep the rig's virtual
   time separate from real time and from native. The rig's wall clock grows
   only in whole seconds per step: measure durations by
   `rig.clock.instant()`.
3. The native swarm rig appears after phases 1b and 2; until then a rig
   scenario is not native acceptance. Do not scale TTL, quorum, periods or
   message counts for speed.
4. Reading hangs: `node_info.mailboxSwarm` (`sent`/`served`/`failureKinds`,
   `lastFailure`, `notary.waiting`, `grants.checking`, `replication.pulls`)
   and `node_info.mailboxHolder`.
5. Do not fake the system time (faketime/CLOCK_REALTIME) and do not use
   `tokio::time::pause` as a substitute seam. In new scheduler code introduce
   no direct `Instant::now()`/`.elapsed()`: the clock is
   `crate::runtime::clock` (`instant()` for the scheduler loop, `wall()` for
   the protocol), the RNG is `crate::runtime::random::fill`.

For new backend functionality write the tests first; after changing them and
before production code, start a separate context-free backend-test-critic and
wait for its decision. After the implementation, run the backend and frontend
checks. Run visual checks in the background, save and review the screenshots.
Local Docker means OrbStack; do not change neighboring projects' settings.

Automated native E2E must not trigger system keychain password prompts. Use
the debug build with `e2e` and a separate `E2eFileStore` inside a temporary
profile; the keys are deleted with it after the processes stop. Do not create
login Keychain entries for such tests and do not change its global settings or
ACLs. A real `system_keychain_roundtrip` is only a separate, explicitly
allowed interactive run. In the regular build the storage stays the system
Keychain.
