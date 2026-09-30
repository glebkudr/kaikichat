# Build environment migration plan

Date: 2026-09-06. The agreed layout: sources and Git on the internal APFS;
heavy, rebuildable data — in an APFS disk image on WD4000. Do not copy old
build results. Backup first, then migrate at a safe point.

## Current state

- Development task: `codex://threads/01a06e6e-fded-7f61-9aa2-1946cf2729fb`,
  "Plan the decentralized V1".
- The last verified commit at inventory time: `ed9bfd7`. V1 is not yet complete
  overall. The verified slice: scoped authority and the embedded finalizer
  service. Per IMPLEMENTATION_STATUS.md: 456 Rust, 29 Solidity, 7 model,
  3 fixture-reader, and 40 frontend checks; native checks passed; Linux results
  for this slice are historical, not new.
- The active unfinished slice: finalizer services in the real daemon/libp2p.
  Production code, tests, a contract, and a new cursor fixture are being
  changed. An uncommitted snapshot must not be treated as a tested release.
- The main Git is currently at `/Volumes/WD4000/Code2/chat/.git`. The internal
  working copy contains only a `.git` link to that directory. So simply copying
  the internal worktree is not enough for Git.
- Measured rebuildable data: `target` about 70 GiB; `recovery` about 61 GiB,
  but before deleting it, Git history, unique sources, fixtures, and diagnostic
  materials must be extracted from it.

## Target placement

| Data | Location |
| --- | --- |
| Main repository with its own `.git` | `/Users/glebk/Code/chat` |
| Published source | `https://github.com/glebkudr/kaikichat` |
| APFS disk image, physically on WD4000 | `/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle` |
| Disk image mount point | `/Volumes/ChatBuild` |
| Rust target, npm dependencies, frontend/Tauri/Foundry builds, temporary verification copies | Inside the mounted disk image |
| Local backup bundles and the migration inventory | `/Volumes/WD4000/Code2/chat/.local/storage-migration` |

The disk image can be created with a maximum size of 250 GiB; it fills up as
work progresses. When creating it, check that the current macOS supports the
needed format and that there is free space. Do not modify WD4000 partitions or
other projects. Mount via `hdiutil` in the background, without Finder and
without stealing focus. During execution, macOS refused to mount inside
exFAT/FSKit; the standard mount point `/Volumes/ChatBuild` was verified
successfully. The disk image physically stays in `.storage` on WD4000; this is
not moving builds back to the internal disk.

## 1. Backup before the switch

1. Check ignore rules: Rust target at any depth, node_modules, dist, Tauri
   binaries/gen, Foundry out/cache/broadcast, run output, Python environments,
   temporary files, local backups, and disk images. Do not ignore Cargo.lock,
   package-lock.json, sources, test fixtures, specifications, and verified
   materials in evidence/.
2. Remove previously tracked generated output from the index in a separate
   maintenance commit, preserving the physical files and their prior Git
   history. Do not clean the index of the active implementation worktree.
3. Create the private development archive. Push existing branches and tags with a
   normal push; no force-push and no history rewriting.
4. Save uncommitted work as a separate WIP commit on a backup branch of an
   independent repository. Copy changed and untracked sources/tests/fixtures;
   do not run `git add -A` in the active worktree. Take hashes before/after
   copying; if the file set or contents changed, repeat the snapshot or wait
   for writes to stop.
5. Mark the WIP commit as unfinished, with no claim of green tests. After
   development stops safely, repeat the snapshot: an early snapshot does not
   replace the final reconciliation before the switch.
6. Save `git bundle --all`, refs, worktree statuses, and a SHA-256 manifest of
   valuable files. Separately save the existing
   `recovery/history-855a736.bundle`; verify the unique history and refs in
   `recovery/history-855a736.git`, including original commits unreachable from
   ordinary branches, before deleting anything from recovery.
7. Verify branch SHAs on the remote and restoration via an independent clone.
   Verify the Git bundle and object integrity. Large build binaries are not
   needed for backing up sources.

## 2. A safe point in the development task

Do not wait for all of V1 to finish. The nearest stop between completed
operations of the current slice is needed: the developer has saved files,
finished or cleanly stopped its processes, and recorded the debug state.
Migration must not happen while files are changing, mutation checks are
running, or the daemon WAL is being written.

The development task must write to `.local/storage-migration/safe-point.json`
on the external disk: `ready: true`, the full HEAD, UTC time, the current
slice, check status, reproduction commands, the list of needed non-standard
files, and confirmation that its own writers are stopped. After that, do not
start new implementation until notified that the migration is complete or
cancelled.

For the current migration, this confirmation is already provided by a message
from the development task and the file
`evidence/reviews/P01-daemon-service-debug.md`: the repeated E2E finished with
exit1, writers are stopped, development is temporarily on hold. Six oracle and
40 frontend tests passed; the current TCP convergence fails. This is a full
checkpoint of unfinished work, not a green release. A new ready file is not
needed to duplicate this confirmation; before the switch itself, reconcile the
state with the saved manifest again anyway.

The signal requires re-verification: a single idle status or an old ready file
is not enough. Reconcile HEAD and the manifest, make sure there are no new
changes and no processes needing the source paths. Record known failing checks
as the initial state; the migration must not mask implementation errors.

## 3. Migrating sources and the build storage

1. Create a full independent repository at `/Users/glebk/Code/chat` from a
   verified backup source, without a shallow clone, alternates, or references
   to the external `.git`. Restore the needed branches/tags and the WIP state.
   If the directory already exists, investigate it first; do not overwrite.
2. Reconcile Git objects, refs, and SHA-256 of all valuable files, including
   local untracked sources. Preserve Git author/config, needed local settings,
   tool parameters, and launch commands; do not copy old worktree links as a
   standalone `.git`.
3. Create an empty APFS disk image in `.storage`, mount it, and verify a real
   APFS volume. Inside, create separate directories rust-target, npm, frontend,
   tauri-binaries, foundry, output, and verification-workspaces.
4. Inside the APFS source directory, use symbolic links to these directories:
   in particular `target` must remain reachable at `root/target`, since
   build-desktop.mjs, check-native.mjs, and tests use that path. exFAT itself
   does not store symbolic links.
5. For npm, install dependencies into a separate directory inside the disk
   image from the exact package.json/package-lock.json and then link
   node_modules. Verify `npm ci` behavior: it must not replace the link with a
   local directory and quietly bring dependencies back to the internal disk.
6. Do not copy old target, node_modules, dist, out/cache, .app bundles, test
   runtime profiles, WAL, and mutation-target. The exception is a provably
   irreplaceable fixture, which is first saved separately as data. User working
   profiles and the Keychain are not build cache.
7. Do not move shared Rust/Node/Foundry installs or shared OrbStack data with
   the project. Record the needed versions and paths; preserve or separately
   restore project tools from the old toolchains. Install dependencies from
   lock files without side updates.

## 4. Protection against a missing external disk

Add a shared environment launcher that verifies that exactly the expected APFS
disk image/volume is mounted, plus the target links, before builds and tests.
A missing disk must produce a clear error, with no fallback to the internal
target and no writing into an empty mount point on exFAT. Reuse one check at
the existing entry points; do not duplicate it across scripts.

Editing sources and Git on the internal disk works without WD4000; builds
require the mounted disk image. A sudden disconnect can damage rebuildable data
and running processes. Reconnecting does not mean a crashed build resumes
automatically: check the disk image and restart the process. Sources and the
internal Git are not in the disk image.

## 5. Verifying the new environment and resuming development

1. Reinstall dependencies from lock files and do a fresh build.
2. Run backend and frontend checks via the existing scripts/check.sh: Python,
   Rust fmt/Clippy/tests, Solidity/Anvil/EVM, Vitest, TypeScript, and Vite. If
   the WIP already had known failures, compare them with the saved state and
   separately verify the last green commit in the same new environment.
3. Run check-native.mjs and check-network.mjs; check native UI with hidden
   windows/WKWebView and saved screenshots, Linux — via OrbStack. Review fresh
   screenshots next to the saved references. Generate new reports: do not pass
   off previous results as a post-migration check.
4. Verify the negative missing-volume scenario without unplugging the cable and
   with no active writes. Make sure sources/Git are accessible and the build
   launcher stops before creating internal artifacts.
5. Update README, IMPLEMENTATION_STATUS, and the working path instructions.
   Continue the existing Codex task with the new internal repository as the
   explicit cwd; do not let it keep editing the old external copy out of
   inertia. Update the saved project path in Codex by a supported means; do not
   create a new task and do not edit the app's internal database.

## 6. Cleanup and rollback on failure

Only after a verified remote, full local reconciliation, writers finished, and
a successful check of the new environment, delete the rebuildable old target
and verification builds. The estimated reclaim is about 130 GiB; measure the
exact figure immediately before cleanup.

Keep old Git history, original bundles, and unique sources until full
restoration is confirmed. Do not delete all of recovery or all of
agentic-internet with one command. Delete by a list of verified directories,
after checking open processes. On a problem, keep the old environment
accessible, restore the links, and continue development from the saved state;
do not run reset --hard or force-push.

The migration is complete when Git is independent of WD4000, sources are
preserved locally and remotely, new builds physically live on WD4000, old build
files were not copied, checks have run, and development continues from the
required slice.
