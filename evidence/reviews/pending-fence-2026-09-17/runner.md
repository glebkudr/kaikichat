# Runner packet for the pending-fence production patch

Run from the clean production revision containing this packet, as identified in the architect's commit link. The RED commit `74049f5` is historical evidence, not the GREEN target. Each `--output` directory must be new; use `r2`, `r3`, etc. for retries. Run target checks first and retain failures before starting the expensive aggregate/native cases.

Linux commands below explicitly select `portable-linux`. On macOS remove `--profile portable-linux`; the existing `mac-apfs` wrapper/profile remains in use. Do not use Rosetta x86_64. No setup/install/doctor repeat is needed merely because a source patch arrived on an already healthy runner.

## 1. First response: six-test GREEN gate

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-green-20260917-r1 run cargo +1.91.0 test -p agentic-node --lib runtime::postage_client::fence_tests:: --locked -- --nocapture --test-threads=1
```

Expected: **6 passed / 0 failed**. Preserve the four `pending_fence_state_reads` JSON records for N=32/33/43/128, P=8; all unrelated active jobs must have exactly one read and total sender-job state reads must be <=48/49/59/144 respectively. These counters observe real `ProfileStore::state` calls, not physical SQLite I/O or IPC wall time. If compilation or any assertion fails, return the entire wrapper `command.log` plus `check.json` at this stage; the failure is actionable without a native rerun.

## 2. Existing targeted controls, format, Clippy, frontend

The following Node selection contains **17 existing tests**: renewal 5, maintenance 1, A03 worker/dispatch lifecycle 5, placement 6 (including the already fixed test #3). The Core selection contains **28 existing tests**: execution 12 + wallet-refresh 3 + history-inputs 4, retirement 6, terminal-GC 3. Libtest accepts multiple positional filters after Cargo's `--`, as already used in RUNBOOK. Check actual nonzero selected counts in the returned logs.

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-node-controls-20260917-r1 run cargo +1.91.0 test -p agentic-node --lib --locked -- runtime::postage_client::renewal_tests:: runtime::postage_client::maintenance_tests:: runtime::public_sender::batch_tests::a03_ runtime::public_sender::batch_tests::placement_tests:: --nocapture --test-threads=1
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-core-controls-20260917-r1 run cargo +1.91.0 test -p agentic-core --test public_postage_wallet --locked -- public_message_preparation::public_sender::execution:: public_message_preparation::public_sender::retirement:: public_message_preparation::public_sender::progress::terminal_gc:: --nocapture --test-threads=1
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-fmt-20260917-r1 run cargo +1.91.0 fmt --all -- --check
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-clippy-20260917-r1 run cargo +1.91.0 clippy --locked -p agentic-core -p agentic-node -p agentic-store --all-targets -- -D warnings
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-frontend-20260917-r1 run npm --prefix apps/desktop test -- --run tests/chat-shell.test.tsx tests/history-recovery-issues.test.tsx tests/desktop-api.test.ts tests/wallet-panel.test.tsx
```

The frontend files contain **45 tests** in the reviewed source (29 + 2 + 9 + 5). This is the existing RUNBOOK frontend selection plus wallet policy/budget controls. No frontend API changes are proposed; a separate TypeScript check is not required by RUNBOOK for this backend-only patch, and the mandatory macOS H11 desktop producer runs TypeScript/Vite already. If desired as part of the runner's usual frontend check, the exact existing script is:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-typecheck-20260917-r1 run npm --prefix apps/desktop run typecheck
```

## 3. Full Rust aggregate after the target gates pass

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-rust-workspace-20260917-r1 run cargo +1.91.0 test --locked --workspace --all-targets
```

Require zero RED across the complete command and actual per-target counts; do not pin the old 1361 total because the six new tests alter the suite. This command is the Rust aggregate from `scripts/check.sh`, with the required explicit toolchain. It does not invoke the unrelated full native/codesign A05 gate.

## 4. Prepared native cases

All preparations must be built from the same final production checkout. Each scenario should run immediately after its preparation, before another build can replace hash-bound artifacts. Take the manifest path from the producer's printed attempt directory. Placeholder `PREPARE_RUN` below denotes that exact new run; it is not the runtime run ID. Preserve both preparation and runtime wrapper logs/checks plus their inner native attempt directories. Prepared-artifact validation is the source/binary identity gate; never reuse a pre-patch manifest.

Minimum matrix matching the existing accepted/failing native baselines: A03 on Linux arm64; A04 on Linux arm64 and macOS; H10 on macOS; H11 on macOS using its release desktop bundle. The Linux H10/H11 equivalents are also given below if the agreed matrix includes all four backend cases on Linux; Linux H11 explicitly reports `portable-linux-backend` and does not substitute for the macOS bundle run.

### A03 lifecycle — Linux arm64, or macOS with the profile argument removed

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-a03-prepare-linux-r1 run env AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_sender_lifecycle.py
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-a03-runtime-linux-r1 run env AIN_NATIVE_PREPARED_MANIFEST=output/sender-lifecycle/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_sender_lifecycle.py
```

Require the existing lifecycle assertions: genuine two-original paid flow, atomic retirement and SQL retry, sender-absent/cold recovery; `passed:true`, empty cleanup errors, and a clean execution guard.

### A04 capacity — run on both Linux arm64 and macOS

Linux:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-a04-prepare-linux-r1 run env AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_sender_capacity_a04.py
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-a04-runtime-linux-r1 run env AIN_NATIVE_PREPARED_MANIFEST=output/a04-capacity/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_sender_capacity_a04.py
```

macOS:

```sh
python3 scripts/build-storage.py --output output/pending-fence-a04-prepare-macos-r1 run env AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_sender_capacity_a04.py
python3 scripts/build-storage.py --output output/pending-fence-a04-runtime-macos-r1 run env AIN_NATIVE_PREPARED_MANIFEST=output/a04-capacity/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_sender_capacity_a04.py
```

Require the unchanged **2 x 129 = 258 originals**, six completed batches [43,86,129,172,215,258], original 3600-second retention, all originals prepared before their unchanged reservation lease, active-slot reuse, old payment bytes unchanged, sender-absent recovery, **774 QC signatures**, **2580 data replicas and 2580 index replicas**, and **25800 location ACKs**. No `custody_storage` 5-second deadline failures. A04's existing own scope remains bounded: its runner explicitly leaves long-duration GC throughput and terminal-retention acceptance open.

### H10 Diagnostic32 — macOS

Use two distinct fresh diagnostic directories. Explicitly setting a new directory for preparation avoids the historical default `output/hrt32-r1`; the runtime directory must also be fresh because the H10 script rejects an existing evidence file.

```sh
python3 scripts/build-storage.py --output output/pending-fence-h10-prepare-macos-r1 run env AGENTIC_DIAGNOSTIC_OUT=output/hrt32-pending-fence-prepare-macos-r1 AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_history_read_trace.py
python3 scripts/build-storage.py --output output/pending-fence-h10-runtime-macos-r1 run env AGENTIC_DIAGNOSTIC_OUT=output/hrt32-pending-fence-macos-r1 AIN_NATIVE_PREPARED_MANIFEST=output/hrt32-pending-fence-prepare-macos-r1/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_history_read_trace.py
```

Linux counterpart, only if included in the execution matrix:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-h10-prepare-linux-r1 run env AGENTIC_DIAGNOSTIC_OUT=output/hrt32-pending-fence-prepare-linux-r1 AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_history_read_trace.py
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-h10-runtime-linux-r1 run env AGENTIC_DIAGNOSTIC_OUT=output/hrt32-pending-fence-linux-r1 AIN_NATIVE_PREPARED_MANIFEST=output/hrt32-pending-fence-prepare-linux-r1/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_history_read_trace.py
```

Require **32 originals, 96 QC signatures, 320 data replicas, 320 index replicas, 3200 location ACKs**, 32-message cold recovery, a captured/verified recipient trace, and a clean execution guard. Return `recipient-read-trace.log`, `recipient-capture.json`, `baseline-comparison.json`, the run's `trace.json` and `check.json`, and recipient boot logs referenced by capture. New semantic new/repeated reads, admitted requests, and total wall time come from this run's trace. Historical `output/hrt32-r1` trace-level comparison stays **null / blocked-by-data-loss**; the summary `evidence/reviews/AR2-wallet-flow/diagnostic32-native-r1.json` is the available baseline. Preserve any existing `h10Complete:false` or comparison-blocked fields. Report the native result separately under the user-approved historical data-loss limitation; no unavailable r1 trace is required to be recreated.

### H11 Full130 — macOS release desktop bundle

The desktop build is a prerequisite for this H11 case. It is not an A05 codesign acceptance run. The build script's own `--output` is a separate, initially nonexistent producer directory from the outer wrapper output. Giving it explicitly makes the desktop manifest path exact.

```sh
python3 scripts/build-storage.py --output output/pending-fence-h11-desktop-command-macos-r1 run node scripts/build-desktop.mjs --output output/desktop-build/pending-fence-h11-macos-r1
python3 scripts/build-storage.py --output output/pending-fence-h11-prepare-macos-r1 run env AGENTIC_FULL130_OUT=output/history-range-pending-fence-macos-r1 AIN_DESKTOP_PREPARED_MANIFEST=output/desktop-build/pending-fence-h11-macos-r1/prepared-artifacts.json AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_history_full130_h11.py
python3 scripts/build-storage.py --output output/pending-fence-h11-runtime-macos-r1 run env AGENTIC_FULL130_OUT=output/history-range-pending-fence-macos-r1 AIN_DESKTOP_PREPARED_MANIFEST=output/desktop-build/pending-fence-h11-macos-r1/prepared-artifacts.json AIN_NATIVE_PREPARED_MANIFEST=output/history-range-pending-fence-macos-r1/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_history_full130_h11.py
```

Default `AGENTIC_FULL130_BUNDLE` selects `target/release/bundle/macos/Agentic Internet.app`. Preserve the same desktop producer manifest in **both** prepare and runtime commands. Do not select `AIN_NATIVE_DESKTOP_E2E` or a debug/e2e bundle for this existing release H11 case.

### H11 Full130 — existing Linux backend case, if requested by the matrix

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-h11-prepare-linux-r1 run env AGENTIC_FULL130_OUT=output/history-range-pending-fence-linux-r1 AIN_H11_PORTABLE_BACKEND=1 AIN_NATIVE_PREPARE_ONLY=1 python3 tests/evm/public_history_full130_h11.py
python3 scripts/build-storage.py --profile portable-linux --output output/pending-fence-h11-runtime-linux-r1 run env AGENTIC_FULL130_OUT=output/history-range-pending-fence-linux-r1 AIN_H11_PORTABLE_BACKEND=1 AIN_NATIVE_PREPARED_MANIFEST=output/history-range-pending-fence-linux-r1/runs/PREPARE_RUN/prepared-artifacts.json python3 tests/evm/public_history_full130_h11.py
```

No desktop bundle/desktop manifest is selected on Linux. The existing script enforces Linux + explicit portable-linux for this flag.

For both H11 cases require the unchanged **130 originals, 390 QC signatures, 1300 independently verified index-receipt signatures, 13000 location ACKs**, exact desktop owner-API pages, failed/129-partial/130-recovered/130-cold history states, pre-network stopped-recipient durable snapshot checks, real loss and SQL faults, recovery before the original expiry, `passed:true`, empty cleanup errors, and a clean execution guard. Keep every batch sample; the preserved failing baseline completed 48 originals and stalled in batch 49–64. Do not call `statusSeconds` an individual IPC duration: it is the sum of the batch's status calls.

## Return evidence

Publish a branch with an identifying README, final tested source SHA, platform/toolchain and exact command, outer `check.json` + complete `command.log`, producer/native `check.json`, `prepared-artifacts.json`, complete `trace.json`, execution-guard reports, and runtime logs/recipient artifacts referenced by the reports. Add SHA-256 for each delivered file. Preserve the original failed RED artifacts separately. No fabricated aggregate PASS: distinguish passed, failed, not-run, and the intentionally unavailable historical H10 trace comparison. A05 remains outside this task.

## Source audit used to derive this packet

- `AGENTS.md`: wrapper/profile/output rules and required backend/frontend checks.
- `Docs/agentic_internet_v1_execution_plan/v1-plan-2026-09-14/RUNBOOK.md`: nonzero selectors, target-before-aggregate, frontend/format recipes, evidence format.
- `crates/node/src/postage_client.rs`, `postage_client_fence_tests.rs`, `postage_client_renewal_tests.rs`, `postage_client_maintenance_tests.rs`, `public_sender_batch_tests.rs`, `public_sender_placement_tests.rs`: actual module names and counts.
- `crates/core/tests/public_postage_wallet.rs`, `support/public_message_preparation.rs`, `support/public_sender.rs`, `support/public_sender_execution.rs`, `support/public_sender_wallet_refresh.rs`, `support/public_sender_history_inputs.rs`, `support/public_sender_retirement.rs`, `support/public_sender_progress.rs`, `support/public_sender_terminal_gc.rs`: actual Core test paths/counts.
- `scripts/check.sh`, `apps/desktop/package.json`, and the four listed frontend test files: exact available commands/counts.
- `tests/evm/public_sender_lifecycle.py`, `public_sender_capacity_a04.py`, `public_history_read_trace.py`, `public_history_full130_h11.py`, `public_paid_ciphertext.py`, `postage_spend_node.py`, `native_prepared.py`: actual scenario entrypoints, env, output/manifests, native oracles.
- `scripts/build-desktop.mjs`: release desktop producer and its supported explicit fresh output option.
- `evidence/reviews/native-acceptance-2026-09-17/README.md` and `evidence/reviews/pending-fence-2026-09-17/{README.md,trace-analysis.md}`: prior baseline matrix and limitations.

This packet was prepared read-only from source. No Rust, frontend, or native check was executed by this subtask; counts are statically derived and must be confirmed in actual output.
