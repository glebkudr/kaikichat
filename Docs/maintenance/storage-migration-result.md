# Working environment migration — 2026-09-06

## Placement

- Main repository: `/Users/glebk/Code/chat`, a standalone internal `.git`.
- Published source: <https://github.com/glebkudr/kaikichat>.
- Build disk image: `/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle`.
- Mount point: `/Volumes/ChatBuild`; APFS, UUID `9256F47B-3330-48F2-A863-85F6BEF141E4`.
- The disk image maximum is 250 GiB; physical space is consumed as it fills.
- Local Node 26.8.1, Foundry 1.8.1, and solc 0.8.36 are preserved in
  `/Users/glebk/Code/chat/.local/toolchains`; the former toolchains path became
  a link.
- Shared Rust/Cargo and OrbStack stayed in their standard locations.
- The Chat project path in Codex was switched by the user and verified via the
  project list.

The disk image file physically resides on WD4000. macOS refused to mount the
volume inside exFAT/FSKit; the standard mount point `/Volumes/ChatBuild` works.
This is the name of the mounted external storage, not a build directory on the
internal disk.

## Preserving the work

The "Plan the decentralized V1" task stopped its writers at a safe point. The
final WIP is preserved by commit
`0387947fa9e21996b3e48d233065529ddb579123` on `codex/storage-migration-backup`,
including the last edit of `finalizer_frames.rs`, the debug checkpoint, and the
unique `tests/evm/fixtures/finalizer-service-cursor.json`. This is an
unfinished implementation.

All 1115 files of the final snapshot were reconciled against the retained old
copy; source blobs were reconciled against the backup commit. The new Git uses
no alternates or external `.git`. Git fsck, a full bundle, and remote
restoration via an independent clone were verified. The main and
implementation/v1 branches and the previous history are preserved.

The directory `/Volumes/WD4000/Code2/chat/.local/storage-migration` contains
manifests, patches, full Git bundles, an archive of 1053 recovery files without
builds, and verification logs. The recovery archive contents were verified by
SHA-256 of every file. Unique sources, fixtures, and diagnostic materials were
not deleted.

The old working copy itself, without builds, is in
`.local/pre-migration-worktree-20260906`; recovery is in
`.local/pre-migration-recovery-20260906` of the new repository. The old
worktree was moved with the standard `git worktree move`; its link to the
original external Git is preserved. After the move, the hashes of all 1115
files matched again. The old `Library/Caches/agentic-internet/worktree` path
now points to the active repository, and toolchains/recovery to the preserved
directories inside the project. The original cache directory no longer holds
heavy builds. The external checkout is marked as an archive in README and
AGENTS.md by a separate main commit `61d2e03`.

## Builds and running

Old target, node_modules, Foundry out/cache, and app bundles were not copied.
npm dependencies were restored via `npm ci` from the original lock file inside
the disk image. Dependency versions were not updated during the migration.

Twelve build directories are linked into the disk image with symbolic links:
Rust target, npm node_modules, frontend dist, Tauri binaries/gen/permissions,
Foundry out/cache/broadcast, cache, output, and temporary verification
workspaces. Ignore rules account for the links themselves. Cargo.lock,
package-lock.json, sources, fixtures, and reviewed evidence stay in Git. The
Docker context excludes builds and local storage.

```sh
cd /Users/glebk/Code/chat
python3 scripts/build-storage.py run scripts/check.sh
python3 scripts/build-storage.py run node scripts/check-native.mjs
python3 scripts/build-storage.py run node scripts/check-network.mjs
```

The wrapper mounts the existing disk image in the background and verifies APFS,
the UUID, disk image file identity, and all links. To restore npm use
`python3 scripts/build-storage.py install`; a plain `npm ci` in the sources can
replace the node_modules link, so it should not be run there.

Before ejecting the disk, stop build and app processes launched from the disk
image, then run `python3 scripts/build-storage.py unmount` and eject WD4000 the
standard way. Forced unmounting is not used. Sources and Git are available
without the disk image. A sudden disconnect can interrupt a build and damage
its rebuildable files; after reconnecting, the process must be started again.

## Fresh checks

- Python: 7 model and 9 fixture/oracle tests.
- Rust: fmt, Clippy across workspace/all-targets, and 456 tests.
- Solidity: 29 Foundry tests.
- Frontend: 40 Vitest tests, TypeScript, and the Vite production build.
- Linux via OrbStack: 7 network E2E scenarios, report `passed: true`, cleanup
  without errors.
- Native: debug bundle with hidden WKWebView E2E; all 5 scenario groups passed.
  Fresh screens of the chat and restored network settings were reviewed next to
  the previous ones; no display regressions found.
- The release `.app` was rebuilt; `codesign --verify --deep --strict` passed.
  No webdriver in the standard dependency graph. The signature is local ad-hoc.

The first combined run stopped at EVM `operator_network.py`, in the offline
phase, with a peer-wait timeout. A separate run of the last verified `ed9bfd7`
in the new disk image passed (167 seconds); a subsequent sequential run of the
same scenario on the WIP also passed — 16 role checks, cleanup without errors.
The original timeout is preserved in the log; the exact cause was not
established. The `ed9bfd7` verification worktree was deleted after saving fresh
results.

The sequential `scripts/check-evm.sh` finished with exit 0 in 1143.7 seconds:
all the Solidity/Anvil/daemon/peer checks listed in it passed. The frontend was
checked separately after the first interrupted combined run. The new
daemon-service E2E result is recorded separately; this scenario is not yet
included in the combined script. This document does not declare unfinished V1
as fully verified.

`tests/evm/finalizer_service.py` also passed: exit 0 in 313.1 seconds,
TCP/Noise and QUIC on chain ID 31337/31338, cleanup without errors. This
verifies the selected daemon voting service with a local signed fixture
application; `productionApplicationAdmission` remains false. The last saved
debug edit is not lost and was verified successfully, but this does not close
full V1 acceptance.

## Final storage check and cleanup

15 pre-verified build directories were deleted. Before deletion, sources, the
absence of tracked files, and process-open files were re-verified. Total
directory volume per `du` is 143 509 737 472 bytes (133.65 GiB). Actual free
space on the internal APFS grew from 17 260 605 440 to 102 539 304 960 bytes,
i.e. by 79.42 GiB; directory volume and free-space gain are different
measurements. Logs and the exact list are in `cleanup-result.json`. The
`cleanup-candidates.json` list is marked as a completed historical document:
the old paths have already become links; deletion must not be repeated from it.

After stopping the test processes, the disk image was unmounted the standard
way, without force. Sources, HEAD, and `git fsck --full` remained accessible;
the guard, backend entrypoint, and desktop entrypoint refused to start builds
without the volume. All 12 links survived; no internal spare directories
appeared. Failure with a missing disk image file and a wrong UUID was verified
separately.

The `run` command automatically remounted the same disk image. The UUID, disk
image identity, the control file, the physical placement of target on the
external volume, and the ignore status of all 12 links were re-verified.
Details: `storage-detach-remount.json` and `storage-guard-checks.json` in the
backup directory. The disk image was left mounted to continue work.

Storage rules and commands are fixed in the root `AGENTS.md`; README uses the
same launcher. The development continuation branch is `implementation/v1`; the
`codex/storage-migration-backup` and `codex/storage-migration` branches
preserve the WIP snapshot and the migration changes. The development task gets
the new cwd and check results when resumed after the final commit/push.
