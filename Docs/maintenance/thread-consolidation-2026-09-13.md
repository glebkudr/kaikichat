# Handing V1 implementation to a single task — 2026-09-13

At the user's direct request, further implementation is handled only by task
`01a08bb1-c0a4-7583-95b5-06d25ed25e26` — "Continue implementation after the
review". Task `01a06e6e-fded-7f61-9aa2-1946cf2729fb` — "Plan the decentralized
V1" — is archived. Its last turn was interrupted, the app status is `notLoaded`;
the task itself confirmed that its Goal is already `paused`, there are no
running subagents, and its test processes are stopped. The archived history
remains available as a context source. Do not resume its implementation or
automatic continuations.

## Cause of the duplication

The second task was created on September 10 with a request to continue the
first after the architectural review. Both kept separate Goals to finish V1:
the first's history has automatic continuations on September 12–13, the
second's including September 13 at 15:38 UTC. The reference to the previous
task passed context but did not cancel its separate goal. No matching
automations were found in local `automation.toml`.

## Unified source state

- Working cwd: `/Users/glebk/Code/chat`; branch `implementation/v1`.
- HEAD at handover: `627bffe10eabe7ae834a306dd766950a53946b09`.
- All current changes, including uncommitted and new files, are preserved in
  place. At first inspection Git showed 333 changed/new entries. The
  coordinator did not revert, move, or commit sources.
- `/Volumes/WD4000/Code2/chat` — a clean archive checkout. Common ancestor
  with the working branch: `0beffaf5ad018069b53f0cf80571f3fee2788849`. The
  only separate archive commit `61d2e03` adds only archive notes to AGENTS.md
  and README.md. There is no unique implementation to Git merge; these archive
  instructions must not be carried over on top of the working AGENTS.md.

## Context that must not be lost when continuing

Rely on the current files and evidence, not on the old message from the second
task where the recovery CLI/MCP/UI were still considered non-functional. After
that, the first task already did the integration, native checks, page caching,
paid history paths, bulk/prefix reads, and holder selection.

Primary sources of the current state:

- [IMPLEMENTATION_STATUS.md](../../IMPLEMENTATION_STATUS.md).
- [Architectural follow-up](../V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md),
  [saved independent review](../reviews/2026-09-10-independent/review.md).
- [History recovery in the product](../../evidence/reviews/AR2-wallet-flow/HISTORY_RECOVERY_PRODUCT.md).
- [Latest accepted holder-selection checks](../../evidence/reviews/AR2-wallet-flow/history-holder-exploration-checks.json):
  109 backend / 51 frontend, Clippy and formatting passed for that stage.
- [Native20 R6](../../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r6.json)
  — a historical successful run; the subsequent
  [Native20 R7](../../evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r7.json)
  failed the first SQL gate within the unchanged 120 seconds.
- [Full130 R12](../../evidence/reviews/AR2-wallet-flow/history-range-native-r12.json)
  also failed the first SQL gate after publishing 130 originals, verifying 390
  signatures, and really deleting copies. Partial/full/cold recovery is not
  accepted. Full130 R13 has not been run yet. V1 remains unfinished.

## The latest unfinished stage: coalescing originals into a leaf

After Native20 R7, reducing the number of sender history pages was started.
The Core API `prepare_custody_history_batch` signs one immutable leaf for 1–12
ready originals, preserving the original leases and the existing graph format.

- [Contract](../../spec/custody-history-batches-v1.md).
- Code: `crates/core/src/custody_history_batch.rs`,
  `crates/core/src/custody_history_pages.rs`.
- Tests: `crates/core/tests/support/custody_history_coalesced.rs`,
  `custody_history_pages.rs`, `custody_history_page_import.rs`.
- [Independent critic: FINAL ACCEPT, four tests](../../evidence/reviews/AR2-wallet-flow/history-coalesced-core-test-review.json).
  Verify the hashes before relying on the acceptance. The first GREEN was
  unsuccessful; `output/ar2-wallet-flow/history-coalesced-core-green-r2.log`
  contains the result `4 passed; 0 failed`. The frontend log for this stage:
  51 passed.
- The extended Core run was **interrupted** at handover: 95 of 96 tests
  reported `ok`, there is no final `test result`. This is not a successful
  full run. Clippy and fmt for this stage have not been run yet.
- Remaining-checks script:
  `output/ar2-wallet-flow/check-history-coalesced-core.py`;
  interrupted log: `output/ar2-wallet-flow/history-coalesced-core-history.log`.
  Preserve the evidence of the interrupted run before overwriting logs.
- Node integration of selecting several paid-ready originals is still ahead.
  Each original must have all ten verified index receipts and all ten
  data-location ACKs per index; use the existing paid queue and the
  child-before-parent ACK order. Core tests do not prove network throughput.

The nearest sequence: reconcile the current tests with the review; finish the
targeted Core checks and record an honest result; then tests-first/a separate
backend-test-critic for the Node integration; after implementation — targeted
backend and frontend checks, a current build, and the unchanged Native20/Full130
gates. Do not weaken deadlines, original leases, quotas, SQL rollback, or exact
oracles to make scenarios pass. The rest of the mandatory V1 scope, including
offline Welcome, old epochs, recovery/rejoin, groups, and standalone R10
repair, stays in the overall plan.

## Mandatory constraints

Read the working AGENTS.md. Run all builds and checks via
`python3 scripts/build-storage.py run ...`; do not bypass the external volume
checks. New backend — tests, a separate backend-test-critic without inherited
context, waiting for ACCEPT, then production code. Run visual checks in the
background and review saved screenshots. Automated native E2E uses debug/e2e
and a temporary E2eFileStore; do not touch the system Keychain.

Only the main task owns the implementation and further decisions. This file
records the handover; it does not replace the plan or acceptance evidence.

## Preserving parallel work

The user separately clarified: preserve the sound results of both tasks,
including independent changes to different parts of the project. The main task
confirmed this requirement and a short write pause for the snapshot;
production, tests, and documentation were not changed by it before the
snapshot. After the snapshot was verified, the write pause was lifted. The main
task's old adapter-review agent was also stopped; stale staged patches were not
applied.

A full snapshot is preserved in
`/Users/glebk/Code/chat/output/thread-consolidation-20260913T155044Z/`:

- `source-worktree.tar.gz`: 4 334 tracked and non-ignored untracked files,
  including sources, tests, and evidence. The archive was read back; every file
  was verified by SHA256 from `manifest.json`. Sources, the file list, and Git
  status did not change during the snapshot. Size: 114 408 160 bytes.
- SHA256 of the source archive:
  `94216c8c5c88dd8ac646f81cfc0181a82322294c3622cf56e156f248ca30a2e7`.
- `staged.patch`, `unstaged.patch`, `git-status.txt` preserve the Git state.
- `draft-artifacts.tar.gz` and `draft-manifest.json`: additionally preserved
  256 draft `.patch`, `.rs`, `.py`, `.md`, and other text sources from
  `output/ar2-wallet-flow` directly, without test profiles and keys.
- `verification.json` records the verification; do not clean this directory
  until the results are confirmed saved in subsequent Git history.

The [contributions map](thread-contributions-2026-09-13.json) matches explicit
`apply_patch` calls from both tasks' histories starting from the second task's
creation. In this sample, 74 paths occur only in the first, 376 only in the
second, 47 in both. This is a map of touched areas, not exhaustive authorship:
changes via shell scripts and subagents were not attributed separately; a patch
call by itself does not prove a successful write. All 47 shared files are
present in the snapshot. The only missing path in the whole sample is
`crates/desktop-host/examples/native_fixture_keychain.rs`; the last operation
in the second task's history is an explicit deletion on September 11. The old
system-Keychain E2E tool should not be restored.

The separate contributions coexist: the architectural review,
wallet/authority/finalizer, paid history, and Core observation from the second
task; the recovery UI, paid paths, prefetch/cache, and fresh coalesced batches
from the first. In the shared modules — Core history, Node custody/index/sync,
and paid storage — the continuation must preserve both sets of contracts and
tests. If an incompatibility is found, merge behavior per the current contracts
and evidence rather than picking one task's version wholesale. This
organizational audit does not claim full proof that no earlier semantic
rewrites happened; both histories and recoverable artifacts are preserved for
targeted verification.

A note on timestamps: in all 47 shared files, the second task's last explicit
patches precede the first task's first patches in the studied period. These
patch intervals do not overlap. After the second task was created, its journal
execs run from September 10 to September 12 19:18 UTC; the first resumes exec
on September 12 at 21:29 UTC and works on September 13. So the available
history shows mostly sequential development of the shared implementation, not
two executors writing the same module simultaneously. The wallet, paid history,
and Core observation foundations from the second task were then used by the
first for UI/CLI/MCP, recovery, and reducing network requests. An identical
file in the map is not by itself a sign of duplicated work. These conclusions
apply to the mentioned exec/patch journals; the separate timing of background
processes and all subagent actions was not reconstructed.

## Main task confirmation

The main task read the handover, accepted sole ownership, and confirmed the
current HEAD and the already working CLI/MCP/UI. It saved the interrupted log,
reconciled the coalesced critic hashes, and continued the targeted checks. Its
`get_goal` returned the objective "act until you finish V1", status `paused`.
The current explicitly launched turn is doing the work; the Goal's automatic
continuation must be resumed separately via the user interface. The available
tools cannot switch someone else's Goal, and driving the Codex UI itself via
Computer Use is forbidden.
