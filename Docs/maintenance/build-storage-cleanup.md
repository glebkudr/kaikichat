**ChatBuild build disk image cleanup**

The goal is to free space from stale build files while preserving current
programs, the finished app, and project data. The main source of reclaimable
space is old Rust build variants and their caches in
`/Volumes/ChatBuild/rust-target/debug`.

Working repository: `/Users/glebk/Code/chat`. Disk image:
`/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle`, mounted volume:
`/Volumes/ChatBuild`. The external checkout `/Volumes/WD4000/Code2/chat`
remains an archive.

By default the cutoff is **24 hours from the start of the audit**. Age alone
does not prove a file is unneeded: an old file may be required by a saved
build. Keep fresh files in an ordinary cleanup. Deleting fresh but already
superseded variants belongs to a separately chosen extended cleanup.

| Priority | Where to look | What to delete | What to keep |
| --- | --- | --- | --- |
| 1 | `/Volumes/ChatBuild/rust-target/debug/incremental` | Old cache directories whole. Determine age by the newest modification **anywhere inside the tree**, including nested directories. | At least the two latest sets for each crate and cache type: with object-file generation and metadata-only. Keep fresh and in-use directories. |
| 2 | `/Volumes/ChatBuild/rust-target/debug/deps`, `/Volumes/ChatBuild/rust-target/debug/examples` | Old `*.rcgu.o` files for which the absence of references from all kept libraries and programs has been verified. | Object files referenced by kept builds, including debug-info references. |
| 3 | The same `deps` and `examples`, plus `/Volumes/ChatBuild/rust-target/debug/.fingerprint` | Full stale build variants of our own workspace packages: the corresponding `.rlib`, `.rmeta`, executables, `.d`, other related outputs, and fingerprint directories. Then find object files that became unreferenced again. | At least the two latest successfully completed variants of each target and build mode, all their needed dependencies, current programs, and programs from the last 24 hours. Keep third-party dependency libraries by default. |

Compare variants accounting for package, target, profile, features, compiler,
architecture, and build flags. The two latest variants are the **minimum to
keep**, not a limit: dependencies of other kept programs may require keeping
more. Use Cargo fingerprints and the corresponding output files to determine
successful completion; a directory's presence after a failed compile does not
confirm it.

First determine the builds to keep, then walk their dependencies. For `.rlib`,
check archive contents with `xcrun ar t`; for Mach-O, check `OSO` references
with `xcrun nm -ap`. On a read error, unknown format, or ambiguous dependency
link, keep the affected files until clarified. Do not consider a library
unneeded just because an old Cargo fingerprint no longer resolves. And do not
consider a new library variant an automatically compatible replacement for an
old dependency.

Current programs without a hash in the name, such as
`/Volumes/ChatBuild/rust-target/debug/agentic-node` and
`/Volumes/ChatBuild/rust-target/debug/libagentic_node.rlib`, are entry points
to keep. Likewise keep programs in the bundle, current examples without a hash,
and actually running executables.

Execution order:

1. Verify the volume with the standard command below. Record free space, Git
   status, and checksums of the current app. Build a fresh candidate list with
   absolute paths, inodes, sizes, and modification times. Store each run's
   report in a new folder under `/Users/glebk/Code/chat/.local/`.
2. Hold an exclusive Cargo lock via `fcntl.flock` on
   `/Volumes/ChatBuild/rust-target/debug/.cargo-lock` during the final audit,
   deletion, and verification. If the lock is taken, wait for a free window; do
   not stop build processes. This lock does not protect objects in other target
   directories.
3. By default, wait for tests to finish. Cleanup in parallel with already
   compiled tests is allowed only if current and pending programs are preserved
   together with their dependencies. Check open files with `lsof`: exclude
   candidates in use. The process check complements the lock but does not
   replace it.
4. Under the lock, re-verify the candidates and kept outputs. Delete only
   specific entries from the fresh list. For directories, verify the whole
   tree, the absence of symlinks, and ownership by the volume. If inodes,
   sizes, times, or tree contents changed, recompute the affected part of the
   plan. Keep the lock file and the parent build directories.
5. Record the actually deleted paths. After cleanup, compare app and
   kept-output checksums, repeat the volume and app-signature checks. Measure
   free space again with `df` or `statvfs` and report the actual difference.

Verification commands; run from the main repository:

```sh
cd /Users/glebk/Code/chat
python3 scripts/build-storage.py check
python3 scripts/build-storage.py run df -h /Volumes/ChatBuild
python3 scripts/build-storage.py run du -h -d 1 /Volumes/ChatBuild/rust-target/debug
python3 scripts/build-storage.py run codesign --verify --deep --strict '/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app'
```

Use `python3 scripts/build-storage.py run COMMAND...` for other
build-environment operations too. On a volume or link check failure, restore
the standard mount first.

Do not include in an ordinary cleanup: sources, `.git`,
`.local/build-storage.json`, archives and backups in
`.local/storage-migration`, working data and `output`, unique evidence,
toolchains, npm/node_modules, RISC Zero guest builds, `debug/build`, Tauri
binaries, and all of `rust-target/release`. In particular, keep
`/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
These areas need a separate review of their purpose.

Do not apply broad commands like `rm -rf target`, a full `cargo clean`, or
`find ... -mtime ... -delete` to `deps` and `.fingerprint`: they do not perform
the described verification of kept builds. Do not manually clean the disk image
file itself, its `bands`, or APFS service data.

Account for hard links: identical data can be present in both `deps` and
`incremental`. The sum of deleted file sizes or of separate `du` passes is not
the guaranteed reclaimed space. Judge results by the change in free space on
the volume. After deleting caches, subsequent builds may take longer and
recreate the needed files.

Track free space **inside ChatBuild** and the physical sparsebundle size on
WD4000 separately. Deleting files inside the volume does not guarantee an
immediate decrease of the external disk image. Compacting the disk image is a
separate operation with standard `hdiutil` tools, outside current builds and
after a standard unmount of the volume. An ordinary cleanup must not change the
disk image size or unmount it.

For reference: the extended cleanup of September 10, 2026 freed another
94.7 GiB by deleting 12 old `agentic_node` variants, 466 incremental
directories, and about 219 thousand files. At that time 3287 kept outputs and
8 app files were verified; the signature check passed. This is a historical
result, not a forecast for the next cleanup.

Materials from that operation are in
`/Users/glebk/Code/chat/.local/build-cleanup-variants-20260910/`. The scripts
`audit.py`, `apply.py`, and `run-locked.py` were one-shot: they use the old
report, the app checksums, and the deletion journal. **Do not run them again
directly.** For a new run, prepare a fresh audit, current checksums, a new
report folder, and verify the application of this instruction's rules.
