# A04 sender CPU follow-up (2026-09-24)

## Finding

macOS `sample` (8 s, 1 ms) of every node of a native A04 attempt during
publication (`4e2e9a8`, two concurrent attempts): the sender's single runtime
thread was 100% busy at 420/600/900 s while custodians and seeds used 0.2–6% of
theirs. Publication was bound by sender CPU, not by custodian response latency.
Inclusive shares of the sender thread:

| Snapshot | fence_public_sender_operations | sender_queue | VerifiedDocument::decode | Runtime::command |
|---|---|---|---|---|
| 420 s | (part of advance/reconcile) | 27% | 27% | 7% |
| 600 s | ~50% | 37% | 18% | — |
| 900 s | 80% | 40% | 15% | 31% |

`fence_pending` ran before and after every IPC command and in several
maintenance paths; each call read and parsed every active job row (~60 KB JSON
each, 258 jobs ~15 MB). `VerifiedDocument::decode` re-ran Ed25519 for the same
checkpoint attestations, receipts, envelopes and index entries on every load.

## Changes

- `perf/v1-verified-document-cache-20260924` (12a7009 test, 27dcf14 fix):
  `VerifiedDocument::decode` remembers, per process, the SHA-256 ids of wires
  whose Ed25519 check passed (16384, FIFO) and skips only the key parse and
  `verify_strict` for them; every other check runs on each decode. Critic
  ACCEPT after two rounds (signature carried over other unsigned bytes,
  process-wide sharing, exact bound).
- `perf/v1-fence-memo-20260924` (541187c test, 0a2bafd fix): Core returns the
  last fence answer again only for the same store write token read after the
  call (`total_changes` and `PRAGMA data_version`, so a commit through any
  connection invalidates it), the same second and the exact ordered
  (message, operation) requests; failures are not kept. Critic ACCEPT after
  three rounds (another connection's commit, token read after the call, no-op
  UPDATE does not change `data_version`).

Checks: `agentic-protocol` 6 + 24, `agentic-store` 42, `agentic-core` all
suites, `agentic-l2-adapter`, `agentic-finalizer`; node lib 420/422 with only
the pre-existing `service_discovery::refresh_tests::unavailable_minority…`
failure.

## Native comparison (unchanged `public_sender_capacity_a04.py`, `AIN_A04_OUT`, one base and one perf attempt started together)

| Attempt | Source | Result | first | last | batch deltas (s) | publication span | to recovery | check.json sha256 |
|---|---|---|---|---|---|---|---|---|
| ab-base `20260924T170214Z-87fc2ec1cae9` | 4e2e9a8 | PASS | 2.8 s | 1049.7 s | 270 293 365 320 379 (mean 325) | 1813 s | 1975 s | `988bd72ca44538b935a2b293227e9161389e79ce1d4010d00b6df11394e1d818` |
| ab-perf `20260924T170214Z-42d41e50f226` | d07810b (both changes) | PASS | 64.3 s | 930.0 s | 161 172 178 178 179 (mean 174) | 998 s | 1133 s | `7e30af127db64e983a9267643e12f4252b6ccd82f7146e24549324d3671a4d15` |

The attempt finished ~15 minutes earlier (37.5 vs 52 min). Sender thread at
600/900 s after the change: `VerifiedDocument::decode` 1.1%/0.4%, fence
0%/16%, `sender_queue` 29%/45% (now mostly from execution checks in
`advance_public_sender_at`), `Runtime::command` 16%/19%.

## Sender queue validation memo

`perf/v1-sender-queue-memo-20260924` (001d407 test, accc435 fix, merged fcc0389):
`sender_queue` reads the store write token before validating and returns the
last validation while it is unchanged; after any write by any connection it
validates every job again and keeps the result only if the token did not move
meanwhile (the fence memo got the same guard). Critic ACCEPT after two rounds
(setup reads pre-warming the cache, a Core write outside the queue row,
corruption reported as InvalidState on every query). The disconnect arm of
`immutable_range_completion_keeps_the_real_queue_pointer_connection_and_request_bytes_fences`
had become flaky once cached signatures made range verification finish before
the local close event; 2212aff (critic ACCEPT) observes the close first.

| Attempt | Source | Result | first | last | batch deltas (s) | publication span | to recovery | check.json sha256 |
|---|---|---|---|---|---|---|---|---|
| ab2-base `20260924T185629Z-025d4a5dcdca` | 3f63711 | FAIL first stage 120 s | — | — | 170 161 185 177 168 (mean 172) | 1005 s | 1138 s | `b386c5290a730bd2e7f54f53d227cc8db57a97740d4ca2be5089ac546b934400` |
| ab2-perf `20260924T185629Z-f7321315eefc` | accc435 | PASS | 63.1 s | 930.5 s | 148 138 150 144 146 (mean 145) | 863 s | 1000 s | `0359aca8a7825caea6d3136a26a079c368a756c533a08bc3c3768f64db5a04ff` |

Sender thread at 600 s: `sender_queue` 34% → 6%; the largest remaining cost is
IPC `Runtime::command` (56% in the perf attempt). By 900 s the perf sender is
93% idle (publication done).

## Open

- IPC command handling is now the largest sender cost during publication.
- First stage: on `4e2e9a8` one validation attempt (`20260924T154631Z-da4f04d575e7`,
  check.json sha256 `01b85d8a1176f179f31892f11bd44c5013ee56fb71025370dde7db36579aa466`)
  failed at 120 s: the recipient spent both read windows (64 reads, 27 failures)
  while the first original's pass yielded to admission; its pair
  (`20260924T154650Z-0a9afaa62e71`, sha256
  `b158603ebf27bbbd27282ed393d4b1f1887480c7960c54729297c7817e35d8b5`) passed
  (7.4 / 1117.3 s). With fix 4: 8 PASS, 2 FAIL at the first stage (the second
  failure is ab2-base above); passing attempts take ~3–7 s or ~62–71 s (second
  read window).
