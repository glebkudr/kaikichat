# V1 implementation status

## 2026-09-13 — Full130 R14 publishes all originals, recovery outlives the earliest lease

All 130 originals publish with 390 independently verified signatures, 1300 data/
1300 index receipts and 13000 location ACKs. Actual 1170+1170 copy loss,
missing-trust refusal and the first exact-message SQL failure/rollback pass.
During partial 129-of-130 recovery, the earliest original expires. Post-stop
state contains exact imports 1..30, 31 deferred bodies and 60 prefetched bodies;
original 31 is absent from both caches. Final receiver counters: 418 reads,
209 bulk reads, 155 path reads, 30 imports, no storage errors after the first
fault was removed. These counters do not identify individual delayed requests.

All 956 inputs and five signed artifacts are unchanged over 3646 seconds;
cleanup leaves no processes/profiles. Signed graph membership, exact persisted
imports/cache sequences and rollback were independently revalidated offline.
Node execution hash matches; CLI post-run verification was not reached. The
last-original SQL fault, full recovery and cold full-graph cycle were not reached.
Evidence: `evidence/reviews/AR2-wallet-flow/history-range-native-r14.json`.

R14's post-failure-only diagnostics retain recipient state; no provider qualifies
for missing-location diagnostics because all location ACKs arrived. R13 remains
an unexplained separate failure. The diagnostic helper and wrapper had independent
FINAL ACCEPT plus four backend/nine frontend tests, build/Clippy/fmt:
`evidence/reviews/AR2-wallet-flow/history-range-r14-wrapper-review.json`.
Next reproduce retrieval scheduling and the separate transient sender-observation
lead. Keep deadlines, leases, allowances and the full 67/22/3 scope unchanged.

## 2026-09-13 — Full130 R13 stops during the seventh publication batch

The ordinary sender completes 96 originals in six batches. The next 16 originals
(97–112) have 160 data receipts, 160 index receipts and 1580/1600 location ACKs,
but do not complete publication within the unchanged 600-second batch deadline.
Originals 107 and 111 both lack all ten location ACKs at index position 9 on the
same provider. That provider is connected in the failure snapshot. This does not
yet identify a remote write, response, local commit or scheduling failure.

All 956 inputs and five signed artifacts remain unchanged over 2375.4 seconds;
cleanup leaves no processes or profiles. The node's reported hash matches the
bundle. The post-run CLI hash check was not reached and remains null, not proof
of a different executed binary. Independent whole-range signature verification,
copy loss, first SQL rollback and recipient recovery were not reached. Native20
R10 remains the accepted shorter flow. Evidence:
`evidence/reviews/AR2-wallet-flow/history-range-native-r13.json`.

Next reproduce competing location/history publication and capture bounded sender
and provider diagnostics after a terminal failure. Do not rerun Full130 by merely
extending deadlines or retention. Full V1 and all 67/22/3 requirements remain open.

## 2026-09-13 — Native20 R10 passes exact recovery and cold completion

All20 ordinary paid originals publish with60 independently verified signatures,
200 data/200 index receipts and2000 location ACKs. Actual180+180 copy loss,
missing-trust refusal, both exact-message SQL faults/rollback, partial1..19,
full1..20 and cold verified completion pass under the unchanged gates. The first
SQL fault takes21 reads; partial19/20 with the last SQL fault takes59 reads.
Cold completion makes no new network reads or imports. The earliest original
is still live. All956 inputs and5 signed binaries stay unchanged over708.4s;
teardown leaves no processes/profiles. Independent offline revalidation confirms
the signed graph, exact persisted sequences and first rollback.
Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r10.json`.

Full130 R13 subsequently failed during publication (see above). Its wrappers received independent
FINAL ACCEPT after binding actual packaged node/CLI paths and hashes to the
verified app. The unchanged130-original scenario reuses post-stop diagnostics,
strict binary/input identity and cleanup audit. Review: `evidence/reviews/AR2-wallet-flow/history-range-r13-wrapper-review.json`. This20-original success does not close130, independent
testnet, three-platform release or full67/22/3 V1.

## 2026-09-13 — Exhausted outbound quota no longer holds cached graph progress

After the unchanged global24/60s limiter rejects a request, an admitted graph
reserves its current durable claim and yields within the existing16-reference
pass. Local ciphertext still needs exact current-root membership and ordinary
atomic MLS import. Initial roots/unadmitted ancestry and per-peer-only limits
keep waiting; quota creates no missing-copy or delivery facts. Actual SQL failure
preserves the selected claim. Five tests received independent FINAL ACCEPT;
the old implementation failed3 regressions and passed2 control cases.

All118 targeted backend tests (56 worker/58 paid wire/4 shared limiter),51 frontend
tests, Node all-targets Clippy and formatting pass on397 unchanged inputs.
Evidence: `evidence/reviews/AR2-wallet-flow/history-admission-yield-checks.json`.
Fresh ordinary release packaging and strict signature/default-feature checks pass
in133.0s on922 unchanged inputs. Evidence: `evidence/reviews/AR2-wallet-flow/history-admission-yield-release-build.json`.
Native20 R10 subsequently passed on these exact bundled binaries (see above).
Full130 R13 subsequently failed during publication; full67/22/3 remains open.

## 2026-09-13 — Native20 R9 captures the stalled recipient after cleanup

All 20 ordinary originals publish with 60 independently verified signatures,
200 data/200 index receipts and 2000 location ACKs. After actual 180+180 copy loss
and missing-trust refusal, the unchanged first-original SQL gate times out at
120s. Final counters: 48 reads, 25 bulk, 12 path requests, 3 multi-body paths,
7 deferrals, 0 imports and 0 storage errors. All 955 inputs and five signed
binaries remain unchanged over 552.6s; teardown is clean.

The independently reviewed wrapper captures persisted state after normal cleanup
stops the recipient, before deleting its profile. Original 1 is eighth in the
signed publication order. Originals 1 and 2 are already prefetched; the exact
snapshot contains 10 prefetched and 7 deferred originals, no imports. Originals
6, 11 and 16 are absent from both local caches. This is post-stop state, not an
exact timeout snapshot; cached ciphertext alone does not authorize history import.
Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r9.json`.

Next isolate redundant data discovery and metadata traversal under the existing
24-total/12-per-peer requests per 60s admission window. The observed plateau is
consistent with that bound but does not attribute individual delayed requests.
Any scheduling change needs a reproducing test and independent critic acceptance
before implementation. Native20 completion, Full130 R13 and full V1 remain open.

## 2026-09-13 — Native20 R8 batches publish, partial recovery misses deadline

All20 ordinary paid originals publish with60 independently verified signatures,
200 data/200 index receipts and2000 location ACKs. The signed graph has16 leaves,
including groups of2 and3, with original1 first. Actual180+180 copy loss,
missing-trust refusal and the first SQL failure/exact rollback pass (11 reads,
4 bulk requests,2 multi-body paths). The unchanged200s partial19/20 gate then
fails: final77 reads/37 bulk/30 path requests/6 multi-body paths/13 provider
preferences/5 received/16 cumulative deferrals/0 storage errors. Those counters
are not a final exact inbox or an attribution of individual delayed requests.
All955 inputs and5 signed binaries stay unchanged over754.3s; teardown is clean.
Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r8.json`.
Next collect a persisted failure snapshot after stopping the recipient, before
its test profile is removed, preserving every gate and normal-loop behavior.
Full20/cold20 and Full130 R13 remain pending; V1 is not accepted.

## 2026-09-13 — Ordinary macOS release includes sender batching

Fresh ordinary app packaging and strict ad-hoc signature verification pass in
149.3s on921 unchanged inputs. The default feature graph excludes the E2E driver.
The release includes Core coalesced leaves and ordinary Node paid-ready selection.
Evidence: `evidence/reviews/AR2-wallet-flow/history-batch-scheduling-release-build.json`.
Unchanged Native20 R8 follows; Full130 R13 and full V1 remain open. This packaging
check does not claim a fresh GUI verification or notarization.

## 2026-09-13 — Ordinary sender batches pass targeted checks

The ordinary post-index stage now collects active paid-ready originals for one
500ms work interval, then signs up to12 references in one leaf. Current execution
authorization and each original's paid receipts are rechecked; optional work
yields at the existing20ms budget. Disposable membership hints change only after
Core commits. The real maintenance caller passes its held active jobs into this
stage before the existing paid page/ACK/pointer publication. All71 related Node
tests,2 Core shared-fixture regressions,51 frontend tests, Node all-targets Clippy
and formatting pass. The five new scheduler tests were independently accepted
before implementation and include permitted/revoked agents, another conversation,
13 ready jobs and real SQL rollback. All464 inventoried inputs stay unchanged.
Evidence: `evidence/reviews/AR2-wallet-flow/history-batch-scheduling-checks.json`.
The preceding paid-route checks remain historical evidence in
`evidence/reviews/AR2-wallet-flow/history-paid-batch-checks.json`.
Fresh ordinary release packaging and unchanged Native20/Full130 gates follow;
this result does not establish network throughput or full V1 acceptance.

## 2026-09-13 — Coalesced Core history batches pass the resumed checks

The independently reviewed Core API groups 1–12 genuine originals into immutable
leaves, preserving existing wire formats, original leases, cold exact retries and
atomic membership/head updates. The interrupted handoff run is retained as
incomplete evidence. A fresh run completed all96 related history tests in1282.3s;
Core all-targets Clippy, formatting and51 targeted frontend tests pass. All174
inventoried Core/crypto/store inputs remained unchanged. Evidence:
`evidence/reviews/AR2-wallet-flow/history-coalesced-core-checks.json`.
Next implement ordinary Node paid-ready selection and actual multi-original leaf
publication, followed by fresh builds and unchanged Native20/Full130 gates.
This Core result does not establish network throughput or full V1 acceptance.

## 2026-09-13 — Native20 R7 misses first SQL deadline despite holder exploration

All20 paid originals publish with60 independently verified signatures,1 actual authority renewal,200 data/200 index receipts and2000 location ACKs. Actual180+180 loss and missing-trust refusal pass. First exact-message SQL-fault gate times out at the unchanged120s: last observer48 reads/18 bulk, final failure snapshot49 reads/19 bulk/24 path requests/6 multi-body paths/1 anchor/5 provider preferences/4 cumulative deferrals/0 received/0 storage errors. Signed publication order starts8,2,5,9,4,14,15,1: original1 is eighth. All949 inputs and5 signed binaries unchanged over689.5s; cleanup/temp0. Partial19/full20/cold20 are not reached. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r7.json`. Native20 R6 remains a historical pass; holder ordering alone is insufficient. Full130 R13 has not started; investigate repeated data reads and paid graph traversal cost without weakening gates.

## 2026-09-13 — Explore unqueried holders within authenticated route priority

Real paid locations retain all candidates but prefer a not-yet-queued prefix within the same route class. Authenticated connections still precede signed locator routes and discovery; the existing bounded ledger and every same-holder fallback remain. Three independently accepted tests reproduce old-first ordering, actual copy loss/fallback and disconnected-route priority, then pass with exact three MLS originals. All109 related node tests (51 worker,58 wire),51 frontend tests, Clippy, formatting and Python checks pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-holder-exploration-checks.json`. Ordinary macOS packaging, strict ad-hoc signature and default features without E2E driver pass in124.5s on915 unchanged inputs (`history-holder-exploration-release-build.json`). Unchanged native20 R7/full130 R13 follow; no throughput or V1 completion claim.

## 2026-09-13 — Full130 R12 fails the unchanged first SQL deadline

All130 ordinary paid originals publish across9 batches with390 independently verified signatures,4 real authority renewals,1300 data/1300 index receipts and13000 location ACKs. Actual loss removes1170 data+1170 index copies; missing-trust refusal passes. The first exact-message SQL-fault gate then times out at120s:48 reads,19 bulk,22 path requests,7 multi-body paths,2 anchor reuses,3 provider preferences,0 received/0 storage errors,2 cumulative deferrals. This is before the earliest original expiry, not a lease timeout. Partial129/full130/cold130 are not reached. All944 inputs remain unchanged over2798.6s; cleanup errors/temp profiles0. Evidence: `evidence/reviews/AR2-wallet-flow/history-range-native-r12.json`. Inspect actual reference order and holder search cost before another native run. Native20 R6 remains green; full130 and V1 remain open.

## 2026-09-13 — Native20 R6 passes every unchanged gate

All20 real paid originals recover with the sender absent after180 data+180 index copies are removed. Sixty independent signatures,200 data/200 index receipts,2000 location ACKs, missing-trust refusal, both exact-message SQL faults, unchanged rollback, partial19/20, full20/20 and cold dedup pass. The original positive bulk/anchor/provider/path gates remain satisfied. First SQL observation:42 reads/20 bulk; partial19:71 reads/32 bulk/6 multi-body paths/18 provider preferences. Cold completion:0 reads,0 reimports. All948 inputs and5 signed ordinary binaries remain unchanged over782.4s; teardown clean. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r6.json`. Full130 R12 is next; no GUI, autonomous R10 or full V1 acceptance is claimed.

## 2026-09-13 — Bounded holder-prefix reads retain earlier paid ciphertext

Ordinary history Work performs one bounded initial prefix read per holder, then exact bulk reads. Actual admission/stream blocks preserve the opportunity; the128-holder ledger never evicts into repeated prefixes. Every paid object and the selected descriptor are checked before Core cache admission. A truncated prefix yields a same-holder exact retry only after successful SQL admission; empty-complete/error fallback remains finite. Eleven new independently accepted verifier/policy/runtime tests pass alongside all106 related node and51 frontend tests; node all-targets Clippy, formatting and Python checks pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefix-policy-checks.json`. Fresh ordinary macOS packaging, strict ad-hoc signature and default features without the E2E driver pass in122.8s on914 unchanged inputs (`history-prefix-policy-release-build.json`). Unchanged native20 R6 follows. Full130 and V1 remain open.

## 2026-09-13 — Native20 R5 provider selection passes, partial deadline fails

All20 funded originals publish with60 independently verified signatures,200 data/200 index receipts and2000 location ACKs, followed by180+180 actual copy loss. Missing-trust refusal and first exact-message SQL rollback pass (23 reads,4 accepted paths,1 provider preference). The unchanged200-second partial19/20 plus second SQL-fault gate fails. Final counters:92 reads,41 bulk,34 path requests,6 multi-body paths,2 anchor reuses,15 provider preferences,14 received operations,0 storage errors; these counters are not an exact final inbox oracle. All944 inputs and5 signed binaries unchanged over782.5s; teardown clean. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r5.json`. Provider ordering is working, but native20 throughput is still not accepted. Investigate remaining paid data/page requests before full130; no deadline/lease/quota changes.

## 2026-09-13 — Cached anchor index reads prefer the verified provider

Ordinary index reads of exact cached leaf anchors reuse existing verified provider preferences while retaining fresh signed reads, full paid response validation and every declared fallback. Other originals in the same legacy flat leaf cannot borrow the anchor hint. Two independently accepted tests include actual queued TCP/Noise and fallback after a real unavailable response. All95 related node tests (51 worker,44 wire),51 frontend tests, node all-targets Clippy, formatting and Python compilation pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-reference-route-checks.json`. Fresh ordinary release packaging, strict ad-hoc signature and default features without E2E driver pass in126.6s on910 unchanged inputs (`history-reference-route-release-build.json`). Unchanged native20 R5 follows; full130 and V1 remain open.

## 2026-09-13 — Native20 R4 completes recovery but fails the provider-preference gate

All20 real paid originals recover after180 data+180 index copies are removed with the sender absent. Sixty independent signatures,200 data/200 index receipts/2000 location ACKs, missing-trust refusal, both message SQL faults, exact rollback, partial19/20, full20/20 and cold completion pass before the final diagnostic assertion. First SQL fault takes23 reads with4 accepted paths; partial19/20 takes69 reads with6 paths. The entire probe remains FAILED because historyRoutePreferences stays0: full paths removed the repeated metadata reads where this hint previously applied. Existing gate was not removed. All943 inputs and5 signed binaries stay unchanged over747s; clean teardown. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r4.json`. Next test reuse of the verified provider for ordinary index reads of cached anchor leaves, preserving full paid verification and fallback candidates. Full130 and V1 remain open.

## 2026-09-13 — Ordinary macOS release includes complete path integration

Fresh ordinary app packaging, strict ad-hoc signature verification and default features without the E2E driver pass in127.9 seconds on909 unchanged inputs. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-release-build.json`. This release includes store/wire/verifier/current-root cache and queued path compatibility. Native20 R4 is next; no GUI verification of this release, notarization, full130 or V1 acceptance is claimed.

## 2026-09-13 — Ordinary path request selection and bounded compatibility

The ordinary queue now signs Path/count32 reads, retries a rejected or failed stream once with a fresh Single/count1 capability, and resets path mode on the next page/provider. Explicit absence/capacity/storage codes survive the pending validator; previous head legacy fallback remains finite. Actual request and accepted-multibody counters extend the exact native schema to15 fields; all previous native20 gates remain. Six independently accepted tests include actual Noise, dropped channels and successful singleton-to-next-branch mode reset. All93 related node tests (49 worker,44 wire),51 targeted frontend tests, node all-targets Clippy, formatting and Python compilation pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-request-checks.json`. Fresh release packaging and native20 R4 are next; full130 and V1 remain open.

## 2026-09-13 — Ordinary paid path consumption and complete metadata caching

The ordinary receiver validates every typed path body and caches complete current-root proofs. Optional root metadata commits only after live-prefix admission and before any durable reference claim. Admitted branches use the existing prefix and actual leaf positions; SQL faults preserve pending data and cold retry. Seven new tests passed independent review after strengthening the root-fault all-state oracle. All87 related node tests (43 worker,44 wire),51 targeted frontend tests, node all-targets Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-consumption-checks.json`. Ordinary queued path requests/legacy compatibility remain next. No new release or native20/full130 result; V1 remains open.

## 2026-09-13 — Whole paid path recipient verifier

`IndexHistoryPath::verify_read` verifies one actual paid provider and the complete signed request/path before returning any Core pages. Wrong late links/signatures or paid proof, count/whole-proof bytes, target/index/range/read/original expiry reject the entire response. Shared typed body/link validation is reused. Three new independently accepted tests plus the extended genuine four-original test cover full4/prefix3 and legacy leaf head probes. All38 related store tests,51 targeted frontend tests, postage all-targets Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-verification-checks.json`. Ordinary node consumption/current-root caching and queued path/legacy compatibility remain next; release bundle still predates the path modules. Native20/full130 and V1 remain open.

## 2026-09-13 — Typed history path server transport

The existing paid custody protocol serves bounded `index_history_path_read`/`history_path` replies using the same compact paid proof codec, connection/relay policy,16-request admission window,1MiB frame and four-stream limit. Four independently accepted real TCP/Noise tests include four originals/two branch levels, cold historical reads, single-page compatibility, full-proof byte limits, partial availability and actual SQL rollback. All42 related wire tests,51 targeted frontend tests, node all-targets Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-wire-checks.json`. Ordinary receiver validation and current-root cache integration remain next; automatic requests and release bundle still use the earlier single-page path. Native20/full130 and V1 remain open.

## 2026-09-13 — Shared-proof history path store export

Store export now returns the exact requested body plus bounded same-anchor signed descendants with one portable paid proof. Count1–32 and full signed bytes, original leases,32KiB metadata quota, privacy/trust and SQL clock commit remain unchanged. Seven tests were independently accepted after adding a genuine four-original two-branch-level case. All35 targeted history store tests,84 frontend tests (an inadvertently broader invocation), all-targets postage Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-path-store-checks.json`; contract: `spec/custody-history-path-v1.md`. Typed transport and ordinary-recipient integration are next; the current release app predates this store extension. Native20/full130 and V1 remain open.

## 2026-09-13 — Release20 R3 fails the unchanged partial deadline

R3 publishes all20 originals with60 independently verified signatures and removes180 data plus180 index copies. First exact-original SQL failure and unchanged rollback pass (44 reads,3 anchor reuses,3 provider preferences). The unchanged200-second partial recovery gate then fails at95 reads,37 bulk requests,5 anchor reuses,13 provider preferences,6 successful import operations and0 storage errors. Cumulative deferrals14 are not a pending inbox count. All933 inputs and5 signed ordinary-release binaries stay unchanged over822.2 seconds; cleanup errors/temp profiles0. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r3.json`. The ordinary release build passed separately in120.9 seconds on899 unchanged inputs (`history-route-release-build.json`). Provider reuse alone has not established recovery throughput; bounded shared-proof history paths are the next tests-first candidate. Full20, full130 and V1 remain open.

## 2026-09-13 — Prefer the actual paid history provider across graph pages

Runtime keeps at most128 small positive hints keyed by exact private index/epoch/original descriptor and finite lease. After current-head graph admission, real root/branch/leaf responders can be preferred for later pages sharing that anchor; all declared fallback candidates remain. Three independently accepted three-peer paid TCP/Noise tests pass, including actual unavailable-branch fallback and provider relearning. All77 targeted backend tests and51 frontend tests pass; final Clippy/fmt/Python pass after boxing the larger internal page output. Evidence: `evidence/reviews/AR2-wallet-flow/history-route-checks.json`; contract: `spec/custody-history-routes-v1.md`. Ordinary release refresh passed; unchanged20-original R3 failed the partial deadline (see entry above). Full130 and full V1 remain open.

## 2026-09-13 — Release20 probe R2 reaches first SQL rollback, misses partial deadline

R2 publishes all20 originals with60 independently verified signatures and removes180 data plus180 index copies. The first exact-original SQL fault and unchanged rollback pass (46 reads,2 anchor reuses). After restart the unchanged200-second partial gate fails with96 reads,29 bulk requests,3 anchor reuses,13 successful import operations and0 storage errors. The9 deferred counter is cumulative, not a remaining-message count. All930 inputs and5 signed ordinary-release binaries remain unchanged over819.2 seconds; cleanup errors/temp profiles0. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r2.json`. The signed current graph has39 pages sharing20 paid anchors; successful index-provider hints are not retained between graph pages, so repeated candidate discovery remains a concrete optimization boundary. No exact request attribution,20-original completion,full130 or full V1 acceptance is claimed.

## 2026-09-13 — Reuse verified leaf anchors through the ordinary paid importer

The current history Work retains one exact paid leaf anchor and its responding provider. A fresh target-bound Core read rechecks current root and original expiry, then the shared ordinary index plan proceeds directly to locations. Other provider candidates remain available. Four independently accepted tests include genuine paid TCP/Noise and MLS import, and reproduce a valid-other-provider capability bug before its explicit target check. All74 targeted backend tests,51 frontend tests, node all-targets Clippy, formatting and Python compilation pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-anchor-checks.json`; contract: `spec/custody-history-anchor-v1.md`. Ordinary macOS release packaging, strict signature and default features without the E2E driver also pass in118.9 seconds on896 unchanged inputs (`history-anchor-release-build.json`). The unchanged20-original probe is next. Full130, other V1 cards and platform acceptance remain open.

## 2026-09-13 — Actual20-original release diagnostic exposes retrieval delay

Probe R1 publishes20 ordinary CLI originals, independently verifies60 signatures and removes180 data plus180 index copies. All926 inputs and5 release bundle binaries remain unchanged over658.8 seconds; clean teardown. The recipient exhausts the unchanged120-second first-SQL-fault wait with48 reads,13 bulk requests,3 genuine MLS deferrals and0 imports/storage errors. Sequence1 is ninth in the genuine publication graph. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-probe-native-r1.json`. This is a failed diagnostic, not full130 or V1 acceptance. Next remove duplicate paid index discovery after an already verified leaf response, retaining exact reference/peer/current-root checks.

## 2026-09-13 — Ordinary release includes verified bulk recovery changes

Fresh ordinary macOS app packaging, strict ad-hoc signature verification and default features without the E2E driver pass in 127 seconds on 891 unchanged inputs. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-release-build.json`. The separately inspected R3 native GUI used isolated debug/e2e secrets; no release GUI or notarization is claimed. Full 130-original recovery and remaining V1 acceptance remain open.

## 2026-09-13 — Native paid prefetch and GUI recovery GREEN R3

Fresh signed debug/e2e app passes actual typed bulk admission SQL rollback, then6 independent signatures,20 original data/20 index receipts/200 location ACKs,18+18 initial copy loss, both message SQL faults, genuine MLS deferral, deletion of the final remote future copy, and exact partial/cold2/2 recovery with sender absent. Five native GUI phases were personally inspected: unknown/0-of-2/1-of-2/2-of-2 states, selected dialog and refreshed draft agree. All923 inputs and5 binaries remain unchanged over281.6s; cleanup errors/temp directories0. Evidence: `evidence/reviews/AR2-wallet-flow/history-prefetch-native-r3.json`. Ordinary release refresh follows. This validates the named2-original slice; full130 throughput, AR4, independent testnet and67/22/3 acceptance remain open.

## 2026-09-13 — Native paid prefetch assertions pass; bounded SQL retry and hidden capture corrected

Native prefetch R2 passes the actual typed-read/admission-SQL-failure gate, then unchanged6-signature paid loss, both message faults, real durable MLS gap, final remote-copy loss and exact cold2/2 recovery. All922 inputs and5 binaries remain unchanged over302.8s; cleanup is clean. Visual review found stale sidebar highlight caused by hidden WKWebView CSS transitions frozen at time0, not wrong selected DOM; a focused real native regression now passes after finishing finite transitions only at screenshot capture. Separately, an independently accepted cached-original SQL regression reproduced Work starvation; failed local imports now preserve their retry and yield within the same bounded visited pass. All38 related backend tests,51 frontend tests, Clippy/fmt pass. Evidence: `prefetch-starvation-checks.json`, `native-selection-checks.json`, `history-prefetch-native-r2.json` under `evidence/reviews/AR2-wallet-flow/`. Fresh combined native run is next. Full130, ordinary release refresh and full V1 remain open.

## 2026-09-13 — Ordinary paid bulk fetch and durable prefetch admission

Connected typed bulk fetch to the normal recipient after existing paid index and holder validation. Whole compact proofs and exact pending descriptor are verified before bounded ciphertext admission; actual SQL failure returns no deliverable output. Compatibility makes one legacy attempt at the same holder, then resets bulk mode for the next holder. New counters observe actual queued bulk requests and actual cache storage failures. Five independently accepted backend tests and the reviewed additional native SQL-admission gate were written before implementation. All68 targeted backend tests,51 frontend tests, node all-targets Clippy, formatting and Python compilation pass. Evidence: `evidence/reviews/AR2-wallet-flow/obligation-wiring-checks.json`. Fresh hidden native debug/e2e app build and run are next; full130, ordinary release refresh and V1 acceptance remain open.

## 2026-09-13 — Ordinary worker consumes prefetched ciphertext

The ordinary graph worker now uses fresh current-root prefetch hits through the common MLS import, actual-gap and SQL retry paths. A new test exposed an active reference left behind after deferred retry had already imported it; the worker now verifies current-root import progress before any repeat fetch, without increasing received counts. Four independently accepted tests recover20 exact originals through real gaps and cold restarts with zero index/data requests at the supplied-paid-response boundary, preserve ciphertext through a real SQL failure, send a typed index request on a true miss and reject obsolete Work roots. All36 worker regressions,51 frontend tests, node all-targets Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/prefetch-worker-checks.json`. Next implement whole paid bulk reply verification/admission and ordinary network request selection; no native130 or V1 completion is claimed.

## 2026-09-13 — Durable ciphertext prefetch Core contract

Added a bounded optional ciphertext cache (128 originals/4 MiB) for already-paid bulk responses. Whole recipient pages are validated before admission; no messages, MLS or delivery progress change. Fresh exact-root reads select original bytes after cold opening and reproduce their deterministic descriptor without extending leases. Successful history imports and real-gap transfers to deferred storage remove ciphertext atomically; SQL failure or full deferred capacity preserves it. Seven tests were independently accepted after addressing exact selection, full deferred capacity and read-expiry gaps. All92 related Core history tests,32 worker regressions,51 frontend tests, Core/Node all-targets Clippy and formatting pass. Evidence: `evidence/reviews/AR2-wallet-flow/prefetch-core-checks.json`; contract: `spec/custody-prefetch-v1.md`. Next connect cache hits to ordinary graph scheduling, then verify and retain whole paid bulk replies. Full native130 and full V1 gates remain open; the ordinary app bundle still predates this extension.

## 2026-09-13 — Paid bulk custody replies over the actual transport

Added `obligation_read`/`obligation_page` to the ordinary paid custody protocol using the existing bounded portable-page store API and shared copy codec. The actual TCP/Noise fixture recovered six exact paid obligations in three two-proof responses and independently verified each proof after cold store continuation. Four new tests and all23 related wire regressions pass, together with51 frontend tests, node all-targets Clippy and formatting. Capability/privacy, expired empty continuation, original leases, full proof-byte limits, SQL/clock rollback, relay policy and the existing sixteen-request server window are preserved. Evidence: `evidence/reviews/AR2-wallet-flow/obligation-wire-checks.json`. Next connect verified future ciphertext from these responses to ordinary recipient history imports; server support alone is not full130 recovery. Last ordinary release bundle predates this wire extension and will need refresh after receiver integration.

## 2026-09-13 — Current ordinary macOS release app refreshed

Built `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app` with default features after durable graph metadata caching and the cold-observation oracle fix. Packaging, strict signature verification and absence of the E2E driver pass; all877 inventoried inputs remained unchanged during134.9 seconds. Bundle is locally ad-hoc signed, not notarized. Evidence: `evidence/reviews/AR2-wallet-flow/history-cache-release-build.json`. Real paid GUI recovery was separately verified in R5 with debug/e2e and isolated file secrets; this release bundle was not launched against system Keychain. Full V1 remains open.

Next transport work: reduce initial paid requests while preserving original leases and the24/60-second reader allowance. Durable metadata cache removes repeated/cold graph requests but not initial root/branch/leaf requests. The verified portable obligation page store API exists; it still needs typed wire/ordinary-recipient use, and an authenticated leaf's paid index anchor can potentially avoid a duplicate index read. Count actual typed requests before rerunning full130. Do not weaken the corrected completed-observation oracle or the130/390-signature/replica/loss/expiry gates.

## 2026-09-13 — Cold full130 oracle observes completed reads

Replaced both claim-only native cold-range checks with a conversation-scoped, bounded SQL observer of actual committed Core leaf observations. The gate requires every new verified reference1..130 for the exact current signed root/index, followed by idle work and unchanged message/MLS/import state. An old completed pass, 130 scheduling claims, issue-only or import-only writes and failed SQL transactions cannot certify a new cold traversal. Six independently reviewed tests include a lone final leaf (only130), actual import-only update and a cold scan after all130 originals were really imported. 32 targeted worker regressions and51 frontend tests pass; node all-targets Clippy, formatting and Python compilation pass. Evidence: `evidence/reviews/AR2-wallet-flow/history-completion-checks.json`. No daemon behavior changed in this step; full paid native130 throughput and V1 remain open.

## 2026-09-13 — Native GUI recovery with durable page metadata cache (R5)

Fresh signed debug/e2e Tauri build passed the real paid two-original GUI recovery scenario in 413.1 seconds: six independently verified signatures, original leases, actual replica loss, both SQL import faults, durable predecessor-gap ciphertext and deletion of its final remote copy, exact cold 2/2 completion and draft retention after a real refresh click. All five screenshots were inspected. All 907 inputs and five signed binaries remained unchanged; cleanup left no processes/temp profiles reported by the harness. Cold recipient verification recorded zero read requests, zero failures and zero reimports (R4 recorded four reads). Evidence: `evidence/reviews/AR2-wallet-flow/history-native-gui-r5.json`. This is the two-original native gate; full130 throughput/completed traversal, ordinary release refresh, other required V1 cards and cross-platform gates remain open.

## Durable graph metadata — 2026-09-13

[Current-root page cache](spec/custody-history-page-cache-v1.md) is implemented in
Core and the ordinary graph worker. It retains at most512 bodies/1MiB and rechecks
full commitments, signatures, actual epoch and expiry on cold reads. Physical
byte-pressure eviction preserves imports and the current path. Independent
[tests-first review](evidence/reviews/AR2-wallet-flow/history-page-cache-test-review.json)
and [111 backend/51 frontend regressions, Clippy and formatting](evidence/reviews/AR2-wallet-flow/history-page-cache-checks.json)
pass. The ordinary worker authenticates130 references after a cold Runtime restart
with zero typed graph-page read actions at the supplied-response boundary.
The old in-flight-response test now explicitly evicts optional metadata to retain
its real network-miss scenario; its initial failure is preserved. Native paid GUI
R5 is next for this source. Full130 wire recovery remains open: initial graph/data
requests still need reduction and its cold oracle must observe completed traversal.
Full67/22/3 remains open.

## Portable custody page export — 2026-09-13

[Recipient-authorized portable pages](spec/custody-obligation-page-v1.md) preserve
full original paid proofs and copied origins under the existing signed read bounds.
The shared compact codec retains the previous copy wire schema. Independent test
review and [66 backend /51 frontend checks, Clippy and formatting](evidence/reviews/AR2-wallet-flow/portable-page-checks.json)
pass, including 134 actual originals with cold continuation and independent
verification. The byte bound fits two complete proofs, requiring 67 store reads.
This API is not yet wired into automatic recipient transport; full130 network
feasibility is not established. Native paid GUI R2/R3 synchronization failures are retained.
[R4 passes](evidence/reviews/AR2-wallet-flow/history-native-gui-r4.json) in398.9seconds
on903 unchanged inputs with all five native recovery states visually inspected,
six independently verified signatures and clean teardown. It uses the real
recipient worker and refresh button; full130 remains separate.
Full67/22/3 remains open.

## Full130 R11 — request-count bound, 2026-09-13

[Release R11](evidence/reviews/AR2-wallet-flow/history-range-native-r11.json) was
interrupted with SIGINT after112 stored originals and four real authority renewals.
Actual exit-2 in2395.8seconds;897 unchanged inputs, clean teardown. This is a
failed/incomplete run. Recovery and the independent390-signature oracle were not
reached. The prior48-original mixed-head failure did not recur.
An independent code audit found a minimum649 requests for initial130 recovery:
130 index reads,130 locations,130 ciphertext reads,130 singleton leaves,128 branches
and one root. The shared receiver allowance is24 requests/minute. Cold imports
still require graph leaves before checking saved progress. Even allowing five
fresh process bursts and the cold oracle's weaker claim-based accounting, the
conservative remaining gate floor was28minutes versus at most22.9minutes of the
original lease. The ideal fully read recovery+cold traversal needs907 requests
and at least33minutes; that stronger number is not an unconditional current-oracle
minimum. Next: reduce actual typed network reads and verify completed authenticated
traversal with unchanged leases/admission bounds. Another full130 run before that
change would be predictably infeasible. The paid native two-original GUI recovery gate now passes separately inR4;
its success does not establish this full130 gate.
Full67/22/3 remains open.

## Pending ciphertext and MLS ordering — 2026-09-13

[Bounded pending ciphertext](spec/custody-history-deferred-v1.md) is implemented
in Core and the ordinary recipient worker. Only authenticated MLS gaps after
actual paid holder/index checks enter a128-entry/4MiB encrypted cache. Cold local
retry obtains fresh authority, preserves exact root/epoch/expiry checks and
commits removal with the message, MLS and import rows. Current cached references
skip redundant network fetches; at most4 local attempts run per Work tick.
Normal network completion also removes its exact cached original atomically.
[Independent tests-first review](evidence/reviews/AR2-wallet-flow/history-deferred-test-review.json)
and [104 backend/51 frontend tests, Clippy and formatting](evidence/reviews/AR2-wallet-flow/history-deferred-checks.json)
pass. This includes actual Core130 recovery with reversed graph order, unrelated
envelope preparation sequence, byte/count bounds, SQL rollback and cold retries.
The first debug130 run was interrupted after profiling repeated proof serialization;
its fixture failure and lack of acceptance remain recorded. The completed release
test and final80-test Core cluster passed with unchanged assertions and bounds.
[Genuine native deferred-copy-loss R1](evidence/reviews/AR2-wallet-flow/history-deferred-native-r1.json)
passes in272.8seconds on895 unchanged inputs: six independently verified signatures,
actual18+18 copy loss and deletion of the last remote copy of a cached original,
both SQL faults, cold pending bytes and exact automatic recovery without the sender.
Teardown is clean. Full130 network recovery and full67/22/3 remain open. The existing desktop bundle
predates this pending-ciphertext change and must be rebuilt after native checks.

## Custody admissions across renewal — 2026-09-13

[Per-receipt admission proofs](evidence/reviews/AR2-wallet-flow/SPLIT_CUSTODY_ADMISSION.md)
address the genuine mixed-head data/index conflict observed in native130 R10.
Sender retention, index location storage/export and the recipient holder checker
preserve each signed receipt's actual proof, with existing byte limits and atomic
rollback. Independent tests-first and implementation review accepted the tested
paths; [116 backend /51 frontend checks, Clippy and formatting pass](evidence/reviews/AR2-wallet-flow/split-admission-checks.json).
[Focused native renewal/cold recovery](evidence/reviews/AR2-wallet-flow/split-admission-native-r1.json)
passes on889 unchanged inputs in300.1seconds: one genuine successor, cold sender
with mixed-head receipts,6 independently verified signatures,20 data/20 index
receipts/200 location ACKs, actual18+18 copy loss, trust refusal and both SQL
faults followed by sender-absent exact cold recovery. Clean teardown.
[Mixed-head inspection](spec/custody-manifest-inspection-v1.md) now passes its
independent test/implementation review and [18 backend/51 frontend checks,
Clippy and formatting](evidence/reviews/AR2-wallet-flow/split-inspection-checks.json).
[Expanded native R3](evidence/reviews/AR2-wallet-flow/split-admission-native-r3.json)
passes in216.6seconds on890 unchanged inputs: actual mixed-head holder inspections
in both directions, exact cold manifests/observations and the unchanged two-original
copy-loss/trust/SQL/recipient recovery flow. Clean teardown. R2's test hex-decoder
failure is preserved separately; it was not a production failure or a passing run.
[Same-head native regression](evidence/reviews/AR2-wallet-flow/split-inspection-same-head-r1.json)
also passes in105.4seconds on890 unchanged inputs, including captured request
proofs, shared queue/stream limits, independent response attestation oracle,
both storage faults, cold manifests and automatic retrieval after source loss.
[Ordinary macOS release bundle](evidence/reviews/AR2-wallet-flow/split-admission-release-build.json)
rebuilt on890 unchanged inputs in88.8seconds; strict ad-hoc signature verification
passes, the default feature graph excludes the automation driver, and all three
unsigned runtime binaries match the native-tested artifacts. This is a local,
unnotarized bundle; native GUI paid recovery and full67/22/3 remain separate gates.
Full130 recovery remains failed. A read-only audit of the genuine R9 graph finds
publication order1,3,6,7,8,9,12,4,5,16,2,... despite chronological envelope
preparation in that run; the existing next-reference retry cannot solve arbitrary
MLS permutations. Resolve that boundary before another expensive full130 run.

## Core predecessor retry — 2026-09-13

[Local retry scheduling](spec/custody-history-retry-v1.md) now preserves one root-bound retry separately from the fair cursor. Only the exact reserved original's committed import atomically resumes the next declared reference; other imports/replays cannot acknowledge it. Persistent faults preserve bounded fair130 attempts, signed expiry can retire the retry without delivery, and legacy cursor/new MLS epoch behavior is covered. Independent test and implementation ACCEPT;[55 backend/51 frontend, Core/node Clippy/fmt pass](evidence/reviews/AR2-wallet-flow/history-page-retry-checks.json). The ordinary Runtime now reserves only actual local import failures after paid index/holder checks;[20 worker/51 frontend regressions](evidence/reviews/AR2-wallet-flow/history-worker-retry-checks.json) and Clippy/fmt pass after independent test/caller review. [Genuine native2](evidence/reviews/AR2-wallet-flow/history-worker-retry-native-r1.json) now passes on881 unchanged inputs in268.2seconds:6 genuine signatures,18+18 actual copy loss, both SQL faults, real MLS gap, exact recovery/cold clear, clean teardown. [Native130 R10](evidence/reviews/AR2-wallet-flow/history-range-native-r10.json) failed the fourth sender batch deadline after48 stored originals: sequences49/51/53 had ten data receipts but zero index receipts. Actual exit1 in1522.6seconds,883 unchanged inputs, clean teardown. Recovery was not reached. Graph publication order can differ from MLS generation order; this retry does not establish fast recovery for every permutation. This does not supersede the failed nativeR9 gate or establish full67/22/3 readiness.

## Recovery diagnostics — targeted checks, 2026-09-13

[Bounded issue contract](spec/conversation-recovery-issues-v1.md) is implemented in Core, ordinary worker and UI. Missing live pages/data, actual MLS predecessor gaps and signed expired subtree observations share nullable lastIssue through owner/CLI/MCP recovery views. Authenticated exact targets govern clearing; issue writes never invent imports or loss counts, and matching import/issue clearing commit atomically. [Targeted checks](evidence/reviews/AR2-wallet-flow/history-recovery-issues-checks.json):99 distinct backend scenarios,51 frontend, production Clippy/fmt and frontend build pass, with five headless screenshots visually inspected. The initial legacy DTO mismatch was independently reviewed and corrected; initial REDs and the mistakenly empty worker filter are retained separately. Genuine paid Envelope diagnostics passes separately below. Expired skipped-subtree accounting, real rejoin, full130 throughput and full67/22/3 remain open.

[Paid Envelope native diagnostics R1](evidence/reviews/AR2-wallet-flow/history-recovery-issues-native-r1.json) now passes:6 genuine QC signatures, actual18+18 copy loss, sender absent, both SQL import faults, a real MLS predecessor gap visible through scoped CLI/owner pages, exact recovery and cold clearing. Actual exit0 in265.9seconds; all873 inputs unchanged, clean teardown. Native GUI paid recovery states remain separate. This does not resolve the full130 throughput failure below.

[Ordinary release bundle](evidence/reviews/AR2-wallet-flow/history-recovery-issues-release-build.json) rebuilt with these diagnostics; strict signature verification passes and the default feature graph excludes the automation driver. [Concurrent page requests](evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_COALESCING.md) now share a live transport job only for the same paid commitment/peer/position/wire. Independent review and17 backend/51 frontend checks pass, including actual Noise ACKs at two paid positions and SQL rollback/retry. The unchanged full130 R9 completed with the expiry failure below; no full130 acceptance is claimed.

## Full130 native R9 — recovery still fails, 2026-09-13

[Release R9](evidence/reviews/AR2-wallet-flow/history-range-native-r9.json) published all130 genuine paid originals with390 verified QC signatures,1300 data and1300 index receipts, and13000 location ACKs. Publication took2522seconds, leaving1078seconds of the earliest original lease. Actual1170+1170 copy loss, missing-trust refusal and first SQL import rollback passed. After the fault was removed and the receiver restarted on the other bootstrap seed, it recorded14 predecessor gaps and0 new imports before expiry (382 reads,8 history checks). The final-message fault and cold full130 traversal were not reached. Actual exit1 in3731.3seconds,876 unchanged inputs, clean teardown. Pending-page coalescing improves observed publication time but does not pass the release gate. [Same-pass preparation reuse](evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_PREPARATION.md) is independently accepted and passes17 backend/51 frontend plus Clippy/fmt; recovery scheduling must preserve actual contiguous imports, bounded fair attempts and the original quotas/lease. Full67/22/3 remains open.

## Full130 native R8 — expiry failure, 2026-09-13

[Release R8](evidence/reviews/AR2-wallet-flow/history-range-native-r8.json) published all130 genuine paid originals:390 verified QC signatures,1300 data receipts,1300 index receipts and13000 location ACKs. Actual loss removed1170 data and1170 index objects; missing-trust refusal and the first atomic import rollback passed. Publication consumed3314 of the earliest original's3600 seconds, leaving286 seconds. The range expired while waiting for129 imports with the final-message SQL fault still installed; full recovery and cold full-range traversal were not reached. Actual exit1, all868 inputs unchanged, no cleanup errors or remaining profiles. This is a failed release gate. Next: expose the accepted bounded recovery diagnostics and fix measured publication/retrieval throughput without weakening the retention or recovery oracle. Full67/22/3 remains open.

## Recovery observation adapters and UI — 2026-09-13

CLI `history status --from <public-id>`, MCP `history.status`, the ordinary graph worker and desktop conversation now share the Core recovery model. The worker observes authenticated leaves before fetching and updates progress only through committed imports. Scoped agent reads preserve inbox leases. The UI keeps loaded messages/drafts, shows the declared range and refreshes signed expiry without a database event. [Targeted checks](evidence/reviews/AR2-wallet-flow/history-recovery-integration-checks.json): 30 backend /45 frontend, production Clippy, frontend build and changed-production Rust formatting pass. Headless wide/narrow/empty/expired visual inspection passes. Separate test critic accepted the worker/adapter tests; the corrected native paid observation scenario [passes on867 unchanged inputs](evidence/reviews/AR2-wallet-flow/history-recovery-native-r1.json): six signatures, actual18+18 copy loss, SQL failure0/2, partial1/2, complete2/2 and cold2/2 through real CLI and owner pages. Clean teardown; native GUI recovery is not claimed. Expired skipped subtrees remain unobserved; missing-page/deferred-MLS diagnostics, real recovery/rejoin, full130 and 67/22/3 remain open.

[Actual Tauri history regression](evidence/reviews/AR2-wallet-flow/history-recovery-native-ui-r2.json) also passes:1051 real MLS messages, daemon restart,22 UI pages with exact IDs/text/no duplicates, then a real reply. All868 inputs and five bundled binaries stayed unchanged; hidden WKWebViews used isolated E2E secrets, no system Keychain. Wide native newest/oldest screenshots inspected, clean teardown. This verifies desktop history with the new observation UI; paid recovery states inside native GUI remain separate. The first local runner import failure occurred before app launch and is retained.

[Ordinary release bundle](evidence/reviews/AR2-wallet-flow/history-recovery-release-build.json) rebuilt from current source; strict signature verification passes and the default dependency graph excludes the automation driver. Builds and targeted/native checks are complete for this integration. Next: the mandatory full130 paid loss/recovery run, then explicit missing/expired-range diagnostics and real recovery/rejoin. Full V1 remains open.

## Core recovery observation — implemented, 2026-09-12

[Root-bound recovery contract](spec/conversation-recovery-observation-v1.md) and
[checks](evidence/reviews/AR2-wallet-flow/history-recovery-core-checks.json): seven
new independently accepted tests plus targeted regressions pass (71 backend,
35 frontend, Clippy/fmt). One bounded audit distinguishes observations from actual
committed imports; its updates commit with message/MLS/import records. Owner pages
and signed `history_recovery_get` share root/epoch/expiry-scoped facts and enforce
ReadInbox for agents. Empty/older local pages do not erase that scope; legacy
profiles keep local messages and report recovery_required. RED, the first stale-proof
fixture failure and its accepted correction are retained. Automatic worker and
expired-subtree observation, CLI/MCP/UI and real recovery/rejoin remain open.
[Next integration contract](evidence/reviews/AR2-wallet-flow/HISTORY_RECOVERY_PRODUCT.md)
includes accepted but still staged/unexecuted CLI/MCP process tests.

## Pending postage renewal — 2026-09-12

[Started payment after a checkpoint change](evidence/reviews/AR2-wallet-flow/PENDING_POSTAGE_RENEWAL.md) passes the full focused native GREEN R3: two real successors, the previous tickets/QC, pending slot release and cold recovery after losing the data/index copies without the sender. All 833 inputs unchanged, 6 signatures verified, clean teardown. The atomic pending-context update and a separate check of the payment time / storage admission, shared by full and compact proofs, were fixed. The latest targeted follow-up: 37 backend / 14 frontend, Clippy/fmt. Full130 release R6 ended after 128 stored originals and 130 accepted CLI sends: the correct over-budget refusal passed, but the full message snapshot comparison included mutable background delivery statuses. All 834 inputs unchanged, clean teardown. The fixed test oracle was accepted by the critic and passed the short release CLI run BO-R3: 6 signatures, loss of 18+18 copies, both SQL refusals and cold recovery without the sender; all 837 inputs unchanged. Full130 release R7 ended with 838 unchanged inputs: all 130 originals published, 390 signatures verified, 1300 data / 1300 index receipts and 13000 location ACKs stored. Before the first copy loss a test SQL helper failed; a short regression reproduced the two-persisted-objects limit and passes after its removal (retain11 for data/index, exact survivors/cold reads; 1 backend / 14 frontend, Clippy/fmt). Clean teardown; recovery of 130 not verified yet. Full recovery of 130 and scope 67/22/3 remain open.

**Architecture review, 2026-09-10:** [review response and new execution order](Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) supersede older next-work ordering below. [Original review](Docs/reviews/2026-09-10-independent/review.md) targets reviewed source checkpoint `eea229c`; its findings are not a new release gate. Preserve 67 mandatory cards / 22 E2E / three platforms. First: durable sender lifecycle and issuer spent-state continuity; then user entry points, network history/R10, groups/recovery and independent release. No narrow-preview scope reduction was authorized.

**Scope change, 2026-09-09:** the user explicitly deferred agent orders, service discovery/A2A, ratings and reviews to **V2**, prioritizing the first messenger release. Stop further feature work in those areas; retain existing code/evidence and shared regression/security checks. CLI-skill/MCP messaging remains V1. [Current decision](Docs/V1_SCOPE_2026_09_09.md) and [effective release graph](Docs/agentic_internet_v1_execution_plan/release-scope.json) override older scope/next-work statements below. Historical evidence remains unchanged.

Goal: a working Tauri/Rust application satisfying all required V1 backend/frontend checks and the 22 current V1 scenarios in `Docs/agentic_internet_v1_execution_plan/release-scope.json`. **Not achieved.** Passing a completed slice does not close the full product goal.

## Current ordinary graph sender — 2026-09-12

[Native 130-original range gate](evidence/reviews/AR2-wallet-flow/HISTORY_RANGE.md)
is written, but its earlier fixture acceptance is superseded by a
[feasibility REVISE](evidence/reviews/AR2-wallet-flow/history-range-fixture-review.md).
Debug R2 failed to publish
its first 16 originals within 600 seconds; all 823 inputs stayed unchanged and
cleanup passed. Release R3 also failed that deadline on 823 unchanged inputs.
[Redundant discovery is reduced](evidence/reviews/AR2-wallet-flow/ROUTING_HINTS.md)
using missing positions and bounded address hints with fresh proof verification.
The full native discovery gate passed on both chains (416 verified positions),
including strict fresh-proof/withdrawal checks and existing negative/cache fences.
31 backend /31 frontend, Clippy and fmt pass. Range R4 completed 32 originals on
823 unchanged inputs, then was deliberately stopped because snapshot1800 cannot
admit the last two originals under existing network quotas. The corrected initial
snapshot and real successor-head fixture passed independent test review; the
[focused renewal gate](evidence/reviews/AR2-wallet-flow/WALLET_RENEWAL.md) R1 failed
on 825 unchanged inputs after actual peer acquisition: the sender policy still
uses its old authority and blocks the second original. The ordinary wallet-to-sender
refresh bridge now passes54 backend /31 frontend and Core/node Clippy/fmt.
Native R2 passes on826 unchanged inputs: real peer successor, exact cold original
evidence/tickets/page ACK rows and sender-absent copy-loss recovery. Full130
[release R5 failed](evidence/reviews/AR2-wallet-flow/history-range-native-r5.json)
at the original fifth-batch deadline after64 stored originals and three real
peer-acquired successors, on826 unchanged inputs with clean teardown. Eight
pending finalizations occupied all client slots after the third successor.
[In-flight request renewal](evidence/reviews/AR2-wallet-flow/PENDING_POSTAGE_RENEWAL.md)
has passed its local client checks; native continuation exposed a separate earlier-QC/new-storage-admission boundary. Message TTL,
quotas, page allowances and the full recovery oracle
remain. The range is **not accepted**.

[Ordinary sender integration](evidence/reviews/AR2-wallet-flow/GRAPH_SENDER.md)
now publishes signed children before parents through the shared exact paid-page
ledger and only then publishes the root/pointer. A bounded stack reuses exact
peaks of Core's completed-root checkpoint; partial roots cannot retire work.
30 targeted backend /31 frontend and Clippy/fmt pass. Native C1 passed on 822
unchanged inputs: two genuine paid CLI originals, five pages /50 ordered ACKs,
SQL faults/cold retry and sender-absent recovery after real data/index loss.

Next: >128 simultaneously live paid originals through ordinary native sends and
recovery after fixing in-flight postage renewal. The [standalone graph native R1](evidence/reviews/AR2-wallet-flow/standalone-sender-graph-native-r1.json) now passes all four SQL fault stages, cold retirement and remote graph reads on 826 unchanged inputs with clean teardown. The two-original gate does not prove the full range. Full 67/22/3 remains
open; this supersedes historical component next-work notes below.

## Current ordinary graph receiver — 2026-09-12

[Ordinary receiver integration](evidence/reviews/AR2-wallet-flow/GRAPH_RECEIVER.md)
now connects exact paid typed pages, live-prefix admission, signed child routes
and durable attempts to the automatic worker. 130 genuine originals recover
across bounded cold passes; current-pointer and INSERT/UPDATE rollback fences
pass. Missing live pages remain pending; expired subtrees need no child bytes.
30 backend /31 frontend, Clippy/fmt and normal paid CLI v1 loss/recovery pass on
816 unchanged inputs. The native regression proves two paid originals;
the 130-original Runtime and paid-response fixtures remain separate evidence.

Next is ordinary sender child ACK ordering and root-before-pointer publication,
then >128 live paid native recovery. This supersedes next-work ordering in the
historical component entries below. Full 67/22/3 scope remains open.

## Current graph receiver cursor — 2026-09-12

[Core graph reference attempts](evidence/reviews/AR2-wallet-flow/GRAPH_SCAN.md)
now persist bounded selection across cold restart, including a wrapped 128-original
legacy leaf followed by two v2 leaves. Authenticated expired subtrees can be
skipped; live/mixed leases, wrong-parent paths and stale root/pointer work cannot.
SQL rollback and real MLS epoch separation pass. Attempts never imply import,
delivery or complete history. 105 targeted backend /31 frontend, Clippy/fmt pass
on 811 unchanged inputs, including the existing full 130-original Core gate.

Ordinary Node graph integration is next; this component did not rerun native.
The previous C8 paid v1 gate remains historical on its own source. Child ACK order,
>128 live paid recovery and the full 67/22/3 scope remain open.

## Current graph integration — 2026-09-12

[Core graph checkpoint and publication queue](evidence/reviews/AR2-wallet-flow/PUBLISHED_GRAPH.md)
now bind exact root/member/pointer completion to atomic sender retirement, with
SQL/cold retry and real MLS epoch separation. Index and history publication share
the tested cooperative queue. 117 targeted backend /31 frontend, Clippy/fmt and
normal paid CLI v1 native C8 pass. Earlier failures and diagnostic runs remain
recorded; one synchronous callback still has no universal latency bound.

Ordinary sender/recipient graph integration and >128 live paid native recovery
remain next. Actual child ACK ordering and the complete 67/22/3 scope stay open.

## Current recipient progress — 2026-09-12

[Durable fair history retry](evidence/reviews/AR2-wallet-flow/HISTORY_SCAN.md)
now reaches later pending originals across the 16-reference work limit and a full
Runtime restart. One small committed cursor records attempts; imports remain
atomic and separate. A real MLS gap releases the old fetch plan and is counted as
deferred. Malformed/unavailable responses retain their candidate/holder fallback.

72 targeted backend /31 frontend, Core/node Clippy and fmt pass; each candidate
keeps 802 inputs unchanged. The ordinary paid CLI scenario again passes actual
payment, SQL faults, exact page ACKs, retirement and two-original recovery after
18-data/18-index-copy loss. This remains flat v1 history. Next integrate ordinary
bounded graph publication/traversal and verify >128 live paid native recovery;
explicit product gap/legacy/rejoin, first offline Welcome, control/epochs and
independent repair remain required. Full V1 is not accepted.

## Active implementation — ordinary purchase and send

The daemon now obtains bounded untrusted RPC proofs from locally pinned profiles
and authenticated checkpoint history. Core commits the book key and immutable
purchase terms together, generates the exact transaction and ERC-681 payment
request, verifies actual funding and configures the existing ordinary sender.
Prepared quotes alone never grant balance; cold reads require renewed authority.

The first real native wallet gate passed payment, refused-RPC retry, cold wallet
refresh, configuration before compatibility import, two ordinary sends and
recipient recovery after sender disappearance, real data/index loss, missing trust
and two SQL failures. Six QC signatures bind the original data/index evidence.
[Candidate 1](evidence/reviews/AR2-wallet-flow/wallet-native-candidate-1.json)
predates the URI extension; [Candidate 2](evidence/reviews/AR2-wallet-flow/wallet-native-candidate-2.json) also passed, executing independently decoded URI arguments through the external Anvil signer.

The desktop panel now exposes exact price, funding lifetime, a separate message
retention setting, owner/agent budgets and verified balance through a constrained
main-window bridge. It preserves saved agent limits and CAS on failed saves,
retries failed preparation with the same book/terms and refuses a new payment
request after the trusted pre-beacon window. Checks so far: **56 Core wallet,
19 L2, 2 real Tauri bridge and 46 targeted frontend tests**; production
Core/L2/node Clippy and frontend build pass. Headless pending/ready/new-purchase and narrow-layout visual inspection passes.
[Contract and evidence](evidence/reviews/AR2-wallet-flow/TEST_CONTRACT.md).

The trust panel now selects storage/finalizer registry files and committee policy
through existing verified Core APIs. The native bridge and component flow pass,
including incompatible profiles, exact retry and size checks; headless screenshots
are retained. A further [native candidate](evidence/reviews/AR2-wallet-flow/authority-native-candidate-1.json)
passes with those hidden setup commands prohibited: Alice obtains public committee
and issuer evidence from an authenticated connected peer, imports it under the exact
current checkpoint, and persists client configuration. Process loss preserves its
revision; two ordinary sends and full loss/recovery then pass. Six QC signatures,
555 unchanged source inputs, 49 Core committee/lifecycle tests, client fence and
shared scheduling regressions, 46 frontend tests and Core/node Clippy are recorded.
The earlier payment candidates retain their original manual-setup limitation.

The standalone messaging CLI now uses the same private credentials, signing and
daemon authorization as MCP. Owner provisioning returns both concrete commands;
the desktop shows the CLI command and bundles its executable. Three separate CLI
process tests, three Core metadata tests, 13 MCP regressions, seven index regressions
and 20 targeted frontend tests pass. Core/node all-target Clippy and the signed
macOS debug bundle pass. Wide/narrow command screenshots are retained.
A [paid CLI candidate](evidence/reviews/AR2-wallet-flow/cli-native-candidate-1.json)
also passes: public NetworkID, exactly two explicitly sponsored sends, exact retry,
third-send budget refusal, independently verified R10 and full sender-absent cold
recovery. Six QC signatures and 561 unchanged inputs bind this evidence. Scoped
delivery remains queued without a recipient ACK even after paid storage succeeds.

The common clocked delivery projection now persists recipient ACK, funding,
data obligations, discovery and work independently. Completed work leaves the
active queue atomically while originals/history remain available. The
[native lifecycle gate](evidence/reviews/AR2-wallet-flow/status-native-candidate-3.json)
passes all three projection/completion SQL faults, cold shared CLI/MCP/desktop
reads, a later send and recovery of both originals after sender/data/index loss.
Six independent QC signatures and 697 unchanged inputs bind the result. Core
tests also cover partial storage with a real recipient ACK, expiry, scoped reads
and bounded paged history; the UI renders partial and expired observations.

The shipped skill is included in the signed macOS debug bundle and exposed
verbatim by the native permissions panel. An [independent Codex host](evidence/reviews/AR2-wallet-flow/skill-host/README.md)
used only the packaged CLI/skill for exact send/retry, signed delivery, a full
4854-byte reply after bounded poll correction, ACK and a fresh empty poll.
Both the host artifact reviewer and separate peer observer accepted the result.
The current [native GUI gate](evidence/reviews/AR2-wallet-flow/status-skill-native-ui.json)
passes on hidden packaged WKWebViews: the displayed CLI command executes,
MCP/CLI share exact retries and delivery, and UI revocation denies both. Existing
network/trust regressions and 1051-message history pass; 739 inputs stayed unchanged.

The [fresh native paid GUI gate](evidence/reviews/AR2-wallet-flow/GUI_ACCEPTANCE.md)
now passes on two empty profiles: real UI contact/trust selection, exact purchase
and external Anvil payment, cold wallet, RPC failure/retry, budget and two composer
sends. The UI shows 10/10 storage independently of recipient ACK; both originals
recover after actual sender/data/index loss and SQL faults. Six independent QC
signatures, 754 unchanged inputs and all five unchanged bundled binaries bind the
result. No owner sender-work/retrieval or prover invocation carries the flow.

This remains one active whole capability. New-contact/host/NAT E11 coverage and
long-running retained evidence capacity remain open. Prepared envelopes now use
immutable per-sequence rows and bounded snapshot pages; 258 real encrypted
originals survive restart and reach the actual recipient. Sparse legacy migration,
SQL rollback and paid reservation checks pass: 55 backend /31 frontend and
Core/node all-target Clippy/fmt. [Storage evidence](evidence/reviews/AR2-wallet-flow/ENVELOPE_PAGES.md).
The network directory still needs the matching lifecycle;
this is not an over-128 ordinary paid-flow pass. No full V1 card is closed.
The [Store traversal prerequisite](evidence/reviews/AR2-wallet-flow/STATE_RANGE.md)
also passes: bounded primary-key ranges over 257 rows and cold continuation,
10 backend /26 frontend tests and Store Clippy/fmt. Its live range is not a
snapshot. The [outgoing evidence layer](evidence/reviews/AR2-wallet-flow/OUTGOING_ROWS.md)
now uses operation rows, indexed conflicts, atomic occupied quotas and bounded
expiry. It retains 134 paid originals through cold restart, reclaims expired bodies
in bounded batches and rolls back seven distinct legacy migration SQL faults.
89 backend /31 frontend tests and postage-spend/node all-target Clippy/fmt pass.
The [Runtime integration](evidence/reviews/AR2-wallet-flow/RUNTIME_RETENTION.md)
now sets outgoing limits of 4096 originals /64 MiB and runs one bounded maintenance
batch per second, including retry backoff after storage failure. The real Runtime
pump and shared constructor pass the signed fixture-network test; 51 backend /31
frontend regressions and Clippy/fmt pass. Incoming operator scheduling is extended
below; signed network pages remain required. These checks do not renew native acceptance.
Incoming ciphertext now shares the operation-row engine with outgoing evidence,
with an independent admission quota, bounded authenticated local scans, explicit
empty continuations, two-object expiry batches and atomic v1 migration. The
134-original gate passes; signed network discovery still needs integration
before the ordinary long paid flow.
[Incoming evidence](evidence/reviews/AR2-wallet-flow/INCOMING_ROWS.md): 92 affected
backend /31 frontend checks, all-target Clippy and formatting pass. These remain
component checks; no native gate was run on this source.

Compact incoming paid indexes now use the same row engine, including their
holder receipts and signed history. Index admission has its own quota, bounded
local pages and expiry; legacy migration preserves existing locations/history
and their byte accounting. [Index evidence](evidence/reviews/AR2-wallet-flow/INDEX_ROWS.md)
records the 134-original, migration, mutable quota and corruption checks.
All 95 affected backend and 31 frontend tests pass, as do Clippy and formatting.
The daemon now configures separate 4096-object /64 MiB allowances for data,
index and outgoing evidence and attempts one bounded expiry batch per store per
second. A failing namespace does not starve the others, and fresh empty stores
need no idle SQL writes. [Operator Runtime evidence](evidence/reviews/AR2-wallet-flow/OPERATOR_RETENTION.md):
96 backend /31 frontend scenarios, Clippy and formatting pass on the recorded
56 focused inputs. These are fixture-network component checks, not native acceptance.
Inspection observations now use separate operation bodies with their own byte
budget and expiry queue. Original read times/signatures survive migration and
cold reads; observation and outgoing receipt changes remain atomic. Runtime
maintains observations as its fourth independent store. The 134-observation,
legacy migration and four-store Runtime tests pass: 98 backend /31 frontend,
all-target Clippy, formatting and 59 unchanged focused inputs.
[Observation evidence](evidence/reviews/AR2-wallet-flow/OBSERVATION_ROWS.md).
The [signed history page prerequisite](evidence/reviews/AR2-wallet-flow/HISTORY_PAGES.md)
now appends immutable v1 leaves through bounded signed branches and roots. Exact
257-page Python vectors, cold live-prefix proofs, signed forks, expired subtrees
and multi-reference legacy wrapping pass: 28 targeted crypto /31 frontend tests.
The ordinary sender still uses the flat manifest. The Core and paid storage
components are described below; ordinary network traversal remains before the
long native paid-flow gate. The wire result does not renew native acceptance.
Core now persists sender-local signed pages, membership and current roots in
bounded atomic batches. [Core page evidence](evidence/reviews/AR2-wallet-flow/CORE_HISTORY_PAGES.md)
covers 130 real originals plus a later append, cold snapshots/proofs, exact
retries, v1 migration and genuine MLS epoch fences. All 81 affected backend /31
frontend tests, Core/crypto Clippy and formatting pass on 66 focused inputs.
[Paid page retention](evidence/reviews/AR2-wallet-flow/PAID_HISTORY_PAGES.md)
now stores exact signed bodies under an already admitted anchor. Its shared
32 KiB history-wire allowance counts every pinned version and the legacy slot;
global quota charges the full page map and original paid evidence. Cold historical
reads, trust/clock fences, exact retries, SQL failures and expiry pass: 106 backend
/31 frontend, all-target Clippy/fmt and 124 focused inputs. A body ACK does not
establish graph availability. Ordinary network publication/traversal, pointer
fences and atomic recipient imports remain required before the long native gate.

[Typed page network ingress](evidence/reviews/AR2-wallet-flow/NETWORK_HISTORY_PAGES.md)
now serves exact signed pages and paid anchor evidence through the existing
authenticated custody protocol, with explicit capacity and commit-before-ACK/read
fences. Six new actual TCP/Noise fixture scenarios plus nine transport regressions
and 31 frontend tests pass; node all-target Clippy/fmt and 133 focused hashes pass.
Ordinary workers remain v1. Durable page/graph ACKs, bounded cold traversal and
recipient imports are still needed before current-root/pointer publication and
the ordinary >128 paid native recovery gate. No native acceptance is renewed here.

[Durable sender page observations](evidence/reviews/AR2-wallet-flow/OUTGOING_HISTORY_PAGES.md)
now keep exact typed bodies and separate paid-position ACK sets inside outgoing
anchor rows. Cold historical reads, late old-root replies, known-peer/wrong-position
refusal, full map/ACK quota, pinned legacy allowance, SQL faults and semantic
corruption pass. All 116 affected backend /31 frontend checks, all-target
postage-spend/node Clippy and formatting pass on 137 focused inputs. Typed wire
verification and allowance are shared with operator retention.

[Authenticated sender page replies](evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_SENDER.md)
now connect this ledger to the ordinary matched request/peer/connection checks.
The bounded client handles both existing leaves and typed leaf/branch/root bodies;
the ordinary leaf publisher already uses independent exact-page ACKs, including
after restart. Late replies cannot confirm a different body or missing children.
All 27 targeted backend /31 frontend checks, node all-target Clippy and formatting
pass. The ordinary paid CLI lifecycle also passes the new exact-ledger guard,
retirement and recovery after actual loss of 18 data and 18 index copies. This
and the four-stage sender SQL fault/restart gate pass on 790 unchanged inputs.
They renew those recorded native scenarios, not GUI or full V1 acceptance. Next are
bounded graph publication/traversal, current-root/pointer fences and atomic
per-reference imports, followed by the ordinary >128 live paid recovery gate.
Ordinary publisher/importer workers still exchange v1 flat directories.

[Core receive admission](evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md)
now protects new conversations from their initial MLS state on both contact paths.
The shared direct/custody reducer defers future applications without ACK or MLS,
message, import-row or cursor advancement. The required test retains arrival
order 1..129,0 and now recovers all 130 exact originals after reference-bound
retries, with cold reopening and constant per-import writes. Profile state v2
persists the policy; old contacts retain explicit legacy behavior. A previously
unguarded epoch cannot acquire a false recovery guarantee by accepting a new root:
root admission and retained-token use return `HistoryRecoveryRequired`.

All 141 targeted backend /31 frontend checks, Core/crypto/node all-target Clippy
and fmt pass. Each run preserves 798 captured inputs. The earlier
[Core failure](evidence/reviews/AR2-wallet-flow/HISTORY_PAGE_IMPORT.md) and
[crypto prerequisite](evidence/reviews/AR2-wallet-flow/MLS_CONTIGUOUS.md) remain
historical evidence. The ordinary paid CLI lifecycle first failed because its
unpublished setup sentinel occupied an earlier generation in the same chain.
That run is retained as failed. An independently reviewed fixture gives the
unrelated payment sentinel a separate real MLS conversation; the repeated native
run passes two paid sends, exact retries/budgets, SQL faults, cold exact-page ACKs,
retirement and both-original recovery after loss of 18 data and 18 index copies.
Six QC signatures are verified. This positive baseline does not demonstrate
recovery across the original missing-sentinel gap or renew GUI acceptance.

Next expose explicit network/product gap and legacy recovery outcomes, connect
bounded graph publication/traversal with current-root fences and per-reference
imports, then prove ordinary recovery above 128 simultaneously live paid originals.
Workers still exchange flat v1 directories; a deferred original remains pending.
Welcome/first-offline contact, control/epoch ordering, rejoin, autonomous repair
and all 67 cards /22 E2E /three platforms remain open as required.

The user-reported E2E Keychain password dialogs are resolved. Debug automation
now uses a private temporary file vault; normal builds retain system Keychain,
and release builds reject the E2E feature. The [rebuilt native gate](evidence/reviews/AR2-wallet-flow/e2e-vault-native-ui.json)
passes profiles, messaging, CLI/MCP, restart and 1051-message history with 754
unchanged inputs. Twelve backend and 23 frontend checks, Clippy/fmt and signature
verification pass. Real Keychain testing remains a separate interactive opt-in.
[Isolation and evidence](evidence/reviews/AR2-wallet-flow/E2E_SECRETS.md).

## Latest accepted source — ordinary recovery across disjoint funded books

Both exact originals now recover through ordinary workers after a cold sender
switches between two genuinely funded books, then disappears. Original data and
index rosters are disjoint before loss; the older 3600-second message remains the
anchor after the newer 900-second message. Actual data/index loss, missing trust,
both SQL import faults, partial progress and cold/cache-loss dedup all pass.
Six independent QC signatures bind 20 data receipts, 20 index promises and
200 location ACKs. No owner work/retrieval or prover invocation performs the flow.

Concrete failures exposed starvation in the repeated first-32 resolver window and
duplicate ordinary sockets after cold reconnect. Assignment-specific continuation
reuses the bounded job cache; duplicate retirement at the ordinary ceiling keeps
one live socket per peer and preserves selected/physical-close accounting.
No production quota or timeout is raised. The accepted native scenario and existing
one-book concurrent-message compatibility share one source/binary. Final checks:
**94 backend /21 frontend, Clippy/fmt, 614 unchanged inputs**. Failed runtime/fixture runs and independent test acceptance remain.
[Evidence](evidence/reviews/AR3-history-books/README.md) ·
[Capability matrix](Docs/agentic_internet_v1_execution_plan/CAPABILITY_EVIDENCE.md) ·
[Next work](evidence/reviews/AR3-history-books/NEXT.md).

This closes the declared two-original range within one MLS epoch. Whole user
wallet/send/status/lifecycle, actual MLS epochs/Welcome/gaps, autonomous repair
and the full 67-card/22-E2E/three-platform V1 remain open.

## Previously accepted — ordinary signed history publication and retrieval

Ordinary sender/recipient workers now use the exact signed directory and each
original's own paid index candidates. Durable original inventory replaces the
live-job roster intersection. Ten exact anchor ACKs precede pointer publication;
actual Noise peer/paid evidence/current pointer checks precede directory use.
Original message, MLS/dedup and per-operation import progress commit atomically.
Missing history cannot become legacy fallback or advance a global bookmark.
A pending pointer job receives time between full sender evidence rechecks.

Three native gates pass on one application source/binary: ordinary sender with
all four SQL faults, cold retries and independent reads from all ten ACK peers;
recipient after actual index/data loss with different surviving indexes/holders,
sender absent, missing trust, both original-message SQL failures, partial progress
and cold dedup; legacy direct-data retrieval with copies/inspection and cold retry.
Final targeted checks pass **76 backend /21 frontend, Clippy/fmt, 613 unchanged
inputs**, including six existing real-node mailbox process regressions. Earlier
failures and independent test-review corrections remain recorded.
[Runtime contract](spec/custody-history-automatic-v1.md) ·
[Evidence](evidence/reviews/AR3-history-automatic/README.md) ·
[Next work](evidence/reviews/AR3-history-automatic/NEXT.md).
This proves one-book recovery with real loss. Genuine disjoint funded books,
real MLS epochs/Welcome, successful retirement, historical lifecycle and
independent R10 repair remain V1 work; no new full-release claim is made.

## Previously accepted — durable outgoing manifest progress (AR3 prerequisite)

Core now returns current-epoch live original envelopes independently of transient
sender jobs and delivery/outbox state. It exposes the saved outgoing history CAS
revision after anchor expiry. One exact signed manifest and local ACK set share
the original outgoing paid carrier. Genuine retained index promises are required;
successors clear old ACKs, and actual paid peer/position/current hash/trust/lease
checks precede durable acknowledgment. SQL failures and precise quota boundaries
preserve prior state. Cold reads verify signatures and retained ACK positions.

Independent critic ACCEPT followed test revisions and compilation RED. Final
targeted checks: **60 backend /21 frontend, Clippy/fmt, 610 unchanged inputs**.
The earlier candidate's internal visibility error is preserved in evidence.
No ordinary network path changed: automatic manifest publication/traversal and
disjoint-book recovery still require integration; roster intersection remains.
[Contract](spec/custody-history-outgoing-v1.md) ·
[Evidence](evidence/reviews/AR3-history-outgoing/README.md).

## Previously accepted — exact paid manifest server transport (AR3 prerequisite)

The existing Noise custody protocol now stores signed manifests under their exact
paid anchor and returns that original anchor obligation with the manifest. A
recipient can address a nonfirst anchor directly; full portable payload accounting
precedes read-clock commit. Original funding/QC/receipt and storage quotas remain
shared. Missing history returns unavailable; only committed writes acknowledge
the exact operation and manifest hash.

Three store tests and a real-node server gate received independent ACCEPT before
production. Baselines record missing-API compilation RED and unsupported native
ingress. A native fixture batching error was corrected and separately accepted;
no production fix was needed. The corrected native gate passes with actual paid
funding, three independently checked QC signatures, cold index/sender absence,
exact manifest/anchor, SQL put/update/read failures and revision rollback checks.
Final affected checks pass **44 backend /21 frontend, Clippy/fmt and 607 unchanged
inputs**, matching the native gate. This is server acceptance, not automatic
manifest publication/traversal; the ordinary runtime still intersects index rosters.
[Contract](spec/paid-history-network-v1.md) ·
[Evidence](evidence/reviews/AR3-history-network/README.md) ·
[Next work](evidence/reviews/AR3-history-network/NEXT.md).

## Previous accepted source — per-reference atomic history import (AR3 prerequisite, `b2d433f`)

Core now prepares reads for exact manifest references and separately reports
imported, pending and expired-unimported operations. The live pointer and loaded
directory must match; missing or stale history cannot become an empty success.
Descriptor/ciphertext verification reuses existing crypto checks. Original message,
MLS/dedup and per-operation progress commit together through the existing receive
transaction. Out-of-order and concurrent imports preserve gaps; exact retry makes
no write, and prior direct delivery does not advance MLS twice. Cold progress
validates actual incoming message identity/scope and descriptor metadata.

Eight independently reviewed tests received REVISE → ACCEPT before production;
the accepted-input baseline has one missing-type and 61 missing-method compiler
errors. The final affected gate passes **57 backend /21 frontend, production
Clippy/fmt and 604 unchanged inputs**. These groups cover the changed Core helpers
and node compatibility; unchanged crypto/paid-store suites were not repeated.
Native manifest transport, ordinary multi-book retrieval and the other AR1–AR5
requirements remain open. The runtime still intersects index rosters.
[Contract](spec/custody-history-import-v1.md) ·
[Evidence](evidence/reviews/AR3-history-import/README.md) ·
[Next work](evidence/reviews/AR3-history-import/NEXT.md).

## Previous accepted source — exact history locator and Core binding (AR3 prerequisite, `16e264a`)

Private locators now carry an optional exact manifest/anchor commitment with
epoch, revision and retention. The legacy payload and fixed outer wire size stay
compatible. Core advertises only its committed outgoing history, checks incoming
MLS scope, rejects downgrade/rollback/equivocation within an epoch and accepts a
fetched directory only against the current live pointer. Cold saved-locator
metadata is checked against signed publication records or the read checkpoint.

Seven independently reviewed tests received REVISE → ACCEPT before production;
the final baseline has 11 crypto /21 Core missing-API/type/field compiler errors.
The affected gate passes **107 backend /21 frontend, production Clippy/fmt and
602 unchanged inputs**. This is codec/Core acceptance: ordinary sender/recipient
integration, reference-bound atomic message import and genuine multi-book
recovery remain next. V1 and AR-R03 remain open.
Six additional existing real-node mailbox process regressions pass on the same
602 source inputs, covering legacy sender-absent lookup, cache loss, hostile DHT
responses and bounded background work. This is compatibility evidence, not new
manifest transport acceptance.
[Contract](spec/mailbox-history-locator-v1.md) ·
[Evidence](evidence/reviews/AR3-history-pointer/README.md) ·
[Next work](evidence/reviews/AR3-history-pointer/NEXT.md).

## Previous accepted source — Core exact history and checkpoints (AR3 prerequisite, `63fd338`)

Core now retains outgoing signed manifests before publication and incoming
checkpoints before releasing routes. Exact cold retry preserves original bytes,
including when a short reference expires while the anchor remains live. Incoming
rollback/equivocation/time rollback and live omissions fail across anchor changes.
Stored signatures are reverified; expiry hides routes but retains the checkpoint.
Actual SQL failures preserve prior state, and acceptance leaves message/MLS/outbox
and fetch progress unchanged. The paid store and Core share one live-reference rule.

Seven independently reviewed tests passed REVISE → ACCEPT before implementation;
the final current-input baseline has 45 absent-API/type compilation errors. The
affected gate passes **91 backend /21 frontend, production Clippy/fmt and 597
unchanged inputs**. This is local persistence acceptance. Exact locator binding,
reference-bound atomic import and native multi-book use remain next; the accepted
`31ab80d` runtime still intersects index rosters. Earlier epoch rows are retained,
without claiming control/Welcome catch-up or discarded-secret recovery.
[Evidence](evidence/reviews/AR3-history-core/README.md) ·
[Contract](spec/custody-history-core-v1.md) ·
[Next work](evidence/reviews/AR3-history-core/NEXT.md).

## Previous accepted source — paid anchor history storage (AR3 prerequisite, `743e122`)

An existing paid index now retains the signed finite history directory under its
exact original anchor, with full historical Core trust, actual operator binding,
unchanged funding/QC/receipt and no added spend. The existing SQLCipher state and
quota count its bytes. Updates preserve all still-live descriptor references;
expired entries can be pruned. Cold retry, stale/equivocating revisions, actual
SQL failures, corruption, capability scope and whole-read byte limits are checked.

Five tests received independent REVISE → ACCEPT before production, including
follow-up acceptance of a shared test helper. The current-input baseline has 34
missing-API compiler errors. The affected gate passes **78 backend /21 frontend,
production Clippy/fmt and 595 unchanged inputs**. This is local persistence/read
acceptance. Core revisions/checkpoints, exact locator commitment and automatic
network use are next; the `31ab80d` runtime still uses index intersection.
[Evidence](evidence/reviews/AR3-history-store/README.md) ·
[Contract](spec/index-history-storage-v1.md) ·
[Next work](evidence/reviews/AR3-history-store/NEXT.md).

## Previous accepted source — finite signed history directory (AR3 prerequisite, `8b49cdd`)

The crypto layer now signs and verifies bounded directories of exact descriptor
references with separate candidate index routes. A longest-lived anchor prevents
a newer short lease from hiding older entries; each reference keeps its original
expiry. Exact fetched descriptors, signer/direction/epoch, canonical bounds and
revision checkpoints are checked. The old signed descriptor/ciphertext formats
are unchanged. The directory does not itself prove paid placement, availability
or full conversation history.

Seven new tests received independent REVISE → ACCEPT before implementation and
actual missing-API compilation RED. The affected gate passes **73 backend /21
frontend tests, production Clippy/fmt and 593 unchanged input fingerprints**.
This is a crypto prerequisite, without a new native release claim. The ordinary
runtime still uses its accepted index-intersection path from `31ab80d` below;
manifest storage, Core checkpoints and automatic use are the next integration.
[Evidence](evidence/reviews/AR3-history-manifest/README.md) ·
[Contract](spec/custody-history-manifest-v1.md) ·
[Next work](evidence/reviews/AR3-history-manifest/NEXT.md).

## Previous accepted source — automatic paid-index retrieval (AR3, `31ab80d`)

The ordinary recipient now verifies the replying index's paid obligation,
checks original/copy holder claims, resolves authenticated holders and fetches
only the exact descriptor-bound ciphertext. Core commits the original message,
MLS/dedup state and **index peer** bookmark atomically. Existing connections and
current signed locator routes precede bounded DHT lookup; empty legacy index
namespaces retain the existing direct-data fallback. Latest sender pointers now
use confirmed index endpoints with all holder-location ACKs, preserving the
existing cross-message reachability fence.

**Four native gates pass on one application source/binary:** two originals from
disjoint surviving holder sets after actual data loss with Alice absent and all
pointer endpoints holding zero ciphertext; ordinary owner/signed-agent sends;
legacy direct-data retrieval; sender index/location publication. Native checks
cover missing public trust, exact-original SQL rollback, cold recipient/index
restart, first-cache loss, dedup, both sender SQL faults and no repeated writes.
The final affected gate passes **49 backend /21 frontend tests, Clippy/fmt and
590 unchanged input fingerprints**. No owner retrieval RPC performs the work.
[Evidence](evidence/reviews/AR3-index-recipient/README.md) ·
[Runtime contract](spec/custody-index-recipient-v1.md).

This proves retrieval within one paid book. The pointer still intersects index
rosters across live jobs, so book/epoch continuity and finite completeness remain
open, along with durable first Welcome, safe successful retirement and autonomous
R10 repair. [Next work](evidence/reviews/AR3-index-recipient/NEXT.md).

## Previous accepted source — recipient Core index progress (AR3 prerequisite)

Core now prepares a one-entry index read from the incoming MLS direction and
the index peer's saved bookmark, validates discovery without writes, and imports
only the exact descriptor-bound ciphertext. A finite local token permits holder
lookup after the remote read capability expires. Message/MLS/dedup/bookmark
persistence shares the existing atomic transaction. Current epoch/index, saved
revision, strict sequence, retention and token time still fence completion,
including empty pages and actual cold MLS epoch transitions.

Seven tests received independent context-free REVISE → ACCEPT before production;
the applied baseline had only 59 missing-method compilation errors. The affected
gate passes **22 backend and 21 frontend tests, production Core/postage-spend/node
Clippy and fmt, 587 matching input fingerprints**. This is Core acceptance only;
native index/holder routing and ciphertext retrieval were accepted subsequently
in the current runtime gate above.
[Evidence](evidence/reviews/AR3-index-recipient/README.md) ·
[Core contract](spec/custody-index-progress-v1.md).

## Previous accepted source — automatic paid-index publication by ordinary senders (AR3)

Ordinary sender jobs now publish the original descriptor to ten selected index
operators after data R10, retain their signed promises, and atomically publish
the ten verified holder claims to each index. Compact outgoing progress shares
the original ciphertext/payment/QC; SQL failures cannot expose successful
progress, and cold restart resumes the same paid work. Completion requires ten
index promises and 100 location ACKs as well as data R10 and pointer publication.
The shared transport keeps its existing stream, byte, time and admission limits.

Five real native gates pass on the same application source/binary: automatic
index publication (20 promises/200 ACKs, both SQL faults and cold recovery),
ordinary owner/signed-agent sending and offline retrieval, existing single-claim
index compatibility, hostile epoch-history ingress, and cumulative epoch
handover (133 spends, two closings, 1605 independently verified signatures).
Affected checks pass **75 backend and 21 frontend tests, production Clippy/fmt**.
All **587 current input fingerprints** match. After the native runs, only the
evidence collector changed to read the handover gate's `full/` output directory;
the original successful logs and current source/binary hashes were rechecked.
This collector correction did not require repeating the network scenario.
[Evidence](evidence/reviews/AR3-index-sender/README.md) ·
[Sender contract](spec/postage/public-sender-index-v1.md).

At this sender-only checkpoint the latest pointer still used common data
holders. The current recipient gate above subsequently added index-aware
retrieval and authenticated holder routing. Durable book/epoch continuity,
completeness, first Welcome, safe successful retirement and autonomous R10
repair remain open. AR-R03 and V1 are not complete; no full workspace or new
cross-platform release gate is claimed.

## Previous accepted source — native paid index server and bounded location reads (AR3)

The ordinary custody transport now serves paid index publication/read, verified
holder-location updates and recipient-authorized count/byte-bounded location
pages. The receiver resolves its own published index-roster proof. A relayed
location must carry complete genuine data-holder evidence; it cannot substitute
the relay's key for the signed holder. Storage commits precede acknowledgments
and page release. Independently observed authority snapshots may differ only in
their valid observation time while retaining identical verified proof evidence.

The complete real native gate passes with a book-index node outside the message's
primary data roster. Actual original and replacement claims survive cold index
restart; recipient pages and the original MLS message remain accessible with the
sender and primary stopped. One actual ticket pays index/data/copy work. The same
source/binary passes **27 targeted backend and 21 frontend tests, production
Clippy/fmt, 580 unchanged fingerprints**. Native evidence has three independently
authenticated QC signatures, 522 owner calls and zero proof-worker invocations.
Independent critics accepted the tests and the real-observer correction before
their corresponding production changes. Diagnostic failures remain recorded.
[Evidence](evidence/reviews/AR3-index-network/README.md) ·
[Server contract](spec/paid-index-network-v1.md).

This server gate uses owner/raw Noise peers. Durable automatic publication,
actual-peer response authentication/import, latest-pointer routing and recipient
discovery across disjoint message rosters are next. Book/epoch completeness, safe
successful sender retirement, R10, AR-R03 and V1 remain open. No full workspace
or new cross-platform release gate is claimed.

## Previous accepted source — durable compact index locations (AR3 prerequisite)

An already paid index now retains finite original/replacement data-holder claims
through `retain_index_location` and exports them through a local Core-checked
`index_locations` read. Original descriptor/funding/QC stays stored once; each
position reuses the compact OutgoingReceipt evidence. Original index receipt and
descriptor pages stay unchanged. Claim updates, quota and read clocks commit
atomically; a live position cannot be silently replaced, even by another genuine
claim. Existing empty-location records remain readable.

Five new tests received independent context-free critic ACCEPT before production.
The accepted affected clusters pass **17 backend tests (16 paid-index + one
native public historical ciphertext test), 21 frontend chat/history tests,
Clippy and fmt**, with **577 unchanged input fingerprints**. Genuine replacement
consent, a disjoint index operator, cold historical reads without payer state,
exact byte quotas, SQL failure/retry, missing Core trust and corrupted evidence
are covered. Earlier revision counts below are not added to this gate.
[Evidence](evidence/reviews/AR3-index-location-storage/README.md) ·
[Storage contract](spec/index-holder-storage-v1.md).

This is local persistence/read acceptance. Bounded remote index/location
publication and queries, recipient discovery across disjoint data rosters,
book/epoch links, completeness and R10 are next. Successful sender retirement
still depends on that network history path; AR-R03 and V1 remain open.

## Previous accepted source — authenticated compact data-holder claims (AR3 prerequisite)

`PortableCustodyLocation` and `verify_custody_location` now authenticate original
native paid custody receipts using a signed descriptor instead of ciphertext.
They reuse the existing historical primary/copy verifier, including genuine
funding, exact QC, selected data operator, original-primary evidence and finite
copy consent. Cold verification needs installed trust but no sender wallet;
expired admission/binding/consent does not extend the original object retention.
Fetched bytes still require `verify_indexed_envelope` before acceptance.

Four tests were independently reviewed before production: first REVISE, then
ACCEPT after stronger genuine proof/member/consent substitutions. The baseline
was compilation RED for the absent API. The accepted affected-cluster gate
passes **59 backend tests and 21 frontend chat/history tests**, matching Clippy
and fmt; all **575 input fingerprints remain unchanged**. An initial runner's
empty public-ciphertext selection is retained as diagnostic evidence and was
corrected before acceptance. No full workspace or native network pass is claimed.
[Evidence](evidence/reviews/AR3-index-holder-locations/README.md) ·
[Protocol contract](spec/custody-holder-location-v1.md).

That verifier slice stopped before persistence; local index locations are now
covered by the newer gate above. Network publication/read, book/epoch links,
disjoint-roster cold discovery, completeness and autonomous repair remain open.
Neither local slice alone allows successful sender-job retirement or closes
AR-R03, AR1, AR3 or the V1 goal.

## Previous accepted source — bounded sender scheduling and responsive consensus/custody work (AR1)

The daemon now keeps a bounded disposable FIFO and per-job retry deadlines.
Current per-conversation sponsorship can defer paused jobs cheaply; every allowed
advancement still reauthorizes through Core. Core reuses policies only within one
read, shares retirement/remaining-queue validation, and reads job plus independent
delivery once. All existing integrity, grant, allocation and atomic retirement
checks remain; 30 focused Core sender tests pass.

Candidate 5 passes the genuinely funded gate with **128 active jobs / 127 paused**.
The signed runtime tail became ready in **1.30s**, resumed after sponsorship
restore in **6.29s**, and obtained an independently authenticated three-signature
QC in **11.66s after cold reconnect setup**. Its original preparation, ticket,
queue and reservations stayed exact; paused jobs remained unspent. Maximum
observed owner RPC latency was **1.64s**. These are local measurements, not a WAN
performance claim. [Native result](evidence/reviews/AR1-sender-fairness/candidate-5-fairness-check.json)
· [Timing and integrity](evidence/reviews/AR1-sender-fairness/candidate-5-fairness-integrity.json).

The same binary passes the complete two-message sender gate: **six independently
checked QC signatures, twenty authentic receipts and 1543 owner calls**. Actual
SQL receipt failure survives restart and retry succeeds after fault removal.
A genuine copied offer is rejected under another Noise peer. Cold stored sends
do not repeat puts; the recipient recovers both originals without the sender
and again after losing its local cache. No owner work/retrieval RPCs or prover
invocations drive the workflow. All **572 source fingerprints stay unchanged**.
[Complete sender result](evidence/reviews/AR1-sender-fairness/candidate-5-sender-check.json).

Candidate 4 added immediate handling of the existing trusted consensus mailbox;
its earlier partial runs and failures remain diagnostic evidence. Candidate 5
also retains fully verified custody-offer metadata in private jobs, rechecking
current Core authority, original verification time, expiry and the exact live
Noise connection on each read. Incoming cryptographic verification is unchanged.
The independently reviewed TCP/QUIC preservation gate passes on both chains:
329 verified positions, 2788 owner calls, unexpired same-peer reconnects,
binding/head expiry, relay-only and cold-cache clearing. Hostile-history ingress
also passes on the same binary: three spends, two closings, 69 signature checks
and 793 owner calls. Exact original evidence recovers after hostile responses
and cold restart without an early signer.

The user now requires targeted clusters during implementation and the complete
suite only at the end of the plan. The previously started workspace run was
interrupted; its completed **46 Core wallet/preparation tests (30 sender tests)**
are retained, without claiming a complete workspace pass. **59 frontend tests,
19 model tests, TypeScript/Vite and fmt** also completed before the policy update.
**96 targeted daemon tests and node Clippy also pass**. All 572 build/test input
hashes remain unchanged. [Actual checks](evidence/reviews/AR1-sender-fairness/checks.json).
Successful retirement, a
complete persistent stage ledger, automatic authority renewal and outage closure
remain open; AR1 and full V1 are not accepted.
[Test contract](evidence/reviews/AR1-sender-fairness/TEST_CONTRACT.md) ·
[Scheduling contract](spec/postage/public-sender-scheduling-v1.md).

## Previous accepted test coverage — hostile native history ingress (AR1)

The additional funded native gate passes: three spends, two actual closings,
69 independently checked signatures and 750 owner calls. An ordinary Noise peer
supplied an authentic closing for the wrong target, an altered original closing
QC and a canonical page carrying another position's original SpendRecord. Every
rejection preserved the exact SQL snapshot and prevented an early signer; actual
ResponseSent events and repeated queries establish working transport. Exact
original answers resumed complete continuity after crash, preserving all three
records/QCs/timestamps and immutable history through cold reads.
[Native result](evidence/reviews/AR1-hostile-history/native-check.json).

Only test/peer-carrier code changed after `d95671b`; production daemon code remains
unchanged. Targeted regression passes: **195 node tests, 59 frontend tests**, node
Clippy, fmt, TypeScript/Vite and the previous carrier recovery mode (two spends,
24 independently checked signatures). All **570 source fingerprints stayed unchanged**.
[Actual checks](evidence/reviews/AR1-hostile-history/checks.json). The full workspace
result below belongs to the preceding production commit; it was not rerun for this
test-only change. Automatic renewal, outage closure and full AR1 remain open.

## Previous accepted production source — legacy verifier build separation (AR2 prerequisite)

The ordinary workspace now excludes the RISC Zero guest/kernel builders. Its
legacy verifier uses the frozen original image ID; explicit proving still
builds the real guest and rejects an image mismatch at compile time. The clean
target gate has verified both original paid receipts without guest generation.
The actual macOS debug bundle passes its separate gate: no prover sidecar or
proving command handler, both original receipts verified with exact outputs,
changed context/journal and expiry rejected, and node availability accurate
across restart. Frontend/TypeScript/Vite and 19 model tests pass. The separate
legacy target passes all 13 tests: original-receipt compatibility with `prove`,
nine genuine-proof/guest checks and three actual process tests, including receiver
verification without the sender wallet and unchanged paid rows after cancellation,
parent death and cold retry. The full workspace passes **871 Rust tests, zero
failed/ignored in 59 nonempty suites**, plus workspace Clippy. All **569 build/test
input fingerprints stayed unchanged**. [Actual checks](evidence/reviews/AR2-verifier-build/checks.json).
The ordinary macOS release package also passes: ad-hoc signature verified,
guest/kernel generation forbidden throughout the build, no prover sidecar or
proving command handler, both original paid receipts accepted and accurate
verifier/prover availability across daemon restart. The packaged verifier is
byte-identical to the accepted debug verifier. [Release result](evidence/reviews/AR2-verifier-build/release-check.json).
This build prerequisite does not close AR-R11, AR2, full historical retrieval,
UI acceptance or the three-platform product release. [Evidence and next work](evidence/reviews/AR2-verifier-build/).

## Previous accepted source — native epoch handover and full regression passed (AR1)

The daemon persists pending successor configuration and unresolved paid inputs
before a signer can start. A bounded ordinary-peer protocol discovers original
closing choices backward, then imports original spent history forward. Selected
activation requires complete cumulative continuity; ordinary clients retain only
public lineage. Configuration and its bootstrap cursor now commit atomically.
The dedicated selected/client SQL-fault gate passes (263 owner calls), including
same-revision live/cold retry and absence of a premature successor journal.

The native gate, funded in a local EVM, passed on actual daemon-owned keys: **133 spends, two
closings and 1605 independently checked signatures** across three genuine chosen
committees (registry epochs 1→3→4 in this run). It covers more than 128 old spends,
partial-page SQL rollback, revocation, cold resumption, incomplete old validator
history, preserved pending input, ordinary-client exact/conflict results and a
fresh third member. The original archive served all 130 old records after its
checkpoint expired, while renewal persistence failed and no signer ran. All
original records survived transfer and cold reads; no owner RPC installed history,
spent rows or QCs. [Native result](evidence/reviews/AR1-native-handover/native-check.json).

Earlier failed attempts are retained as diagnostic evidence. Full6 exposed an
insufficient test deadline: 33 pages need three 16-request/60-second admission
windows. The independently accepted fixture correction keeps the production
quota, real clock, 600-second lease maximum and client/consensus deadlines.
Full7 completed with a 160-second history timeout.

Full regression passes: **871 Rust tests, zero failed/ignored in 59 nonempty
suites**, **59 frontend tests**, TypeScript/Vite, 19 model tests and workspace
Clippy/fmt. All **582 source fingerprints stayed unchanged** through the full
workspace gate. [Actual checks](evidence/reviews/AR1-native-handover/checks.json).
[Evidence and test contract](evidence/reviews/AR1-native-handover/).

AR-R01, automatic renewal/outage closure, sender lifecycle and the full
67-card / 22-E2E / three-platform V1 remain open.

## Previous source — overlapping validator history completion (AR1)

A validator retaining only part of the old epoch can now extend its authenticated
index through the closing. Previously, any nonempty index selected the complete
local path and rejected a missing tail. The new explicit extending import keeps
old entry/operation rows, authenticates exact matches and blocks negative
membership until the complete new chain reaches genesis. Equal/older checkpoints,
extension after terminal and conflicting signed rows are refused.

SpentHistoryImport chooses this path only when the local index has not reached
the exact closing tip. Existing SpendRecords retain their original QC and
verification time; already complete local history retains its earlier path and
tip QC. SQL failure rolls back new history rows, records and progress together,
including a page crossing into the old retained prefix. Cold restart resumes
the same checkpoint and cannot expose successor continuity early.

Four new tests received independent backend-test-critic ACCEPT before production.
The focused suites pass **22 finalizer history tests and 8 postage history tests**.
The full workspace passes **871 Rust tests, zero failed/ignored in 59 nonempty
suites**; **59 frontend tests**, TypeScript/Vite, 19 model tests and workspace
Clippy/fmt also pass. All **576 source fingerprints stayed unchanged** through
the full workspace gate. [Actual checks](evidence/reviews/AR1-overlap-history/checks.json).
[Contract and evidence](evidence/reviews/AR1-overlap-history/) ·
[Next network integration](evidence/reviews/AR1-successor-spending/NEXT.md).

This prerequisite does not enable the actual daemon's successor lifecycle.
Ordinary-peer cumulative transfer, client network lineage and a funded gate with
overlapping/new validators across multiple epochs remain required. AR-R01 and
the full 67-card / 22-E2E / three-platform V1 remain open.

## Previous source — successor application spending and public client lineage (AR1)

SpendSession now has an explicit constructor requiring both current selected
Core authority and complete owner-bound cumulative continuity. Successor sessions
must use store-aware proposal/verification for ordinary and closing entries;
legacy storeless paths fail closed. Global old spent keys are checked before a
new epoch's prefix. A valid successor QC cannot spend an old ticket again, even
with the same journal. Only an exact same-entry retry returns the original
SpendRecord; its first QC/time survive a later valid retry.

Continuous historical lookup authenticates the original QC under the exact
chosen predecessor, using the bounded finality parser only as an untrusted hint.
Cold continuity still checks all old records. Warm new-ticket lookup remains
independent of old history length. Five tests execute genuine application policy
in epochs 2 and 3 after 130 actual paid epoch-1 spends, covering both predecessor
spent sets, cold restart, revoked roles, lease expiry/renewal, altered entries,
damaged records and same-committee duplicate positions.

Public EpochLineage separately authenticates every chosen transition back to
epoch 1, without validator spent records or operator keys. Five client tests
cover 1→2→3 and 1→3, missing/invalid ancestors, owner/scope/current-fence checks,
SQL interruption and original-QC preservation. Existing partial or complete
handover metadata supports public lineage without migration; incomplete state
still cannot authorize voting. All seventeen new/reused focused tests pass.
**867 Rust tests pass, zero failed/ignored in 59 nonempty suites**; **59 frontend
tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt also pass.
All **576 source fingerprints stayed unchanged** through the full workspace
gate. [Actual checks](evidence/reviews/AR1-successor-spending/checks.json).

The actual daemon still uses the legacy first-epoch constructors. Pending
successor startup, authenticated ordinary-peer transfer, public client network
lineage and fresh funded multi-epoch consensus remain required. No new native or
cross-platform gate is claimed. [Contract](spec/postage/successor-spending-v1.md) ·
[Policy evidence](evidence/reviews/AR1-successor-spending/) ·
[Client evidence](evidence/reviews/AR1-client-lineage/).
AR1 and the full 67-card / 22-E2E / three-platform V1 remain open.

## Previous source — cumulative original spent-history import (AR1)

The application evidence layer transfers a closed epoch in pages of four entries.
Each ordinary entry requires its exact original SpendRecord, authenticated under
the original source snapshot and matched to the full canonical entry. The
existing HistoryImport now commits caller-verified application rows with its
index and head; the postage layer adds its own progress to that transaction.
The unchanged sixteen-state SQL batch limit is sufficient. Failed pages do not
advance the cached or persisted cursor, and original QC/time survive retries.

Complete local validator history is reused without resetting its tip. A cold
SpentContinuity scan checks every predecessor epoch and every original record
back to epoch 1. Missing earlier state prevents completion; chosen transitions
may skip registry epochs. Owner and progress CAS guard both local and fresh
imports. A genuine later-epoch QC cannot overwrite an issuer-global spent ticket.

Seven focused application tests pass, including 130 real paid public spends,
bounded/cold transfer, SQL rollback, authentic but mismatched archive evidence,
stale/foreign local handles, 1→2→3 continuity, skipped epochs, zero-spend closure
and cold corruption. The separate fixture buys 160 tickets in a real EVM and
captures three authenticated registries at one root. Fixture QCs are genuine
P-256 signatures; the epoch-2 historical carrier does not claim live successor
consensus. All nineteen generic history-index tests also pass.

**857 Rust tests pass, zero failed/ignored, 57 nonempty suites**. **59 frontend
tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt pass. All 571
captured source inputs stayed unchanged through the full workspace gate.
[Actual results](evidence/reviews/AR1-spent-history-import/checks.json).

No non-first-epoch spending guard was relaxed. Current selected authority plus
complete continuity, global old-spend proposal checks, role-free client lineage,
actual authenticated peer transfer and pending successor startup remain the next
integration layer. No new native or cross-platform acceptance is claimed.
[Contract](spec/postage/spent-history-handover-v1.md) ·
[Next work](evidence/reviews/AR1-spent-history-import/NEXT.md).
AR1 and the full 67-card / 22-E2E / three-platform V1 remain open.

## Previous source — authenticated postage epoch closing (AR1)

Core authenticates a strictly later successor roster at the old committee's current
checkpoint without replacing the published predecessor. Both public snapshots and
live fences bind a canonical closing candidate to one terminal operation. The
actual node persists this history policy before starting its signer. Ordinary
spends and closing decisions enforce it; only a direct old-committee QC can close.

ClosingRecord retains both original authority snapshots, payload, QC and time.
Historical verification checks both snapshots, checkpoint identity, canonical
successor binding and finite windows. SQL commit precedes history/retirement and
actual delivery ACK. Selected peers recover the exact original record after its
input retires; revocation stops live recovery. Closed journals retain their
configured scope for serving old evidence while suppressing candidate broadcasts.
Unresolved paid inputs remain durable and appear in the epoch read model.

The fresh funded native gate uses ordinary daemon-generated keys for all old and
future registry members. One initial owner closing proposal reaches three active
validators through ordinary gossip. The lagger misses it, fails the closing SQL
commit without advancing its cursor, is revoked, and cold restarts. One source
then supplies the exact original record; two live nodes cannot form a new quorum.
All 24 P256 signatures are independently checked, including the direct closing QC
against the actual first spend. The gate preserves the unrelated pending input's
exact persisted hash/revision, old SpendRecords and closed history after another
cold restart with the operator disabled. New old-epoch spends and a competing
successor are rejected. A first failed run exposed and fixed the candidate-scope /
historical-server coupling; both failed and successful evidence are retained.

**847 Rust tests pass, zero failed/ignored, 56 nonempty suites**. **59 frontend
tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt also pass.
All 567 captured source inputs stayed unchanged through the full workspace gate.
The unchanged ordinary recovery and full-queue renewal native scenarios also pass
on this source: respectively 2 spends / 24 signature checks and 18 spends / 216
checks. Each native run records its own pinned binary and source fingerprint.
[Evidence](evidence/reviews/AR1-epoch-closing/) ·
[Contract](spec/postage/epoch-closing-v1.md).

This closes an old epoch while its original selected authority is still current.
Complete cumulative spent-state transfer, successor bootstrap/spending, closing
after an old-lease outage and automatic authority renewal remain open. All
non-first-epoch spending guards remain; no fresh packaged native or other-platform
acceptance is claimed. AR1 and the full 67-card / 22-E2E / three-platform V1 remain
open. [Next work](evidence/reviews/AR1-epoch-closing/NEXT.md).

## Previous source — durable terminal history boundary (AR1)

The P-256 history index can now bind a trusted application's terminal operation
before its signer starts. The rule persists even for an empty history, survives
ordinary reopen/import resume, invalidates older handles and refuses a different
binding. A terminal entry needs its own finalization QC; a proof through a
finalized child cannot certify closure. Old spent lookups and original QCs remain.

The checked-prefix guard refuses children after terminal, including while that
entry is still in the actual consensus suffix. Seven-entry transfer pages preserve
the rule and remain unavailable until complete; a terminal entry below the tip
rejects its whole page. SQL binding/activation failures preserve prior state.
No Entry wire format, Simplex algorithm, quorum or resource bound changed.

The new gate runs four actual P-256 engines on the deterministic network. All
applications authorize entry9 and at least one attempts it, but terminal entry8
finalizes with its own archive QC and prevents entry9. After all runtime/SQL
handles are destroyed, ordinary SQL reopen and actual consensus storage recovery
preserve the exact terminal proofs and continue rejecting entry9. Separate real
QC/SQL tests cover 130 ordinary entries plus terminal131 and bounded transfer.
These operations are generic signed test payloads, not funded postage.

**839 Rust tests pass, zero failed/ignored, 55 nonempty suites**. **59 frontend
tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt also pass.
All 547 captured source
inputs stayed unchanged during the full workspace regression.
Validation is recorded in [evidence](evidence/reviews/AR1-terminal-history/).
[Contract](spec/finalized-terminal-history-v1.md).

This is the old-log stopping rule, not full epoch handover. The node postage
service still requires epoch1; authenticated successor payloads, complete
issuer-wide spent transfer, successor bootstrap and actual epoch crash/partition
and lease-outage gates remain open. No new native funded-spend gate is claimed.
AR1 and the **67-card / 22-E2E / three-platform** V1 goal remain open.

## Previous source — checkpoint renewal with a full pending queue (AR1)

A selected validator can replace stale checkpoint envelopes for the same verified
receipt even when all 16 active slots are occupied. The old receipt must verify
to the same canonical journal under the new current context; matching operation
fields alone cannot evict another ticket. The replacement commits before its
cached/network form is released, preserves queue order and survives SQL failure
and crash. Immutable spent records and original QCs remain unchanged.

Historical record verification now accepts earlier entries from the immutable
committee's original registry window after checkpoint renewal, while retaining
the authenticated checkpoint's narrower upper expiry and observer-time bound.
This does not relax live authority or permit a new epoch to begin with empty
spent history.

The combined native gate passed **18 funded spends / 216 signature checks** using
four ordinary generated validator keys and one pinned daemon executable. It
filled one isolated validator with 16 pending spends, refreshed their genuine
checkpoint evidence through repeated SQL failure and cold restarts, rejected a
different valid ticket sharing the same operation, and distributed the refreshed
envelopes to the other three peers only after the old lease actually expired.
All records form the original hash chain through height18. Original records
predate the new checkpoint issue time and remain exactly readable after a final
solo restart with the operator role disabled. Retained evidence independently
repeats 216 signature checks and the complete 18-entry chain offline; it does not
newly authenticate checkpoint/EVM proofs or provide cryptographic signing time.

**833 Rust tests pass, zero failed/ignored, 55 nonempty suites**. **59 frontend
tests**, TypeScript/Vite, workspace Clippy/fmt and 19 model tests also pass. Final
workspace source inputs stayed unchanged throughout validation. The final native
production/Python inputs are unchanged; the only later source delta is the
independently accepted legacy Rust test alignment from checkpoint-issue to registry
lower bounds, covered by the final workspace and Clippy runs.
[Evidence](evidence/reviews/AR1-checkpoint-refresh/) and
[contract](spec/postage/checkpoint-refresh-v1.md).

This is explicit selected-validator refresh. Automatic client/sender authority
acquisition and context renewal, epoch closing/handover, unresolved expired
inputs, successful sender retirement and the remaining V1 release gates stay
open. The full **67-card / 22-E2E / three-platform** goal is not complete. Previous
144-spend lifetime and two-spend recovery gates remain distinct historical evidence.

## Previous source — recovery of missed original spend evidence (AR1)

A selected validator can now apply archive finality after it missed the owner's
receipt and the other replicas retired their active candidates. The service holds
one deferred delivery and requests the original SpendRecord through the existing
bounded submissions transport. Local historical verification binds the entire
entry and preserves the original QC/time. Record, history and cleanup still commit
before ACK. Role revocation cancels the held delivery and request permission;
restart derives the target from the actual archive again.

A fresh funded native gate passed with four ordinary daemon keys. Three sources
finalized while the lagger was offline and cleared their queues. A real recovery
SQL failure left its consumer at zero; role disablement then prevented writes and
traffic with the fault removed and peers still online. After solo cold replay,
a genuinely selected peer's changed QC signature was rejected. One ordinary
source then supplied the exact original record without owner resubmission or a
new quorum. The lagger retained it through another cold restart and applied the
next entry in the same issuer log.

Final regression: **832 Rust tests, zero failed/ignored, 55 nonempty suites**;
**59 frontend tests**, TypeScript/Vite, workspace Clippy/fmt and 19 model tests
pass. The native gate uses one pinned daemon executable and checks 24 QC
signatures across eight record observations. Those checks also pass on the saved
trace after all daemons/EVM stop. Offline checks use the retained committee and
do not newly authenticate checkpoint/EVM proofs. Production did not change during
regression; the independently accepted test-only carrier/diagnostic refinements
are fingerprinted separately. [Evidence](evidence/reviews/AR1-spend-recovery/)
and [contract](spec/postage/spend-recovery-v1.md).

This network recovery requires a live selected service in the same epoch. Epoch
closing/handover, authority refresh, expired pending-work reconciliation and
successful sender retirement remain open. The earlier 144-spend gate was not
rerun in this slice; its distinct lifetime evidence remains below. AR1 and the
full 67-card / 22-E2E / three-platform V1 goal are not complete.

## Previous source — bounded runtime history and terminal spend retirement (AR1)

The actual built-in P-256 spend service now uses the authenticated SQL index and
checks only the complete bounded suffix to its exact finalized anchor. An
operation-bound capability prevents forgetting old spent keys; a missing index
requires the full suffix to genesis and cannot omit prior consensus history.
Legacy applications retain their complete-prefix guard. Entry/QC/journal formats,
quorum, voting WAL and the epoch !=1 refusal remain unchanged.

A fresh real EVM purchase funded 160 public tickets. Four ordinary daemons with
independently generated validator keys finalized **144 distinct spends under the
same issuer/log/committee**. The independent oracle checked **2,163 signatures**.
The gate crossed 128, held new work without quorum, recovered delivery 129 after a
real spend-write failure with every peer down, and recovered delivery 130 after
the spend committed but candidate cleanup failed. Both stages check consumer ACK
ordering. Two valid competing operations were admitted on a sole live validator
before remote distribution; only one finalized. All terminal alternatives retire
from the active sixteen-entry queue, while the original spend/QC stays durable.

After actual expiry of the original 600-second authority, a cold restart still
reads the original QC from persisted public authority evidence; a valid unused
ticket cannot create a new spend. No prover, seeded WAL, synthetic quorum,
epoch reset or funding source during voting was used. This is a native daemon
gate, not a fresh packaged desktop/ciphertext custody or cross-epoch acceptance.

Four new prefix-capability tests and the fresh gate pass; workspace Clippy/fmt,
59 frontend tests, TypeScript/Vite and 19 model tests also pass. The final Rust
workspace passes **829 tests, zero failed/ignored, 54 nonempty suites**. Rust source
remained unchanged during that final regression. The final native run passes
with one copied daemon executable pinned by hash across every restart. After all
daemons and the EVM stopped, the retained public trace independently passed
committee derivation, all 432 retained QC signatures and the complete 144-entry
chain. This offline check does not newly authenticate checkpoint/EVM proofs.
The independent critic accepted R2 before production
and accepted the old concurrent gate's intentional post-finality queue assertion
update separately; the full legacy 100-client gate was not rerun. Evidence:
[evidence/reviews/AR1-runtime-lifetime/](evidence/reviews/AR1-runtime-lifetime/).

AR1 still requires epoch closing/handover, authority refresh and pending-work
reconciliation. Cold reads with unresolved inputs and unavailable current authority now
return unavailable and preserve pending work instead of claiming absence. A
validator missing original receipt evidence is addressed by the current recovery
implementation above; that behavior was not a claim of this earlier lifetime gate.
AR-R01 and full V1 remain open; preserve 67 cards / 22 E2E / three platforms.

## Previous source — authenticated finalized-prefix index (AR1)

The P-256 history index derives membership from a complete QC-authenticated
canonical prefix, stores entries and operation keys in separate indexed SQL rows,
and commits each append with its retained checkpoint. Cold open verifies the
whole prefix using bounded working memory. Transfers use at most seven entries
per transaction, persist their cursor, and refuse membership queries until the
correct genesis is reached. Wrong-owner/stale handles, incomplete or corrupted
indexes, failed SQL commits, duplicate operations and snapshot rollback fail closed.
An alternative valid quorum retry preserves the original checkpoint bytes.

Eleven new history tests and ten ancestry tests pass. The history tests use real
P-256 quorum signatures and SQLCipher, including 130 entries and resumed transfer
between different owners. They are not a live engine or funded-postage gate.
Ancestry proofs now bound path length at 128 rather than absolute target height;
wire entry/journal/QC formats and quorum requirements remain unchanged. Full
regression passes **825 Rust tests, zero failed/ignored, 53 nonempty suites**, plus
**59 frontend** tests, TypeScript, Vite, workspace Clippy/fmt and 19 model tests.
After Clippy's sole test-only clone cleanup, all 21 targeted tests and Clippy/fmt
were rerun successfully; production did not change during regression. Final input
fingerprints and the documentation/test-style deltas are retained in evidence.
Test critic R2 ACCEPT preceded production.

This is a prerequisite for runtime integration. Engine/service/spend guards and
`epoch !=1` rejection remain. The total cold snapshot/revalidation is O(history),
not a compact non-membership proof. Reconciliation to the actual consensus archive,
bounded unfinalized-tail validation, issuer policy integration and authenticated
epoch closing/handover remain required. See [spec](spec/finalized-history-index-v1.md)
and [evidence/continuation](evidence/reviews/AR1-finalized-history/).
AR-R01, AR1 and full V1 remain open.

## Earlier source — expired prepared sender retirement (AR1)

Following the independent review, Core now atomically retires genuinely expired
prepared sender jobs from the active queue and retains their terminal status,
original message/outbox, sponsorship counters and exposed ticket allocations.
Daemon maintenance performs this after cold restart even without fresh checkpoint
authority. Status preserves the independent recipient ACK fact: an expired job may
belong to either a delivered or an undelivered message. Idle passes do not rewrite
the profile. Unprepared work does not expire by message age.

Six new funded-Core tests and one real daemon-process wiring test pass. The complete
Rust workspace passes **812 tests, zero failed/ignored**, on **656 unchanged
code/spec/test/build inputs**. Workspace fmt/Clippy, **59 frontend** tests, TypeScript,
Vite, 7 model and 12 EVM-model tests pass. Targeted tests are part of the workspace
count, not additive. Separate critic R3 ACCEPT preceded production; the missing-API
and real daemon RED results are retained. Evidence and continuation:
[evidence/reviews/AR1-expired-sender/](evidence/reviews/AR1-expired-sender/).

This accepts prepared-expiry retirement only. The 129-message test crosses active
queue admission capacity with two allocations; it does not cross the independent
128 finalized-spend limit. Successful terminal retirement, unprepared timeouts,
ready scheduling and issuer-global snapshots/handover remain open. No new fresh
live-EVM or packaged-native gate is claimed; the last native build remains the
paid-index checkpoint below. AR1 and the full 67-card/22-E2E V1 remain unfinished.

## Earlier implementation evidence

**Postage change, 2026-09-10:** [the user's public signed postage decision](Docs/V1_POSTAGE_BOOK_2026_09_10.md)
removes ZK from new V1 funding/preparation/spending and removes payment-source
privacy. Existing proof evidence below remains historical. Native Core public
authorization now passes seven new tests, all 234 Core tests and all 59 frontend
tests. Actual local EVM purchases, MPT inclusion/exclusion and independent CBOR/
Ed25519 vectors establish funding/key binding and isolation from the fixed legacy
relation. Test-critic R2 FINAL ACCEPT preceded implementation. Evidence:
`evidence/reviews/P02-public-postage-native/`.

Public spend and historical custody integration now passes six targeted tests:
five integration cases and one real four-engine partition/heal/finality case.
The existing issuer-global log, QC, placement and retained obligations accept the
new native signed format; malformed tagged hybrids cannot fall back to the legacy
verifier. Restart restores the full original obligation and exact ciphertext after
admission expires. Test-critic R2 FINAL ACCEPT preceded production; R3 accepted
the test-support module path correction. Evidence:
`evidence/reviews/P02-public-spend-integration/`.

The public book wallet now passes eight Core tests (zero failed/ignored). Public
preparation reuses the encrypted custody keystore and returns the actual outer
commitment. Native funding checks derive the finite balance. The issuer/commitment
key shares allocations across local aliases; a single checkpoint + SQLCipher
transaction saves the counter and exact operation signature before return. Restart,
checkpoint refresh, exhaustion, SQL failures, corrupt saved evidence and exclusive
profile-owner handoff are covered. R2 test critic ACCEPT and actual missing-API RED
preceded production. R3 accepted correction of a test that had incorrectly assumed
two concurrent profile owners; the existing OS lock was preserved. Evidence:
`evidence/reviews/P02-public-book-wallet/`.

The earlier full-image failure is preserved as infrastructure evidence. External
free-space restoration was observed (74 GiB); the managed image/UUID/links passed
without cleanup, resize or detach by this task. No resize permission is needed for
the current run. All eight wallet tests, workspace fmt/Clippy, 59 frontend, 7 model,
12 EVM model tests, TypeScript and Vite now pass. The full Rust workspace run
passed **760 tests, zero failed/ignored, 50 nonempty suites**, in 963.398 seconds
on 504 unchanged frozen inputs. This also closes the earlier storage-blocked
combined regression for native public postage and spend/custody integration.

The public book owner IPC adapter now passes a fresh live-EVM gate on two chains:
daemon-generated keys, canonical purchases, native receiver verification and
independent Ed25519/CBOR checks, four concurrent clients sharing one wallet,
duplicate reservations, exact restart, compatible checkpoint refresh, retained
history after chain shutdown, exhaustion and current expiry. Both missing-worker
and self-tested executable-tripwire modes pass without worker invocations. The
accepted run holds the daemon binary hash unchanged; an earlier concurrent Cargo
build correctly invalidated a run and is retained as rejected evidence. All
194 node tests (zero failed/ignored), workspace fmt/Clippy and 59 frontend tests,
TypeScript and Vite pass. Evidence: `evidence/reviews/P02-public-book-daemon/`.

The composed public paid-ciphertext gate now also passes on actual daemons and
fresh canonical EVM funding. The same sender owns the random book key and actual
872-byte MLS envelope. Native public authorization, independently checked
Ed25519/CBOR and a real three-signature spend QC feed the existing primary storage,
primary/replacement copies, authenticated holder inspections and automatic
recipient retrieval. Sender and original sources are absent during retrieval;
restart and real SQL failures/retries pass. The wallet still has one allocated
ticket and three available after the full cycle. All daemon starts/restarts use
self-tested executable worker tripwires: zero workflow invocations. The tested
production code required no additional changes. New model (7), EVM model (12),
frontend (59), TypeScript and Vite checks pass. Evidence:
`evidence/reviews/P02-public-paid-ciphertext/`.

Core now couples original MLS message preparation to a public postage reservation
in one checkpoint + SQLCipher transaction. Eight new tests cover independent
receiver verification, exact retry/restart, finite multiple-message allocation,
SQL INSERT/UPDATE faults, canonical aliases, current checkpoint refresh/expiry,
corrupt saved association and refusal to silently renew expired preparation.
All 16 wallet tests and **444 Core/daemon tests** pass with zero failed/ignored;
445 frozen source files are unchanged. Workspace fmt/Clippy,
59 frontend tests, TypeScript and Vite pass. This dependency is not yet wired to
a new daemon command, and no new live-EVM or packaged-app run is claimed.
Evidence: `evidence/reviews/P02-public-message-preparation/`.

The shared daemon now exposes that preparation through a strict owner IPC route.
The actual public paid-MLS gate allocates its stamp through this route, checking
the original envelope and independently verified signature, invalid retention/
message refusals, exact restart and updated-head retry. Real QC, storage, copies,
holder inspections and automatic recipient recovery pass with one allocated
ticket and zero ZK/owner-retrieval calls. All 194 daemon tests, 59 frontend tests,
7 model and 12 EVM-model tests, workspace fmt/Clippy, TypeScript and Vite pass;
530 frozen source files and the live daemon binary remain unchanged. Evidence:
`evidence/reviews/P02-public-message-daemon/`.

Core now atomically enrolls ordinary owner/runtime messages into a separate
public custody queue with explicitly configured sponsorship limits. Ten new
tests prove direct ACK/restart persistence, exact retry at exhaustion/full queue,
verified grant attribution, scope/owner API denials, policy remove/readd/alias
conservation and real SQL rollback of MLS/outbox/work/budget/nonce. Test critic
R3 ACCEPT preceded production; R4 accepted a compact signed-request fixture fix.
The full workspace passes **779 Rust tests, zero failed/ignored**, on
612 unchanged inputs; workspace fmt/Clippy, 59 frontend, 7 model and
12 EVM-model tests, TypeScript and Vite pass. These are queued sponsorship units;
the automatic daemon worker is not implemented by this Core admission module.
Evidence and next full sender gate: `evidence/reviews/P02-public-sender-admission/`.

A fresh public paid-MLS EVM/network regression on the same current sources also
passes: three real QC signatures, actual encrypted custody/copies/inspection,
sender-absent automatic retrieval, one allocated ticket and zero ZK/owner-retrieval
calls. Its sender orchestration remains manual; this does not accept the new
automatic sender worker. Evidence: `evidence/reviews/P02-public-sender-admission/`.

Core execution now reauthenticates queued ordinary messages before exposing a
public stamp. Current pause/ceilings and the actual runtime grant's signature,
epochs, scope, revocation and expiry are checked on first execution and retries.
Native preparation preserves the original MLS envelope and allocation; no new
nonce or sponsorship debit is created. Separate current proof and retained
ancestry support explicit/retained checkpoint refresh and restart. Eleven new
tests and **465 Core/daemon tests** pass, zero failed/ignored. A failed existing
node timer fixture was corrected with R3 critic acceptance; the final node run
and all unchanged Core results pass, with 615 frozen final inputs.
Workspace fmt/Clippy, 59 frontend,
7 model, 12 EVM-model tests, TypeScript and Vite pass. R2 test-critic ACCEPT
preceded production. The daemon scheduling/QC/custody loop remains open; no new
live EVM or packaged-app run is claimed by this Core module. Evidence:
`evidence/reviews/P02-public-sender-execution/`.

## Last verified application build — paid independent index storage

The backend now stores compact index references independently of ciphertext.
Index placement uses only the genuinely funded native public book commitment and
authenticated future beacon; all tickets in the book share its finite roster.
The existing ciphertext selection and receipt encodings remain unchanged.
New admission verifies the exact paid operation/resources, P256 spend QC and
actual selected primary's current registry/transport binding. A separate signed
index receipt binds the descriptor and all original paid evidence, and escapes
only after the SQLCipher commit. Full evidence counts against separate index
quotas. Exact retries, paged private reads, expiry, clock rollback, cold verification
after admission expiry and corrupted-storage refusal are implemented.

Seven new scenarios and all **270 postage-spend/daemon backend tests** pass, zero
failed/ignored. Workspace fmt/Clippy, **59 frontend**, 7 model, 12 EVM-model tests,
TypeScript and Vite pass on **626 unchanged recorded inputs**. Test critic R2
accepted strengthened tests before production; R3 accepted a page-budget oracle
correction and R4 accepted one lint-only test change. Original failures remain
in evidence. The ordinary macOS arm64 app was rebuilt, its ad-hoc signature
verified, and the default graph checked to exclude the automation driver.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D05-paid-index-store/`.

This is backend storage acceptance only: no new live-EVM funding, full-workspace
or native UI gate is claimed. The daemon does not yet use these independent
index entries for network history discovery. Data-holder receipt/route linkage,
sender-side index receipts, network adapters, automatic sender/recipient wiring,
range/gap evidence, lifecycle and autonomous R10 data/index repair remain required.
Full D05/E05–E07 and the application goal are **not achieved**; all remaining
effective V1 messenger/platform requirements remain mandatory. Concrete next
dependencies are recorded in `evidence/reviews/D05-paid-index-store/NEXT.md`.

## Previous verified application — compact custody index descriptors

Core and the shared daemon now export a compact signed description of the exact
retained envelope using the existing custody key and canonical document format.
The record binds its hash, index, epoch, sequence, size and finite deadline without
the ciphertext. Verifying a matching record still requires authentication of the
actual envelope. Export reuses the validated SQLCipher loader and changes no
allocation, original packet/stamp, MLS state, sequence, outbox or delivery status.
The owner-only `export_custody_index_entry` route rejects signed runtime grants
and external metadata/time overrides.

Five new crypto tests and two real Core tests pass, plus the extended actual
daemon Noise/ACK/restart/export/dedup scenario. All 35 crypto and
468 Core/daemon regression tests pass, zero failed/ignored;
workspace fmt/Clippy, 59 frontend, 7 model and 12 EVM-model tests, TypeScript and
Vite pass on 647 unchanged final inputs. Independent critic R2 accepted
the Core/crypto tests before production; R3 accepted two test lint corrections;
R4 accepted the owner IPC gate before that route. Failed lint runs are retained.

The ordinary macOS arm64 app was rebuilt and its ad-hoc signature verified.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D05-custody-index-entry/`.
This increment has no new UI, live-EVM or full-workspace acceptance run. The
preceding `d1c2fe3` module below retains its 791-test workspace and seven native
UI/screenshot results; those must not be relabeled as a new descriptor UI gate.

Independent paid index storage/replication, complete-range and gap reporting,
autonomous R10 repair, epoch lifecycle and all remaining messenger/platform V1
requirements are still open. Full D05/E05–E07 and the application goal remain
**not achieved**. The descriptor is not a durable index receipt or storage proof.

## Previous verified application — automatic public-postage sender

Ordinary owner and independently signed runtime messages now execute through
the daemon's automatic public-postage worker. It reauthorizes Core preparation,
exports current authenticated checkpoint ancestry, obtains the real spend QC,
resolves selected custodians against their actual Noise identities, commits ten
distinct primary receipts and publishes the private pointer through DHT ACKs.
Restart restores the original packet/stamp/QC/receipts without repeated holder
puts. Owner sponsorship/configuration/status routes remain unavailable to
scoped signed messaging grants. Test-critic acceptance preceded production.

The fresh actual-daemon/EVM gate passed: two ordinary messages, six independently
verified finalizer signatures, twenty independently verified custody receipts,
real receipt-commit failure and retry across sender restart, copied genuine
offer refusal on another Noise transport, unchanged holder ledgers after stored
sender restart, automatic sender-absent recipient recovery, recipient restart
and first-cache loss. Owner work/retrieval calls and ZK workflow invocations are
zero. Failed development runs and corrected test-oracle assumptions are retained.

All **791 workspace Rust tests** pass, zero failed/ignored, 50 nonempty suites,
on 635 unchanged frozen inputs; all 59 frontend, 7 model and 12 EVM-model tests,
fmt/Clippy, TypeScript and Vite pass. All seven hidden native scenarios and
seven personally reviewed old/new screenshot pairs pass, including actual
MCP/revocation, relay/restarts and 1051 history messages over 22 pages.
The ordinary macOS arm64 app was rebuilt, its ad-hoc signature verified and the
default dependency graph checked to exclude the automation driver.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/P02-public-sender-daemon/`.

This accepts initial ten-primary execution only. The native UI gate exercises
existing flows; public wallet setup in this gate uses owner IPC. Independent
durable index/control metadata, autonomous R10 repair, terminal queue retirement,
pending-spend authority refresh, committee/epoch handover and admission-expiry
lifecycle, complete desktop funding/sponsorship and CLI/MCP lifecycle, remaining
messenger/platform acceptance and Linux/Windows remain incomplete. Distinct
localhost identities do not prove physically independent hosts. Full V1 is
still **not achieved**; the parked 64-validator/R24 gate and V2 deferrals remain.

## Previous verified application — custody manifests and holder inspections

All 739 Rust tests passed on 609 frozen inputs (zero failed/ignored,
four threads, 47 nonempty suites), including 47 paid-store and nine actual
Tauri command tests. All 59 frontend, 7 model and 12 EVM model tests, fmt/Clippy,
TypeScript and Vite passed. Seven hidden native scenarios and seven personally
reviewed screenshot comparisons passed, including 1051 actual messages, restart,
22 history pages and a reply.

Holders can now exchange authenticated bounded receipt manifests through a
separate actual-read scope. The request contains no ciphertext/proof; a target
must possess the original ciphertext and authenticate the actual requesting
Noise peer. Signed responses bind the challenge, holder and exact page metadata.
Verified receipts and signed observations commit atomically in the existing
outgoing evidence store plus bounded observation state. Exact retries preserve
signed read time; stale observations, expiry, clock rollback, genuine conflicts,
shared quotas and SQL INSERT/UPDATE failures are covered. Ten enrolled units on
one transport do not imply ten independent copies or current availability.
Ten new storage tests, one actual-daemon owner-only scenario and one codec regression
passed. R2 test-critic FINAL ACCEPT preceded production; R3 accepted the independent
fixture null encoding correction and early preflight. R4 accepted the test-local lint annotation; all R4 hashes are unchanged.

The fresh proof (413700 ms; 872-byte actual MLS envelope; three QC
signatures) passed the extended live primary→replacement gate. With both clients
stopped, holders learned and retained exact evidence across restart. Actual
captured ciphertext-free requests shared four slots with copying; retries,
conflicts, capacity and deadline release passed. Target/source disk failures
reported no success and retry recovered. After both sources stopped, the recipient
automatically retrieved exactly the original message from the replacement.
Independent signature/CBOR/metadata and historical/fresh proof checks passed.

The ordinary macOS arm64 app was rebuilt and ad-hoc signature checked without
test driver/helpers: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D03-custody-manifest-inspection/`.
Contract: `spec/custody-manifest-inspection-v1.md`. Raw: `output/manifest-inspection/`.

Full V1 and D03/E05–E07 are not accepted. Autonomous placement/repair, R10,
independent durable index/control metadata, historical discovery across cached
proof replacement, remaining messenger/platform acceptance, Linux/Windows and
notarization remain open. The 64-validator/R24 gate remains RED/parked; orders,
ratings and reviews remain V2.

## Previous verified application — paid custody copies and forwarding

All 727 workspace Rust tests passed on 601 frozen inputs (zero failed/ignored,
four threads, 47 nonempty suites), including nine actual Tauri command tests.
All 59 frontend, 7 model and 12 EVM model tests, fmt/all-target Clippy, TypeScript
and frontend build passed. All seven hidden WKWebView scenarios passed, including
1051 actual messages, recipient restart, all 22 UI pages and a reply. All seven
screenshot pairs were personally inspected without visual regressions; live
timestamps and one transient DHT startup status differ as expected.

Existing paid ciphertext can now be copied and forwarded between finite selected
custodians after admission expires, without a sender wallet, a new spend or a
longer object deadline. Every receiver authenticates the original full proof/QC,
local trust, original primary membership, actual source Noise identity and exact
envelope. A bounded current consent is bound to one owned operator, selected
position and original receipt. Its permission defaults off and has an independent
revision per operator. Copy provenance stays flat across forwarding.

The target commits the full obligation before returning it; the source verifies
and commits its outgoing receipt before reporting success. Both paths share
existing quota/evidence logic. Actual SQL failures, revoked-permission retries,
restart, stale clocks, independent quotas and malformed provenance are covered.
Eleven new paid-store tests and one actual-daemon owner-only scenario passed.
Independent test-critic R3/R4 ACCEPT preceded production; R5 accepted a restart
fixture correction. Reviewed test hashes remain unchanged. Final gates were rerun
on unchanged source after a fixed-length per-operator namespace correction.

A fresh genuine proof paid for an actual 872-byte MLS envelope with three verified
quorum signatures. With both clients stopped, the object crossed original primary
0 → primary 1 → replacement 10. Both sources then stopped. After sender publication
of the private pointer and renewed absence, the recipient automatically recovered
exactly the original message from the replacement, including disk failure/retry,
endpoint fallback and restart. Proof generation took 435370 ms; independent oracle
and packaged verifier checks passed. This live gate uses current admission;
genuine historical fixture tests establish expired-admission copying.

The ordinary macOS arm64 app was rebuilt and its ad-hoc signature checked without
test driver/helpers. Historical proof compatibility remains intact.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D03-paid-custody-copies/`.
Raw checks: `output/custody-copies/`.
Contract: `spec/paid-custody-copies-v1.md`.

Full V1 and D03/E05–E07 are not accepted. Automatic placement/scheduling,
independent durable index/control metadata, R10 and autonomous repair remain
open. Cached membership replacement may make an old epoch unavailable; complete
historical operator discovery is not established. Remaining messenger scope,
Linux/Windows and notarization remain open. The 64-validator/R24 engineering
gate remains RED and parked; orders/ratings/reviews remain V2.

## Previous verified application — historical custody obligation verification

All 715 workspace Rust tests passed on 594 frozen inputs (zero failed/ignored,
four threads, 47 nonempty suites), including nine actual Tauri command tests.
All 59 frontend, 7 model and 12 EVM model tests, fmt/all-target Clippy, TypeScript
and frontend build passed. All seven hidden WKWebView scenarios passed, including
1051 actual messages, recipient restart, 22 UI pages with exact IDs/text and a
reply. Seven screenshot pairs were personally inspected without visual regressions.

A cold peer can now verify an existing paid custody obligation after the original
admission window expires. Core reconstructs the original bounded public context
under its own installed trust, including after 80 head advances and restart.
The full verifier checks the genuine fixed-image seal, complete QC/journal,
original operator proof and transport binding, exact signed envelope/receipt,
finite selected position and object deadline. It rejects isolated damaged QC or
authority data, substitutions, stale clocks and incompatible trust. Clock commits
are durable before checked results escape; no current admission handle is created.
Shared placement and receipt construction serve both live and historical paths.

The strict owner-only `custody_verify_obligation {operation}` authenticates local
retained evidence using the daemon's actual transport key and reports all finite
primary/replacement ordinals. Scoped agents cannot call it. Four new paid-store
and three new Core tests, plus the extended actual daemon access scenario, passed.
Independent backend test review R1/R2 REVISE, R3 FINAL ACCEPT preceded production;
accepted test hashes remain unchanged. Independent Python vectors cover all 14
positions. This verifies obligations; historical new-copy permission is still open.

The ordinary macOS arm64 app was rebuilt and ad-hoc signature checked without the
test driver/helpers. Two historical genuine receipt compatibility checks passed.
A fresh genuine proof paid for an actual 872-byte MLS envelope, with three verified
quorum signatures. After custodian restart, the new owner method returned the
exact verified assignment, actual transport identity, expiry and all 14 positions.
With sender absent, the returning recipient automatically found/read one original
message through storage failure/retry, custodian/recipient restart and unavailable
pointer-source fallback. Proof generation took 398894 ms; independent oracle and
packaged verifier checks passed, including operation substitution and expiry.
The live query ran during admission; genuine fixture tests cover expired admission.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D03-historical-obligation/`.
Raw checks: `output/historical-obligation/`.
Contract: `spec/historical-custody-obligation-v1.md`.

Full V1 and D03/E05–E07 are not accepted. This network gate uses one live primary;
automatic placement, historical new-copy authorization, independent durable index/
control metadata, R10 and autonomous repair remain open, along with the remaining
messenger/platform scope. Linux/Windows and notarization remain open. The
64-validator/R24 engineering gate remains RED and parked; orders/ratings/reviews
remain V2.

## Previous verified application — portable original custody evidence

All 708 workspace Rust tests passed on 587 frozen inputs (zero failed/ignored,
four test threads), including nine actual Tauri command tests. All 59 frontend,
7 model and 12 EVM model tests, fmt/all-target Clippy, TypeScript and frontend
build passed. All seven hidden WKWebView scenarios passed, including 1051 actual
messages, recipient restart, all 22 UI pages and a reply. Seven screenshot pairs
were personally compared with the previous application without visual regressions.

Custodians now commit the exact original zkVM receipt and public context together
with the original envelope, signed storage receipt, full SpendRecord, authority
snapshot and operator presentation. Senders retain one public proof/context per
operation across their outgoing receipts. Original ancestry remains bound to its
original checkpoint after 80 current-head advances; the actual requested operation
is preserved. A cold independent Core reconstructs and checks the public context
and genuine proof without another node's cache or wallet.

Strict owner-only `custody_obligation {operation}` exports a live incoming
obligation after restart and durable clock commit. SQL failures expose no success;
renewed retries preserve all original fields. Expired admission does not erase a
still-live audit record or authorize new work. Legacy profiles missing the raw seal
report `evidence_unavailable` while existing private ciphertext reads remain usable.
The existing 16 MiB serialized limit per ledger bounds the added proof bytes.

Independent backend test review R1 REVISE, R2 ACCEPT and strengthened R3 ACCEPT
preceded production. Six new paid-store tests, the strengthened 80-head Core test
and actual daemon owner/agent access scenario passed. The reviewed test hashes
remain unchanged. The ordinary macOS arm64 app was rebuilt and ad-hoc signature
checked without the test driver/helpers; two historical genuine receipt checks
passed with the packaged verifier. Linux/Windows and notarization remain open.

A fresh genuine proof paid for an actual 872-byte MLS ciphertext, with three
verified quorum signatures. After custodian restart, owner export returned the
exact original proof/context, envelope, full SpendRecord, receipt and presentation.
The sender SQL failure/retry and restart checks passed; with sender absent, the
returning recipient automatically found and read exactly the original message,
including custodian/recipient restarts and storage failure/retry. This gate uses
one live primary. Proof generation took 452227 ms; independent oracle and packaged
verifier checks passed, including operation-substitution and expiry rejection.
Checked evidence: `evidence/reviews/D03-portable-obligation/`.
Raw checks: `output/portable-obligation/`.
Contract: `spec/portable-custody-obligation-v1.md`.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.

Full V1 and D03/E05–E07 are not accepted. Historical new-copy authorization,
automatic placement, independent index/control metadata, R10 and autonomous repair
remain open, along with the remaining messenger/platform scope. The 64-validator/
R24 engineering gate remains RED and parked; orders/ratings/reviews remain V2.

## Previous verified application — durable outgoing custody receipts

All 702 workspace Rust tests passed on 586 unchanged source inputs (zero
failed/ignored, four test threads), including nine actual Tauri command tests.
All 59 frontend, 7 model and 12 EVM model tests, fmt/all-target Clippy, TypeScript
and frontend build passed. All seven hidden WKWebView scenarios passed, including
1051 actual messages, recipient restart, all 22 UI pages and a reply. Seven
screenshot pairs were personally inspected without visual regressions.

The sender now commits the original verified storage receipt, full SpendRecord,
authority snapshot and exact envelope before reporting `stored`. Per-position
operator/transport evidence survives restart, while shared evidence is stored once
per operation. Incoming and outgoing quotas are separate; ten obligations fit one
operation and one envelope allowance. Original binding/checkpoint expiry does not
erase a live obligation; SQL failures expose no successful result and clock
rollback cannot revive expiry. Strict owner-only `custody_receipts {operation}`
recovers retained receipts. These are signed obligations, not physical-independence
proofs or recipient delivery acknowledgments.

A fresh genuine fixed-image proof paid for an actual 872-byte MLS message; three
real quorum signatures were checked. The real network gate forced sender SQLCipher
failure after remote storage success, required `storage_error`, retried the same
payment/receipt, restarted the sender and recovered exact evidence while chat
remained queued. With sender absent, the returning recipient automatically found
and read exactly the original message through independent pointer caches, storage
failure/retry and custodian/recipient restarts. It uses one live primary, not R10.
The fresh proof took 666603 ms and was separately verified by the independent
oracle and the packaged verifier, including operation-substitution/expiry rejection.

The ordinary macOS arm64 app was rebuilt and ad-hoc signature checked without
test driver/helpers. Its proof/verifier hashes match the previous bundle; both
historical receipt compatibility checks passed. It is not notarized; Linux/Windows
acceptance remains open. Independent test review R1 REVISE/R2 ACCEPT preceded
production. Five new paid-store tests and one actual daemon test passed; reviewed
test hashes remain unchanged. Raw logs are outside Git with retained hashes.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D03-outgoing-receipts/`.
Contract: `spec/outgoing-custody-receipts-v1.md`.

Next transport work must supply portable paid placement/repair authorization,
automatic placement, independent index/control commitments and finite frontiers,
R10 and autonomous repair. The receipt ledger does not retain the raw zkVM seal or
authorize historical new-copy work. Full virtualized history, groups/recovery,
attachments, credentials and remaining platform/UX acceptance are still open.
Full D03/E05–E07 and V1 are not accepted. The 64-validator/R24 engineering gate
remains RED and parked; orders/ratings/reviews remain V2.

## Previous verified application — original outgoing packet archive

All 696 workspace Rust tests passed on 584 unchanged source inputs, with zero
failed/ignored and four test threads, including nine actual Tauri command tests.
All 59 frontend tests, 7 model and 12 EVM model tests, fmt/all-target Clippy,
TypeScript and frontend build passed. All seven hidden WKWebView scenarios passed;
seven screenshot pairs were personally inspected. The ordinary macOS arm64 app was
rebuilt and ad-hoc codesign verified without test driver/helpers. Two genuine
historical RISC0 receipt compatibility checks passed; proof/verifier binary hashes
are unchanged. It is not notarized; Linux/Windows acceptance remains open.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence: `evidence/reviews/D03-outgoing-archive/`.
Contract: `spec/outgoing-archive-v1.md`.

Direct delivery acknowledgment now archives the exact original signed transport
packet in the same SQLCipher transaction that consumes the outbox row. A first
custody preparation after receipt and restart reuses that packet. Failed archive
INSERT or queue DELETE keeps original work retryable. Repeated receipts, operation
retries and envelope preparation preserve message identity, MLS and deduplication.
Existing pending profiles upgrade without packet loss; already-delivered legacy
rows with no surviving packet return explicit unavailability rather than another
message. This local archive does not increase network replica counts.

Seven new store/Core/actual Noise daemon tests passed. Independent R1 ACCEPT
preceded production; R2 ACCEPT corrected two test recipient-derivation arguments
without weakening exact-byte assertions. Both original behavior RED and the test
setup failure are retained. The unchanged native history scenario again delivered
1051 messages, restarted, traversed all 22 UI pages and sent a reply.

Next transport work includes durable outgoing receipt aggregation before reporting
stored, automatic sender placement, independent index/control logs, authenticated
finite frontiers/gaps, R10 and autonomous repair. Full virtualized history and the
remaining daily UX/groups/recovery/attachments/credentials/platform acceptance
remain open. Full V1 is not accepted. The 64-validator/R24 engineering gate remains
RED and parked; orders/ratings/reviews remain V2.

## Previous verified application — bounded desktop history

On 581 unchanged source inputs, all 689 workspace Rust tests passed (zero
failed/ignored, four threads), including nine actual Tauri command tests. All
59 frontend tests, 7 model and 12 EVM model tests, fmt/Clippy, TypeScript and
frontend build passed. All seven hidden WKWebView scenarios passed; seven
screenshot pairs were personally inspected. The ordinary macOS arm64 app was
rebuilt and ad-hoc codesign verified without test driver/helpers. Two genuine
historical RISC0 receipt compatibility checks passed; proof/verifier binary hashes
are unchanged. It is not notarized; Linux/Windows acceptance remains open.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence, source and binary hashes: `evidence/reviews/U01-desktop-history/`.

Desktop no longer uses the first-1000 full-body snapshot. It reads contact pages
with short previews and separate latest text pages capped at 50 messages/512KiB,
with an explicit older-page button. SQLCipher event indexes provide recent text
and complete unread counts. Native polling uses a small invalidation counter.
Views are owner-only and do not acknowledge agent inbox or transport work.

The actual native gate sent 1051 messages over Noise/MLS, checked the latest 50,
restarted the recipient daemon, read all 22 UI pages with exact original IDs/text
and no duplicates, then sent a reply. Core tests cover mixed control/text events,
large Unicode bodies, restart, cursor scope and read-only state. Independent
test review preceded production. The first native run caught unbounded live
accumulation; a separate frontend RED preceded its fix. Another frontend RED
preceded refresh of older queued delivery statuses. These failures are retained.

Full virtualized history and the rest of daily UX remain open. Independent
index/control logs, authenticated gaps, automatic sender placement, R10 and repair
remain required. Next transport work must preserve the original outgoing wire
before acknowledgment consumes the outbox; a plaintext row cannot reproduce its
original signed MLS packet. Full V1 is not accepted. The 64-validator/R24 engineering
gate remains RED and parked; orders/ratings/reviews remain V2.

## Previous verified application — durable recipient traversal

On 576 unchanged source inputs, 684 Rust tests passed (zero failed/ignored, four
threads), with 51 frontend tests, model suites, fmt/Clippy, TypeScript and frontend
build. Eight actual Tauri command tests and all six hidden WKWebView scenarios
passed. Five screenshot pairs were personally inspected. The ordinary macOS arm64
app was rebuilt, ad-hoc codesign verified and checked without test driver/helpers.
It is not notarized; Linux/Windows acceptance remains open.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Evidence, source hashes and binary hashes: `evidence/reviews/D05-durable-progress/`.

With the sender stopped, an independent Noise fixture delivered 128 original MLS
messages (5748073 ciphertext bytes) over 26 pages. Bob was killed after 10 committed
messages while the third request was held; its first signed restart cursor was 10.
The 17th resumed request required a new lookup/visit, preserving the existing
16-page/120-second work limit. All original text/authors/IDs arrived once, using
27 requests including the held request. The harness only observed Bob. This fixture
claims traversal/restart behavior, not paid storage or authenticated completeness.

Core persists a bounded source-specific bookmark with the last page message in the
same SQLCipher transaction. Actual progress INSERT/UPDATE and final-message INSERT
failures, restart/retry, source isolation, stale responses, real page limits and
out-of-order reconciliation passed. The shared page validator serves ordinary and
automatic reads. The 33-message Noise regression also passed.

Fresh genuinely paid 872-byte ciphertext retrieval passed with sender absence,
custodian/recipient restart, SQLCipher failure/retry, unavailable endpoint and one
DHT cache absent. The complete new RISC0 receipt is retained and independently
reverified with the packaged verifier; wrong operation and expiry are rejected.
Two historical genuine receipt compatibility checks passed as well.

Independent index/control logs, authenticated gaps, automatic sender placement,
R10 and repair remain open. The ordinary desktop also needs bounded recent history
and pagination: current Core snapshots expose the first 1000 records per dialog,
so later messages can be absent, and large combined histories can exceed the IPC
response limit. Full D05/E05–E07 and V1 are not accepted. The 64-validator/R24
engineering gate stays RED and parked.

## Previous verified application — automatic recipient retrieval

On 572 unchanged source inputs, 678 Rust tests passed (zero failed/ignored, four
threads), together with 51 frontend tests, models, TypeScript, frontend build,
fmt/Clippy, eight Tauri command tests and all six hidden WKWebView scenarios.
Five screenshot pairs were personally inspected. The ordinary macOS arm64 app was
rebuilt and ad-hoc codesign verified without test driver/helpers. It is not notarized;
Linux/Windows acceptance remains open.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Hashes, manifests, logs, native/visual results and live network evidence:
`evidence/reviews/D05-automatic-retrieval/`.

With the sender stopped, the returning daemon automatically discovered its private
pointer and retrieved the original genuinely paid 872-byte MLS ciphertext. One
unavailable endpoint, actual SQLCipher read failure/retry, custodian restart and
recipient restart with one DHT cache absent passed, preserving one original message.
The recipient harness only observed state; it did not perform lookup/dial/read/import.
A separate independent Noise fixture delivered all 33 original MLS messages over
advancing pages and deduplicated after restart. Three QC signatures and the storage
receipt were independently verified. The full fresh RISC0 proof is retained and
reverified by the independent oracle and packaged verifier; operation substitution
and expiry fail closed. Two historical receipt compatibility checks also passed.

A full-run regression showed background mailboxes occupying both shared DHT slots.
An independently accepted real held-GET test preceded the fix: background discovery
now leaves one slot for peer/owner work. It and the original moved-contact scenario
pass in the complete run. Earlier RED and failed timing assertions remain evidence.

This remains one primary with explicitly orchestrated sender placement/publication.
The recipient's 16-page/120-second visits still reset their in-memory cursor on a
new visit/restart; large valid backlogs may not catch up. Durable progress, independent
index/control logs, authenticated gaps, automatic sender placement, R10 and repair
remain required V1 work. Full E05–E07 and V1 are not accepted. The 64-validator/R24
engineering gate remains RED and parked. Contract: `spec/custody-sync-v1.md`.

## Previous verified application — paid ciphertext transfer

On 567 unchanged source inputs, all 675 Rust tests passed (zero failed/ignored,
four test threads), with 51 frontend tests, model/EVM oracles, TypeScript, frontend
build, fmt and Clippy. Eight Tauri command tests and all six hidden WKWebView
scenarios passed. Five screenshot pairs were personally inspected. The ordinary
macOS arm64 app was rebuilt and ad-hoc codesign verified; test driver/helpers are
excluded and two historical genuine RISC0 receipt compatibility checks passed.
The app is not notarized; Linux/Windows acceptance remains open.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
All five binary hashes, source manifest, full regression logs, native results,
visual findings and live network evidence are in
`evidence/reviews/D04-paid-custody-network/`.

The ordinary daemon now stores a fresh, genuinely paid MLS ciphertext at its
selected primary over TCP/Noise. The live gate passed with the sender stopped,
custodian restart, recipient restart and exactly one original recipient message.
The source chain was stopped after proof renewal and before storage. Three QC
signatures and the complete transport-signed storage receipt were independently
verified. Real SQLCipher failures, copied receipts, forged cursors, malformed
requests, bounded held streams and deadline recovery passed as well.

This is one primary with explicit owner orchestration. Automatic retrieval,
independent indexes, R10 and autonomous repair remain open. This gate does not
claim a complete E05–E07 or V1 release. The prior intermittent announcement-test
failure remains documented in D02; the unchanged test passed in this full run.
Live evidence: `evidence/reviews/D04-paid-custody-network/network-evidence.json`.
Contract: `spec/paid-custody-network-v1.md`. Sender receipt verification is retained
separately in `evidence/reviews/D03-custody-receipts/`.

## Previous source checkpoint — paid private storage

On 564 frozen inputs, 671 Rust tests (zero failed/ignored, four test threads),
51 frontend tests, models, TypeScript, frontend build, fmt and Clippy passed.
Paid storage now retains actual ciphertext, the original signed receipt, spend QC
and authority/operator evidence atomically in SQLCipher. Genuine fixed-image proofs
gate admission; quota, paid bounds, selected roles, private reads, expiry/restart
and write-failure behavior are tested. Core and owner daemon APIs prepare finite
read capabilities using the incoming MLS exporter without changing conversation
state. Twelve new tests and six new genuine receipts support this checkpoint.

At that source/library checkpoint, custodian network transfer, automatic retrieval,
copy counts, independent indexes and R10 repair were open. Native UI and ordinary
release packaging were not rerun then; the preceding verified app was `000f8c9`.
The first full parallel run had one intermittent existing announcement-test failure;
the unchanged test passed twice separately and in the complete four-thread rerun.
Its cause remains unestablished; failure evidence is preserved. See
`evidence/reviews/D02-paid-custody/README.md` and `spec/paid-custody-store-v1.md`.

## Previous verified application — `000f8c9`

That 545-input source snapshot passed 659 Rust tests (zero failed/ignored),
51 frontend tests, TypeScript, model/EVM oracles, fmt and Clippy. All eight real
Tauri command tests and six hidden WKWebView acceptance flows passed. The new flow
uses two actual packaged MCP clients and independent daemons: private RFQ, signed
bid and bilateral acceptance, offline signed result, both daemon restarts and exact
retry, automatic desktop result, explicit owner validation, native creation and
live revocation. Screenshots were personally compared with accepted references.

The new private custody envelope wraps the entire existing signed MLS text/job
packet in direction/epoch-specific authenticated encryption. Exact prepared bytes
and their future postage operation hash survive restart; only recipient processing
can acknowledge delivery. Strict owner-only daemon APIs prepare/import the object,
including with the sender process absent. Eleven new tests cover independent crypto
vectors, real MLS/SQLCipher, bounded retries/expiry and actual daemon authorization.
This is a D01 prerequisite: network custodian storage/retrieval and R=10 repair
remain unimplemented. The newer paid store component is described above.
Contract: `spec/custody-envelope-v1.md`.

Private orders now share the existing encrypted MLS transport and transactional
persistence. Scoped agents and owners can create, bid, accept, submit, validate,
cancel, heartbeat, list and read. Response material requires current ReadArtifact
and finite byte authority before any mutation commits, including cached retries.
Accepted receipts persist the future review right. Public discovery, publication of
reviews, settlement, binary artifacts and reassignment remain open. Price and
transport budget are separate; payment is always not_funded.

The ordinary macOS arm64 app was rebuilt and ad-hoc codesign verified. All five
binary hashes, exclusion of test driver/helpers and two historical genuine RISC0
receipt compatibility checks are retained in `evidence/reviews/D01-private-envelope/`.
The preceding private order implementation evidence remains in `evidence/reviews/A02-private-jobs/`.
App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Native acceptance uses the debug test-feature bundle from the same application
sources; the ordinary release excludes that driver. Not notarized; Linux/Windows
and heavy genuine-spend/EVM E2E were not rerun here. Guide: `Docs/AGENT_JOBS_RU.md`.

Committee capacity remains unaccepted: one complete real 64-node TCP run passed,
its QUIC run missed five routes at the unchanged cold deadline; a later TCP
diagnostic missed two. Actual announcement retries hit shared processing capacity.
The test remains RED; no R24 production change is present. Evidence and critic
decisions: `evidence/reviews/N03-committee-capacity-open/`.

The bounded readiness audit and private MCP order gate are complete. The later
64-node engineering capacity target remains parked. Next product work is actual
ciphertext custody/offline R=10 delivery and repair, then remaining V1 groups/recovery,
credentials, transport economy and desktop/release acceptance. Agent orders/reviews/ratings are V2. See `Docs/V1_READINESS_2026_09_09_RU.md` for
requirements and scope. **Full V1 remains incomplete.**

### Previous verified DHT-role checkpoint

Ordinary nodes now explicitly run Kademlia as clients. Owners can offer DHT
service through the desktop network setting or `agentic-node serve --dht-server`.
Saved preferences override the CLI, including saved false. Existing profiles lacking
the field read false without rewriting original bytes. Validator authority remains
independent. Confirmed external addresses cannot silently promote the selected mode.
Relay-only suppresses direct DHT while retaining the preference for direct networking.

Core reuses encrypted persistence and revision/retry semantics; the node reuses
construct/persist/swap and the guarded cache. The desktop distinguishes saved
selection, unsaved edits and live state, including lost replies and stale revisions.
No dependency, wire protocol, quorum or resource limit changed.

All 600 Rust tests across 74 suites and 42 frontend tests
passed, zero failed or ignored Rust tests, with fmt/Clippy. R2 ACCEPT preceded
production. Baseline RED includes an actual decoded TCP Kad reply before new
diagnostics where refusal was required; Core RED was missing-field compilation,
frontend RED was the absent control. R3 accepted native coverage, R4 adapted the
exact Tauri schema, R5 explicitly configured three DHT servers in the old finalizer
fixture while retaining its recovery subject as a client. Failed runs and exact
review/source hashes are retained. See review-history.md.

Independent stock Kad probes verify replies/refusals over TCP/Noise and QUIC,
including enable/demote, saved restart and actual external-address confirmation
through successful relay reservations. Real encrypted failures preserve role,
revision and queued MLS delivery. Existing moved-recipient and sender-offline
mailbox scenarios pass with explicit ordinary DHT servers.

Real TCP/QUIC finalizer regressions retain 64 ordinary connections and unchanged
quorum, certify 15 effects per selected profile and refuse the next effect after
actual expiry. Signed cache isolation, crashes, real Marshal cursors, cross-scope
replay refusal and processing load controls remain. Genuine client and announcement
regressions generated four fresh proofs, verified QCs and passed cold pending and
unseeded moved-address recovery. Selected clients discover through two ordinary
DHT servers with no operator registrations. Separate controls toggle validator
authority independently of DHT service.

The hidden macOS WKWebView gate passed five product scenarios using the packaged
daemon and real Tauri commands. DHT UI checks cover enable, full daemon restart
with the same identity, disable and actual relay-only policy. Native DHT screenshots
were visually inspected. Separate headless component screenshots at 1440×1100 and
850×650 checked layout; these are UI fixtures, not network evidence.

All 506 source inputs stayed fixed through the normal Tauri release
build, ad-hoc codesign, test-driver/helper exclusion and two historical genuine
receipt compatibility checks. App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
The app is not notarized; Linux and Windows native matrices were not rerun.

Evidence: `evidence/reviews/N03-dht-roles/`; contract: `spec/dht-roles-v1.md`.
Larger committee route/cache capacity, private ciphertext custody and R=10 repair,
authenticated spent-state handover, remaining UI/MCP and full E01–E26 acceptance
remain open. **The full V1 product goal is not achieved.**

### Earlier verified checkpoints

All ten application request/response protocols now share one per-swarm processing
budget. Authenticated selected peers retain 48 operations alongside 16 ordinary;
per-peer, weighted-byte and independent rate limits apply before decoding/writing.
Actual worker codecs retain charges while waiting for responses. Original signed
sources govern in-flight IO; revocation does not release memory or revive old tokens.
Unverified frame claims are refused before expensive profile reconciliation and cannot
occupy the selected frame-admission table. Existing scope responses and Core checks remain.

All 596 Rust tests across 74 suites and 40 frontend tests passed,
with zero failed or ignored Rust tests and fmt/Clippy. Independent R2 ACCEPT preceded
production; R3/R4 accepted test refinements. Actual old-code TCP failed early-refusal
before new diagnostics; all held/idle RED behavior was retained. Failed development
traces and their corrections are preserved; original assertions remain unchanged.

Actual TCP/Noise and QUIC retain 64 ordinary peers, cap incomplete readers at 16,
reuse slots after real timeouts and independently certify fresh effects 13/14 under
held/completed traffic. Completed traffic triggers rate refusals and never enqueues
ordinary selected-key claims. All four profiles reach 15 effects; effect 16 stays refused
after real authority expiry. Previous replay/SQL/cursor/cold/revocation controls remain.
Genuine client TCP/QUIC and announcement/DHT/genuine-spend regressions also passed,
including four fresh proofs, automatic client discovery and unseeded cold address recovery.

All 503 source inputs stayed exact through the ordinary Tauri rebuild,
ad-hoc signing, driver/helper exclusion and historical receipt checks. Native UI/Linux
were not rerun; no UI changed here and the app is not notarized.

Evidence: `evidence/reviews/N03-processing-reservations/`; contract:
`spec/processing-reservations-v1.md`. DHT roles, larger committee route/cache alignment,
actual private ciphertext custody / R=10 repair, spent-state handover, remaining UI/MCP
and complete E01–E26 acceptance remain open. **The full V1 product goal is not achieved.**

Ordinary postage clients now discover selected nodes from current public committee
keys when peers is omitted or empty. Explicit PeerID pins must match an authenticated
binding. Only genuine-verified pending requests create discovery interest and finite
connection reservations, fenced by both current public authority and the individual
request. Durable completion releases the request's source; failed SQL commits retain
it. Local services and other requests sharing a peer remain independent.

All 589 Rust tests across 74 suites and 40 frontend tests passed,
with zero failed/ignored Rust tests and fmt/Clippy. R1 independent ACCEPT preceded
production. The reviewed tests stayed exact; baseline RED was a missing private API
at compilation plus omitted peers rejected by owner API schema before genuine proofs.

The full genuine client gate preserves its original fresh ingress/consensus and failure
checks. New TCP/Noise and QUIC ordinary clients discover 4 selected keys through 2 seeds,
cold-restore a pending request and original signed addresses with all selected nodes
and seeds offline, then preserve 64 ordinary clients while selected nodes rejoin without
seeds. Actual SQL failure holds the genuine result pending. Durable completion releases
the reserve; a second automatic request reacquires it and verifies the winning conflict
QC. A wrong explicit PeerID pin grants neither reserve nor work. The new clients retrieve
an already certified result; the original gate proves fresh consensus. Client gate:
30 signature checks, 2343 owner calls, two fresh genuine proofs.

Local-service TCP/QUIC and announcement/DHT/genuine-spend regressions also passed,
including 13 effects per selected profile, 64 ordinary fan-in, revocation/reuse and
unseeded cold recovery. All 497 source inputs stayed fixed through the ordinary
Tauri rebuild, ad-hoc codesign, driver/helper exclusion and historical receipt checks.
Native UI/Linux were not rerun; the app is not notarized.

Evidence: `evidence/reviews/N03-client-discovery/`; contract:
`spec/postage/client-discovery-v1.md`. Shared processing capacity, DHT roles, larger
committee route/cache alignment, actual private ciphertext custody/R=10 repair,
spent-state handover, remaining UI/MCP and complete E01–E26 acceptance remain open.
The full V1 product goal is not achieved.

Selected local services now retain bounded connection capacity under 64 ordinary
clients. Signed current service hints or newer verified routes identify eligible peers;
Core authority and finite leases govern the reserve. The existing ordinary and per-peer
limits remain, with 192 total established and 48 pending outgoing as absolute ceilings.
Revocation retires excess newly ordinary sockets while preserving existing ordinary
clients; capacity returns only after actual close events.

All 588 Rust tests across 74 suites and 40 frontend tests passed,
with zero failed/ignored Rust tests and workspace fmt/Clippy. Seven new lifecycle tests
preceded implementation and independent R2 ACCEPT. Actual old-code TCP failed with 64
ordinary connections on late selected reconnect. R3 accepted an equivalent test iterator
lint fix. The first implementation exposed a real demotion-order defect in the same
live test; its failing trace is retained. The corrected full TCP/Noise and QUIC runs
both preserved 64 ordinary clients, admitted 3 selected peers, refused/reused a 65th
ordinary client, revoked/restored the reserve and finalized 13 effects per selected profile.
No test assertions were weakened for the correction.

Announcement/DHT/paid-finality regression also passed two fresh genuine proofs and
unseeded cold service recovery. All 495 source inputs stayed fixed through ordinary
Tauri rebuilding, ad-hoc codesign, test-helper exclusion and historical receipt checks.
Native UI/Linux were not rerun; the app is not notarized.

Evidence: `evidence/reviews/N03-reserved-connections/`; contract:
`spec/reserved-connections-v1.md`. Ordinary-client reservations and shared processing
capacity still require separate test-first work. DHT roles, larger committee route/cache
alignment, actual private ciphertext custody/R=10 repair, remaining UI/MCP and complete
E01–E26 acceptance remain open. The full V1 product goal is not achieved.

Selected service addresses now survive process and network restarts in the existing
SQLCipher profile. The node reauthenticates original signed records on load, preserves
sixteen bounded version floors and refuses stale or conflicting addresses. Core commits
opaque cache bytes and its checkpoint clock atomically; no owner/MCP mutation API was
added. Finite expiry, current membership and actual transport checks remain mandatory.

All 581 Rust tests across 73 suites passed (zero failed/ignored), with 40
frontend tests and fmt/Clippy. Seven new tests cover actual encrypted cold restore,
clock/CAS/SQL failures, stale writers, transport rotation, signature poisoning,
well-formed persisted signature corruption, shorter expiry and 16→17 capacity controls.
Critic R2 accepted the tests after two substantive revisions; R3 accepted a narrow
SQLite test-helper type correction before production. Corrected baseline compilation
failed on absent cache APIs; the cache API baseline RED is compilation-only. All 41 focused Core and five node
tests then passed. All final reviewed test inputs remained exact, with only runtime.rs
production module registration normalized for comparison.

Actual announcements/DHT/paid-finality gate passed two fresh genuine receipts,
12 QC signature checks and 3365 owner calls. All four selected
daemons cold-restored exact service routes with both DHT seeds offline after removing
only their ordinary bootstrap caches; all other profile states remained exact. No
selected endpoint was injected. Full TCP/Noise and QUIC service regression passed 13
durable effects per profile, 72/69 signature checks and 911 owner calls.
Both gates cleaned up without errors. The first TCP cold-replay failure is preserved.
A separately reviewed real 1100ms
callback reproduced false authority revocation before the fix; time now propagates from
the fresh post-effect check to subsequent slots and discovery. Both transports then
kept both cold-started services at generation 1 and recovered the saved effect without
quorum. No genuine expiry, rollback or failure-before-ACK guard was removed.
All 490 source inputs stayed fixed through the ordinary Tauri rebuild, ad-hoc codesign, driver/helper exclusion and historical receipt
verification. Native UI/Linux were not rerun; the app is not notarized.

Evidence: `evidence/reviews/N03-service-cache/`; contract: `spec/service-cache-v1.md`.
Next: reserved committee capacity, DHT client/server roles, ordinary-client selected-key
inputs, actual private ciphertext admission/storage and R=10 repair, then remaining
UI/MCP and E01–E26 acceptance. The full V1 product goal remains open.

Core now persists a common selected-transport binding floor for compact and full-proof
verification. Older and conflicting equal-issued bindings are refused after restart
and genuine checkpoint/roster renewal. Current membership and actual transport checks
precede the floor; it commits atomically with the durable checkpoint clock. A shorter
new lease cannot revive an older still-live binding. Exact duplicates retain their
original expiry and do not rewrite the floor at unchanged observed time.

All 574 Rust tests across 73 suites passed (zero failed/ignored), with 40 frontend tests
and workspace fmt/Clippy. Five new tests and the reconciled existing deadline test use
real signed two-chain fixtures, SQLCipher reopen, current rosters and actual SQL write
failures. The live TCP/Noise and QUIC gate passed thirteen durable effects
per selected profile, 81/75 independently verified signatures and
936 owner calls, cleanup errors empty. All 483 source inputs
stayed fixed through the ordinary Tauri rebuild, ad-hoc codesign, driver exclusion
and both historical genuine receipt checks.

Evidence: `evidence/reviews/N03-binding-floors/`; contract:
`spec/finalizer/binding-floors-v1.md`. Independent R2 ACCEPT preceded production;
R3 accepted a one-line test signer API correction after a preserved compile failure.
Actual behavioral RED then recorded 33 passed/six failed, followed by 39 focused GREEN.
The first live TCP run failed at an owner checkpoint CAS race; QUIC was not started.
R4 accepted an explicit Core interleaving and a bounded exact-conflict-only harness
retry with a forced real daemon conflict. All production remained unchanged; the
failed run is retained. The complete rerun passed with all prior assertions preserved.
All four final reviewed test/spec files stayed byte-exact. The critic's optional capacity
and persisted-corruption controls remain untested. Native UI, Linux, new proof creation
and the complete announcements/DHT paid gate were not repeated in this slice;
the ordinary application is not notarized.

Next: persist signed service addresses with version/clock protection, reserve committee
capacity, separate DHT roles and support ordinary clients without supplied selected
PeerIDs. Private ciphertext admission/storage, R=10 repair, UI/MCP and the remaining
full V1 scenarios remain open. The full product goal has not been achieved.

Ordinary daemons now exchange bounded active-service announcements with connected peers.
Untrusted labels filter local interest before current Core membership, finite binding and
actual transport checks. The shared active-key batch feeds DHT publication, deduplicates
keys across local applications and invalidates on role changes. Automatic selected-key ×
neighbor probing is removed; explicit full-proof requests and known-route renewal remain.

All 569 Rust tests across 73 suites passed (zero failed/ignored), with 40 frontend tests
and workspace fmt/Clippy. Actual TCP/QUIC preflight passed producer authentication,
copied-binding refusal, thirteen ordinary neighbors including a silent peer, eighth/ninth
request admission controls, role disable/re-enable on one connection and direct-path
refusal under relay-only. Fourteen initial TCP peer queries required two interested binding
checks. Two fresh genuine receipts then passed unchanged DHT/address-move/cold-recovery
and paid-finality acceptance: twelve QC signatures, 2,428 owner calls, no
cleanup errors. No selected endpoint was supplied.

The extended full TCP/Noise and QUIC service gate preserved all previous checks and added
ordinary producer batch deduplication: thirteen durable effects per selected profile,
78/78 verified signatures, 914 owner calls, no cleanup errors.
All 481 source inputs stayed fixed through the ordinary Tauri rebuild, ad-hoc codesign,
driver exclusion and both historical genuine receipt checks. Native UI and Linux were not
rerun in this slice; the app is not notarized.

Evidence: `evidence/reviews/N03-service-announcements/`; reviewed contract:
`spec/service-announcements-v1.md`. Critic R2 ACCEPT preceded actual compile RED and
production. The first live failure was an asynchronous QUIC-listener readiness race in
the test; the full failed attempt remains preserved. R3 accepted only that predicate
correction with the same deadline, all production unchanged; the complete rerun passed.

Next: durable service-binding/address rollback protection, reserved committee capacity,
DHT roles and ordinary-client unknown-peer inputs. Private ciphertext custody, paid
admission and R=10 repair, UI/MCP flows and all remaining full V1 scenarios remain open.

The ordinary daemon now publishes and resolves signed selected-service addresses through
the existing bounded Kademlia transport. Mailbox and service queries share two active
slots and sixteen retained results, with separate consumers and signature/expiry checks
at consumption. A service route requires current Core membership and the actual
authenticated connection; DHT hints cannot grant authority.

All 565 Rust tests across 73 suites passed (zero failed/ignored), with 40 frontend
tests and workspace fmt/Clippy. A real four-daemon gate used only two ordinary DHT seeds,
found exact selected keys without supplied endpoints, recovered a changed address and
cold restarts, and finalized a genuine paid operation with twelve independently checked
signatures. Two fresh genuine receipts, 1,696 owner calls and no cleanup errors. The
unchanged TCP/Noise and QUIC service gate also passed thirteen durable effects per
profile, 81/72 verified signatures and 899 owner calls, with no cleanup errors.

All 477 source inputs stayed fixed through the rebuilt ordinary Tauri application.
Ad-hoc codesign, driver exclusion and both genuine historical receipt checks passed.
Native UI and Linux were not rerun in this slice; the app is not notarized.

Evidence: `evidence/reviews/N03-service-kad/`; contract: `spec/service-kad-v1.md`.
Six tests were accepted by the independent no-context critic before production and
actual compile RED. Four accepted files stayed exact; runtime.rs retained its exact
test registration after normalization of four production additions. Bounded announcements
and removal of legacy key/peer probing are next. Persistent route protection, reserved
connections, DHT roles, ciphertext custody/repair and the full V1 product remain open.

Signed finalizer address records now use the existing libp2p SignedEnvelope and
finite P256 binding. Both signatures, lookup scope and transport PeerID are checked;
the public record contains no chat/root identity. The existing DHT store reserves
32 service entries alongside 128 private mailbox entries, with independent quotas,
original deadlines and ordered binding/address versions. A hint grants no selection
or custody authority.

All 559 Rust tests across 73 suites passed (zero failed/ignored), with 40 frontend
tests and workspace fmt/Clippy. Existing real daemon mailbox and routing tests passed,
including offline sender/cache loss and moved-contact recovery through intermediaries.
All 473 source inputs stayed fixed through the rebuilt ordinary Tauri application.
Ad-hoc codesign, driver exclusion and both genuine historical receipt checks passed.
Native UI, Linux and the specialized finalizer EVM network gate were not rerun in
this slice; the app is not notarized.

Evidence: `evidence/reviews/N03-service-records/`; contract:
`spec/service-records-v1.md`. Eight tests were accepted by the independent no-context
critic before production changes and actual compile RED. Five reviewed files stayed
exact; runtime.rs retained its exact test registration while gaining the production
module declaration. Automatic service publication/query dispatch, announcements,
unknown-PeerID committee formation, persistent route protection, reserved connections
and DHT roles remain open, followed by the remaining full V1 product work.

Compact finalizer bindings now share the existing signed wire and verifier. Core
returns only enabled local selected keys and verifies received bindings against its
own current proven roster and actual transport identity. A stateless signed hint
proves authorship without granting membership. Both Core APIs retain durable clock
fences; current-head renewal never extends an old binding's expiry.

All 551 Rust tests across 73 suites passed (zero failed/ignored), with 40 frontend
tests and workspace fmt/Clippy. The unchanged real TCP/Noise and QUIC selected
service gate passed 13 effects per selected profile, 66/60 independently checked
signatures and 1,082 owner calls, cleanup errors empty. All 468 source and five
accepted test/spec inputs stayed fixed through the rebuilt ordinary Tauri app.
Ad-hoc codesign, driver exclusion and genuine historical receipt compatibility
passed. Native UI and Linux matrix were not rerun; the app is not notarized.

Evidence: `evidence/reviews/N03-compact-bindings/`; contract:
`spec/finalizer/compact-discovery-v1.md`. These are compact Rust primitives;
automatic daemon discovery still uses its existing algorithm. Next are signed
address records, bounded announcements, actual unknown-PeerID DHT lookup, cache
rollback protection, committee capacity and network roles. Full V1 remains open.

Private custody placement now works through the ordinary owner API
`plan_postage_custody`: a genuine fixed-image receipt yields a deterministic,
verified registry assignment without exposing a funded owner opening. Actual
membership proofs, paid class bounds and retained registry snapshots are checked.
The response explicitly has `admission: false`; ciphertext storage and repair are
still unfinished.

Validation passed 545 Rust tests (zero failed/ignored), 40 frontend tests, seven
models, nine oracles and workspace fmt/Clippy. The new live placement gate passed
with two fresh genuine receipts, 223 owner calls, exact 10+4 positions across cold
restart and zero queued spends. The existing paid-client regression passed with
1,283 owner calls and 18 QC signature checks, including real expiry and cold
offline results. Both runs had no cleanup errors. The ordinary Tauri bundle was
rebuilt and passed ad-hoc codesign, driver exclusion and historical receipt checks;
it is not notarized. Native UI and Linux matrix were not repeated in this slice.

All code/tests and 29 accepted inputs stayed fixed. A user-authorized architecture
handoff changed one Markdown input during validation and added another document.
The original manifest barrier stopped the wrapper after all eight actual test
gates passed. That failure and the exact documentation transition are preserved;
release gates then passed against the updated 465-input manifest. See
`evidence/reviews/D01-private-placement/` and
`spec/postage/private-custody-placement-v1.md`.

Next: implement the accepted service-discovery direction in
`spec/service-discovery-v1.md`, then actual paid ciphertext admission/storage,
offline retrieval and R=10 repair. Authenticated spent-state handover, UI/MCP flows
and all remaining E01–E26 acceptance remain open.

The real 100-request gate now passes on 13 ordinary clients and four selected
daemons: two fresh genuine receipts compete for one paid ticket. All 100 requests
persist before sending; after an actual 2+2 partition and healing,50 winning retries
and 50 independently verified conflicts share one finalized operation. Every actual
durable consensus cursor remains 1. Capacity refusal/reuse, exact retries and cold
offline results pass, with 6,025 owner calls and 315 signature checks (including
repeated checks of the winning certificate), cleanup[].

The first run failed because selected-peer discovery spent its bounded probes among
13 ordinary clients. Three separately reviewed scheduling tests preceded alternating
preference for owner-configured bootstrap hints, with independent progress for other
peers. Critic R 1 rejected insufficient one-hint fairness coverage; R 2 accepted four
wrong hints plus all 39 unhinted peer/key pairs. The original failed live run is
retained. Existing timing, authority, proof and receiver limits were not increased.

Validation passed 539 Rust tests (zero failed/ignored),40 frontend tests,7 models,
9 independent oracles, workspace fmt/Clippy, TypeScript/Vite and the unchanged full
TCP/QUIC finalizer gate:13 effects per profile,1,155 owner calls and 78/69 checked
signatures, cleanup[]. All 451 source and 16 accepted test inputs stayed fixed through
validation and the ordinary Tauri app rebuild. Deep/strict ad-hoc codesign, driver
exclusion and genuine historical receipt compatibility passed. It is not notarized;
native UI, Linux matrix and all remaining V1 scenarios were not rerun.

Evidence: `evidence/reviews/P03-concurrent-spend/`; contract:
`spec/postage/concurrent-spend-v1.md`. The full V1 goal remains incomplete. Next are
private paid ciphertext admission/custody/repair, authenticated spent-state handover,
UI/MCP payment flows and the remaining E01–E26 product scenarios. The present result
is a spend certificate and grants no ciphertext storage admission.

Ordinary daemon profiles without operator keys now submit genuine postage receipts
through `/agentic-internet/postage-submissions/1`. Core shares canonical issuer,
roster and receipt checks with selected services. Owner IPC retains at most eight
active requests, persists before sending, verifies actual selected P256/Noise routes
and QC ancestry, and persists a verified result before publication. Exact retries
and conflicting-operation results survive cold restart, including after real
checkpoint expiry. Historical reads preserve the original finite authority without
creating a signer or granting current custody admission.

The full client gate passed with four selected daemons and a fresh ordinary client:
two genuine proofs, 1,167 owner calls and 18 checked QC signatures. Actual SQL request
and result failures, a forged result from a genuinely selected peer, malformed
public ingress, a 2+2 partition, healing and cold expired reads passed. The existing
candidate-network and ordinary-spend regressions passed four more fresh proofs,
1,877/929 owner calls and 12/12 checked signatures, with no cleanup errors.

The first client live failure revealed an incorrect historical lower time bound;
a separately accepted genuine-QC regression preceded its correction. The following
required finalizer TCP gate exposed discovery starvation under staggered peer
cooldowns. A real diagnostic reproduction and three separately accepted scheduling
tests preceded per-peer target progress. The unchanged full TCP/QUIC gate then
passed 13 effects per profile, 1,136 owner calls and 66/66 checked signatures without
extending deadlines or limits. Both failures and the original Clippy failure remain
in the evidence rather than being replaced by successful reruns.

Validation passed 536 Rust tests (zero failed/ignored), 40 frontend tests, workspace
Clippy/fmt, plus the previously unchanged seven model and nine oracle checks.
TypeScript/Vite passed in the final build. A reviewed one-line cfg(test) Clippy
annotation was the only change after full Rust/TCP/QUIC validation; all production
inputs stayed identical and the exact transition is recorded. All 449 final source
and 13 accepted-input hashes remained fixed through the fresh client/network/spend
gates and package build.

The rebuilt ordinary Tauri app passed deep/strict ad-hoc codesign, automation-driver
exclusion and bundled-verifier compatibility with both genuine historical receipts
and the unchanged fixed image. It is not notarized. Native UI, Linux network matrix
and the remaining E01–E26 cases were not repeated. Evidence:
`evidence/reviews/P03-client-submission/`; contract:
`spec/postage/client-submission-v1.md`.

The full V1 goal remains incomplete. Next are 100 concurrent attempts with verified
single-effect results, actual paid ciphertext custody/repair, authenticated
spent-state handover, UI/MCP payment flows and the other required product scenarios.
The current result is a finite spend certificate, not resource admission; retained
candidate and consensus-prefix limits remain enforced.



Selected ordinary daemons now distribute genuine postage receipt candidates over
`/agentic-internet/postage-candidates/1`. The receiver verifies its canonical scope,
actual selected Noise/P256 peer, current public context, fixed-image receipt and
journal-derived candidate ID before durable import. Both consensus and bulk writes
share revocation/expiry fences; a transport acknowledgement never becomes finality.

The new four-daemon live gate passed with two accepted owner seed submissions,
automatic selected-peer distribution, two fresh genuine proofs, 2,607 owner calls
and twelve independently checked QC signatures. A corrupted genuine receipt from
a selected identity was rejected before persistence. Foreign/oversized input,
recipient crashes, stable duplicate counts, a real 2+2 partition, role disable and
re-enable, convergence to one spend and cold remote result recovery all passed.
The first live run failed at re-enable delivery. Pacing automatic route discovery
below the existing proof admission limits fixed it without extending the timeout;
the failed evidence is retained. Independent no-context test review accepted the
original scenarios and the later diagnostics-only wrapper before the correction.

Final checks passed 525 Rust tests (zero failures/ignored), forty frontend tests,
seven model tests, nine independent oracle tests, workspace fmt/Clippy, TypeScript
and Vite. The selected-service TCP/QUIC regression passed thirteen effects per
profile, 955 owner calls and 75/69 independently checked signatures. The original
ordinary-daemon spend regression also passed on the final sources: 820 owner calls,
another two genuine proofs, twelve checked signatures, actual SQLCipher failure
before acknowledgement, role revocation and cold recovery without peers. All runs
reported no cleanup errors. The 439 frozen source and three accepted test inputs
remained unchanged through backend/live validation and the final package build.

The ordinary Tauri app was rebuilt and passed deep/strict ad-hoc codesign verification,
automation-driver exclusion and bundled-verifier compatibility with both genuine
historical receipts and the unchanged fixed image. It is not notarized. Native UI,
Linux network matrix and the remaining full E01–E26 scenarios were not repeated.
Evidence: `evidence/reviews/P03-candidate-network/`; contract:
`spec/postage/candidate-network-v1.md`.

The whole V1 goal remains incomplete. Selected-replica distribution is implemented;
nonmember-client ingress, saturation/100-request acceptance, cold historical reads
without live selected authority, authenticated spent-state handover, actual paid
ciphertext custody/repair, UI/MCP payment flows and the remaining V1 product
scenarios are still required. Retained inputs are capped at sixteen and the
consensus prefix at 128; receipt/finality responses grant no custody admission.


The ordinary daemon now runs the built-in issuer-bound postage spend service.
Owner IPC configures its canonical selected scope, submits genuine public receipts
and returns exact durable finality evidence. Application decisions run through a
bounded mailbox to the Core actor. Encrypted configuration/candidates survive
restart; SQLCipher spent commits precede Commonware acknowledgement. Existing
locally compiled application registrations retain their API.

The new live gate passed with four freshly enrolled selected daemon profiles, an
actual canonical-issuer purchase, two genuine fixed-image competing receipts and
700 owner calls. Isolated TCP pairs produced no finality. Healing agreed one spend;
twelve QC signatures were independently checked. A real SQL insertion failure held
the durable consumer cursor at zero; disabling the role prevented commit after
removing the fault. Cold recovery with all other peers stopped restored the first
result without re-submission. Exact retry and competing-operation refusal passed.
The source chain was offline during voting; cleanup reported no errors.

Tests received separate no-context FINAL ACCEPT after correcting a test startup
bug. All 525 ordinary Rust tests passed (zero failed/ignored), plus forty frontend
tests, seven models, nine oracles, workspace fmt/Clippy, TypeScript and Vite.
The 437 frozen source inputs and five accepted test inputs remained unchanged.
The existing selected-service TCP/QUIC regression also passed: thirteen effects
per profile, 872 owner calls and 75/87 independently verified signatures; no cleanup
errors. The ordinary Tauri release was rebuilt, deep/strict ad-hoc signature
verification and driver exclusion passed, and its bundled verifier passed both
genuine historical receipt compatibility checks with the unchanged fixed image.
The app is not notarized. Native UI, Linux network matrix and remaining full V1
E01–E26 scenarios were not repeated or declared complete.

The whole V1 goal remains incomplete. Current owner ingress distributes candidates
to each replica explicitly. Remote candidate distribution, bounded overload and
100-request acceptance, cold historical reads without live authority, authenticated
spent-state handover, actual paid ciphertext custody/repair, UI/MCP payment flows
and other V1 product scenarios remain required. Retained inputs are capped at
sixteen; the existing consensus prefix is capped at 128. Finality responses do not
grant custody admission. Evidence: `evidence/reviews/P03-daemon-spend/`; contract:
`spec/postage/daemon-spend-v1.md`. The preceding library checkpoint follows.

The new Rust-only `agentic-postage-spend` module validates genuine fixed-image
receipts under current Core issuer/service authority, uses nullifier as the
canonical consensus key, and persists the exact first finalized result in
SQLCipher. The full journal binds the actual operation and resources. The same
prefix rule governs proposal and vote validation; actual QC ancestry is required
before any effect. Historical reads authenticate stored evidence, including from
a fresh session opened later. There is no refund/cancel or empty later-epoch start.

Nine targeted tests pass, including genuine competing-operation receipts, real
four-engine P256 consensus under a 2+2 partition followed by healing, exact retry,
cold-session restoration, corrupt saved proof, SQL transaction/clock failure,
role/head revocation and a genuine subsequent-epoch authority rejection. Test
review required two revisions, then independent FINAL ACCEPT before production;
the sole post-GREEN lint correction removed one unused `mut` and received ACCEPT.
Full ordinary validation passed 524 Rust tests (zero failed/ignored), forty frontend
tests, seven models, nine oracle tests, workspace fmt/Clippy, TypeScript and Vite.
All 433 final source hashes and fifteen accepted input hashes stayed unchanged.

This is an application-policy/effect boundary, not yet a daemon spend service.
Receipt ingress/distribution, the 100-request daemon scenario, admission tied to
real custody work, epoch handover and full V1 remain unfinished. No desktop/UI/MCP
spend endpoint was added, and the ordinary app was not rebuilt for this library-only
increment. Evidence: `evidence/reviews/P03-spend-decisions/`; contract:
`spec/postage/spend-decisions-v1.md`. The preceding application checkpoint follows.

Finalized service deliveries now include a portable canonical ancestry proof.
The same bounded exporter reads finalized Marshal archives for both Running and
the embedded service. Ed25519/P256 consumers verify the exact target, hash/height/
scope chain and real descendant QC under current finite committee authority.
An ancestor still has no fabricated direct certificate; effect acknowledgement
and historical replay semantics are preserved.

Nine frozen test/contract inputs received independent no-context FINAL ACCEPT
before production. The 42 targeted tests pass, including actual archive recovery
without peers and real four-service fault scenarios. Final ordinary validation
passes 515 Rust tests (zero failed/ignored), seven models, nine oracles, fmt/Clippy,
forty frontend tests, TypeScript and Vite. The initial Clippy loop-counter failure
and its manifest are retained; its correction changed no accepted test.

The actual selected-daemon EVM gate passed TCP/Noise and QUIC: four selected
profiles per transport converge on thirteen effects, with 81/87 independently
verified certificate signatures and 850 owner calls. Cross-scope replay, invalid
proposals, effect failure/cursor ordering, cold replay and finite role/head/network
revocation pass; the source chains stop during voting and cleanup has no errors.
This is a local signed-work fixture application using production node code,
not production postage admission. All 420 final source hashes stayed fixed.

The ordinary release app was rebuilt and passed deep/strict ad-hoc codesign,
driver exclusion and both genuine historical receipt compatibility checks with
the unchanged fixed image. It is not notarized. Native UI, Linux network matrix,
remaining full EVM and new-proof suites were not repeated. Evidence:
`evidence/reviews/P03-finalized-ancestry/`; contract:
`spec/finalizer/ancestry-proof-v1.md`. Canonical spent state, epoch handover,
paid custody/repair and full V1 remain unfinished. The preceding current service
authority checkpoint follows.

Core now prepares a Rust-only issuer-bound postage service authority. Every request
verifies its explicit binding proof at the exact current authenticated checkpoint
against the installed P256 policy. Canonical application/log IDs are derived from
the issuer, with no caller-selected scope. Both preparation paths share full roster
validation, selected enabled keys, revocation fences and durable clock commits.

The four test-first scenarios received separate no-context FINAL ACCEPT after a
stale-proof case was strengthened. All 24 Core finalizer tests passed. A fresh
actual two-chain Anvil gate passed six binding and eight selection CLI checks,
then the four Core scenarios after both chains stopped. A real purchase changes
the issuer root while preserving its policy; only the fresh proof is accepted.
Validation passed 507 ordinary Rust tests (zero failed/ignored), seven models,
nine independent oracle tests, fmt/workspace Clippy, forty frontend tests,
TypeScript and Vite. All 415 source and twelve accepted input hashes stayed fixed.

The ordinary release app was rebuilt, passed deep/strict ad-hoc codesign and excludes
the automation driver. Its verifier accepts both prior genuine receipts at their
historical time, preserving the fixed zkVM image and full independent journals;
operation substitution and expiry fail. No new proof was generated. The app is not
notarized. Native UI, remaining full EVM gates, Linux network matrix and new-proof
process suites were not repeated for this Core-only increment. Evidence:
`evidence/reviews/P03-current-spend-service/`; contract:
`spec/postage/current-spend-service-v1.md`.

This prepares current service authority; it starts no spend runtime, creates no
spent ledger and grants no admission. Issuer-global spending, epoch handover,
paid custody/repair, UI/MCP spending and full V1 remain unfinished. The preceding
issuer deployment and fixed-image compatibility checkpoint follows.

CanonicalPostageIssuer now requires a one-time deployer binding to a nonzero
canonical P256 finalizer selection policy before any purchase. It preserves the
existing funded-ticket ABI, domain/leaf format and slots 0..3; slots 4 and 5 bind
policy and issuer domain. Replacement and issuance under a changed chain ID fail.
The host Rust verifier authenticates those exact words against a supplied root and
the selected deployment; its strict CLI explicitly returns admission:false.

All 33 contract tests passed. The actual two-chain Anvil scenario passed 57 policy
CLI checks, four funding checks and two independently expected finalizer selections
after both source chains stopped. Exact payment, real rollback/replacement, restart,
foreign proofs, malformed input and legacy/unbound rejection passed, with no cleanup
errors. Tests received separate no-context critic ACCEPT before implementation.

A release compatibility check found that adding the host module changed the fixed
zkVM image. A new genuine-receipt regression test was written and separately accepted
before the fix. Excluding the host-only module from target_os=zkvm restored the
previous image without changing the funding relation or receipt expectations.
The final ordinary gates passed 503 Rust tests, seven models, nine independent
oracle tests, formatting/workspace Clippy, forty frontend tests, TypeScript and Vite.
The rebuilt ordinary release package passed deep/strict ad-hoc codesign and excludes
the automation driver. Its verifier accepts both earlier genuine receipts at their
historical time, with matching full independent journals, and rejects operation
changes and expiry. The fixed image is preserved; no new proof was generated and
the app is not notarized. All 410 final source and 14 accepted test/evidence hashes
remain unchanged. Release and receipt compatibility outcomes are recorded in
`evidence/reviews/P03-issuer-spend-policy/`; domain contract:
`spec/postage/issuer-spend-policy-v1.md`.

This binds the deployment to one policy; it does not implement current Core spend
authority, issuer-global consensus spend, epoch handover or admission. UI/MCP
spending, paid custody/repair and full V1 remain unfinished. The native UI, full
remaining EVM suite, new-proof process suites and Linux network matrix are not
repeated by this increment. The preceding packaged daemon checkpoint follows.

Owner IPC now explicitly retains authenticated public epoch context, using Core's
existing atomic store without sender custody state or proof sidecars. Receipt
verification can omit history (or use null) to derive a fresh checked context from
that pin and a current registry proof. Explicit history remains non-retaining;
failed supplied evidence never falls back to the cache. Owner-only access, strict
input fields, current-head checks and admission:false remain enforced.

The complete combined actual-time EVM suite passed on the ordinary release app's
bundled daemon/prover/verifier. A receiver retained policy without sidecars,
followed eighty genuine EVM-backed successors over the actual P2P protocol, lost
its provider, restarted, and verified without resubmitting history. Twenty pages
were accepted with no rejected or failed responses. A cold receiver still failed
after successful explicit-history verification and restart until it explicitly
pinned. Genuine-seal/operation rejection, pending and completed head revocation,
graceful active-worker reaping, crash after completed verification, finite expiry,
fresh renewal, shared proving capacity and unchanged paid rows all passed.

Two genuine proofs took 351499 and 345767 ms and produced 584691 and 584688-byte
receipts. Every independently derived journal field, nullifier and resource field
matched. The combined suite also ran all preceding common-proving, foreign-verifier
and legacy daemon regressions. The source EVM chain stopped before proving.

Tests preceded production and received separate no-context critic ACCEPT. The
first packaged run produced a genuine proof, then hit a too-short peer wait. A
separately accepted test-only correction allowed the existing 30-second idle poll
plus four pages; production limits and every acceptance assertion stayed unchanged.
The successful run measured 33.739–33.837 seconds for the final four batches, with
zero rate limiting. The original failure remains recorded as passed=false.

Validation passed 502 ordinary Rust tests, seven models, nine oracle tests,
formatting/workspace Clippy, forty frontend tests, TypeScript and Vite. Five hidden
packaged WKWebView scenarios passed; four screenshot pairs show no layout regression.
The subsequent ordinary release app passed deep/strict ad-hoc codesign and excludes
the test driver. It is not notarized. Production/native sources and all packaged
binaries stayed unchanged; the final 401-source manifest includes the independently
accepted EVM timing correction. Owned test processes and profiles were cleaned up.
Evidence: `evidence/reviews/P02-daemon-retained-context/`; contract:
`spec/postage-circuit/daemon-retained-context-v1.md`.

This supplies explicit warm-node owner retention and cached receipt verification.
Cold-peer epoch-history availability, UI/MCP spending, issuer-global canonical
spend, paid custody/repair and full V1 acceptance remain unfinished. The full EVM
remainder, Linux network matrix and separate real-proof process suites were not
repeated for this daemon increment. The underlying bounded Core retention remains
covered by `evidence/reviews/P02-public-epoch-retention/`.

The preceding packaged daemon proving checkpoint follows.

The packaged daemon now also proves paid tickets under the common public policy.
Owner start_common_postage_proof takes public history and proofs, with no caller
interval; Core derives the statement and prepares the owned ticket using an opaque
current-authority context. The existing proof record/read/cancel methods and shared
bounded worker pool are reused. Completion rechecks the same authority and owned
ticket. The shorter current-head lease stops pending work even while the common
statement remains valid. No result grants spend/storage admission.

The full combined EVM scenario passed on the ordinary release app's bundled
daemon/prover/verifier. The legacy proof took 316091 ms (585044 bytes); the new
common-policy proof took 309412 ms (584978 bytes). Their entire independently
verified journals, nullifiers and resources match. Foreign verification without
sender wallet/prover, shared capacity, cancellation, SIGKILL/wallet recovery, late
head revocation, fresh preparation and actual finite-authority expiry passed,
together with every preceding daemon/foreign-verifier regression. Paid rows were
preserved and owned test processes/profiles cleaned up.

Validation passed 493 ordinary Rust tests, seven models, nine oracle tests,
formatting/workspace Clippy, forty frontend tests, TypeScript and Vite. Five hidden
packaged WKWebView scenarios passed; chat, agent access, network and restored-trust
screens were visually compared with the preceding native evidence. The debug
automation bundle was used for UI scenarios; the subsequent ordinary release app
passed deep/strict ad-hoc codesign and excludes the test driver. The EVM scenario
then used that release package; its bytes remained unchanged. It is not notarized.
All 394 frozen source inputs, six accepted test inputs and five native inputs stayed
unchanged. Evidence: `evidence/reviews/P02-daemon-common-proving/`; contract:
`spec/postage-circuit/daemon-common-proving-v1.md`.

Public epoch-history availability, issuer-global canonical spend, paid custody/
repair and full V1 acceptance remain unfinished. This increment does not repeat the
Linux network matrix, full EVM remainder or separate full real-proof suites.

The preceding daemon verification checkpoint follows.

The daemon now verifies foreign postage receipts using Core's public common
context and current authority, without sender custody state or a local prover.
Owner-only asynchronous jobs share the existing proving worker/retention limits;
completed verification is revoked when its current head changes or expires. Every
result remains admission:false. The full combined actual-time EVM scenario passed
on the release daemon, including all prior proving/cancellation/restart checks.
Its genuine 584820-byte proof took 309400 ms and matched the full independent
journal oracle. No paid-wallet records changed. Earlier incomplete runs and the
separately reviewed fixture corrections are preserved in the evidence.

Validation passed 492 ordinary Rust tests, seven models, nine oracle tests,
formatting/workspace Clippy, all forty frontend tests, TypeScript and Vite. The two
IPC scenarios also passed in release mode. Five hidden packaged WKWebView scenarios
passed: messaging, history recovery, scoped MCP/revocation, relay and checkpoint
trust/recovery. Chat and agent-access screenshots match the previous verified
layout. All 328 final source inputs, five accepted test inputs and five native gate
inputs remained unchanged. Evidence: `evidence/reviews/P02-daemon-verification/`;
contract: `spec/postage-circuit/daemon-verification-v1.md`.
The ordinary release Tauri app was rebuilt, deep/strict ad-hoc signature verification
passed and its default dependency graph excludes the test driver. It is not notarized.

Public epoch-history availability, common-policy daemon proving, issuer-global
canonical spend, paid custody/repair and the rest of full V1 acceptance remain
unfinished. This increment does not repeat the Linux network matrix, the full EVM
remainder or every separate real-proof test suite.

The preceding Core increment derives the common postage statement from public registry evidence and
signed checkpoint history, without the sender's custody wallet. It selects the
latest pre-beacon checkpoint and a shared interval ending at snapshot admission
expiry. An opaque current-head context is revoked on head change or expiry; its
clock checks persist across denials and restart. Owned preparation reuses that
exact context and preserves original paid evidence and nullifiers.

Six tests preceded production and received separate no-context critic ACCEPT.
Current validation passed 491 ordinary Rust tests plus nine genuine proof/CLI and
three Core process tests (503 total), seven models, nine independent-oracle tests,
formatting/Clippy and 40 frontend tests with TypeScript/Vite. A new real-process
scenario produced a 584984-byte proof in 371391 ms, verified its whole journal with
an independent oracle, and checked renewed receiver authority without sender wallet
state. These Core tests use historical fixture clocks; they are not current-time
daemon admission. All 313 final source inputs and nine accepted inputs stayed
unchanged during the deeper process validation. Evidence:
`evidence/reviews/P02-common-context/`; contract:
`spec/postage-circuit/common-context-v1.md`.

That Core increment did not include daemon integration, native, Linux network or
real-chain EVM acceptance. The newer daemon receipt verification and native results
are described above. The preceding application/network checkpoint follows.

The observed TCP/QUIC provider-restart regression is corrected. The daemon drains
transport closure for a bounded 500 ms before exit; surviving peers retry their
last verified bootstrap connection after the existing 500-ms delay instead of
waiting for the 30-second healthy refresh. Active slots, failed-attempt backoff,
relay policy and authority checks remain unchanged. Two real outgoing-client
tests first failed at the eight-second reconnect bound; the separate critic
accepted the strengthened test package before bootstrap production changes.

That network checkpoint passed 485 ordinary Rust tests, seven models, nine oracles,
formatting/Clippy and 40 frontend tests with TypeScript/Vite. The unchanged operator
EVM scenario passed twice: all 16 roles and both chains per run, with real bundled
prover CPU advancing during both. The release app passed actual proving,
cancellation, clean shutdown, daemon SIGKILL, wallet recovery and independent
oracle verification; its proof took 408315 ms and was 584913 bytes. It remains a
proof with admission:false, not an authorized spend.

All seven isolated Linux network outcomes and five hidden WKWebView product flows
passed. Native chat, agent access, network and restored-trust views were visually
compared with the previous baseline; no layout regression was observed. Owned test
processes, profiles, containers and networks were cleaned up. The release app still
passes deep/strict ad-hoc codesign and has unchanged tested daemon/worker bytes;
the default app graph excludes the native automation driver. It is not notarized.
All 330 source, seven accepted-test and five native-runner inputs stayed unchanged.
Evidence: evidence/reviews/network-bootstrap-reconnect/ and the preserved earlier
failure in evidence/reviews/network-graceful-shutdown/. A fresh full aggregate and
unrelated EVM/proof suites were not repeated for this network-only correction.
At that checkpoint, common statement intervals, fresh remote receipt authority,
canonical spend and the complete V1 goal remained unfinished.

The preceding daemon proof-job checkpoint follows.

The daemon now runs owner-only, cancellable proof jobs from the encrypted Core
wallet. Private input travels only through supervised child stdin; fixed sibling
prover/verifier binaries are packaged in the Tauri release app. Exact retries,
capacity and retention bounds, real clock expiry and current authority checks are
in place. `proof_ready` always carries `admission:false`; canonical spend is still
required.

Actual packaged E2E passed cancellation, clean shutdown, daemon SIGKILL, wallet
reopening with the same paid nullifier, MLS responsiveness during real worker CPU,
strict input/output limits, independent verification, damaged-seal rejection and
head-change rejection. The proof took 375776 ms and was 585000 bytes. The first
packaged run exposed a prover lifetime-thread panic when daemon-owned stderr was
closed; fallible diagnostic writing now guarantees the subsequent process exit,
and the same SIGKILL assertion passed. The failed report is retained.

The final backend command passed 491 Rust tests (480 ordinary, nine genuine
proof/CLI, two Core process), formatting and Clippy. All 40 frontend tests,
TypeScript and Vite passed. An upload test initially failed under prover load; a
250-ms read control reproduced a second upload into a disabled field. It now waits
for that field to become enabled without changing assertions or deadlines, and
passes both delayed control and the full suite. Default-helper EVM regressions
passed 28 paid CLI checks and 28 restored assignments on two chains. The release
app passed deep/strict ad-hoc signature verification; it is not notarized.

The unchanged operator-network EVM scenario failed again after provider restart.
Diagnostics show a stale QUIC connection at the consumer with no reciprocal
provider connection; the original eight-second timeout is unchanged. The focused
clean-shutdown regression has independent test-critic ACCEPT and is next to run
RED before any network production change. There is no aggregate GREEN. Evidence:
`evidence/reviews/P02-daemon-jobs/`; contract:
`spec/postage-circuit/daemon-jobs-v1.md`. No fresh full EVM remainder, native
WKWebView or Linux suite is claimed for this checkpoint. All eight accepted test
inputs stayed unchanged; the only input changed after packaged E2E was the single
frontend-test synchronization line, captured in the final 327-input manifest.

The preceding retained-wallet checkpoint follows.

P02 preserves original paid-wallet ancestry beyond the ordinary 64-checkpoint
archive. Immutable funding anchors share one bounded current-head index; all 32
paid intents survive 80 authenticated successors and SQLCipher restart without
exceeding Store's 16-change limit. Binding uses at most four state changes and
advancement at most three. Original payment/anchor bytes remain unchanged. Fresh
authority, paid expiry and monotonic time are still required when using a ticket.

Eight test-first retention scenarios and the shared-lineage implementation received
independent FINAL ACCEPT. The current components passed 490 Rust tests (479 ordinary,
nine real proof/CLI, two Core process), 40 frontend, 29 Solidity, seven models,
nine oracles, formatting/Clippy/TypeScript/Vite and all 19 fresh EVM reports.
The initial aggregate exited 1 on an operator-network pending timeout after restart;
an unchanged rerun and the remaining EVM commands passed. Its cause remains open:
`Docs/maintenance/operator-network-timeout-20260907.md`. No aggregate GREEN is claimed.
All six accepted test inputs and 323 source inputs stayed unchanged during validation.
Evidence: `evidence/reviews/P02-retained-wallet/`; contract:
`spec/postage-circuit/retained-wallet-v1.md`. Native/Linux and the release bundle were
not rerun. At that checkpoint production daemon proof jobs were still pending; their later
integration is recorded above. Fresh receipt admission, canonical spend and
complete V1 remain unfinished.

The preceding P02 checkpoint added an actual supervised local prover mode connected to the encrypted
Core wallet in process acceptance. EOF, an unexpected stdin byte and SIGKILL of a
separate parent stop the real worker without a receipt; CPU activity and actual
PID disappearance are checked. SQLCipher reopening preserves the paid mark and
nullifier, and a subsequent real proof succeeds with stdin open. Both a separate
default verifier and independent upstream oracle confirm the complete statement.
The proof took 327921 ms and used 584987 bytes, with the unchanged guest image.

All focused components passed: 471 ordinary Rust plus nine existing proof/CLI and
two Core process tests (482 Rust total), 7 models, 9 oracles, 40 frontend tests,
formatting, default/proving/process-feature Clippy and TS/Vite. The 215 source inputs
and four binary files remained unchanged. No fresh full EVM, native/Linux, release
bundle or single aggregate-script result is claimed. The test parent is not the
production daemon; public deterministic historical fixtures do not establish live
admission. Evidence: `evidence/reviews/P02-supervised-process/`; contract:
`spec/postage-circuit/supervised-process-v1.md`. Daemon jobs, retained paid anchors,
common interval policy, fresh authority, canonical spend and full V1 remain open.

P02 now connects the encrypted Core custody wallet to local proof preparation.
An already paid ticket produces an opaque, zeroizing CLI request at the latest
retained authenticated pre-beacon common root. Original funding and fresh common
membership are both checked; expiry, current registry authority and durable clock
commit are enforced before the private request leaves Core. Reopening or dropping
preparation preserves the paid state and exact nullifier. There is no owner/MCP
endpoint, running proof job or spend admission yet; archive retention and daemon
supervision remain open.

The five reviewed new tests and full ordinary regression passed: 471 Rust tests,
7 models, 9 oracles, 40 frontend tests, formatting, workspace Clippy, TypeScript
and Vite. The actual two-chain corpus passed 16 existing Rust CLI checks. The 211
recorded source inputs remained unchanged through validation. ZK proof, full EVM,
native/Linux and release-bundle gates were not repeated for this Core-library
increment. Evidence: `evidence/reviews/P02-core-postage/`; contract:
`spec/postage-circuit/core-preparation-v1.md`. The complete V1 goal remains open.

P02 now has real shared-root funded-ownership acceptance. Two independently random
owners paid from distinct accounts before the same beacon and produced recursive
STARK receipts against exactly the same public context. The chain was stopped
before proving; both the default verifier and independent upstream oracle accepted
the proofs. Actual clock checks after both computations remained within paid
lifetimes, and stable nullifiers were distinct. Private witnesses were kept in
memory; retained artifacts contain public receipts and an unordered deposit cohort.
This establishes the tested relation/privacy boundary, not formal anonymity,
current checkpoint authority or global spend admission.

The focused runs all exited 0: 466 ordinary Rust plus nine genuine proof/CLI tests
(475 total), 7 models, 9 fixture/oracle checks, 40 frontend tests, formatting,
default/proving Clippy, TypeScript/Vite, the original 28-check funded-custody EVM
regression and two new random-owner proofs. The proofs took 356731 and 321274 ms
and used 584886 and 585071 JSON bytes. Thirty source inputs and three binaries
remained unchanged across verification. Other EVM runners, native/Linux flows and
the desktop bundle were not repeated for this test/infrastructure increment.
Evidence: `evidence/reviews/P02-shared-root/`; contract:
`spec/postage-circuit/shared-root-v1.md`. Core wallet/prover integration, durable
interruption recovery, common interval policy and full V1 remain open.

P02 now exposes actual `agentic-postage prove|verify` CLI processes. The default
verifier has no proving feature and accepts only public context/receipt input.
The prover keeps upstream execution segments in memory and rejects filesystem
profiling; bounded stdin, exact receipt bytes and fixed safe errors are enforced.
A real positive proof succeeds with TMPDIR/TMP/TEMP pointing to an ordinary file,
where the previous implementation failed. The separate default verifier and
independent upstream oracle both accept it. This removes explicit private segment
files, without claiming control of OS swap/core dumps or full wallet recovery.

The new backend/frontend runs exited 0: 466 ordinary Rust plus nine actual proof/CLI
tests (475 total), default/proving Clippy and formatting, 40 frontend tests,
TypeScript and Vite. The proof took 360038 ms and584675 JSON bytes with the unchanged
guest image. The25-file source snapshot stayed unchanged. Historical fixture clocks
prove correctness, not live admission: the fixture lifetime is35 seconds, shorter
than proving time. Core still needs real-time checks, authenticated common roots,
retained funded marks and interruptible jobs. Evidence: `evidence/reviews/P02-postage-cli/`;
contract: `spec/postage-circuit/cli-v1.md`. EVM/native/Linux/app bundling were not
repeated for this unconnected CLI increment; the last full EVM aggregate is below.

The preceding P02 increment introduced a real local recursive-STARK backend in `crates/postage-zk`.
The fixed RISC Zero 3.0.6 guest proves the existing postage relation; the default
verifier checks its compiled image, exact context/journal, receipt kind and size.
All seven real proof tests pass, including an independent upstream verifier
process, a genuine wrong-image proof, a genuine same-image Composite, fake/tampered
receipts and direct invalid guest execution. The final proof cost 360279 ms and
584596 JSON bytes on this M4 Pro; this is not interactive message-send performance.

The unchanged aggregate exited 0 with 473 Rust, 29 Solidity, 7 model, 9 oracle and
40 frontend tests, formatting/Clippy, TypeScript/Vite and all 19 fresh EVM reports.
Selected daemon service TCP/QUIC acceptance completed 867 owner calls with no
reported cleanup errors. Earlier driver/lint failures are retained explicitly.
No current native/Linux rerun or new packaged application is claimed for this
unconnected proof library. Evidence: `evidence/reviews/P02-local-postage-zk/`.

That first backend used upstream LocalProver's temporary segments; the subsequent
CLI/memory change above removes that path. Tests use public deterministic seeds.
Shared-root anonymity, finalized-mark persistence/interruption, Core integration,
global spending and private custody selection still remain. Neither increment
completes P02 or V1.

The preceding P02 increment added a tested deterministic private-postage guest relation in
`crates/postage-proof`: existing real Ethereum funding membership and custody
opening verification, finite ticket indices, an operation-bound public journal,
and a stable HMAC nullifier. It has no proof/admission ingress. Independent Python
vectors cover 16 funded tickets on two chains; all eight new tests and four
compiled-negative mutations pass their expected outcomes. A separate no-context
backend-test-critic first rejected weak size-limit tests, then accepted corrected
valid-JSON boundary tests before production implementation. Workspace regression
passed 466 Rust tests, formatting/Clippy, and 40 frontend tests with TypeScript/Vite.
The real local proof backend is now verified above; shared-root anonymity,
interruption recovery, canonical BFT spending and private custody selection remain
required. This is not P02 completion. Evidence: `evidence/reviews/P02-private-postage-statement/`;
relation and backend candidate: `spec/postage-circuit/statement-v1.md`.

A macOS arm64 release `.app` contains agentic-desktop, agentic-node and agentic-mcp. It supports local keychain-backed profiles, signed invitations, direct P2P encrypted personal messages, durable sender retry/recipient receipts, and background daemon delivery while the UI is closed. The owner can create scoped agent access in the native panel, receive a real MCP config and revoke it. MCP supports inbox.poll, inbox.ack, messages.send and delivery.get using runtime-signed proofs and the shared core broker. The authenticated agentic://runtime resource exposes allowed dialog IDs/titles, actions and limits, so the agent can discover its scope directly.

Latest implementation: **selected daemon voting over authenticated libp2p connections**.
Trusted local Rust applications register up to four distinct scopes. Core supplies the
actual selected signer and revocable authority; the daemon supervises each engine,
automatically discovers and renews selected peer proofs, bounds wire traffic and queues,
and commits application effects before acknowledging the durable archive. No owner/MCP
command registers code or treats arbitrary payload bytes as authorized spend/group work.

The saved implementation at 0d5596b5 passed the actual paid-registry daemon-service
fixture over both TCP/Noise and QUIC: four selected profiles, thirteen effects per profile,
126 independently verified quorum signatures and 815 owner calls. Byzantine rejection,
quorum withholding, role/network/head changes, automatic route renewal, crash recovery,
actual failed-effect cursor retention, isolated cold replay and real expiry passed;
cleanup errors are empty. Source hash:
67a0d11c0eb67865e7acf616d90e7615a26304a8c460985fe848c9dbc7dd5c29.
This supersedes the failed daemon-service debug runs, without deleting their evidence.

Migration validation also passed 456 Rust, 29 Solidity, 7 model, 9 fixture/oracle and
40 frontend tests, formatting/Clippy, TypeScript/Vite, seven Linux network outcomes,
five hidden native flows and a rebuilt ad-hoc-signed release. Validation was phased:
an initial aggregate stopped on an intermittent operator-network timeout; isolated
baseline/current reruns and the full sequential EVM script then passed. Its cause is
not established, so this is not a claim of a clean initial aggregate or complete V1.
Details: Docs/maintenance/storage-migration-result.md.

The next full aggregate is now GREEN with quiet-tip recovery and actual cross-log
certificate replay: 458 Rust, 29 Solidity, 7 model, 9 oracle and 40 frontend tests,
formatting/Clippy, TypeScript/Vite and all EVM runners. Real daemon TCP and QUIC each
completed 13 effects on four selected profiles, 159 independently verified quorum
signatures in total and 922 owner calls, with no cleanup errors. Service source hash:
0bb8c447a57929fd48b7c38645c5883647574bd218e721a4225ae1270018bd07.
Five hidden native flows, seven isolated Linux outcomes and four paired visual
comparisons passed for this change. The macOS arm64 release was rebuilt, its ad-hoc
signature verified, and the default dependency graph excludes the automation plugin.
The source fingerprint is unchanged since the clean aggregate. Earlier failed runs
and their limits remain retained; production spend/group admission and
full V1 remain open. Contracts: spec/finalizer/daemon-scope-replay.md and
spec/finalizer/quiet-tip-recovery.md.

Previous integration at ed9bfd7: **scoped selected authority and an embedded dynamic finalizer service**.
Core now creates the actual enabled selected signer for a specific application/log, retaining
the shorter verified proof lease. An opaque local fence detects profile, head, role and
saved-authority changes. The embedded Commonware service supplies bounded channels,
complete application ancestry, durable at-least-once deliveries and explicit effect
acknowledgements; stop and expiry terminate the runtime. Existing validator executables
reuse its private directory lock.

Eleven new Rust tests preceded production and received independent no-fork FINAL ACCEPT
after revisions. Four actual-engine service scenarios cover dynamic authorized work,
observed invalid proposals, partition/heal, restart/cold replay, pending acknowledgement,
route renewal and finite service expiry. These engines use a local message pump; selected
daemon voting over libp2p and production spend/group application admission remain open.
Contract: spec/finalizer/service-v1.md; evidence: evidence/reviews/P01-finalizer-service.md.

Regression validation completed in phases:456 Rust,29 Solidity,7 model,3 fixture-reader
and40 frontend tests pass (535 functions). The initial aggregate exited1 on an incomplete
JSONL fixture line; after independently reviewed tests and the shared reader repair, all
three affected real EVM runners passed. All18 fresh EVM reports match current source hashes.
Formatting, Clippy, TypeScript and frontend build passed. Five fresh native WKWebView flows
and four paired visual comparisons pass; a new macOS arm64 release was bundled and its
ad-hoc signature verified. The seven Linux outcomes at3337966 are historical, not rerun
for this increment. The initial aggregate failure remains recorded in
evidence/reviews/P01-finalizer-service-validation.json; full V1 acceptance remains open.

Previous integration at3337966: **selected P-256 keys prove their live daemon transport peer**.
Core stores a complete verified current roster with atomic CAS/clock commits. Enabled
selected encrypted keys issue bindings lasting at most60 seconds and no longer than the
current authority. The daemon exchanges these proofs over authenticated TCP/Noise and
QUIC and verifies them against the actual Peer ID. Retained successes are reverified on
read and become stale after head, connection or lifetime changes. New owner commands are
publish_finalizer_roster, check_finalizer_peer and finalizer_peer_result.

Four new crypto tests, four new Core tests and one new process test preceded production;
both independent no-fork critics required revisions and then returned FINAL ACCEPT.
A fresh paid-registry network E2E passed333 owner calls with actual endpoint checks,
copied-proof rejection from a foreign Peer ID, four received requests and no fifth,
provider restart, role disable/re-enable, short binding expiry while the head remains
live, and subsequent head expiry. Contract: spec/finalizer/routing-v1.md; evidence:
evidence/reviews/P01-finalizer-routing.md. This is authenticated discovery; selected
daemon voting, its application admission and native operator controls remain open.

Fresh aggregate acceptance passes445 Rust +29 Solidity +7 model +40 frontend functions
(521 total), all18 newly generated EVM reports, formatting, Clippy, TypeScript and frontend
build. Seven Linux network outcomes and five native WKWebView flows pass with clean fixture
shutdown. Four paired screenshot comparisons show no visual regression. A fresh macOS
arm64 release app was rebuilt and its ad-hoc signature verified; no notarization is claimed.
Details and source/binary hashes: evidence/reviews/P01-finalizer-routing-validation.json.

Previous Core/daemon integration at66599d7: **owner-selected P-256 policy, encrypted enrollment keys
and current-head committee verification**. A separate immutable finalizer policy coexists
with custody settings. Key creation, owner enable/disable and clock observations commit
atomically. Stored secrets and role intent survive restart; intent alone does not start
voting. The daemon supplies time and loads the actual current checkpoint for every complete
roster verification. Old witnesses, incomplete/substituted rosters and expired proofs fail.

Twelve Core tests and two actual process tests preceded their production modules and each
received separate no-fork FINAL ACCEPT. A fresh two-chain EVM gate registers34 random
daemon-generated keys, independently verifies their P-256 PoP/commitments with OpenSSL,
and checks24 committee outcomes through96 owner calls. It stops the chain, advances the
saved head, restarts the daemon and waits for actual expiry. Ordinary MLS delivery remains
working after owner setup, storage failure and corrupt finalizer state.

The aggregate passes436 Rust +29 Solidity +7 model +40 frontend functions (512 total),
all17 fresh EVM reports, formatting, Clippy, TypeScript and frontend build. Fresh Linux
network acceptance passes7 outcomes with no leftover test containers/networks. Five fresh
native WKWebView flows pass, and four paired screenshot comparisons show no visual regression.
A new macOS arm64 release app is bundled and its ad-hoc signature verifies; notarization
and other supported-platform releases remain open. Contract: spec/finalizer/owner-v1.md; evidence:
evidence/reviews/P01-finalizer-owner.md. The selected daemon voting service, native operator
controls and full V1 acceptance remain open.

Previous runtime integration at42d1747: **P-256 voting, durable recovery and actual TCP validators**.
Typed public P-256 committees now drive the same Commonware Simplex engine, Marshal,
authenticated channels, voting WAL and archives as Ed25519. The shorter proof lease stops
the entire validator; a renewed head resumes the same journal and exact old finalized prefix.
A sealed committee trait keeps concrete schemes and authority checks together. The original
96-byte Ed25519 binding is preserved; P-256 uses97 bytes with its compressed public key.
Two small executables share the same configured batch/TCP code and private storage lock.

Tests preceded implementation and received separate no-fork FINAL ACCEPT for both engine
and TCP; a two-line test lint correction received another ACCEPT. Both suites pass14 engine
and3 actual Unix process scenarios, including Byzantine application rejection, partition,
SIGKILL, three-survivor progress, cold disk recovery and full catch-up. An independent
Python/OpenSSL checker validates144 signatures from four actual P-256 TCP processes and
refuses144 wrong-subject signatures. All three isolated compiling mutations are detected.

The complete aggregate passes422 Rust,29 Solidity,7 model and40 frontend functions (498),
all16 fresh actual EVM reports, formatting, Clippy, TypeScript and frontend build. Custody
resolution verifies295 positions through2619 owner calls with empty cleanup; the old and
P-256 selection CLIs pass36 and28 checks. Evidence: evidence/reviews/P01-p256-runtime.md.
At42d1747 application sources/locks were unchanged and no app package depended on finalizer.
That earlier increment reused735d7f2 native/Linux evidence. Core now depends on finalizer;
the fresh validation described above supersedes that earlier application boundary.

The P-256 selection CLI pins the reviewed registry code hash and authenticates the full
selected public roster and actual certificates; it remains an offline verifier of explicitly
trusted inputs. Core/daemon policy, current-head ownership and operator keys are now integrated;
the selected voting service is next. The older direct-certificate journal is still Ed25519;
Marshal handles the runtime's indirectly finalized ancestors. Spend/custody/R=10 and full
V1 acceptance remain open. Contracts: spec/finalizer/engine-v1.md and p256-selection-v1.md.

Previous registration prerequisite: **public eligible finalizer keys before the beacon**.
FinalizerRegistry reuses the fixed paid-unit registry, tree, epoch snapshots and delayed exits.
Every admitted unit publishes a valid P-256 key with an owner/deployment-bound proof of
possession and permanent global uniqueness. The opaque bond route is closed. Public keys
remain available after exit/withdrawal; an operator need not answer a later opening request.
OpenZeppelin5.7.0's unchanged portable verifier is vendored with exact provenance. A real
Commonware P-256 compatibility test matches the independent OpenSSL signature byte for byte.

Five new Solidity tests and the actual two-chain runner preceded production and received
separate FINAL ACCEPT. Two harness/lint corrections received further independent ACCEPT
without weakening assertions. All29 Solidity tests and the focused Anvil scenarios pass;
six compiling negative controls are detected. The Anvil checks reconstruct the complete
root independently, reject front-run before honest admission, and retain all four frozen
keys through actual reorg/restart/withdrawal with zero operator processes. The aggregate
passes396 Rust,29 Solidity,7 model and40 frontend test functions (472 total), all15 actual
EVM runners, formatting, Clippy, TypeScript and frontend build. Final custody resolution
verifies294 positions through2640 owner calls; the previous selection CLI still passes36
checks. EVM cleanup errors are empty. Evidence: evidence/reviews/L02-finalizer-keys.md.

This is a separate registry/signature profile. Rust membership/selection verification was
then implemented above; upstream P-256 voting and daemon role/key controls remain required.
The previous Ed25519 selected API does not consume this registry as a P-256 committee.
No Rust application production code, UI, Cargo/npm lock or packaged binary changed in this
increment; the9ff1b3a native5-flow/Linux7-outcome evidence covers the unchanged application.
Contract: spec/registry/finalizer-keys-v1.md. Full P01/V1 and complete E01–E26 remain open.

Previously added runtime integration: **authenticated fixed epoch committee selection and proof leases**.
A locally pinned policy derives all paid-unit ordinals from the complete proven L2 registry,
then authenticates every selected member and signing-key opening. Committee identity is stable
across accepted checkpoint refreshes; certificate verification and the actual Simplex runtime
enforce the shorter evidence lifetime. A refreshed proof resumes the same voting WAL and
archives without rewriting the old finalized prefix. The strict `select-committee` CLI exposes
the verified roster, quorum and provenance. Custody and finalizer share one bounded sampler.

Six selection tests and one actual-engine recovery test preceded production and received
independent FINAL ACCEPT. Six isolated compiling negative controls are detected. A separately
reviewed fresh two-chain EVM runner passes36 actual CLI checks with source shutdown, elapsed
proof expiry, independent roster/ID oracle and positive controls around rejection attempts.
The full aggregate passes395 Rust,24 Solidity,7 model and40 frontend test functions (466 total),
all14 actual EVM runners,5 packaged native flows and7 Linux network outcomes. Custody resolution
verifies295 positions through2625 owner calls; EVM/Linux cleanup errors are empty. The rebuilt
macOS arm64 release passes strict deep signature/default-driver checks; it is ad-hoc signed,
not notarized. Current chat/restored-trust screenshots visually match2d56a44 except timestamps.
The selected finalizer tests run on macOS; Linux checks existing daemon network behavior.
Contract: `spec/finalizer/registry-selection-v1.md`; evidence: `evidence/reviews/P01-selection.md`.

At that milestone the CLI validated a supplied complete roster; opaque registry commitments
did not make withheld key openings available. The new public-key registry above supplies the
registration prerequisite; Rust authentication is now above and runtime integration remains outstanding.
Core/daemon policy/current-head/key integration, authenticated selected voting service,
handover, global spend, actual ciphertext custody/repair and other product scenarios remain.

Previously added runtime integration: **configured Simplex voting and actual TCP validators**.
The `agentic-finalizer` crate now runs upstream Commonware Simplex/Marshal with authenticated
channels, durable voting WAL, finalization archives, cold restart and ordered at-least-once
application delivery. Guards check ancestry, operation uniqueness and application validity
for both local and received proposals. A separate executable runs a configured batch across
real authenticated TCP processes with private locked storage and finite committee leases.
It is a reusable consensus runtime and executable fixture; it is not yet an owner/MCP spend
API or a desktop payment feature.

Tests preceded production and received independent FINAL ACCEPT. Twelve actual-engine fault
and recovery tests plus three Unix subprocess tests pass. Four native TCP validators produce
identical12-entry histories; an independent Python checker verifies144 Ed25519 signatures
and rejects144 wrong-subject signatures. Five isolated compiling negative controls are
detected, including a finalized unauthorized operation when the application verdict is ignored.
The aggregate passes388 Rust,24 Solidity,7 model and40 frontend test functions (459 total),
all actual EVM runners,5 packaged native flows and7 Linux network outcomes. The final
custody-resolution runner checks285 positions through2623 owner calls with all mandatory
controls and empty cleanup errors. The rebuilt macOS arm64 release passes strict deep
signature/default-driver checks; it remains ad-hoc signed and not notarized. Current chat
and restored-trust screenshots visually match796a74d except expected timestamps. The new
finalizer tests ran on macOS; the Linux gate checks existing daemon network behavior.

The earlier certificate verifier and SQLCipher application journal remain available.
At that milestone P01/V1 remained open: committee selection, daemon operator/key integration,
handover, global spend, actual ciphertext custody/repair and other product scenarios were pending.
Contract: `spec/finalizer/engine-v1.md`; evidence: `evidence/reviews/P01-engine.md`.

Previously added implementation: **automatic resolution of paid custody positions**.
Core checks exact paid ordinals against actual current custodian bindings. The daemon finds
providers over TCP/QUIC among up to32 connected peers without owner placement overrides,
with four concurrent requests and finite job deadlines. Every read rechecks binding lifetime,
actual connection and current paid head; relay-only/head/network changes and restart invalidate
old offers. Legacy publications need no migration. Seven new Core tests, the store boundary
case, owner/agent IPC and targeted Clippy pass after separate test-first FINAL ACCEPT. The
focused actual two-chain runner after the cancellation fix verifies292 position outcomes through2693 owner calls,
including held stream/deadline controls, a reproduced and fixed physical-slot cancellation bug, independent binding expiry, disconnect/reconnect,
copied proof, wire size limits, relay-only, accepted successor, source shutdown and restart.
The final aggregate passes360 Rust,24 Solidity,7 model and40 frontend tests (431 total),
all earlier EVM gates plus293 checked resolution position outcomes/2693 owner calls,5 actual
native flows and7 Linux outcomes. The macOS arm64 release, strict deep signature and default
driver-exclusion checks pass; it remains ad-hoc signed and not notarized. Final chat/restored
trust images visually match9a10ce0 except generated timestamps. The first pre-fix Linux run
had a transient route-observation assertion after successful relay failover delivery; its
unchanged retry and the final source both pass, with the original failure retained. Evidence:
`evidence/reviews/N05-custody-resolution.md`; contract: `spec/registry/custody-resolution-v1.md`.
This resolves live selected providers; disconnected-operator routing, actual ciphertext storage,
receipts, spend/admission, retained funding history and autonomous R=10 repair remain open.

Previously added implementation: **durable owner custody intents and funded assignments**.
The daemon creates independent private intent keys under an actual pre-beacon registry seal,
commits them atomically with observed checkpoint time, binds their actual funding through its
own authenticated history and re-verifies assignments under its current head after restart.
Strict owner IPC returns public metadata; agents cannot access these commands. The existing
CLI and Core share a read-only assignment DTO. Focused tests and targeted all-targets Clippy
pass after separate test-first FINAL ACCEPT. The actual two-chain runner passes28 independently
checked assignments through124 owner calls, including source shutdown, daemon restart after
historical lease expiry and refusal at actual current expiry. The full aggregate passes351 Rust,
24 Solidity,7 model and40 frontend tests, all earlier EVM gates,5 actual native flows and7
Linux outcomes. The rebuilt macOS arm64 release/signature/default-driver checks pass.
Current chat/restored-trust screenshots match their67cbb56 references; Linux/EVM cleanup
errors are empty. The app remains ad-hoc signed and not notarized. Evidence:
`evidence/reviews/N05-custody-owner.md`; contract: `spec/registry/custody-owner-v1.md`.
Spending, actual ciphertext custody, selected-operator discovery and autonomous R=10 remain
open. Current archive pruning can discard old funding evidence and fails closed; required
history retention must be completed before long-term admission/repair acceptance.

Previously added implementation: **paid pre-beacon custody selection**. The Rust adapter and
actual CLI authenticate historical funding through the current signed checkpoint chain,
reject payment at/after beacon and derive primary/replacement ordinals from the complete
proven registry. Paid resource classes determine bounds; a selected member must prove its
actual ordinal, opening, current head and finite lifetime. Sparse sampling agrees with an
independent full-list oracle across two actual Anvil deployments. After test-first FINAL ACCEPT, all8 focused tests,4 isolated compiling mutations,
341 Rust/24 Solidity/7 model/40 frontend tests,28 new actual two-chain CLI checks,5 native
flows and7 Linux outcomes pass. The macOS release/signature/driver gates pass; current
chat/restored-trust screenshots match their611afec references. Cleanup errors are empty.
Evidence: `evidence/reviews/N05-funded-custody.md`; contract:
`spec/registry/funded-custody-selection-v1.md`. This does not yet persist private sender
intent, authorize spend, retrieve/store data or execute R=10 repair. N05/D03/V1 remain open.

Latest added model tool: **conditional quorum and fixed-unit selection risk**.
`tools/risk-simulator/risk.py` reports exact distributions for sampling fixed bond units without
replacement, structural honest-intersection failure, availability under assumed honest uptime,
key-splitting invariance and an explicit unsafe sqrt-per-key counterexample. Its quorum model
returns concrete conflicting signer sets when the threshold is unsafe; partition stops progress
without lowering quorum. Unknown grinding limits remain unknown. These are conditional models,
not consensus implementation, operator independence or a production parameter recommendation.
Tests came first and received separate FINAL ACCEPT; all7 model tests and four isolated compiling
negative controls pass.333 Rust and40 frontend tests pass after implementation. Six real CLI
example reports are saved; the model suite is part of `scripts/check.sh`. No application/network/
contract code or dependencies changed, so the preceding full native/Anvil/Linux evidence remains
applicable to the same app. Evidence: `evidence/reviews/F05-committee-risk.md`; model contract:
`spec/models/committee-risk-v1.md`. Complete F05/N05 and full V1 acceptance remain open.

Previously added implementation: **operator proof exchange on authenticated TCP/QUIC peers**.
Owners persist and disable publications for their own paid units. A daemon serves a fresh
finite role binding using its actual transport key; a remote daemon verifies the complete
checkpoint/registry/member proof against the authenticated connection, requested index and
role. Four pending and64 retained ephemeral checks are bounded; current time, head and live
connection are rechecked before any verified result is used. Restart restores no cached
peer authority, and network replacement cancels outstanding checks.

Tests preceded implementation and all test revisions received separate FINAL ACCEPT.
Aggregate: **333 Rust,24 Solidity,40 frontend tests**, five actual packaged hidden WKWebView
flows, macOS release/signature/driver checks and seven Linux outcomes pass. The fresh two-chain
Anvil gate verifies16 remote roles over TCP and QUIC, rejects copied bindings and wrong indices,
checks the wire limit and recovery, serves after source shutdown/provider restart, and removes
cached authority at actual expiry and an accepted successor while the connection remains live.
An initial E2E fixture assumption about unchanged global EVM state was disproved and corrected
with each actual header's account proof; the failed run and review are retained. Cleanup errors
are empty. Current native chat and restored-trust views match the preceding de269a2 references.
Evidence: `evidence/reviews/L02-operator-network.md`; contract: `spec/registry/operator-network-v1.md`.
Placement, paid custody/R=10 repair, native registry controls and other complete V1 requirements
remain open. The local macOS arm64 app is ad-hoc signed and not notarized.

Previously added implementation: **durable operator identities and actual daemon role signing**.
Core and owner IPC prepare independent Ed25519 keys for multiple paid units in SQLCipher,
return public commitments, and preserve exact idempotent registrations after restart. Role
signing authenticates the stored opening under the current selected registry/checkpoint and
binds it to the daemon's own persistent transport key. Secrets remain private; wrong owner,
agent access, arbitrary key/time/root overrides, stale proofs and expired leases fail.
Atomic store failures cannot publish a successful registration or signature.

Tests preceded production; the separate critic returned FINAL ACCEPT. Six new Core tests,
two real daemon tests and the shared commitment oracle pass; four isolated compiling
mutations are killed. A new real two-chain Anvil gate bonds newly generated daemon keys,
checks the four-unit root, independently verifies16 exact role wires/PeerID bindings through
72 owner calls, signs after source shutdown/restart and refuses after actual expiry. All
previous live contract/registry/CLI gates remain. Aggregate: **326 Rust, 24 Solidity,
40 frontend tests**, five actual packaged hidden WKWebView flows, release/signature/driver
gates and seven Linux network outcomes, all passing with cleanup errors empty. Current chat
and restored-trust screenshots were visually compared with the preceding84f917f references.
Evidence: `evidence/reviews/L02-operator-identity.md`; contract: `spec/registry/operator-identity-v1.md`.
The app is a local macOS arm64 ad-hoc signed build. Remote operator handshake verification,
placement, paid custody/R=10 repair, native registry controls and other full V1 requirements remain open.

Previously added implementation: **operator role signatures bound to authenticated registry membership**.
The Rust adapter and CLI now verify a finite signature by the committed node key over a
particular registry root/member, role and expected transport key. The shared signed document
codec and checkpoint/MPT/membership verifiers are reused. The result is bounded by both the
signature and current checkpoint/snapshot lease; copied openings, another author/key/role,
weak keys, altered context and stale/expired bindings fail. The CLI expectation does not
prove an actual network handshake; durable key provisioning and daemon integration remain.

Tests preceded production. The separate critic required a matching weak-key expectation to
eliminate a false-green oracle, then returned FINAL ACCEPT. Six new tests pass and five
isolated compiling negative controls are killed. Fresh Anvil checks make32 operator CLI calls
with 8 successful role/member combinations across two chains, while retaining all 64 daemon
registry calls and offline expiry scenarios. The aggregate passed **317 Rust, 24 Solidity and
40 frontend tests**, five actual packaged hidden WKWebView flows, release/signature/driver
checks and all seven Linux network outcomes with cleanup errors empty. Current native
screenshots were visually compared with the preceding committed references.
Evidence: `evidence/reviews/L02-operator-role-bindings.md`; contract: `spec/registry/operator-binding-v1.md`.
Operator provisioning/connection authentication, placement, finite spend admission, actual
R=10 custody/repair and the remaining V1 scenarios stay open.

Previously added implementation: **registry selection and verification in the actual daemon**.
The authenticated owner API exposes the existing durable Core selection and verifies registry
members under the current saved checkpoint. Strict bounded input preserves raw proof/profile
JSON; the actor supplies time. Wrong credentials, signed agents, profile/root/time overrides,
invalid proofs and expired leases are refused. SQL failure and restart preserve the original
selection and continued exact MLS delivery/receipts.

Tests preceded production and the separate critic returned FINAL ACCEPT after replacing
invalid-typed override inputs with otherwise-valid live requests. A real two-chain Anvil →
signed checkpoint → daemon gate makes 64 owner calls, verifies both exact members before and
after source shutdown, and refuses both after real offline expiry. The aggregate passed
**311 Rust, 24 Solidity and 40 frontend tests**, five packaged hidden WKWebView flows,
release/signature/driver checks and seven Linux network outcomes with empty cleanup errors.
Current native screenshots were visually compared with prior committed references.
Evidence: `evidence/reviews/L02-registry-owner-ipc.md`; contract: `spec/registry/owner-ipc-v1.md`.
Native registry controls, role possession, placement, paid admission and actual R=10 custody
and repair remain open. The full V1 goal is still active.

Previously added implementation: **durable owner-selected registry in Core**.
The application core now pins an explicit registry deployment to saved checkpoint trust in
SQLCipher. Installation and the shared monotonic clock commit atomically; semantic retries
and reopen preserve exact selection bytes. Verification uses only the saved deployment and
current checkpoint head, returns exact finite snapshot/member provenance, and persists clock
observations before replying, including on denial. Invalid state, replacement, stale head,
expired admission/lease and clock rollback fail. Scoped agents cannot use these owner methods.

Tests preceded implementation; separate critic FINAL ACCEPT followed a stronger successful
clock-write oracle. All eight new Core tests pass and four isolated compiling mutations are
killed. The aggregate passed **308 Rust, 24 Solidity and 40 frontend tests**, five packaged
hidden WKWebView flows, release/signature/driver checks and all seven Linux network outcomes
with empty cleanup errors. Current native screenshots were compared visually with the prior
committed images. Evidence: `evidence/reviews/L02-durable-registry-selection.md`; contract:
`spec/registry/durable-selection-v1.md`. Daemon/UI registry adapters, role possession,
placement, paid admission, R=10 custody and repair remain open.

Previously added implementation: **authenticated registry proofs through Rust and CLI**.
The shared Ethereum verifier authenticates the registry's five storage words, finite snapshot
lease, exact ordered Merkle-sum membership and owner-bound key commitment opening. The
selected trusted checkpoint binds chain, genesis, code and state root. The actual CLI reports
explicit attestor provenance; it does not yet select keepers or authorize paid storage.

Tests preceded implementation and separate critic FINAL ACCEPT. Final native acceptance
passed 300 Rust, 24 Solidity and 40 frontend tests plus five packaged hidden WKWebView flows;
the live two-chain registry gate made 28 actual CLI verifications. All nine isolated compiling
mutations were killed after strengthening an authenticated total-count mismatch case.
Current native screenshots were visually checked alongside component references.

The final Linux run failed once on AutoNAT withdrawal after a firewall change. Its evidence
is retained. A full retry on the identical source/image passed all seven outcomes; four
additional isolated executions of the unchanged AutoNAT scenario also passed. The intermittent
failure is unreproduced and remains open for diagnosis if it recurs; no transport fix is claimed.
WD4000 briefly disappeared during Git checks and then returned mounted. Current source and
a separate backup are on APFS; both real Git branches remained at the expected prior commit.
A standalone APFS bundle additionally preserves their history.
See `evidence/reviews/L02-authenticated-registry-proofs.md` and
`spec/registry/authenticated-proofs-v1.md`. Daemon registry integration, role possession,
placement, paid admission, actual R=10 custody and repair remain open.

Previously committed implementation: **bonded operator registry with immutable epoch snapshots**.
The local Solidity contract records exact fixed-price units in a 32-level Merkle-sum tree,
freezes root/count before a specified future block, and keeps exit funds locked through the
finite admission and obligation horizon. Owner-only withdrawals apply state before callbacks;
failed callbacks preserve every claim. The complete public ABI excludes administrative methods.
The EVM blockhash profile is explicitly biasable, and bonded units do not prove independence.

Tests preceded implementation. A separate critic required stronger reentrancy, fresh-bytecode
and exact-ABI oracles, then returned FINAL ACCEPT. The aggregate gate passed **288 Rust tests,
24 Solidity tests with 256 runs per fuzz test, 40 frontend tests and five packaged hidden
WKWebView flows**, including real Anvil funding/checkpoint tests and the new registry runner.
Two disposable registry chains verify actual transactions, exact root/count and events,
gas-adjusted principal, immutable snapshots, reorg, disk/process restart and withdrawal.
Seven isolated compiling mutations were killed; an added confiscation API passed ordinary
behavior tests and was rejected by the exact ABI gate. Release bundling, deep/strict ad-hoc
signature and production driver exclusion passed. Chat and restored checkpoint screenshots
were viewed beside the previous native references with no layout regression.

The Linux network source hash is unchanged from the successful seven-outcome run below;
that gate was not rerun for this contract-only increment. No dependency was installed.
Evidence: `evidence/reviews/L02-bonded-registry.md`; contract/wire:
`spec/registry/bonded-snapshots-v1.md`. **Key possession, placement, paid admission, actual R=10 custody and repair remain open.**
The contract is local-only and is not yet used by the daemon to select keepers.

Previously added implementation: **automatic peer checkpoint catch-up in the daemon and desktop**.
After explicit trust selection, a client fetches bounded public certificate pages from connected,
authenticated peers. Core verifies the pinned profile, exact requested head and every signed
successor. It can catch up through expired intermediate pages without renewing their leases,
retry after disk/source failure, and serve another client after the original source stops.
One shared outbound slot/start budget, finite retries/frames and per-peer/global serving limits
bound work. Relay-only uses actual circuits. Network replacement cancels outstanding work while
preserving the start budget; stale replies cannot rebase onto an owner's newer head.

Tests preceded production and separate critic FINAL ACCEPT after strengthening the scheduling
and stale-response oracles. The full gate passed **288 Rust tests,14 Solidity tests including
256 fuzz sequences,40 frontend tests**, existing real Anvil/CLI/daemon funding verifications and
**five actual packaged hidden WKWebView flows**, including the extended automatic peer flow.
A second native client selects trust and enters a peer through UI, receives the exact checkpoint
without entering/submitting a certificate, then restores identical head/lease/trust/identity after
source shutdown and local UI/daemon restart. Screenshots were visually compared with the prior
native reference. Release bundling, deep/strict ad-hoc signature and driver exclusion passed.

The current-source Linux gate passed all seven outcomes with empty cleanup errors:
ain-nat-0f2c57b4, sourceHashbf6b526469681740190503978ea3907c15ecbd04abf0e04452eff000c283515e.
Six isolated compiling mutations were killed: shared slot, global throttle, wrong response rebase,
lost swarm cancellation, serving admission and response frame limit. Evidence:
`evidence/reviews/L06-checkpoint-network.md`; wire/limits: `spec/postage/checkpoint-network.md`.

The native "Network trust" screen retains explicit immutable trust selection and manual import as well as
automatic updates. The archive remains bounded to64 entries/256 KiB, preserving exact original
wires in atomic SQLCipher head/clock/archive transactions. Finite history does not establish
unlimited long-offline availability. **Live chain observation and attestation production are
still unimplemented**; current network acceptance uses synthetic signed test headers, never a
claim of live chain finality, spend authorization, available balance or R=10 storage.

The previous owner-authenticated checkpoint daemon API remains part of the aggregate gate:
strict raw JSON commands, current SystemTime, SQL failure isolation, exact state after restart
and unchanged actual MLS delivery/receipts. The real Anvil gate made 53 owner IPC calls with
four successful funding verifications across normal/full-width profiles, including source
outage within lease and real wall-clock expiry while daemon and source are stopped.

The prior durable Core layer preserves exact certificates, the selected manifest and maximum
observed time in one SQLCipher CAS row. Denied acceptance/funding also record higher time;
clock rollback, expired funding, forks, skips, stale revisions and profile replacement fail.
Its six tests and five isolated compiling mutation failures remain in the aggregate suite.

The prerequisite trusted checkpoint verifier remains an explicitly selected >2/3 attestor
profile with finite lease/root-age bounds and real Ethereum account/storage proofs. A dishonest
selected quorum can attest false or historical roots. There is no built-in company manifest,
trustless chain finality, whole-database rollback protection, live observer, spend authority or
available balance. Profile selection is implemented; payment UI and live observation remain required integrations. Contract
deposits remain local-only without a withdrawal path. Full V1/E01–E26 acceptance remains open.

Previously verified mailbox integration gate: **PASS on 2026-09-05**, including actual private mailbox pointer
publication/retrieval through the bounded DHT. Owner IPC now prepares durable exact publication
batches, publishes them to remote cache nodes and resolves recipient locators through real GET_VALUE
queries before persisting the checked head. Restart, cache loss, forged replies, SQL failures and
network work limits are covered with independent processes/wire counters. `scripts/check-native.mjs`
and the current-source Linux network gate both passed. The native UI continues to support the
previously verified direct/relay personal chat and agent operations; pointer controls, automatic
ciphertext/index retrieval, paid custody and R=10 repair are not yet integrated. Full V1 acceptance
remains open. That integration checkpoint covered:

- 242 Rust tests: protocol24, SQLCipher store25, OpenMLS18/private mailbox6, capabilities10, application core64, node unit26/process53, native host10 and Tauri commands6.
- 30 frontend tests, strict Rust formatting/Clippy, TypeScript and production Vite build.
- Private mailbox: nine independent Python wire vectors, six cryptographic tests, two new MLS exporter tests and six Core tests cover daily key separation, bounded month-offline lookup, fixed padding/AEAD, exact signatures/schema, epoch/member/pending-commit gates, durable batch retries, rollback and endpoint equivocation, daily lifetime renewal, SQL INSERT/UPDATE failures and bounded latest state across18 generations. This is real crypto/SQLCipher integration, not network keeper acceptance.
- Private pointer network: eight unit tests and five actual-process scenarios verify signed/finite128-record caches, expiry without replay renewal under frequent maintenance/clock rollback, shared two-query capacity,32 attempts/query,4 held jobs and16 retained rows. A sender stops after publishing; a recipient resolves from either of two real caches with its saved head removed between attempts. Independent raw Kad servers capture exact persisted retry wires and absence of publisher PeerID, return forged/valid data around SQL failures, and count exactly32 requests on a40-peer chain. A fifth held job is refused before Core commit; relay-only blocks this direct path without mutation. Publishing requires two remote cache ACKs per record but leaves ciphertext queued/replicas0. These are ephemeral pointers, not independent durable custodians. Evidence: `evidence/reviews/N03-private-mailbox-network.md`.
- Peer routing: four new unit tests and six actual-process scenarios cover a bounded table, per-peer route limits, unrelated-dial isolation, record refusal, 5/60second automatic scheduling boundaries, two active slots, two independent intermediaries, a moved recipient with an occupied old port, automatic outbox recovery, exact message/receipt persistence after all intermediaries stop, a reachable target with a damaged signature, absent-target/fallback handling and exactly32 requests measured on a real40-peer referral chain. Healthy delivery starts no DHT lookup.
- Bootstrap: six actual-process and three scheduler/admission tests verify fallback, actionable missing/unavailable state and recovery, encrypted-cache restart, self-only record exchange, isolated invalid domain/signature/session/expiry/route attacks, exact persisted-cache preservation, four held outbound slots, backoff and64/global/8-per-peer inbound limits. Relay-only uses a real circuit and sends no requests to the forbidden direct endpoint.
- LAN discovery: bounded untrusted hints expire without refresh, retain the signed cache root during route changes and share the bootstrap request limits. A new isolated Linux scenario proves real multicast joining without seeds, rejects a forged PeerID at a reachable endpoint, verifies no5353 socket after failed save/opt-out/relay-only and preserves actual encrypted chat after restart. Native tests preserve the opt-in flag while verifying relay-only suppression; no host-LAN broadcast is part of the native gate.
- Native bootstrap configuration: two new Core tests cover independent legacy bytes and bounded input; three real daemon tests cover saved hints replacing reachable CLI hints, exact retry, persisted restart without discovery cache, SQL INSERT/UPDATE failure isolation during actual MLS/receipt exchanges and live relay-only policy. UI tests cover preserved drafts, exact retry and signed-peer status.
- Real daemon address-change regression: same Bob profile/root/PeerID gets a new TCP endpoint; an actual message announces the signed route; both daemons stop; Alice restarts and queues one operation while Bob is down. Bob's restart receives that exact operation at the new endpoint and its signed receipt drains the outbox. The old port is held by the test fixture and no existing connection survives. Bob's disposable discovery cache is deliberately cleared so reverse bootstrap cannot hide a broken restored Alice route; Alice's signed persisted new route and actual outgoing endpoint are independently asserted.
- Actual packaged hidden WKWebViews: UI onboarding/invitation/contact, peer message/reply/receipts, detached daemon delivery, UI reopening with identical identity/history.
- Actual native panel → displayed config → bundled MCP process → resource-discovered dialog ID/limit → peer UI message → delivery.get observes signed receipt → reply/poll/ack → UI revocation denying context/send/delivery on the same live client. Exact histories are checked. All test processes/keychain profiles are cleaned up.
- Actual native settings first save a bootstrap address in a fresh profile, verify reciprocal signed handshakes with the independent bundled provider and render verified status without creating a chat. The same scenario continues through native settings → independently spawned bundled relay/AutoNAT service → confirmed UI diagnostics → signed invitation → MLS message/receipt. Stopping both recipient UI and daemon leaves the peer operation queued; relaunch restores preferences, PeerID/listener ports/root identity and delivers the original operation, with exact histories and receipt checked. Save is disabled for unchanged values after both save and reopen.
- Release bundle creation, deep/strict ad-hoc signature verification, and no WebDriver dependency in normal builds. Apple notarization and other platforms are not complete.
- Current-source Linux gate: all7 outcomes passed again at the native checkpoint UI increment, including actual firewall withdrawal/recovery, fresh refusals from disabled AutoNAT service and preserved relay reservations under combined service. Final run ain-nat-739ead40/sourceHash 2c92d44cf30f69fac055970fd4354263527a6003c71f17071e27ecea301a8638 passed with cleanupErrors empty. Log: `evidence/reviews/L06-checkpoint-ui-linux.log`.

Artifact: `/Users/glebk/Library/Caches/agentic-internet/worktree/target/release/bundle/macos/Agentic Internet.app`, rebuilt and verified again at the automatic checkpoint exchange increment; the desktop payment flow is not yet integrated. Release source includes native agent/network panels, bootstrap/cache reconnect, persistent route refresh and MCP packaging. Debug E2E includes the optional driver; release cannot compile with that feature.

## Implemented foundations

- Signed protocol documents: bounded canonical CBOR, strict Ed25519/network/time checks, independent wire tests.
- Trusted checkpoint verifier: explicit immutable profile, canonical SignedDocument quorum and independent checkpoint IDs, finite time bounds, pure successor checks and actual EIP-1186 funding integration. No trustless finality or persisted high-water is inferred from this stateless boundary. See `evidence/reviews/L06-trusted-checkpoints.md`.
- Automatic peer checkpoints: bounded public CBOR pages, one global outbound slot/start budget, authenticated connected sources, exact pinned-head Core verification, durable retry, rate-limited read-only serving and relay-only connection guards. Actual native follower/restart and raw-wire process evidence: `evidence/reviews/L06-checkpoint-network.md`.
- Bounded checkpoint history: strict signed historical pages, exact head anchors, atomic head/clock/archive updates, bounded prefix pruning and read-only legacy serving. Expired history cannot renew funding. See `evidence/reviews/L06-checkpoint-history.md`.
- Native checkpoint selection: read-only exact-key preview, explicit owner consent, strict Tauri ACL, manual certificate acceptance, honest lease/clock/unavailable states and durable actual native restart. See `evidence/reviews/L06-checkpoint-ui.md`.
- Owner checkpoint IPC: bounded, strict local daemon commands use the shared Core and daemon time, with separate owner/agent authority and preserved raw JSON validation. Actual daemon/Anvil integration extends the prior stateless CLI gate. See `evidence/reviews/L06-checkpoint-ipc.md`.
- Durable checkpoint Core: immutable selected manifest/issuer, one bounded CAS head, historical signature validation on restore, current-time funding validation and durable denial-time observations. Exact retries preserve original bytes; SQL failures do not release success. See `evidence/reviews/L06-core-checkpoint.md`.
- SQLCipher profile: stable root identity, single writer, atomic CAS/history/outbox/idempotency and authorization retry states; real rollback/reopen tests.
- OpenMLS: staged snapshots, 1:1/group cryptographic primitives, membership changes and bounded delayed traffic; independent raw OpenMLS peer tests. Full group application control is still pending.
- Shared core: invitations/contact binding, actual 1:1 messages and receipts, authenticated NodeRecord/reverse routes, transactional MLS and outbox updates.
- Private mailbox codec and Core state: committed MLS exporter capabilities bind network/group/recipient/epoch; daily HKDF-derived Ed25519 keys address fixed-size AES-GCM locators. Finite retention covers up to31 daily records with a three-key read window. Public validation is separate from secret decryption. SQLCipher commits exact publication batches and sequence/hash/locator checkpoints before release; same-head daily rotation renews expiry without erasing rollback barriers, and bounded state replaces old generations. No mailbox keys in scoped agents or UI snapshots and no false custody/delivery counters. Network pointer publication/retrieval is implemented below; retained indexes, registry and repair remain pending. Wire/limits/review evidence: `evidence/reviews/N03-private-mailbox.md`.
- Signed peer records: durable positive v2 sequence, canonical v1/v2 verification, same-second address changes, renewal/reopening, a64-entry encrypted cache and a confirmed-contact high-water mark that survives cache eviction/expiry. Actual successful application traffic refreshes contact routes while preserving original queued ciphertext/IDs. Cache/contact writes are atomic; a cache error after a committed incoming message recovers through durable deduplication without changing MLS/history twice. Empty reachability publication keeps the last contact dial hint while the actual current cache record remains empty; this fixed a real Linux combined-provider restart regression. See `evidence/reviews/N02-signed-peer-records.md` for wire compatibility and failure evidence. Peer-address lookup now uses the adapter below; mixed-version rolling upgrades remain open.
- Bootstrap self-record exchange: up to4 distinct explicit direct/circuit IP hints merge with up to64 signed encrypted-cache records. The new bounded libp2p protocol exchanges only each side's own NodeRecord; it never exports contact inventories or third-party records. Domain/signature/TTL/session PeerID/routes are checked before shared Core cache/route mutation; cached responses also bind the expected root. Four in-flight requests retain their slots across source refresh, retries back off up to30seconds, and successful peers refresh every30seconds. An8192byte frame bound,4096byte signed-record bound and64/min global/8/min per-peer admission limit network work. The owner sees live verified peers, network domain, attempts and actionable bootstrap-needed; disconnect/expiry removes live status. Reconfiguration restarts discovery under current relay-only policy. Actual bootstrap creates no chat or agent authority and does not validate registry/checkpoints. No new dependencies. See `evidence/reviews/N02-bootstrap-exchange.md`.
- Bounded peer routing: actual libp2p-kad0.48.0 on a dedicated protocol, seeded by already authenticated records, up to128 retained candidate peers/four routes each, two active searches,32 outbound attempts per search,128 accepted referral peers per query and bounded deadlines. Only the exact target's successful signed bootstrap and Core commit produce a verified result. Outbox failures automatically start the same resolver, limited to one start per5seconds globally and one per60seconds per recipient. Owner `lookup_peer` and `node_info.routing` expose manual diagnostics; results do not create contacts, agent authority or operator trust. Relay-only disables this initial Kad adapter while keeping existing relay delivery. New guards prevent unsolicited referrals from supplying other protocols' dial routes and reject record/provider storage. See `evidence/reviews/N03-peer-routing.md` for test-first review, wire/metadata limits and unimplemented private-mailbox work.
- Optional LAN discovery: `lanDiscovery` defaults to false in existing/new owner preferences and is explicit in native controls or the CLI. Relay-only suppresses multicast independently of the saved flag. A wrapper keeps up to32 untrusted IPv4 peers/four routes each for60seconds, rejects invalid/loopback/self/relay hints, and prevents unsigned addresses from entering other behaviours. The shared bootstrap scheduler merges these with explicit/cache sources while retaining expected signed roots; total candidate state is capped at100. Native controls preserve edits/exact retry and explain suppression. Actual Linux tests cover discovery, forged PeerID, no chat authority, real MLS/receipts, SQL failure, opt-out/restart and socket-level relay-only enforcement. `libp2p-mdns0.48.0` uses an existing locked resolution, verified current before enabling; its deprecated cache expiry API and upstream multicast flood bounds need future upgrade/resource review. See `evidence/reviews/N02-lan-discovery.md`.
- Owner network preferences: a bounded, versioned Core DTO persists relay/AutoNAT client preferences in the existing encrypted CAS store. Exact same-value retry returns the existing revision, a stale changed value is rejected, and scoped agents cannot read/change this owner configuration. Real SQL INSERT/UPDATE failures leave preferences, history and pending MLS delivery intact; reopening and subsequent actual receipt complete successfully. This was first verified as a Core slice at169 Rust/20 frontend tests, then integrated through the daemon and native controls below.
- Capabilities/broker: attenuated owner→agent→runtime grant verification, trusted epochs/revocation, finite scope/limits, per-ancestor budget preparation, nonce/fence checks and transactional real sends.
- Agent inbox: bounded UTF-8 pages, durable per-agent/service/dialog cursor, runtime-bound leases, expiry takeover, exact poll/ack retries, scan-only advancement and SQL rollback.
- Owner provisioning: random runtime key and signed grant commit together in encrypted state. Same intent/operation restores the exact key/grant; changed/revoked/expired retry is denied. Public metadata excludes keys. Private fixed-path MCP credentials are recoverable after local I/O failure.
- Independent node: libp2p QUIC + TCP/Noise, stable encrypted transport key, bounded private IPC, disjoint owner token vs agent proof, queued cancellation, actual retry/backoff and network status. TCP restart listener-port reuse bug on macOS is fixed.
- Circuit Relay v2: explicit server opt-in and up to4 distinct configured providers; validated one-hop signed routes, confirmed reservations, bounded retry, diagnostics, finite server connection/reservation/circuit limits. Real TCP/QUIC subprocesses exchange MLS messages and signed receipts, kill the sole active circuit provider, fail over, then lose both relays and restart sender/provider. Original queued operation delivers autonomously before any repeat send. Capacity pressure denies a third reservation while existing chat works, then admits it after disconnect. Ordinary daemon rejects relay service until explicitly enabled. Native host defaults to direct routing but now restores saved owner relay/AutoNAT preferences; native acceptance behind actual NAT remains open.
- Relay lifecycle: supported admission hook fixes upstream full-capacity renewal without a dependency fork. Actual pending/active peer admissions stay within configured capacity while both original clients renew repeatedly and a third remains denied. SIGSTOP causes real reservation TTL expiry while its TCP socket remains open; another client takes the slot and exchanges a message/receipt. Real circuit time and byte quota closures reconnect and preserve exact histories/outbox. Per-peer renewals and typed quota/expiry counts are bounded owner diagnostics. Raw-byte financial accounting, simultaneous pending-admission stress and broader flood/fairness remain open.
- Linux network E2E: actual compiled daemons on separate private LANs behind isolated Linux routers exchange MLS messages/receipts through TCP and QUIC relay providers. Failed direct probes increment router DROP counters; both providers observe NAT-translated sources. Active provider failure switches to the independent survivor; sender restart/provider recovery drains the original durable operation. A separate direct network control also passes. SIGTERM during live setup invalidates the report and removes only run-labelled resources. Evidence: output/network-e2e.
- DCUtR: Identify0.47.0 and DCUtR0.14.1 automatically upgrade relay connections to direct QUIC through actual Linux NAT mappings. Independent unsolicited public probes and private-IP bypass are dropped. After the sole relay is killed, both clients exchange further MLS messages and signed receipts directly. A blocked-upgrade case observes real failure and packet drops while relay chat remains usable. Explicit relay-only mode disables upgrades. Identify has no unsigned peer address cache; observed addresses are candidates rather than signed application routes or confirmed reachability. Connection admission runs before stateful protocols, fixing a real post-upgrade phantom-handler/outbox stall reproduced by the same E2E. TCP hole punching and native desktop NAT acceptance remain open.
- AutoNAT0.15.0 v1: up to4 explicit verification peers, recurring bounded probes and opt-in server service. Public-route publication requires a fresh authenticated inbound callback with matching tested transport/port; stale/superseded witnesses and bare success reports cannot publish a route. Private or unavailable outcomes withdraw public routes, successful facts expire, and later probes restore service. With verification configured, new signed invitations/NodeRecords contain only verified direct and confirmed relay routes; no-route invitation returns network_unavailable while existing chat remains usable. Actual Linux firewall E2E proves publication/withdrawal/recovery and service failure; an independent Core profile verifies the signed invitations. Closed-NAT clients keep working relay delivery and receive new explicit refusals when AutoNAT service is disabled. A combined public AutoNAT+relay case retains its reservation and accounting after callbacks. The initial recovery failure exposed a late callback retaining a connection slot; the server now tracks/closes precisely its own callback ConnectionIds. Distinct observed candidates are capped at8 per process because upstream cannot remove them; only configured providers feed that set. Direct address migration now has bounded peer routing; relay discovery and private mailbox addressing remain pending. Native network configuration is implemented below. A subsequent real process test exposed the response-before-callback ordering race; bounded pending state now pairs both events in either order without trusting bare success, and expires/supersedes incomplete probes.
- AutoNAT callback lifetime: service-side local connection establishment no longer immediately tears down the callback. A diagnostic trace proved that teardown could cause client multistream BrokenPipe despite a successful service response. Exact callback ConnectionIds now have finite leases using the existing8second probe timeout; client close/dial failure removes tracking and100ms maintenance closes abandoned or late connections. Overlapping callbacks are independent and ordinary control/relay connections are preserved. The final Linux run confirms initial public detection and firewall recovery without extra failed probes. See `evidence/reviews/N04-autonat-callback-lifetime.md`.
- Native host/UI: system Keychain bootstrap, private paths, startup serialization, detached daemon reuse/restart, explicit main/local Tauri ACL, CSP, live chat invalidation and agent permissions panel. Chat draft survives panel navigation; read defaults, send opt-in, finite TTL/data bound, exact retry and revoke errors are tested.
- Live network configuration: `network_settings` and `configure_network` are owner-only APIs with strict provider route/PeerID validation. Configuration construction and encrypted CAS commit precede any transport change; SQL failure preserves the working circuit and exact history. A fresh swarm retires old handlers while preserving transport identity, assigned TCP/QUIC listeners, shared core/IPC and durable operations. Saved client preferences override defaults on restart; server opt-in/quotas remain separate operator flags. Exact same-value retry returns the existing revision without reconnecting. A new TCP first-hop connection avoids macOS TIME_WAIT reuse on relay replacement/restart. Existing authenticated inbound circuits remain usable for replies to peers with direct-only signed records. Actual node tests verify provider replacement, a fresh incoming contact, subsequent reply, relay-only closure of previous direct connections, and automatic draining of the original queue when allowed routing returns.
- Native network panel: explicit local main-window ACL, bootstrap/relay/AutoNAT lists, relay-only switch, explicit save/reconnect, live confirmed route/reachability diagnostics, exact original-revision retry and explicit stale reload. Background polling preserves edited fields and chat drafts survive navigation. A real native screenshot exposed JSON field-order comparison mistakenly enabling Save for unchanged settings; tests now use real response ordering and the component compares field values. Native Save-disabled assertions and visual review cover both save and restart.
- Native bootstrap preferences: the new `bootstrapPeers` field extends the version1 encrypted settings contract with an empty legacy default and no read-time rewrite. Saved hints replace CLI defaults; current signed cache remains independent. Validation/construction and encrypted commit precede new dial activity. The panel exposes verified signature counts and blocked direct hints while preserving the original save revision during polling/retry. A fresh native profile joins through an entered provider and restores that field after daemon restart. See `evidence/reviews/N02-bootstrap-preferences.md`. The full gate also corrected the relay failover oracle to inspect all reciprocal circuits and require one common provider before killing it; no alternative provider circuit can hide a failed reconnection.
- Official rmcp3.2.0: modern2026-07-28 discovery and legacy2025-11-25 initialization, strict tool schemas, bounded stdio, private credentials, cancellation/EOF and process lifecycle tests. Listed capabilities never expand a daemon grant.
- Runtime discovery: authenticated metadata with max32 dialogs/max16KiB and no message history. Send-only runtimes discover recipients without read permission. Each resource read checks live authority and atomically commits nonce/fence; SQL failure is retryable with the same proof. Legacy signed snapshot is narrowed to metadata with its existing ReadInbox requirement. Modern resources are private/immediately stale. Both protocol versions exercise real scoped discovery, send, denied read and revoke; cancellation closes resource IPC too.
- Actionable agent errors: shared typed failure DTO for signed IPC/MCP distinguishes busy/expired leases, page size, changed idempotency intent, availability and opaque authorization. Retryability is explicit; private SQL/path diagnostics are excluded. Actual SQLCipher failure leaves all persisted states/history/outbox unchanged; repaired same-intent/same-proof retry delivers one message. Same live MCP resumes after a real daemon restart, and sibling runtime recovers expired inbox work.
- Scoped delivery status: runtime principal + original operation ID select its exact durable send, with current SendMessage authority and matching conversation required. No history text or other runtime/owner operations are exposed. Status shares the UI's receipt/outbox calculation and survives restart. Reads preserve budget/MLS/application state; failure to commit nonce returns no result and permits proof retry. Both MCP versions exercise offline recipient and sender/recipient restart before actual delivered status. Replicas remain0/target10; direct delivery is not replica durability.

Every new backend slice started with tests and a separate backend-test-critic without context fork. Review decisions and RED/GREEN evidence are retained in evidence/reviews. No complete upstream task is declared accepted merely for its implemented subparts.

## Visual evidence

Headless component fixtures in output/playwright were inspected at1280×840 and900×650 against prior chat/contact references. Agent form/config/details/revoked screenshots are retained. Actual packaged WKWebView screenshots in output/native-e2e (including alice-agents-active.png and alice-agents-revoked.png) were then viewed beside those references. The native result.json records all four successful product flows above. Network component screenshots at1280×840 and900×650 (including scrolled diagnostics) and actual native restored settings were also viewed alongside their references. LAN and bootstrap component wide/compact Save/details views and actual native bootstrap-connected/restored screenshots were also compared by own vision. The form scrolls without overlapping controls. Component fixtures never enter the production entry point.

## Quiet-tip recovery checkpoint

Two real-service liveness regressions are implemented test-first and independently
accepted: finite finality-packet loss and a quiet notarized tip after two honest
view timeouts. Baseline runs failed with [4,5,5,5] and [4,4,4,4] histories after
healing. The combined candidate passed all 75 finalizer tests and Clippy, then the
main workspace passed 458 Rust tests, 29 contract tests and all EVM/TCP/QUIC,
frontend, formatting and static checks. Native, Linux, paired visual checks and the
new ad-hoc-signed release also passed; this is not full V1 acceptance. Evidence: spec/finalizer/quiet-tip-recovery.md and
evidence/reviews/P01-daemon-scope-replay/quiet-notarization-recovery.json.

## Next active work

Follow [AR1–AR5](Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md) and
[the 24-finding action register](Docs/agentic_internet_v1_execution_plan/architecture-followup.json).
Begin with durable expiration/retirement of already prepared sender jobs, preserving
message identities and exposed ticket allocations. Successful terminal retirement,
unprepared queue deadlines and ready scheduling remain separate steps. Then replace
issuer full-prefix lifetime128/epoch1 with indexed spent state and authenticated
handover without resetting spent keys. These are independent limits.

Continue network index integration from `evidence/reviews/D05-paid-index-store/NEXT.md`
within AR3: authenticated holder locations, real put/read, stable book/epoch routes,
complete history and autonomous R10 with both clients absent. Finish remaining UI/CLI,
groups/recovery, credentials, transport economy and three-platform release gates.
Tests precede backend changes; separate no-context critic ACCEPT precedes production.

Agent orders, service discovery/A2A, reviews and ratings are V2 by explicit user
request on 2026-09-09. Do not expand these features before the first release or use
their unfinished acceptance as a V1 blocker. Existing code and evidence are retained.
The 64-node capacity defect stays parked with its failure evidence.

## Required unfinished acceptance

The 22 mandatory current V1 scenarios have not all passed; E12/E13/E15/E16 are V2. Passing this checkpoint does not make the whole application V1-ready. Required work still includes discovery/bootstrap/relay/NAT, independent R=10 storage and autonomous repair, offline recipient retrieval/indexes, group control/finality, complete device/recovery epochs and delegated registry, identity credentials/issuers, transport economics and budget settlement, read-state/search/attachments/notifications/backup/settings, and supported-platform release gates.

The historical package retains 84 cards. V1 acceptance now requires the 67 active cards in their amended scope; 17 cards are deferred to V2, not accepted. Scope remains Docs/agentic_internet_v1_execution_plan and Docs/agentic_internet_v1_1_plan; no V1 requirement is silently postponed by these incremental milestones.

## Workspace

Canonical source and independent Git: `/Users/glebk/Code/chat` (internal APFS).
Published source: `https://github.com/glebkudr/kaikichat`. The unfinished daemon-service
checkpoint is preserved in `codex/storage-migration-backup` at `0387947`; storage
changes are retained in `codex/storage-migration`, with development continuing on
`implementation/v1`. The old external checkout is archival.
Build data uses symlinks into `/Volumes/ChatBuild`, backed by
`/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle`. Use
`python3 scripts/build-storage.py run COMMAND...`; it verifies the configured image,
volume UUID and links before execution. Sources and Git do not require WD4000.
Dependency versions remain locked. Fresh backend, frontend, Linux/native and
daemon-service fixture gates passed after migration; full V1 acceptance remains
open. Storage rules and commands are in `AGENTS.md`; verification, the initial
intermittent EVM timeout and its successful rerun are recorded separately in
`Docs/maintenance/storage-migration-result.md`.
