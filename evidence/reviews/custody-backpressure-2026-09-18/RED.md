# A04 custody scheduling: static RED contract

Base: `be8ec6328fada9fb67f90f0181ed711450e9397a`.
Execution is delegated to the user's Linux/macOS runners. No Rust build, test,
native scenario or formatting command was executed in this authoring session.
RED below is a source-derived prediction, not a recorded failing run.

## Evidence and corrected diagnosis

The source of the traces is private development archive (published source: `glebkudr/kaikichat`), branch
`evidence/pending-fence-runner-20260918`, commit
`f4a23344c42761bdf4188287ea72211e5819a77c`.

| File | Git blob | SHA256 in the branch manifest |
| --- | --- | --- |
| `a04-macos/native-trace.json` | `7ccf9198093a10cda2afac83bb2a094596808ec9` | `ea1c73eaf2744e04086f1956b2ad6ee23c8361c3c01dc1016987718146109db2` |
| `a04-linux/native-trace.json` | `96fc53465504469f91814366fd90c5bb8dd3b52d` | `ea3e1b76f2a087f53a16e2f182ef74b87781b8a9737c9f417d09b2760fd54f46` |
| `h11-macos/native-trace.json` | `dda205a9eaa5e1155fe633fd88cd03cbb28f52d5` | `ee0a950f953e0495348f0dbc51a18906e0c66d7e75f689bee6961fd69309a08d` |

The macOS trace contains 43 originals, 173 data receipts and no index receipts
at its final sample. `failureActor0.custodyResolution.transportFailures` is
139896; `processingCapacity.rateRejected` is 140917. The close counts support
a rate-refusal feedback loop, but do not identify every transport failure as
a local rate rejection. The renewal is issued at 1789692135; the first
original is issued at 1789691537. The 11 checkpoint errors first appear in the
last of 57 samples, near the end of the 600-second publication wait. This
trace does not demonstrate permanent failure after renewal.

The Linux trace is a different failure phase: `authorityRenewals` is empty,
and all 43 final sender views have 10 data and 10 index replicas. Forty-two
are `publishing` and one has `checkpoint_rejected`. This patch must not be
represented as resolving that separate publication failure without a new run.

`live_wallet_authority.py` already accepts and republishes the new head and
registry proof on every custodian before recording renewal success.
`serve_custodian_proof` and `serve_operator_proof` require the exact current
checkpoint. The sender's saved `resolution.assignment` is its own immutable
plan, and `admission:false` is unconditional planning metadata, not refusal.
Pending resolutions already become stale on a changed head; terminal result
reads recheck the candidate through Core. No second operator refresh or
sender authority bypass is justified by these observations.

Two independent defects can be demonstrated in the resolver:

1. At 16 retained jobs, a new lookup evicts a finished job even if the caller
   has not read its result. Ordinary sender work sees `not_found` and starts
   another search. Fast completions and 43 callers can discard useful work.
2. Each unavailable response or outbound failure immediately schedules the
   next candidate. There is no outbound per-peer pacing across jobs. Every
   unsuccessful remote lookup consumes the existing 128-per-peer/minute
   admission allowance; rapid refusals can consume entire scans and drive
   the local 256-per-second shared processing limit.

## Required test distinction

`crates/node/src/custody_resolution_capacity_tests.rs` uses the existing paid
fixture and actual public resolver admission. Empty connected-peer sets
produce truthful `partial` results, not fabricated successful custody.

- Sixteen unread completions must retain all sixteen IDs. A seventeenth call
  must return `custody_capacity`; on the base it returns success and evicts
  an unread result. This is the principal static RED assertion.
- A terminal result read allows eviction of that result only. The replacement
  is protected until it too has a terminal read.
- Reading a pending result cannot mark its later completion as consumed.
  The test establishes an actual connection and request, then ages only the
  disposable search clock to exercise the existing deadline and draining.
- Original 60-second completion retention, network replacement, signed
  checkpoint renewal, and live transport slot accounting remain enforced.

`crates/node/src/custody_resolution_pacing_tests.rs` exercises the existing
resolver scheduler with genuine paid context and actual connected peers.
It must reject rapid repeated dispatch to the same peer, preserve an untried
candidate while deferred, allow other eligible peers to progress, and share
pacing between data/index jobs. Expiry must still stale the job and retain
physical draining slots. No successful receipt or offer is invented.
The progress control waits 510 ms with the signed wall clock unchanged and
requires the next actual dispatch. A separate admission after job retirement
must retain the first job's unexpired peer cooldown. The 64-entry cooldown
map bound is reviewed statically in production, not claimed as an executed
65-peer integration test.

## Production contract for the independent critic

- Keep 2 active resolutions, 4 physical requests including draining,
  16 retained results, 32 candidates per window, the original 60-second
  search and retention deadlines, and the 5-second transport/IPC deadlines.
- Protect unread terminal results until a terminal read, existing expiry,
  or invalidation; saturation returns existing `custody_capacity`.
- Charge a 500 ms per-peer cooldown when submitting a custody request,
  regardless of how it later finishes. Cooldowns belong to the whole resolver,
  survive job churn, and never advance an untried candidate on refusal.
- At most 64 unexpired cooldown entries; expired entries may be removed.
  A full cooldown map defers a new peer rather than evicting a live cooldown.
- Keep paid assignments, original bytes, tickets, finality, receipts,
  sponsorship, TTL, leases, allowances, and current authority verification.
- No new offer cache, paid selection, success flag, retry deadline extension,
  processing-limit increase or remote protocol change.

Pacing bounds this sender's attempts. It does not reserve the receiver's
192-total/minute allowance against other senders or guarantee completion
within a deadline. Native A04 and the existing positive custody/network gates
remain necessary on the executor's side.

## H11 remains separate

The first exact SQL fault passed and rollback snapshots match. After the
recipient restart, the final trace reports 115 received imports, zero local
SQL import failures, and exhaustion of the unchanged 1300-second wait with
about 412 seconds of original retention left. The last-fault predicate first
requires exactly 129 visible originals. Its timeout does not establish that
the last-original SQL hook is defective. Range recovery needs separate
investigation; this change does not modify its oracle or claim H11 PASS.
