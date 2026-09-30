# Incoming paid index in separate rows

Compact paid indexes now share the custody row transaction engine with incoming
ciphertext and outgoing evidence. Each operation retains its original portable
anchor, finite holder receipts and signed history in one body. Nullifier and
index/sequence claims, expiry queues and the small head support addressed reads,
atomic occupied quota and bounded cleanup. The adapter keeps the existing
transport/descriptor/receipt validation; exports still authenticate payment,
finality, membership and current trust through Core. Local digests are not
network authority.

`with_index_limits` changes only index admission. Data and outgoing quotas remain
independent, and a lower cap cannot evict a live promise. Index bytes include the
serialized portable bundle, each retained holder receipt and signed history.
Adding a location or replacing history commits the body, metadata and aggregate
counters together. No update rewrites a different operation's body.

The strict legacy decoder accepts the old index `entries` document without
relaxing the data/outgoing `objects` schema. Same-time reads and exact retries do
not migrate or renew receipts. The first change atomically retains the exact
bounded archive and installs the v2 head and new rows. Legacy locations and
history remain available and counted after migration and cold reopen. Old
readers reject the new version.

`read_index_page` uses the existing signed target-bound read capability. One
shared data/index traversal bounds examined sequence slots and portable output
bytes; one additional small claim determines whether a suffix exists. An expired
slot can advance an empty page. `complete` means exhaustion of this operator's
current local inventory, not a fixed snapshot or signed global completeness.
The compatibility API refuses an empty continuation it cannot represent. The
read clock and any cleanup commit before bytes are returned. Damaged addressed
bodies fail; a bounded unrelated range does not load them.

`prune_index` reclaims at most two queued new rows per call. Body, entry and queue
payloads become small revision-preserving tombstones; conflict claims remain
and expired claims can be reused. Occupied quota includes not-yet-reclaimed work.
The bounded legacy archive follows the shared compatibility lifecycle. These
rules do not promise constant physical database size over an unlimited lifetime.

Independent backend-test-critic accepted the tests before implementation. Three
new scenarios use real public-funded spends, signed descriptors and selected
custodians. They cover 134 originals with cold exact evidence, bounded writes and
pages, quotas, both direct SQL-observed expiry batches, rollback and corruption
isolation. Both legacy/new orders exercise seven SQL failure points. A genuine
legacy location and signed history must survive failure, migration, cold reopen
and exact retry with full byte accounting. Mutable locations/history also face
exact quota boundaries and separate body/entry/head write failures.

The semantic corruption tests maintain consistent local digests and byte counts,
so a missing signature check cannot hide behind an unrelated hash rejection.
The shared paid fixture now constructs index presentations through the same real
operator binding helper as data. It introduces no extra prover or synthetic
admission bypass.

[Final targeted checks](index-rows-checks.json) pass: 95 distinct backend scenarios
(42 paid-index, 52 paid-custody and one Runtime), 31 frontend tests, postage-spend/
node all-target Clippy, formatting and whitespace. Fifty-five focused source
hashes were verified after checks; this is not a full application input manifest
or a native acceptance run. Earlier runs are not counted again.

The first RED failed on the three missing APIs.
A later compile needed an explicit `sum::<u64>()` in the test. The initial
migration test incorrectly required two large portable proofs to fit one 256 KiB
page; it now requires a bounded continuation, exact results, byte limits and
explicit completion. Both corrections were separately accepted by the critic.

## Required continuation

Inspection observations still use a bounded document. The daemon needs incoming
operator quotas and scheduled data/index cleanup. The network workers must adopt
explicit continuations and authenticated history pages/roots. Ordinary native
acceptance must exceed 128 simultaneous paid originals, restart/remove the
sender, lose actual data/index nodes and recover the complete declared range.
The earlier GUI native evidence remains bound to its recorded source; these
component changes do not renew it or close a full V1 card. MLS epochs/Welcome,
autonomous repair, full E11 and all 67 cards /22 E2E /three platforms remain.
