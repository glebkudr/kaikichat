# Outgoing retention in the ordinary Runtime

The daemon now sets an outgoing quota of 4096 originals and 64 MiB of payload/index
bytes, separately from incoming operator admission (still 128 objects /8 MiB).
The existing outgoing storage component also bounds evidence bytes independently.
`Runtime::pump` invokes custody maintenance before recipient synchronization and
sender work. Maintenance attempts one bounded expiry batch per second, including
when no sender or custody request is active. Failed attempts use the same delay.
The durable cursor and occupied quota advance only with the storage transaction.

The public `Service::open` remains fixed to the daemon's `NETWORK_DOMAIN`. Its
private `open_in_domain` constructor holds the common storage policy and is also
used by the test with the existing fixture network. The first runtime test failed
correctly because the fixture's signatures belonged to a different network. The
accepted correction selects that signed network at construction; it does not
rewrite signatures, weaken domain checks or introduce a test-only storage policy.

## Verified behavior

The new test runs the real Runtime and shared constructor with the existing paid
fixture, real Core admission and the existing independent P-256 QC oracle:

- Retain 134 distinct paid originals under the daemon's outgoing policy, reopen
  the encrypted database, and read every exact receipt. Incoming storage remains
  empty and there are no active custody jobs or pending requests.
- Expire three originals, inject a real SQL UPDATE failure, and prove that all
  rows remain exact. Removing the fault does not bypass the retry delay. The next
  scheduled attempt reclaims exactly two bodies; repeating that wakeup does no
  extra work. Cold opening performs no hidden cleanup; the next attempt reclaims
  the third. All 131 other bodies retain exact bytes and revisions.
- With the historical fixture now expired at wall time, call actual
  `Runtime::pump`. It reclaims exactly two more bodies without an owner endpoint,
  read-triggered cleanup or current paid authority in Runtime Core. An attempted
  clock rollback then fails without changing SQL.

The context-free backend-test-critic accepted the scenario before production and
separately accepted fixture registration and network-domain corrections. The
original RED contains five missing `maintain` callsites. The shared paid fixture
was extracted from the existing custody tests; it preserves the same six ordinary
carriers, 140 distinct indexed spends, operator verification and exact fixture
hash checks. Both test roots now reuse a single existing P-256 oracle.

Validation passes: **51 backend tests**, **31 frontend tests**, postage-spend/node
all-target Clippy, formatting and whitespace checks.
[Commands, review and focused source hashes](runtime-retention-checks.json) record
these results. The 50 custody
regressions overlap earlier component evidence and are not additional independent
coverage. Raw logs remain under `output/ar2-wallet-flow/`, including the incorrect
fixture-domain run and the intermediate fixture compilation/lint failures.

## Next required work

This accepts Runtime storage policy and maintenance wiring in the signed fixture
network. It does not accept an ordinary over-128 paid network flow in the daemon
network or renew the packaged native evidence. Incoming data/index/inspection
stores still need per-object storage, bounded expiry and explicit operator
admission. The signed network history directory still has a 128-reference limit.
Complete those layers, then perform ordinary long paid send, sender restart,
sender/data/index removal and full recipient recovery. All remaining V1 cards,
E2E scenarios and three-platform release requirements stay active.
