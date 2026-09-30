# Executor handoff: custody backpressure and peer pacing

Base: `be8ec6328fada9fb67f90f0181ed711450e9397a`.
Tests and static RED: `9c748613d4406c01483c719830ac36f959965faa`.
Branch: `fix/v1-custody-backpressure-20260918`.
The production commit is the next commit on that branch; it changes only the
resolver and its specification/handoff documents, not the accepted tests.

## What changes

Ten regression/control tests precede the production change. The independent
backend-test-critic accepted their static contract before implementation.
No compilation, RED/GREEN, formatting or native run was performed by the
architect. Source/diff review is not runtime acceptance.

The production change is confined to `crates/node/src/custody_network.rs`:

1. Unread terminal results stay inside the existing sixteen-slot cache.
   New work receives `custody_capacity` until a terminal read, invalidation
   or original expiry makes a slot reclaimable. Reading pending work does
   not consume its future result. Completion time is never refreshed.
2. Request submission charges a shared 500 ms per-peer interval. A cooling
   peer does not consume a candidate position. Other positions and jobs can
   still use eligible peers. At most 64 live intervals are retained; a full
   map defers new peers and never evicts an unexpired interval.
3. Maintenance invalidates terminal jobs on head/authority expiry as well
   as pending jobs, so the new cache protection cannot pin old-head results.
   Core authority and offer verification on every result read remains intact.

There is no new operator proof, offer cache, paid selection or stored success
flag. Original paid identity, envelope/stamp/finality/receipts, TTL, lease,
allowances, 5 s deadlines, 2 active jobs, 4 physical streams including
draining, 16 results, 32-peer windows, and 60 s search/retention remain.

The hard 64-entry map bound is verified statically. These tests do not claim
an executed 65-peer saturation run, successful remote custody, or A04 PASS.

## RED and GREEN on an isolated executor checkout

Fetch the branch and retain its exact production SHA before checking out RED:

```sh
git fetch origin fix/v1-custody-backpressure-20260918
git rev-parse FETCH_HEAD
git switch --detach 9c748613d4406c01483c719830ac36f959965faa
```

Run the ten focused tests using the existing configured build-storage wrapper.
For the Linux portable profile:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/custody-backpressure-red-001 run cargo test -p agentic-node --lib runtime::custody_network:: -- --nocapture
```

The source prediction is seven assertion failures and three controls passing.
The primary RED point is the seventeenth unread resolution returning `Ok`;
the pacing failures are immediate extra actual dispatches. Compilation/setup
failure is not the expected RED and must be reported separately.

Switch to the exact production SHA captured above, and run:

```sh
python3 scripts/build-storage.py --profile portable-linux --output output/custody-backpressure-green-001 run cargo test -p agentic-node --lib runtime::custody_network:: -- --nocapture
python3 scripts/build-storage.py --profile portable-linux run cargo fmt --all -- --check
python3 scripts/build-storage.py --profile portable-linux run cargo clippy --workspace --all-targets -- -D warnings
```

On macOS use the existing `mac-apfs` wrapper from the normal source checkout;
omit `--profile portable-linux`. Preserve the executor's existing preparation,
toolchain and artifact-identity procedure for native binaries. No old prepared
binary should be relabeled as a run of this source change.

## Required native follow-up

- Preserve the existing positive custody-resolution gate, including fresh
  presentations, address-hint fallback, exact assignments and both chains.
  `tests/evm/custody_resolution_slots.py` must still retain all four draining
  streams across checkpoint cancellation and complete its finite scan.
- Run the existing `public_wallet_custody_renewal.py` mixed-head scenario,
  preserving the first real data receipt, old original bytes and ticket,
  and completing new-head data/index custody after a cold sender restart.
- Re-run A04 with the original 43-message batch, timeouts and leases. Compare
  custody transport failures, shared rate rejections, query counts, data/index
  receipts and eventual completion, not just absence of checkpoint errors.
- Preserve the prior sender/fence controls, workspace/frontend gates and H10
  guard through the established executor matrix.

Keep RED/GREEN commands, exact source and prepared artifact identities, logs,
native traces, `check.json` and SHA256SUMS in the evidence branch.

## Remaining acceptance boundaries

The supplied macOS A04 trace shows the custody request storm before renewal;
the supplied Linux A04 trace has no renewal and already has all data/index
receipts, with the failure in later publication. The patch addresses the
demonstrable resolver defects; a fresh run must determine subsequent blockers.

H11 is unchanged. Its first exact SQL fault passed; after recipient restart
the last snapshot has `custodySync.received = 115` and no local SQL import
failures. The last-fault predicate first requires 129 visible originals.
For separate range-recovery work, capture the final inbox IDs, durable graph
cursor, pending/deferred operation IDs and MLS revisions, plus reference/route
selection and import outcomes. Do not extend its 1300 s wait or weaken either
exact SQL oracle on the basis of the current aggregate trace.
