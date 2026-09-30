# Independent test contract

Before production, a separate context-free backend-test-critic reviewed the four
new library tests, real daemon/EVM scenario and SQL fault helper. First verdict:
REVISE. Second verdict: ACCEPT after these corrections:

- Allow real network configuration while continuing to prohibit Alice's owner
  sender-work RPCs.
- Admit a second real outgoing ciphertext after index metadata fills the quota;
  object capacity remains available and ciphertext-only accounting would fit.
- A genuinely signed index obligation for the same operation/QC and a false
  mailbox association passes standalone index verification, then must fail the
  sender's actual-ciphertext binding without SQL changes.

The library baseline had missing API compilation errors. The first native
baseline stopped before the scenario because sources changed; it is diagnostic,
not feature RED. The stable second baseline reached the real ordinary-send
scenario and failed on absent `indexReplicas` status. See baseline.json.

Library cases cover real selected operator publications, independently observed
authority times, exact reconstruction without duplicated payment, immutable
positions/transport uniqueness, trust loss, SQL failure and cold retry/read,
ACK prerequisites, both quota-admission orders, expiry and monotonic clocks.

The native scenario uses a fresh genuinely funded book, two real ordinary MLS
messages, consensus QCs, data/index daemons and actual Noise transport. Separate
sender SQL triggers reject index-promise growth and location-ACK growth while
allowing data receipts, clocks and exact retries. Cold restarts must preserve
all committed progress and original ticket allocations. Independent probes
read actual remote index/location pages after Alice stops. One index bundle per
message is independently signature-checked; every published position/transport
and all 200 location ACKs are checked. Production verifies every index bundle.

A forged response from a different Noise peer remains an optional additional
native case; the current gate does not claim that adversarial callback test.

Native candidates 1 and 2 timed out without changing protocol bounds. Read-only
telemetry was separately accepted; CPU samples identified duplicate historical
authentication in `Bootstrap::configure` and the same synchronous `advance`.
The critic accepted reusing configure's `PostageHistoricalAuthority` only inside
that same call with unchanged snapshot/time. Every later advance still fully
authenticates. No warm readiness/authority cache was introduced; such a cache
would require a separate cold corrupted-closing/bootstrap test.

Candidate 3 still timed out during index progress. A fifth library regression
was added before bounded outgoing-index proof reuse: warm and cold reads/ACKs
reject a changed shared QC at the same SQL revision, and only exact restored
bytes recover. The critic requested an additional warm wrong-peer check; it now
rejects the identical bundle under a different expected peer, preserves SQL and
accepts the subsequent correct retry. Final cache-test verdict: ACCEPT.

The cache retains at most 128 existing opaque VerifiedIndexObligation values,
keyed by the complete serialized portable carrier hash. It grants no storage
presence; every operation loads actual SQL and commits its clock. Cache hits
check peer/position/expiry and call the existing Core-bound require_unexpired.
It is empty after restart. Native timeouts are performance RED; the new negative
regression passed before the cache and is not misrepresented as a failing test.

Candidates 4 and 5 exceeded the unchanged owner IPC timeout. Diagnostic sampling
of candidate 5 attributed 246/487 provider main-thread samples to background
client reconciliation/history and 437/536 sender samples to job advancement.
These samples guide investigation; they are not acceptance or a benchmark.

The next correction changes background scheduling only. An independent critic
first required a real-authority permit-expiry regression, then ACCEPTed
`maintenance_backoff_revokes_expired_client_permit_without_waiting_for_reconcile`.
It uses the existing genuine Core fixtures, establishes successful maintenance
and a live permit, sets a future scheduler deadline without sleep, then requires
expiry to revoke authority and the permit immediately. Baseline was compile RED
(two absent maintain calls and next_maintenance), not a runtime failure.

Runtime background reconciliation now waits one second after a full reconciliation
finishes. Every tick still checks the fence; explicit request/response paths keep
full reconciliation. Idle/unavailable history retries wait one second after work;
real queries keep 150 ms, and pending permit checks precede deadlines. No authority
or bootstrap-ready cache is added. Native liveness and existing native epoch/
revocation gates remain required.

Native candidates 6–8 still exceeded owner IPC while index promises accumulated.
Static inspection then identified a concrete integration defect: location request
preparation read the incoming carrier, which an ordinary outgoing-only sender
does not hold; repeated not-found failures could traverse all 100 transfers.
Late profiling attempts did not capture this stall and are not cited as evidence.

The critic accepted a sixth outgoing-storage test after correcting its untrusted
Core setup to include an identity. It exports genuine original and copied holder
claims from an outgoing-only sender, compares complete evidence and holder keys,
passes them to a real paid index store, and proves export/remote acceptance do not
write a local ACK. Unknown operation/index/data, wrong peer, absent trust and
read-clock SQL failures reject without sender row changes; cold export recovers.
The existing altered-QC test now also covers warm/cold exports. A test-only helper
type mismatch was corrected; baseline-2 contains seven missing-API compile errors.
All six outgoing-storage cases pass after implementation.

Export and ACK share verified claim construction. The node reads the outgoing
carrier through that export API. Each sender pass attempts at most four transfers;
failed preparation additionally yields after the existing work budget. Physical
wire limits, deadlines and the native acceptance assertions are unchanged.

Candidate 9 passed both SQL-failure/cold-recovery boundaries without an IPC
timeout, but reached only 65/100 and 100/100 location ACKs by the unchanged
180-second completion deadline. The critic ACCEPTed the unchanged native gate as
behavioral RED for the next narrow scheduling correction: a disposable per-stage
round-robin cursor visits each key at most once per pass and advances after failed
as well as successful attempts; data-major iteration spreads claims across index
peers. No new counter-order unit test or protocol/rate/deadline change is needed.
The custody server's 16 requests per peer per minute is a source-based potential
contributor to repeated Rejected responses, not proof of each response's reason.

Candidate 10 reached both complete promise sets but failed the 90-second cold
location-fault observation: ACKs stayed zero and errors stayed null. Result
polling on cursor revisit could lose finished failures to the transport cache's
16-result eviction. The critic ACCEPTed the existing cold-fault assertion as the
regression for polling each tracked ID once before new dispatch. Completed
results set errors/backoff; only durable rows determine completed transfers.

Candidate 11 passes its complete native gate and 74 backend/21 frontend tests,
Clippy and fmt. The existing ordinary sender native regression subsequently
exceeded its unchanged 90-second completion deadline; full slice acceptance
remains open. Candidate-11 reports and this regression failure are preserved.

Before bounded location-proof reuse, the critic required a data-only corruption
guard because changed shared QC could fail at index verification first. The
outgoing-only test now warms a genuine copy, changes only its original-primary
member proof at the same SQL revision, proves the index promise remains valid,
then requires warm export and ACK rejection with unchanged rows. Exact restored
bytes recover the genuine copy. The shared-QC test also warms an exported claim.
First baseline failed because generic JSON serialization reordered fields and
caused a canonicalization commit; baseline-2 preserves exact typed field order
and all six tests pass. Final independent verdict: ACCEPT. These are invariant
guards; the unchanged native sender deadline supplies the performance RED.

Production now shares a bounded typed cache helper for index and location proof
results, with at most 128 entries in each map and full serialized carrier keys.
Current Core trust/time, exact holder/position/expiry and actual SQL reads/commits
are always checked; no authority/readiness cache or protocol limit changed.

The location-cache candidate passes six storage and 21 frontend tests, but the
ordinary native sender again exceeds its unchanged 90-second deadline. Passive
timing samples in regression-sender-candidate-2.json show one message complete
and the other with 12/100 location ACKs after late data/index completion.
observe_sender.py returns the original status unchanged and adds no IPC calls.

The next proposed correction batches up to 16 compact holder receipts for one
already-paid index. Its new storage test precedes production and currently has
21 missing-method compilation errors (location-batch-baseline-2.log). It requires
atomic receiver insertion and sender ACKs, genuine primary/copy provenance,
bad-second-claim rejection, holder/position and current Core trust checks,
empty/duplicate/size/unknown-position bounds, exact quota-1/exact quota, real SQL
faults, cold retry and no ACK on export. The critic first required moving the
no-ACK check before the shared Core clock advanced, then ACCEPTed the correction.
Semantic negative cases use the ordinary quota; exact quota checks are separate.
All 26 paid-index tests pass after storage implementation. Existing
native deadlines and the 20 promises/200 ACK acceptance requirements remain.

The full ordinary sender gate passes after batching (candidate 3 regression),
but index fault candidate 12 stalls at promises 10/10 and 9/10. The second job's
missing position is 9; fresh resolution still starts again at already-retained
positions and searches unused replacements. The critic ACCEPTed this unchanged
120-second native failure as behavioural RED for requesting only missing primary
positions. Original numbering/full plan and every actual offer check remain;
skipped positions have no offer and only a truthful partial result. Default
owner/data resolution is unchanged. No request-count unit test is claimed.

Candidate 13 passes the complete index fault/restart gate after that correction:
20 actual selected promises, 200 location ACKs, both SQL faults, cold sender and
independent index recovery, and no repeated index writes. Its 75 targeted backend
and 21 frontend tests, Clippy/fmt pass with 587 unchanged inputs. Existing native
sender/index/history regressions on that exact source remain required before
slice acceptance. All four existing regressions subsequently pass on the same
application source/binary, including full 133-spend/two-closing/1605-signature
handover. The collector initially read the wrong handover output directory even
though the actual command succeeded. Its error and initial evidence recovery
are preserved separately. Independent review required --collect to compare
the complete saved native report and trace, not just the original log. After
that correction the critic returned ACCEPT; affected checks were rerun and the
previous native reports were collected with exact saved-output equality.
