# A04 retention and H10 comparison criteria

Task: `01a09fc7-b11a-7df0-876b-4890ae19a78b`; source base
`implementation/v1@252e0f5`. These criteria clarify the existing V1 scope.
They do not change protocol TTLs, scenario deadlines, trust, allowances or
limits, and do not declare any native scenario accepted.

## A04: retain compact facts until profile deletion

Keep compact `postage/sender/completed/` and `postage/sender/terminal/` facts
for the lifetime of the local profile. Do not introduce automatic age-based
deletion of these facts. They preserve historical status and work with the
retained original/operation mapping to make retries idempotent. Envelope or
reservation expiry makes a retired full job eligible for compaction; it does
not make its historical outcome disappear.

Acceptance requires all of the following:

1. Compact only already completed, expired or canceled jobs, after their
   existing immutable envelope expiry or effective unexposed reservation
   expiry. Never use GC to retire queued work or extend an expiry.
2. Atomically replace each eligible full `postage/sender/job/` row with its
   existing version-1 compact representation. Preserve exact observation,
   failure code and observation time, retention, and terminal reservation
   deadline. Preserve completed/v1 compatibility. No renewable authority is
   copied into the compact row.
3. Preserve the original message, operation mapping, allocated/spent resource,
   immutable preparation, history/mailbox links and unrelated live evidence.
   An expired current custody state must remain expired even when historical
   publication evidence is retained. This collector does not reclaim all
   other evidence stores.
4. Measure `count`, `maxPayloadBytes` and `totalPayloadBytes` separately for
   completed and terminal rows, using the actual serialized `states.bytes`.
   Each compact row must be at most **4,096 bytes**. For the same compaction
   cohort, report full-job bytes before and compact bytes after, and show a
   reduction. Report retained original/preparation/history/mailbox/evidence
   bytes separately; do not hide them in the compact-row measurement.
5. Keep the existing bound of **16 job namespaces scanned per pass**, including
   queued or ineligible jobs. In one continuously running daemon, schedule the
   next pass **60 seconds after the preceding pass finishes**, including a
   failed pass. Startup is immediately due and resets the in-memory cursor;
   this is not a global 16-per-minute limit across restarts. Observe actual
   backlog progress and per-boot monotonic timestamps.
6. A real SQL refusal late in a multi-row GC transaction must leave the entire
   batch and rows outside its write set unchanged. Repeat the refusal after a
   cold restart. After removing it, bounded maintenance must finish, preserve
   the live guard, and not block unrelated sender work.
7. Cold restart must preserve exact delivery/failure/funding projections.
   Replaying the same authorized original must not import, allocate, spend,
   enqueue or increment counters again. Conflicting bodies and revoked
   agents must remain rejected; canceled/expired jobs must not revive.
   Repeated maintenance must leave compact facts unchanged.

The compact ledger grows with operation count: for N compact rows, payload
size is at most `4096 * N` bytes. This is not bounded lifetime storage.
SQLite keys, indexes, pages, encryption overhead, WAL and other stores are
outside that payload bound. Do not claim physical database shrinkage from a
smaller serialized payload; report physical file sizes separately if measured.

The Core tests in `crates/core/tests/support/public_sender_progress.rs` and
`public_sender_terminal_gc.rs` cover the transactional contract. The actual
daemon test in `crates/node/tests/support/public_sender_gc.rs` must additionally
produce its three `A04_GC_METRICS` records. Its local GC fixture is not paid
network acceptance. The two real paid series of 129 messages, slot reuse and
sender-absent history recovery remain separate A04 requirements.

## H10: preserved aggregates and unavailable historical measurements

The user confirmed that the original `output/hrt32-r1` files were permanently
lost during the WD4000 reformat. Their SHA-256 values in
`evidence/reviews/AR2-wallet-flow/diagnostic32-native-r1.json` identify the
historical inputs; hashes do not recover the missing bytes.

| Measurement | Retained R1 evidence | Valid comparison |
| --- | --- | --- |
| Prepared page kinds / leaf occupancy | 23 leaf, 23 root, 19 branch; leaves `1:16, 2:5, 3:2` | Same definition on the new verified publication graph |
| Queued / finished requests | 57 / 57 | Descriptive observed counts; recovery endpoints differ |
| Applied / selected | 29 / 19 | Keep event/selection definitions distinct from semantic reads |
| Global read-rate waits | 60 | Wait-event count only; **not admitted requests** |
| Recipient read / bulk / history-path counters | 48 / 16 / 15 | One process snapshot before the first exact SQL gate; no delta against successful phase totals |
| Publication interval | 542 seconds | First original issued to last sender observation; **not full run wall time** |
| Semantic new / repeated reads | Not retained | Historical value and delta remain `null` |
| Admitted requests | Not retained | Historical value and delta remain `null` |
| Full run wall time | Not retained | Historical value and delta remain `null` |

The new Diagnostic32 must record full functional results and per-boot raw
traces with semantic query identity, explicit admission, queued/finished
attempts including failures/crashes, phase boundaries and outer monotonic
wall time. Trace identities exclude transport nonce/signature/request time
but preserve query bounds, target and trust scope. Their numerical results
must come from the current trace, not reconstruction of historical counters.

Mark missing historical comparisons `blocked-by-data-loss`. Current functional
acceptance and the completeness of current observations are separate from
full R1 comparison acceptance. Same-definition aggregate deltas are
descriptive only, because R1 stopped before the first exact-original SQL
gate. Do not claim a read-efficiency or wall-time improvement against R1.
A new baseline may support future comparisons, but cannot replace R1 silently
or retroactively change its measured hardware/time.

## H11 and platform boundary

The full publication counts remain 130 originals, 390 signatures, 1,300 data
receipts, 1,300 index receipts and 13,000 location ACKs, followed by actual
loss of 1,170 data and 1,170 index records. Required receiver states remain
129/incomplete, 130/complete and cold 130/complete, with sender offline,
untrusted-source refusal, exact SQL-failure recovery and no duplicate import.
Cold recovery preserves recipient progress/trust/head/root/cursor and traverses
all 130 links using surviving custodian evidence. It does not require all
pre-loss receipts to be stored in the recipient profile.

A05 still requires a real macOS bundle and codesign. Linux preparation or
component test results cannot satisfy that gate.
