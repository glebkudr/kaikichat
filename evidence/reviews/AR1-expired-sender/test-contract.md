# AR1: retirement of prepared expired public sender jobs

Requirement: stop rescanning genuinely expired prepared work after restart; free active
queue capacity without renewing or refunding an exposed stamp, erasing sponsorship,
removing message/outbox/history, or inventing storage/delivery success. Unprepared
work does not expire at createdAt + retention: retention begins at preparation.
Only native public sender is in scope. Full spent log continuity, successful
terminal retirement and fair scheduling are separate remaining AR1 work.

Proposed internal Core API:
- retire_expired_public_sender_jobs(now) -> number of atomically retired jobs;
  authenticate original immutable prepared envelope/expiry, set job.state=expired,
  remove from active list in same ProfileStore CAS transaction, keep exact job
  and all wallet/receipt/idempotency evidence. Trust daemon time, keep a durable
  monotonic retirement floor on the queue; no writes for an idle pass.
- public_sender_job(message_id) -> optional retained job, including expired rows;
  invalid IDs/corrupt state fail closed. Active list returns only queued jobs.
- Node pump calls retirement before reading active jobs. Read status of retired
  IDs returns durable expired state and actual delivery fact, without manufacturing
  final spend/storage success; restart is allowed without live checkpoint authority.

Tests reuse actual funded-book fixture, SQLCipher, OpenMLS conversation, signed
broker grants and SQL triggers. They cover preparation-time expiry boundaries,
unprepared preservation, ACK/retry/cold restart, rollback refusal, >128 admission
capacity with 129 real delivered messages and next stamp index 1 (not 128 spends),
SQL rollback after first of two job writes, corrupt signed envelope metadata,
missing queue, revoked runtime and conservation of all unrelated persisted state.

Tests: crates/core/tests/support/public_sender_retirement.rs, imported by existing
public_postage_wallet fixture tree. Production untouched at critic submission.


R2/R3 test additions before implementation:
- First cleanup after cold restart happens with prepared work still active and
  now beyond checked.valid_until(), retaining all unrelated rows exactly.
- Real daemon process fixture exercises pump/status, SQL refusal/retry, two
  restarts, one ACKed and one undelivered original MLS message. Signed envelopes
  are genuine; queue/preparation state is restored for this boundary test.
  No live funding/finalization/storage claim is made by this fixture.
- Node obtains delivery through Core's existing delivery_status logic via an
  owner-only internal accessor; no new agent command is introduced.
