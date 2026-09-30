# AR1: expired prepared sender retirement

Addresses a bounded part of architecture finding AR-R02. Full V1 and AR1 remain open.

Original review: `eea229c4850235a22d28a5ddc60208e62a60ee46`.
Current working source: `validated-inputs.json` records code, test, spec and build
inputs for this validation. Reports count each gate separately; targeted tests
are a subset of the final workspace run, not additional unique tests.

## Change

Core authenticates the signed expiry of saved public message preparations, then
CAS-commits job.state=expired and removal from the bounded active index together.
The retained empty index carries a monotonic retirement floor and old queue records
remain readable with its default zero value. Idle passes do not rewrite the store.
The original job, sponsorship usage, native ticket allocation, MLS message,
direct outbox, dedup records and custody evidence remain intact. Current policy
retention cannot extend a prepared promise; unprepared work remains queued.

Daemon maintenance invokes this transition before selecting active work. Its status
route reads retained terminal jobs after restart and derives recipient delivery from
Core's actual outbox state. Expiry does not imply delivery, spend finality or refund.

## Evidence boundaries

- Six new Core tests use real SQLCipher, actual MLS peers, signed runtime calls and
  the existing independently generated funded public-book fixture. They cover the
  expiry boundary, first cleanup after checkpoint lease expiry/cold restart,
  128 active slots followed by admission/decryption of message 129, exact replay,
  next ticket index 1, revoked runtime, corrupt evidence and SQL transaction rollback.
- One real daemon-process test uses actual MLS delivery/ACK and original signed
  envelopes, then restores explicit queue/preparation state. With the recipient
  stopped, a second message remains undelivered. Automatic expiry, SQL failure,
  retry and two restarts preserve delivered:true/false respectively and invent no
  spend, receipts, pointer or replicas. This is a process-wiring oracle, not a
  fresh live-EVM funding/QC/custody scenario.
- The 129-message test crosses the active admission cap and allocates two stamps.
  It does **not** cross the separate 128 finalized-spend limit.
- `red.json` records the expected missing Core APIs. `critic-revisions.json` records
  REVISE → REVISE → ACCEPT before production and the actual daemon behavioral RED.
  Original run logs remain under `output/ar1-expired-sender/` on managed build storage.

## Still required

Successful nonexpired terminal retirement must preserve all history routes before
removing work. Unprepared admission timeout, ready scheduling, historical-record GC,
indexed issuer spent state, bounded snapshots and epoch handover remain AR1 work.
The spend lifetime cap is unchanged. Independent network index/full history/R10,
UI/CLI/wallet, groups/backup, legacy build isolation and three-platform release
remain mandatory in the updated plan. No new packaged native or live-EVM gate is
claimed for this slice. Detailed results: [checks.json](checks.json).

Final combined regression: 812 workspace tests, 0 failed/ignored; 59 frontend, TypeScript/Vite, fmt/Clippy, 7 model and 12 EVM-model tests passed on 656 unchanged source inputs.
