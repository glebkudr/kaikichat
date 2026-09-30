# Agentic Internet R14: read-only review measurements

These files were produced by reading the uploaded review bundle. No project commands,
native scenarios, builds, dependencies, keys, or network services were executed.
Project sources were not modified.

## Reproduce the structural measurements

Python 3, standard library only:

```sh
python analyze_r14.py --archive /path/to/review-bundle.zip --out measurements.json
```

Or use an extracted bundle:

```sh
python analyze_r14.py --root /path/to/extracted-bundle --out measurements.json
```

This is NOT the short native acceptance scenario and NOT a cryptographic verification.
The script decodes the archived CBOR graph to count current reachable pages and
cross-check its reference ordering against the archived R14 evidence. It also
calculates compact JSON size using the current page-cache schema and intersects
archived cache/import sequence sets.

## Results

- 117 leaves for 130 originals: 106 singleton leaves, 9 two-message leaves, 2 three-message leaves.
- 117 prepared root versions and 112 branch pages. Prepared does not imply all
  superseded roots were published or acknowledged.
- Current root reaches 117 leaves, 112 branches and one root: 230 pages total.
- Estimated full current-root non-root page-cache representation is 857,593 bytes,
  below the source limit of 1,048,576 bytes. This is a representation-size
  calculation, not a captured runtime cache high-water mark.
- Original 31 is graph ordinal 30 (one based).
- Eight operations overlap between deferred and prefetch.
- Prefetch includes already imported originals 1 through 24.
- The two caches contain 83 unique originals; together with the 30 imports they
  cover 89 unique originals. This is a post-stop snapshot, not an exact-deadline snapshot.
- Sampled publication status RPC waiting time sums to 343.358 seconds. This is
  caller elapsed waiting time, NOT measured CPU consumption or proven removable delay.

## Principal source locations

All paths are relative to `project/` inside the uploaded bundle.

- `crates/node/src/public_sender.rs:38-50,247-327,361-470,567-640`: per-message Work,
  full advancement, observation, history and pointer publication.
- `crates/node/src/public_sender_history.rs:20-92`: already-paid ready selection,
  maximum 12 and the 20 ms preparation pass constraint.
- `crates/core/src/custody_history_batch.rs:13-33,82-117,166-212`: existing batch
  commitment, original sequence ordering within leaves and graph append.
- `crates/node/src/custody_history_sender.rs:77-104,123-145,184-228`: four advertised
  index routes, root changes, canonical saved page acknowledgements.
- `crates/node/src/custody_sync.rs:20-45,193-349,449-556`: Work lifecycle,
  admission and deferred retries, completed output discarded at elapsed Work deadline.
- `crates/core/src/custody_history_page_scan.rs:70-158`: durable attempt cursor,
  last and single retry reservation.
- `crates/node/src/custody_history_graph.rs:288-404`: bounded ordinal traversal,
  graph page reuse and operation selection.
- `crates/node/src/custody_fetch.rs:18-34,76-126`: prefix from zero, per-Work ledger,
  broad compatibility fallback on capacity/unavailable/rejected/transport failure.
- `crates/node/src/custody_obligation_read.rs:5-68,122-168`: whole paid reply validation;
  range cursor and complete bit used for validation but not retained in returned FetchedPage.
- `crates/node/src/custody_index_sync.rs:455-505`: all validated range bodies enter
  prefetch, but only selected requested envelope drives the current import.
- `crates/node/src/bootstrap_schedule.rs:160-189`: fixed-window minute counters.
- `crates/node/src/paid_custody.rs:328,736-748`: provider Admission<32,16> and combined
  unauthorized/rate-denied Rejected response.
- `crates/node/src/processing_budget.rs:14-23,96-140`: per-swarm shared slots/rate/byte budgets.
- `crates/core/src/custody_history_deferred.rs:214-292`: unchanged MLS-state retry,
  queue rotation and durable writes; proofs duplicated into per-operation entries.
- `crates/core/src/custody_prefetch.rs:76-147,153-183`: page admission and exact
  current-root local fetch; no already-imported/deferred filter during page insertion.
- `crates/core/src/custody_history_page_cache.rs:6-7,58-128`: bounded root-bound proof cache.
- `crates/core/src/custody_history_import.rs:181-236` and
  `crates/core/src/custody_history_page_import.rs:184-240`: common completion and
  state changes joined to the message/MLS import transaction.
- `crates/crypto/src/lib.rs:302-319`: contiguous receive, followed by a second
  candidate decrypt to authenticate a ReceiveGap, without committing that candidate.
- `tests/evm/public_index_recipient.py:181-250`: real loss fixture; surviving index
  chosen among advertised candidates, not any of ten providers.
- `tests/evm/public_history_read_trace.py:45-55` and
  `crates/node/examples/custody_read_trace.rs:13-19`: new diagnostic wrapper expects
  queued/finished trace event names. Those production emit sites were not found in
  this snapshot. File presence does not establish diagnostic acceptance.

## Evidence boundaries

`review/CURRENT_VS_R14.json` lists four changed inventoried paths, only one of
which is production code (`public_sender.rs`). Historic source or binaries were
not reconstructed. New sender observation tests do not demonstrate native
throughput improvement. R13 remains an independent unresolved publication failure.
