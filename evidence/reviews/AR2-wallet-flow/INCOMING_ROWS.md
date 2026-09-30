# Incoming paid ciphertext in separate rows

The incoming data store now reuses the outgoing custody transaction engine for
operation bodies, conflict claims, occupied quota and bounded expiry. Outgoing
version-2 namespaces and serialized fields are preserved. Receipt validation
remains specific to incoming transport-bound commitments or outgoing evidence.
No network authority is derived from the local row digest or counters.

`with_incoming_limits` sets an explicit ciphertext admission quota independently
of the constructor's existing index quota and the sender's outgoing quota. Lower
limits do not evict live promises. New body, entry, nullifier, sequence, expiry
and head changes are atomic. The first v1 change retains one exact bounded
archive; same-time reads/retries do not migrate or renew a receipt. Old readers
reject the v2 head. Legacy entries remain in the bounded archive until their
last retained obligation expires.

`read_page` checks the existing signed, target-bound read capability. Its limit
bounds examined sequence slots and its byte limit bounds returned ciphertext.
One extra small claim detects a remaining suffix. Expired slots can produce an
empty advancing page. `complete` means exhaustion of this operator's current
local inventory, not a fixed snapshot or authenticated global completeness.
The v1 reader rejects an empty continuation it cannot represent. Addressed
corruption fails explicitly; an unrelated damaged body is not loaded by the
bounded page. Reads return only after the required clock/cleanup commit.

Expiry maintenance processes at most two queued objects per call. Bodies,
entries and queue payloads become small CAS-preserving tombstones; claims remain
to support ordered scans and expired claims may be replaced. This does not promise
constant database size forever. Quota includes expired work until reclaimed.
Inspection skips incoming background GC while composing it with outgoing GC,
the addressed incoming object and observation changes in the existing transaction
limit. It does not split that commit into independent successes.

Independent backend-test-critic accepted tests before implementation. The fixture
uses actual Core admission, public payment, finalized spends, signed envelopes
and selected operator receipts. It retains 134 originals at one custodian,
reopens and checks every exact receipt, record and authority snapshot, bounds
append writes, reads all pages and checks byte limits and capability separation.
Direct SQL observations verify both expiry batches before any later read can run
maintenance. It also checks rollback, released quota, monotone time and bounded
corruption isolation. Both old-A/new-C and old-C/new-A migrations cover seven SQL
failure points, exact archived bytes and legacy/new conflict claims.

The pre-proof compatibility test now explicitly reconstructs the v1 format;
removing proof bytes from a v2 body would test corruption instead. One historical
regression's old `one SQL row` assertion is replaced by exact equality of every
source row before and after verification. Its signatures, payment, all fourteen
positions, decryption and absence of new Core authority remain checked. The critic
accepted that correction separately.

[Final targeted checks](incoming-rows-checks.json) pass: 92 backend scenarios
(two incoming, fifty existing paid-custody, thirty-nine paid-index and one Runtime
regression), 31 frontend tests, postage-spend/node all-target Clippy, formatting
and whitespace. The first compile errors and the obsolete single-row assertion
failure remain recorded. Forty-eight focused source hashes were verified after
checks; this is not a full application input manifest or a native acceptance run.

## Required continuation

Incoming index and inspection observations still use their old bounded documents.
The daemon still needs the coordinated incoming operator quotas and scheduled
maintenance. Network workers must consume explicit continuations and signed
history pages/roots; the current local pages do not replace those proofs. The
ordinary native gate must exceed 128 simultaneous paid originals, restart/remove
the sender, lose real data/index nodes and recover the full declared range.
These component checks close no full V1 card and do not renew native acceptance
for the earlier GUI bundle. MLS Welcome/epoch recovery, autonomous repair,
full E11 and all 67 cards /22 E2E /three platforms remain required.
