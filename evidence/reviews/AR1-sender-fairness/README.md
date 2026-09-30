# AR1 — bounded sender scheduling, accepted on candidate 5

The daemon uses a disposable bounded FIFO with per-job retry deadlines. Core
reuses each conversation policy only within one synchronous validated queue
read, and shares retirement/active-queue and job/delivery observations. Current
authority and grants still gate every allowed advancement. The actor handles
its existing trusted consensus mailbox immediately. Private custody jobs retain
fully verified offer metadata, while every cached read checks current Core
authority, original verification time, expiry and the exact Noise connection.
Full cryptographic verification remains mandatory for every incoming offer.

| Check | Observed result |
|---|---|
| Funded full queue | 128 jobs / 127 paused; ready tail 1.30s, live resume 6.29s, cold finality 11.66s after reconnect setup; maximum owner RPC 1.64s; original allocation/queue/reservations preserved |
| Complete ordinary sender | Two messages, six independently checked QC signatures, twenty authentic receipts, SQL-fault restart/retry, wrong-Noise offer refusal, offline and cold recipient recovery; zero work/retrieval RPCs or prover invocations |
| Custody cache preservation | Two chains, 329 verified positions, same-peer TCP/QUIC reconnect without intermediate cache read, binding/head expiry, relay-only, capacity and cold-cache clearing |
| Hostile epoch history | Three spends, two closings, 69 signature checks; wrong target, altered QC and mixed page preserve state and cannot enable an early signer; original evidence resumes after crash |
| Core wallet/preparation | 46 completed tests, including 30 sender tests |
| Daemon | 96 library tests and node Clippy pass |
| Earlier completed checks | 59 frontend tests, 19 model tests, TypeScript/Vite and fmt pass |

All four native gates used daemon SHA256
`c8d28a3134a1217b7cd4b6894d312ce143d4c0ed31adefb2350c09a62c95a8dc`.
All 572 inputs in `candidate-5-inputs.json` stayed unchanged through verification.
Native observations are local acceptance evidence, not WAN performance claims.

The user changed intermediate testing policy during the already-running workspace
check: affected clusters now run during implementation; the full suite is reserved
for the end of the complete plan. The broad process was interrupted. Its completed
Core wallet target is recorded in `core-wallet.json`; a complete workspace pass is
**not** claimed. Completed frontend/model/fmt checks were retained, and only daemon
tests/Clippy were then run. See `checks.json` and `workspace-interrupted.json`.

Independent test reviews 1–12 preceded their corresponding production changes.
They require real timing bounds, original paid messages, unchanged reservations,
live sponsorship pause/restore, exact hostile-response observation and unexpired
same-identity reconnection under a new authenticated connection. TEST_CONTRACT.md
records those requirements.

Earlier candidate results remain distinct. Baselines observed real startup or
signed-admission IPC failure under an authentic 127-job backlog; no measured
64-second tail delay is claimed. Candidate 1 missed readiness; candidate 2 missed
cold finality/IPC bounds. The unprefixed `native-check.json`, `native-trace.json.gz`
and `source-inputs.json` belong to candidate 3, which passed fairness but failed
the complete sender scenario. Candidate 4's immediate mailbox handling reached
finality/storage in two runs; corrected one-shot carrier setup and exact-response
observation retain all negative assertions. A later run still missed storage.
The custody baseline passed its first reconnect case, then failed a later provider
restart. These failures are diagnostic history, not acceptance of those candidates.

Current results are `candidate-5-{sender,fairness,custody,hostile-history}-check.json`,
with matching integrity reports and compressed original public evidence where
available. Run/build logs remain in ignored output; `local-log-manifest.json`
retains their paths and hashes. The bounded regression driver can be invoked as
`python3 scripts/build-storage.py run python3 evidence/reviews/AR1-sender-fairness/verify-candidate-5.py`.
It refuses source changes and runs only the daemon tests and node Clippy.

Successful terminal retirement, a full persistent stage ledger, disjoint-roster
history, automatic authority renewal/outage closure and full V1 remain open.
See [required continuation](NEXT.md); the 67-card / 22-E2E / three-platform scope
is unchanged.
