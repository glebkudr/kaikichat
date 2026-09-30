# Prepared ciphertext inventory — bounded rows and pages

The local prepared-envelope layer of the [retained lifecycle](../../../spec/custody-retained-lifecycle-v2.md)
is implemented. New immutable envelopes live in separate sequence rows. A small
checkpoint and direct message lookup join each append in the existing SQL
transaction. Adding an original does not serialize or overwrite earlier
ciphertext. The paid reservation and immutable message association share that
same transaction; no new stamp is released on failure.

A page examines at most 32 retained slots and bounds the entire encoded response,
including its continuation. The cursor binds conversation, exporter index, MLS
epoch and a fixed sequence upper bound. Later appends appear only in a fresh scan.
Expired originals can yield empty pages with a continuation. Valid holes from
legacy expiry pruning are skipped using the bounded legacy inventory.

An existing v1 profile can be read and retried without writes. Its first new
append archives the exact old document once and installs a v2 checkpoint with
the original sequence counter, atomically with the new rows. Old readers reject
the changed checkpoint version. A subsequent append changes neither that archive
nor previous immutable rows. Epoch advancement retains older sequence rows;
this does not provide cross-epoch mailbox/control-log recovery.

Independent backend-test-critic ACCEPT preceded implementation. Its requested
revisions added byte-triggered page splitting, sparse legacy sequence/counter
migration, explicit version/archive preservation and an additional append.
Positive reads at the same clock isolate the missing-body rejection.
Accepted test hashes are in [checks](envelope-pages-checks.json).

Both compiler REDs are retained: the final accepted tests fail at 18 callsites
because the new page API does not exist. The implementation then passes:

- 47 affected custody/conversation tests, including all four new cases and real
  MLS epoch changes. The long case creates 258 live originals, checks bounded
  writes, cold exact retry, stable pagination and actual recipient ingestion
  with the sender closed.
- Eight paid preparation tests, including independent sequence/lookup SQL
  failures, rollback, cold retry and real recipient stamp/message verification.
- 31 chat/wallet frontend tests, Core/node all-target Clippy, formatting and
  whitespace checks. The earlier 10-test envelope run is a subset of the 47.

[Input manifest](envelope-pages-inputs.json) records the source after these
checks; it is not a before/after native-run attestation. Raw test logs remain
under output/ar2-wallet-flow with digests in the checks artifact.

The ordinary network worker still consumes the bounded flat-manifest compatibility
API, which refuses an oversized inventory rather than silently returning a
prefix. Outgoing/incoming paid evidence, operator admission, signed directory
pages and ordinary network traversal are not yet converted. This artifact does
not establish more than 128 simultaneously live paid sends, remote history
completeness, autonomous repair or whole V1. The prior native paid GUI evidence
retains its recorded earlier source and is not reused as acceptance of this change.
