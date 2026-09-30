# A04 lastImportFault follow-up (2026-09-24)

Branch: `fix/v1-custody-lookup-pacing-20260924` from `da80cbb`.
Commits: `8f663c0` tests → `7ad55a8` fix (pointer lookup pacing);
`3211284` tests → `21d1328` fix (exhausted holders last);
`4b86b56` test → `0874bbc` fix (route suppression lapses with the read window).
Merged into `implementation/v1`: fixes 1–3 (through `7b09b73`). Fix 4 (section 4,
`4025510` → `f23fdcf`) is held on `fix/v1-bootstrap-route-announce-20260924` until
the publication stall seen once on `f23fdcf` is shown to be unrelated.
Diagnostic-only branch kept aside: `wip/a04-admission-40-diagnostic-20260924`
(`01cfbe4`, Admission 32→40; not a fix, must not be merged).

## Handoff hypothesis: not confirmed

The 2026-09-24 handoff suspected admission tokens leaking on `custody_capacity`
in `queue_custody_sync_in_domain`. Traces refute it for A04:

- 21.09 Diagnostic trace (`output/a04-trace-r1`): 438 admitted = 438 queued,
  0 `custody_read_not_queued`.
- Probe of five 22–23.09 runs: recipient `paidCustody.pending ≤ 1`,
  `verifying ≤ 1`; `custody_capacity` (4 in flight or 16 unfinished results)
  is unreachable for one work item per conversation.
- HEAD trace runs below: `not_queued = 0` in every stage.

The pre-check/gate mismatch in `custody_sync.rs:686` vs `paid_custody.rs:649`
exists in code but is not an A04 factor. `9f1ca73` had already raised the
recipient window from `Admission<24,12>` to `<32,16>`.

## Root causes found and fixed

### 1. DHT pointer lookup storm (regression from 9f1ca73)

The 1 s rescan recreated every released custody-sync work with a new mailbox
pointer lookup (3 read keys). Firstimport trace on `da80cbb`
(`output/a04-trace-head/runs/20260923T215514Z-bcbb59412e83`): 148 work exits
and 149 lookups in 120 s (~900 getRecord) against the seed's 128/peer/min
record admission; the seed recorded `getRecord.admission` 1393 rejections and
the recipient 193 `UnexpectedEof` query errors, so holder route lookups failed.

Fix `7ad55a8`: a conversation's pointer is looked up at most once per 30 s idle
interval (per conversation, bounded, cleared on network replacement); released
work is recreated from the retained head. After the fix: 8 lookups per 120 s,
0 query errors, 0 seed rejections
(`output/a04-trace-head/runs/20260924T000525Z-43190b697ceb`).

### 2. Paid reads spent on holders that already answered complete

The recipient read window (32/min) is saturated in lastImportFault
(acceptance run on `7ad55a8`: 704 reads in 1300 s, 251/257 imported, 2.8
reads per import; 257 needs ≤ 2.7). Trace of lastImportFault on `7ad55a8`
(`output/a04-trace-last/runs/20260924T004400Z-9d17706599cf`, diagnostic run
with a 600 s first-stage wait only): 672 reads = range 376, index 145
(75 rejected), locations 82, history_page 69 (41 unavailable).
87 range reads (13%) went to holders that had already answered a complete
range; 0 of them returned a body (82 to the four data-less index peers).

Fix `21d1328`: holders whose durable prefetch position (same head) is
complete are ordered after every other holder, across route classes; they
remain fallback candidates. Existing route/unqueried order is unchanged
otherwise.

No admission limit, byte/entry limit, lease or deadline changed.

## Tests

- New managed-time tests `custody_sync_lookup_tests.rs` (3) — RED on `da80cbb`,
  independent backend-test-critic ACCEPT after one revision.
- New `a_holder_that_declared_no_further_copies_yields_to_an_unconnected_holder`
  — RED on `7ad55a8`, critic ACCEPT.
- `cargo test -p agentic-node --lib` (RUST_MIN_STACK=16 MiB): 414 passed,
  1 failed — `service_discovery::refresh_tests::unavailable_minority_…`
  fails identically on clean `da80cbb` (pre-existing).
- Pre-existing on `da80cbb`, unrelated: `route_preferences::a_missing_page_…`
  overflows the default 2 MiB test stack; `cargo fmt --check` and
  `clippy -D warnings` fail on files from `9f1ca73` (mailbox_queries.rs,
  routing.rs, postage-spend). No new fmt/clippy findings in changed lines.

## Native runs

| Run | Source | firstImportFault | lastImportFault |
|---|---|---|---|
| r27 `20260923T102626Z` | `9f1ca73` | 61.4 s | 243/257 FAIL |
| trace `20260923T215514Z` | `da80cbb` | FAIL 120 s | — |
| acceptance `20260923T230920Z` | `7ad55a8` | 64.6 s | 251/257 FAIL |
| trace `20260924T000525Z` | `7ad55a8` | FAIL 120 s | — |
| diagnostic `20260924T004400Z` | `7ad55a8` | 63.1 s (600 s cap) | 251/257 FAIL |
| **acceptance `20260924T014941Z-7a4ac321ef1e`** | **`21d1328`** | **61.7 s** | **PASS 257/257 in 1047.7 s** |

The `21d1328` acceptance run is the unchanged command
`python3 scripts/build-storage.py run env -u AIN_NATIVE_PREPARE_ONLY
AIN_NATIVE_PREPARED_MANIFEST=/Users/glebk/Code/chat/output/a04-budget-21d1328-prepare/release/prepared-artifacts.json
python3 tests/evm/public_sender_capacity_a04.py` (production binary, no probe).
`passed: true`; coldPointerPass 2.7 s, firstImportFault 61.7 s,
lastImportFault 1047.7 s of 1300 s, fullImport 258/258, fullColdGraphCycle
98.3 s; 2580 index promises, 2580 data replicas, 25800 location ACKs, 774
verified signatures, 3 authority renewals.
check.json sha256 `dca21a2edce11984cbed600878efe64a51321cbb98f01a650a19f4853dd6753e`,
sourceHash `9f4a2256bdfc15879dbef74317e3078dfe3d7ccfb857488fd0d6bd03ae8bb04a`,
node binary sha256 `6c11a57ca4e0a4358af037bd7a563b2f50c90a08a9ee9034cd825bcab01c4603`.

## H10 Diagnostic32 regression (same `21d1328` prepared artifacts)

Command: `python3 scripts/build-storage.py --output output/a04-budget-h10-runtime-macos-rN run env
AGENTIC_DIAGNOSTIC_OUT=/Users/glebk/Code/chat/output/hrt32-a04-budget-rN
AIN_NATIVE_PREPARED_MANIFEST=/Users/glebk/Code/chat/output/a04-budget-21d1328-prepare/release/prepared-artifacts.json
python3 tests/evm/public_history_read_trace.py` (the output path must be absolute:
the worker tripwire is passed as `--postage-prover`; r1 aborted on a relative path).

| Run | Result |
|---|---|
| r2 `20260924T024007Z-134428db8af0` | FAIL firstImportFault 120 s (same route-suppression race as below) |
| r3 `20260924T024910Z-b333f8ba0ebc` | **PASS** `diagnostic32NativePassed: true`; first 69.6 s, last 10.0 s, cold graph 7.1 s; 96 signatures, 320+320 replicas, 3200 location ACKs |

Earlier H10 passes (17–18.09, before 8dcedc3): firstImportFault 63.1 s / 61.2 s,
lastImportFault 153.4 s / 213.4 s.

## 3. firstImportFault race (pre-existing since 8dcedc3/9f1ca73)

A second `21d1328` acceptance run (`20260924T025451Z-49a761648434`, check.json
sha256 `9a740b9fc6aef1aabdceba1db739e32347d7fc0e5a861387b2714683057bd2a5`) and
two of four H10 runs failed at firstImportFault. Cause: right after the
recipient restart, exact DHT route lookups for data holders return not-found
unless the holder is already in the recipient's small peer cache (bootstrap
exchange returns only the peer's own record, and the seeds' live route map is
empty for the first ~17 s; probe below). Since 9f1ca73 a concluded not-found lookup marks the
holder failed at once, and `ROUTE_FAILURE_WINDOW` (60 s, 8dcedc3) suppresses it
until ~61.6–62.7 s, while the second 60 s read window opens at ~60.5 s (the
first read precedes the first lookups). The pass-leading retry of original 1
meets only suppressed holders, yields, and waits for the third window (~120.5 s)
— after the 120 s gate. Passing runs (61–70 s) win this race by 1–2 s.
The route-suppression window is documented as matching `Lookup::automatic_after`,
but that gate applies only to automatic outbox lookups, not to custody
`lookup_peer`.

Probe (diagnostic H10 wrapper polling recipient and both seeds every 0.5 s,
`output/h10-route-probe/bob-seeds.jsonl`, run `output/hrt32-route-probe-r3`):
both seeds' live `routing.knownPeers` hold 2–3 peers from the recipient restart
until ~17.6 s and refill to 18–19 within ~2 s, while their node-record cache
(`cachedHints` 18–19) and verified peers stay full; every holder lookup in the
empty phase concludes not-found (2 requests = both seeds). In 17.09 H10 runs
(before 9f1ca73) the same early lookups succeeded. The seed-side lapse is filed
as a separate task; it is not fixed here.

Fix `0874bbc`: a route failure stays suppressed for the rest of the read
window it failed in (still capped by 60 s) and lapses once a read opens the
next window, so each failed holder gets one fresh route attempt per window.

### Native validation of `0874bbc` (prepared `output/a04-route-0874bbc-prepare`)

| Run | Result |
|---|---|
| H10 r1 `20260924T040909Z-c71ec1b11050` | **PASS**; firstImportFault 61.1 s, lastImportFault 67.6 s |
| H10 r2 `20260924T041456Z-a38b050804c7` | **PASS**; firstImportFault 61.6 s, lastImportFault 61.4 s |
| A04 r1 `20260924T042101Z-c6e0754b5f49` | FAIL firstImportFault 120 s; `routeSuppressions` 0, the first original's surviving holder `UxbVey` was connected and verified — a different first-stage cause, not routes (needs a trace) |
| A04 r2 `20260924T045506Z-b3a1749dafd7` | **PASS**; first 62.3 s, last 1182.8 s (257/257), full 258/258, cold graph 72.9 s; check.json sha256 `054be0753f655f9ad76191c9c54346c80c947567f1f5bbdf3e4f0c9abfdd3b12` |

## 4. Seed route lapse after a network change

`configure_network` (the scenario's `node.connect(seed_addresses)`) clears the
listener set and makes the explicit bootstrap peers due at once; listener
addresses return only with `NewListenAddr`. The first exchange therefore signs
a node record with no route. The seed verifies and caches it, but
`routing.remember(peer, [])` early-returns, so its live route map (used for
exact FIND_NODE referrals) lacks the node until the ordinary 30 s success
refresh brings a routable record. The probe timeline matches: custodians
reconnected at ~183 s, seeds' `knownPeers` stayed 2 and refilled at ~213 s in
batches of 4.

Fix (`4025510` test → `f23fdcf` fix, after merging `origin/implementation/v1`
with PR #4 and #5): a node remembers the advertised routes carried by its last
record; when they change, verified healthy bootstrap candidates are due now
(failure backoff and in-flight requests keep their schedule; an unchanged
route adds no exchange). Two real runtimes on the managed clock: RED on
`0874bbc`, independent critic ACCEPT after one revision.

## Final validation on `f23fdcf` (prepared `output/a04-final-f23fdcf-prepare`)

| Run | Result |
|---|---|
| H10 r1 `20260924T060420Z-02dce372fb84` | **PASS**; firstImportFault **5.3 s** (was ~61 s), lastImportFault 62.1 s, cold graph 2.4 s; check.json sha256 `92e1080b0b108a3a83218a659c4eb082dda515ae457a3df2f120a97604eec5de` |
| A04 r1 `20260924T060901Z-505c422ad068` | FAIL in **publication**: 5/6 batches complete; in the last batch one original kept 9/10 data replicas, position 4 cycling `resolving`/`blocked unavailable` (put state `rejected/unavailable`, custody resolution `offersDropped` 4) for ~600 s. No custodian recorded a put refusal. This state never appeared in the earlier runs checked (`capacity`, `not_found`, `stale_response`, `transport_failure` only); a link to `f23fdcf` is neither shown nor excluded. |
| A04 r2 `20260924T064311Z-0ac7de328d68` | **PASS**; firstImportFault 63.7 s, lastImportFault 1230.0 s (257/257), full 258/258, cold graph 88.9 s; check.json sha256 `e08c19689c4f867e592985a29561c24abbc50d4dc8ea5809774c536d2660d9b1` |

## Summary of unchanged A04 acceptance runs after the fixes

| Source | Runs | lastImportFault when reached |
|---|---|---|
| `21d1328` | PASS, FAIL first stage | 1047.7 s |
| `0874bbc` | FAIL first stage, PASS | 1182.8 s |
| `f23fdcf` | FAIL publication, PASS | 1230.0 s |

lastImportFault passed in every run that reached it (3/3), but its margin to
the unchanged 1300 s budget varies (70–252 s). Remaining risks: the new
publication stall above, and the A04 first stage still completing at ~62–64 s
(the second read window) even with the seed fix, while H10 now completes it in
5.3 s.

Not done: merge into `implementation/v1` and push (awaiting owner approval).
