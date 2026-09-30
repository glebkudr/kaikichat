# Daemon service integration: unfinished debug checkpoint

## Superseding result after migration

The migration task verified the preserved code at 0d5596b5 on the new storage layout:
the actual daemon-service E2E passed TCP/Noise and QUIC, with 13 effects on each of
four selected profiles, 126 independent quorum signature checks and no cleanup errors.
The connection-handling fix below is now compiled and exercised by those successful
runs. Source hash: 67a0d11c0eb67865e7acf616d90e7615a26304a8c460985fe848c9dbc7dd5c29.
Fresh Clippy, Rust, Solidity, frontend, Linux/native gates and release validation are
recorded in Docs/maintenance/storage-migration-result.md, including the initial
intermittent operator-network timeout and phased reruns.

Development resumed in /Users/glebk/Code/chat. Use the build-storage wrapper required
by AGENTS.md. The historical failed runs and migration hold below are retained for
provenance; they no longer describe the current daemon-service validation state.
A later clean aggregate includes actual cross-log replay and two quiet-tip fixes:
458 Rust tests, 40 frontend tests and TCP/QUIC service acceptance passed. Evidence:
P01-daemon-scope-replay/quiet-notarization-recovery.json. The preserved debug history
below remains historical; production spend/group admission and V1 remain incomplete.

## Historical pre-migration checkpoint

Fresh migration checkpoint, 2026-09-06: source edits and builds are now held again
at the migration executor's request after the user's explicit move authorization.
Backup 99697b69 is historical and predates the latest connection-handling fix.
Storage paths have not been changed by this task. Main has independent maintenance
commit 0beffaf5; preserve it when integrating this implementation branch.

The full V1 goal remains incomplete. Current uncommitted source is based on
ed9bfd712b37333ce149f92f93201873f0784b71 in the APFS worktree. Do not treat this
snapshot as a completed feature or replace the previous release with it.

Independent no-fork test critic returned FINAL ACCEPT before production edits;
see P01-daemon-service-tests.md for the exact accepted test hashes.

Implemented so far: local trusted registration (maximum four scopes), supervised
Commonware service launch, Core authority fences, finite frame protocol and queues,
automatic selected-key proof discovery/renewal, owner diagnostics, durable effect
before acknowledgement and network/role/head revocation. Queued outgoing writes
are fenced by a runtime permit and route/service expiry inside the async codec.

Validation so far:

- Six oracle tests pass, including independent real on-disk Marshal cursor decoding.
- Frontend: 40 tests, TypeScript and Vite build pass.
- Workspace/all-target Clippy passed before the most recent CBOR decoding fix.
- First actual EVM/TCP run reached four entries on all selected profiles, then
  foreign-peer probes failed at CBOR decoding instead of the intended authorization
  gate. The decoder now accepts bounded CBOR byte strings and arrays.
- Second actual run failed earlier: three profiles persisted four entries, one
  persisted only three before the 35-second convergence deadline. Cleanup passed.
  The CBOR fix's foreign-peer scenario has therefore not yet been revalidated.
- QUIC, later lifecycle scenarios, current full Rust regression, all EVM regression,
  native acceptance and a new packaged release are still pending.

The second runner (exec session 65877) is terminal with exit 1. The latest source
fix below was applied and formatted, but the interrupted third invocation did not
start its E2E run: no third-run log or live process exists. A fresh process check
found no fixture, validator, Cargo, Anvil or finalizer-service runner from this task.
After this checkpoint write there are no source/build writers from this task.
Nothing has been moved or deleted. This task has made no commit during this
increment. HEAD remains ed9bfd712b37333ce149f92f93201873f0784b71.

Latest applied fix (UNCOMPILED AND UNTESTED): finalizer_frames.rs now accepts an
actual allowed connection of the same authenticated peer when the proof connection
is also still allowed, the verified transport key matches, and the route interval,
permit and Core authority remain live. Previously ingress required the exact proof
connection, although request-response distributes requests across connections.
Global accepted counters count every accepted frame; per-connection diagnostics
only increment for the reported proof connection. Relay-only outgoing requests
also reject peers with direct connections, since the library can select any of
their connections. The hypothesis that this caused run 2's lag remains unproven.
After migration, compile and rerun the accepted live E2E before claiming a fix.

Reproduce from the existing APFS worktree:

```sh
AIN_FOUNDRY_BIN=/Users/glebk/Library/Caches/agentic-internet/toolchains/foundry-v1.8.1-darwin-arm64 \
AIN_SOLC=/Users/glebk/Library/Caches/agentic-internet/toolchains/solc-v0.8.36-macos \
python3 tests/evm/finalizer_service.py
```

The runner builds the fixture itself with the lockfile and rejects source changes
during a run. Adapt paths only after the migration executor confirms the new layout.
Backend tests: scripts/check.sh with the same Foundry/Solc environment and
AIN_NODE pointing to toolchains/node-v26.8.1-darwin-arm64/bin/node.

Preserve Git, all crates/tests/spec/Docs, all retained evidence, and especially
tests/evm/fixtures/finalizer-service-cursor.json plus its generator and oracle tests.
The corpus contains real public four-validator output, binary hash and raw cursor
copies; it is test input, not a rebuildable cache. Toolchains, target, node_modules
and ordinary build output can be recreated. Existing scripts currently assume
target under the worktree, so a future build relocation needs explicit adaptation.

async-trait0.1.92 was already present in Cargo.lock and reused as a direct Node
dependency after checking https://docs.rs/async-trait/latest/async_trait/.
No external package version changed in this increment.
