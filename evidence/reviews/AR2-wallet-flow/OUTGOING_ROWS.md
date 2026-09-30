# Paid outgoing originals stored separately

Outgoing custody evidence now uses one body and one metadata entry per operation,
separate nullifier and index/sequence claims, an ordered expiry queue and a small
version-2 head. Adding an original does not rewrite older bodies. Index/history
publication and ACK changes update the addressed original and accounting in the
same SQLCipher transaction. Existing public-payment, receipt, peer and Core trust
verification remains required; a local body checksum is not authority.

`with_outgoing_limits` configures sender evidence independently of incoming
operator admission. Occupied quota includes expired rows until cleanup commits.
`prune_outgoing` visits at most two queued originals per call and reports pending
work. An ordinary addressed read may additionally reap that expired original.
Bodies and metadata become small CAS-preserving tombstones; expired claims are
reusable. This releases payload/evidence capacity without claiming a constant
database size over an unlimited lifetime. Damaged addressed evidence fails closed;
an unrelated live body is not loaded. Bounded expiry work can still encounter and
reject a damaged expired body.

Version-1 data is preserved in one exact bounded archive on the first change.
Same-time legacy reads and exact retries do not migrate. At most 128 legacy entries
remain in the head and are checked against the archive until promoted or expired.
Legacy expiry can reclaim that bounded compatibility set in one call; the two-item
queue bound applies to new rows. Failed migration changes no old row or quota.

## Checks and independent review

The new tests use the existing paid fixture and a SHA-bound extension of its
160-ticket funded book. They authenticate the public postage in real Core and
finalize indexed spends with the existing independent P-256 QC test oracle.
This is component evidence with real cryptographic carriers, not a new live-chain
or native network run.

- Retain 134 distinct paid originals, reject a 135th without eviction, reopen and
  export exact receipts, envelopes, SpendRecords and authority snapshots. Exact
  retries preserve body bytes and revisions. Additional writes are bounded by one
  body plus 4096 bytes of metadata in this fixture.
- Advance time past three original deadlines. Inject a real SQL UPDATE failure
  and prove exact rollback. Direct SQL checks show exactly two bodies reclaimed,
  reopen without hidden extra work, then reclaim the remaining one. All 131 live
  originals remain unchanged; precisely three quota slots can be reused.
- Check exact global payload-byte limits and both nullifier and sequence conflicts
  across different originals and cold restart. Clock rollback is rejected; a
  missing addressed body cannot become an empty successful read.
- Independently reconstruct legacy data, try both legacy/new object orientations,
  and inject seven distinct archive/body/entry/claim/queue/head SQL faults. Cold
  retries preserve the exact archive, original evidence and both conflict rules.
- Adapt prior index/history/inspection corruption checks to the body rows. A
  shared test helper maintains local hashes and byte counters so semantic proof
  rejection cannot be hidden by a checksum failure. Failed reads preserve damaged
  SQL; exact restoration restores valid reads.

The separate context-free backend-test-critic accepted the tests before production
implementation. The first revision required direct SQL observations between GC
batches. A later runtime failure exposed a test-oracle error: an object refused by
the byte quota did not occupy its sequence. The correction first proves its cold
absence, explicitly raises quota, then admits it before asserting that conflict.
The critic accepted this correction and the format/lint adaptations. Production
quota behavior was not relaxed to satisfy the mistaken oracle.

Final targeted validation passes: **89 backend tests**, **31 frontend tests**,
postage-spend/node all-target Clippy, formatting and whitespace checks.
[Commands, hashes and review decisions](outgoing-rows-checks.json) record the scope.
Raw execution logs are retained in `output/ar2-wallet-flow/`; preserved RED and
intermediate failures distinguish missing APIs, fixture adaptation and lint errors
from the final result.

## Required continuation

The component was validated before daemon integration. The subsequent
[Runtime checks](RUNTIME_RETENTION.md) now cover explicit sender limits and periodic
bounded maintenance through the ordinary pump. Incoming data, index and inspection
stores still require their row lifecycle and operator admission work.
The ordinary network directory remains limited to 128 references and must move to
authenticated pages with visible gaps and incremental publication/retrieval.

Then run the declared over-128 live paid native flow: ordinary send, sender restart,
sender/data/index removal, and full recipient recovery. This component does not
complete that flow, the full retained lifecycle or any whole V1 capability card.
Earlier native evidence remains tied to its own recorded sources and binaries.
