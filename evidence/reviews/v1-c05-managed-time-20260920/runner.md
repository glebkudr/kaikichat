# V1-C05 runner: managed time, early-stop diagnostics, one-process reproducer

Base: `d278f2abae35bf5e50848bee4ee03ae1efa48210` (`implementation/v1`).
Scope: `Docs/agentic_internet_v1_execution_plan/v1-plan-2026-09-14/00-tasks.md`
§ V1-C05. Motivation: the A04 straggler (one job stuck at 9/10 data-custody
receipts) needed a fast diagnostic contour and a deterministic reproduction
instead of a long native E2E per hypothesis.

## What changes

### Managed-time seam (`crates/node/src/clock.rs`, `random.rs`)

- Four documented time domains: protocol wall time (`clock::wall()`),
  scheduler monotonic (`clock::instant()`), chain time (Anvil, untouched),
  real execution time (watchdogs, IPC, `elapsed_us` telemetry — deliberately
  not virtualized).
- Thread-local sources; production installs nothing and reads OS clocks.
  Test-only `clock::Virtual` installs one controller shared by every logical
  node in the process; `advance`/`advance_to`/`set_wall` move the domains
  independently; nested installs restore on drop. `Virtual` also drives the
  embedded finalizer crate's wall seam (`service::__install_wall`, doc-hidden
  non-cfg API because node tests compile finalizer without `cfg(test)`).
- All scheduler `Instant::now()`/`.elapsed()` call sites (~35 files) converted
  to the seam; real-execution sites (`processing_worker` timing, IPC
  deadlines) intentionally untouched.
- `random::fill` seam for scheduler-visible randomness (job ids, sync
  nonces); `random::Deterministic` installs a `sha256(seed||counter)` stream.
  Client-side paths (`runtime_client`, `mcp`) keep the OS RNG.

### Diagnostics

- `historyDiagnostics` extended with the retry schedule, pending requests, a
  snapshot series and expected post-sender-shutdown work.
- `custodySync` info now reports cursor, due and per-work expected-work
  records (conversation, elapsed budget, chased head, in-flight request) —
  the H11 "what is still owed after the sender is gone" surface.
- `custodyResolution`/`custodyNetwork` job info carries active/pending/
  retained counts and per-job positions, candidates and elapsed — the
  straggler-leg view. `custodyImportFailures` keeps exact SQLite code +
  error SHA-256 evidence, separate from injected-fault markers.

### Python diagnostic harness (`tests/evm/sender_diagnostic.py`)

- Phase selection, `wait(...)` with a ~100 s no-progress budget used only
  for diagnosis (acceptance/liveness deadlines unchanged), `Stalled`
  carrying a complete snapshot (node info, sender status, import failures,
  fault-reader events, selected message records).
- Integrated as an opt-in wrapper at the paid-batch `stored` wait and the
  recovery waits in `public_index_recipient.py` — the exact site of the A04
  straggler; `public_sender_diagnostic.py` is a focused lifecycle runner.

### One-process reproducer (`crates/node/src/reproducer_tests.rs`)

- `Rig`: several logical nodes in one process, each a real `Runtime` on a
  real SQLCipher store with a real libp2p swarm, sharing one installed
  clock/RNG controller.
- Driver pumps production maintenance, drains real swarm events in fixed
  node order, waits real responses/IO failures (bounded grace), then
  advances the shared clock only to the nearest declared `next_due`.
- Ordered `trace` is kept for diagnosis; the asserted artifact is a
  canonical digest — semantic event counts plus stable custody counters —
  because real libp2p reorders IO nondeterministically.
- Scenario: two originals whose index routes list a dead transport key
  first; the recipient walks the unavailable leg (real dial/transport
  failure), rotates and gets answers from the surviving holder — the A04
  straggler failure class.

### Event-driven wakeup (`runtime.rs`)

- `Runtime::next_deadline()` aggregates Option deadlines across retries,
  listener retry, custody query pacing, paid-custody maintenance, custody
  sync scan/dial windows and the sender lane. After every select branch,
  a deadline already due triggers `pump()` without waiting for the 100 ms
  tick; a 10 ms real-time floor only paces pump invocations. Due times
  inside `pump` still gate the work — backoff and admission unchanged.

### Anvil adapter (`tests/evm/issuer.py`)

- `Chain.mine_at(timestamp)`: `evm_setNextBlockTimestamp` → `evm_mine` →
  verify the sealed head, so a scenario continues from real chain evidence.
  Applied at the pure empty-block sites (`live_wallet_authority`,
  `finalizer_service`, `custody_resolution`, `generate_service_capacity`);
  transaction-mined sites keep the raw pattern.

## Verification

- `cargo test -p agentic-node --lib`: 346 passed / 0 failed
  (includes both reproducer tests; two runs share one canonical digest).
- Clock-seam RED→GREEN: 4 scheduler-path tests now expire/stall/detect on
  installed virtual time; backend-test-critic: ACCEPT before production code.
- Harness fake-node: stall at 100 s with full snapshot; progress resets the
  budget; acceptance deadline preserved; import failures separated from the
  injected fault.
- `cargo clippy -p agentic-node -p agentic-finalizer --tests`: clean.
- `cargo fmt`: clean.
- Finalizer crate suite: green after the wall-seam change.

## Boundaries kept

- No acceptance timeout, liveness bound or admission/backoff constant moved.
- No fabricated ACKs/receipts/imports; the reproducer observes real
  transitions only (paid import without authorization is still rejected —
  the scenario asserts rotation evidence, not a fake success).
- A04/full E2E remain the acceptance gate; the reproducer is a diagnostic
  and regression instrument, not a substitute verdict.
