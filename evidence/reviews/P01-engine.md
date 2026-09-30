# P01 configured Simplex engine and actual TCP validator

Full P01 and the V1 application goal remain **OPEN**. This delivers an actual reusable
configured voting runtime and an executable TCP fixture. It does not select a committee
from L2, connect that committee to daemon operator routing, finalize global spending,
store ciphertext replicas or perform application effects atomically.

## What runs

- `crates/finalizer/src/engine`: Commonware2026.9.0 Simplex Ed25519, standard Marshal/Inline,
  buffered block broadcast, P2P backfill, immutable finalized archives and voting WAL.
  Five channels accept an authenticated peer provider/blocker supplied by the caller.
- Canonical `ConsensusBlock` preserves the previous Entry digest and certificate format.
  The runtime checks committee, sequence, parent ancestry, operation uniqueness and128-entry
  bound, then application external validity. Locally proposed entries are checked too.
- A persisted committee/genesis/signer binding prevents a changed configuration from
  silently reusing the same voting log. Restore the highest durable finalization as the
  engine floor. Startup opens the application archives; upstream WAL replay precedes
  voting. Startup does not assert a connected quorum. Lease expiry terminates the
  supervised runtime and its descendants.
- Marshal delivers durably archived blocks in order, including ancestors finalized by
  a descendant. Explicit application acknowledgement gives at-least-once delivery.
  The older direct-certificate-only `FinalizedLog` is deliberately not used to fabricate
  certificates or parent views for these deliveries.
- `agentic-finalizer-validator` runs the same runtime over actual authenticated encrypted
  lookup TCP. Its exact configured batch is committed into logID. Stdin carries its key;
  private storage is locked. It emits the durably processed historical prefix and durable
  Marshal deliveries. A merely cached proposal is not a receipt. This fixture acknowledges
  output after flushing; it has no separate storage/payment effect.

Contract: `spec/finalizer/engine-v1.md`. Commonware version matches the previously pinned
family2026.9.0; new direct dependencies were already present at those versions in Cargo.lock.
Official runtime release index: https://docs.rs/crate/commonware-runtime/2026.9.0.

## Test-first and independent review

Production followed independent FINAL ACCEPT from `/root/node_test_critic`, reused without
fork context. Initial runtime review required cold recovery without peers, authenticated
post-restart participation, isolated fault controls and a correct first-parent oracle.
All were fixed before implementation. Twelve runtime tests received FINAL ACCEPT at
SHA2560887e6c745982ad6fcd3d137c8e9fd9e142fa132936ec12c8206fd85cdca8e41.
The sole subsequent compile fix passed the same trailing bytes as a slice; it received
separate FINAL ACCEPT. Test-only lint allowances also received FINAL ACCEPT.

The executable tests were written before the executable existed. Review required final-tip
QCs, draining all stdout after SIGKILL, a valid foreign remote public key, and an actual
storage lock test using a different free endpoint. All were fixed. Three process tests
received FINAL ACCEPT at SHA256c3b1fe2166ad506e291ab1ce6e1d7b3e78279847c7411dbd54216263544dabba;
the only later change was a separately accepted helper lint allowance.

RED logs: `P01-engine-red-final.log` (absent engine module) and
`P01-engine-tcp-red-revised.log` (absent executable). Earlier compile/review iterations
are retained too. They are not counted as passing evidence.

## Focused observed results

12 engine tests and3 actual process tests pass. Targeted all-targets Clippy passes.
`P01-engine-green.log`, `P01-engine-tcp-green.log`, `P01-engine-clippy-clean.log`.
The final aggregate reruns these tests on the final source.

The engine cases cover real four-engine convergence and authentic QCs,2+2 partition/heal,
one unauthorized proposer, malformed structural proposals,128-entry capacity, offline
backfill, cold recovery of an unacknowledged block without any peers, cold recovery after
an authenticated own vote before QC with new authenticated votes afterwards, lease and
configuration binding, nonmember key refusal, canonical wire and descendant-only finality.
These are deterministic network simulations using actual upstream signing/consensus/storage,
not premanufactured live consensus results. Only the separate descendant-mailbox contract
case supplies a prebuilt real QC to exercise transitive finality explicitly.

The process cases run four separate native executables and storage folders over TCP.
After a verified receipt, actual SIGKILL stops one process before it emits the complete
24-entry batch. Every remaining process must obtain a verified tip QC. The killed process
then recovers its emitted prefix with all peers stopped, and catches up to the same full
hash-linked history and verified tip after peers restart. Concurrent storage access is
refused before ready on a different free port; the same configuration starts after the
first process exits. Invalid signer, changed policy batch and foreign authenticated peer
configuration are rejected before ready. No private seeds are on argv.

## Retained independent live evidence

`P01-engine-demo/` retains inputs and JSONL outputs from four separate native TCP processes.
All keys are explicit public test fixtures, not production credentials. Each process
finalized the same12 entries. A separate Python cryptography46.0.5 verifier reconstructed
canonical committee/entry bytes, followed the entire genesis-to-tip chain and verified
144 real Ed25519 signatures. The same144 signatures failed for the wrong signing subject.
Every final tip has a QC. Stderr and exact process cleanup are empty.

`P01-engine-demo/report.json` records the binary hash and result; the retained
`verify-live-fixture.py` regenerates this finite fixture and checks its wire independently.
It is a fixture parser, not a general alternative consensus verifier. The input leases
are historical evidence; regenerate them to execute a new live run.

## Negative controls

Five compiling mutations in an isolated APFS source copy and cloned independent target
are detected (`P01-engine-mutations.json` and individual logs). The application tree,
binaries and ongoing native gate are unchanged by these experiments.

| Mutation | Observed failure |
| --- | --- |
| Ignore false application verdict after calling the hook | An unauthorized operation is actually finalized and rejected by the independent test oracle |
| Admit repeated operation ID | Duplicate operation appears in finalized history |
| Lose stable storage namespace across cold restart | Previously delivered block cannot be recovered without peers |
| Continue runtime beyond lease | Required termination times out |
| Bypass explicit committee binding | Upstream genesis-anchor mismatch panics instead of the required controlled startup error |

The last control demonstrates the startup error boundary; it does not imply absence of
upstream's independent anchor guard. The first version of the external-validity mutation
only skipped the callback. The refined control preserves the callback and ignores its
verdict, producing the stronger observable unauthorized-finality failure. Its initial
syntax error is retained separately and is not counted as a detected compiling mutation.
Scratch sources were restored after every experiment. Current source hashes and file list
are recorded in `P01-engine-source.json`.

## Aggregate verification

The full native gate exits0 (`P01-engine-native.log`):388 Rust,24 Solidity,7 model
and40 frontend test functions pass (459 total). All actual Anvil runners pass, including
285 independently checked custody positions through2623 owner calls, mandatory expiry,
held-slot and wire-limit controls, and empty cleanup errors. Five actual packaged hidden
WKWebView flows pass (`P01-engine-native.json`). The rebuilt macOS arm64 release passes
strict deep signature and default driver-exclusion checks. It remains ad-hoc signed and
not notarized. Current chat and restored-trust screenshots were viewed beside796a74d
references: same layout and content, with expected generated time differences.

The independent Linux node transport gate passes7 outcomes with empty cleanup,
run `ain-nat-13eda234`, source98943a9baf0f8453cbb09e7a85c3e2b0518351a8a4ffbac1b0af5ae16517e6a4.
The Linux gate builds/tests the existing daemon transport, not the new finalizer executable.

## Remaining product boundaries

Full P01 still needs authenticated L2-selected committees, daemon/operator transport/key
integration, epoch handover and product-specific external-validity/effect transactions.
These project tests include an unauthorized proposer and cold vote recovery; they do not
claim a complete adversarial network campaign for every equivocation/withholding behavior.
A complete valid malicious filesystem rollback/deletion is not detectable from local files;
no voting-state reset/import is exposed. The CLI is a configured batch fixture, not a
production wallet, MCP job endpoint or desktop payment feature. Actual R=10 custody/repair,
global spend and the remaining V1 product scenarios remain open. The application goal must
not be closed on these green subsystem tests.
