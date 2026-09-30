# V1-H10 semantic read observations: tests-only contract

Base: `252e0f5`. Task: V1-H10, related to public-history Diagnostic32. These
synthetic Python events test the diagnostic oracle only. They do not prove a
native run or a Rust producer's extraction of fields from authenticated requests.

Owned source change: `tests/evm/test_history_read_trace_oracle.py`.
Production oracle, trace producer and native harness are unchanged at this stage.
The original five parser tests remain intact.

## Semantic identity

Each modern `custody_read_queued` span adds `semantic_key` and `semantic_query`.
The latter is a JSON object serialized as a string by Rust tracing; the parser
decodes it and recomputes the key, rather than accepting a claimed digest.

`semantic_key` is lowercase SHA-256 hex of:

```
b'ain-custody-read-semantic-v1\0' + canonical_compact_json(semantic_query)
```

Canonical JSON sorts object keys, uses no whitespace and has ASCII field values.
Numbers are integers; query fields are nonsecret request metadata. The object is:

```
{
  "wire_kind": "obligation_read",
  "root": "<64 lowercase hex; checked outer history root>",
  "index": "<64 lowercase hex>",
  "epoch": 0,
  "target": "<32-byte transport key as lowercase hex>",
  "after_sequence": 30,
  "limit": 32,
  "max_bytes": 262144,
  "operation": null,
  "commitment": null,
  "after_position": null
}
```

`wire_kind` is the actual Request variant, retaining the distinction between
`index_history_page_read` and `index_history_path_read`; `read_kind` alone loses
it. The optional fields describe actual Request extras: IndexHistoryRead has its
anchor operation, IndexLocations has operation/after_position, page/path reads
have the complete serialized HistoryCommitment. All commitment fields belong
to identity, including its issuance and expiry; they are not renewable read-token
timestamps. Absent extras are explicit nulls.

Wire read nonce, read token issue/expiry, signature, request ID, Work ID, PID,
boot identity, and the selected original are excluded. Changing those while
holding the query identical is a repeat. Conversely changes to query root/index,
epoch, target, byte bound, cursor, count limit, wire mode, operation, location
cursor, or commitment are new reads. Outer root/index/epoch must agree with the
independently checked graph bindings. Results cannot change the queued identity.

The synthetic controls cannot detect a Rust producer that consistently lies
about both query and digest. Producer acceptance still requires a regression
over actual prepared Request bytes with new nonce/time and changed query fields,
or explicit extraction review backed by native observations. No trust, TTL,
deadline, request allowance, or admission policy changes are authorized here.

## Counting and capture scope

`check_trace(..., capture=None)` returns existing observations plus:

- `readMetrics.semanticCoverage`: complete / partial / unavailable.
- `readMetrics.newReads`, `repeatedReads`: exact queued-read counts when every
  queued request has an audited identity; otherwise null, not invented zeros.
- `readMetrics.queued`: requests actually sent to the existing network queue.
- `readMetrics.admitted`: explicit `custody_read_admitted` observations; null
  for a legacy trace without admission observations.
- `readMetrics.notQueued`: explicit post-admission `custody_read_not_queued`.
- `captureScope.processes`: ordered observed `{processId, boot}` pairs.
- `captureScope.complete`: null without an expected capture inventory, true
  only when it matches; missing, extra or duplicate expected boots fail.
- `wallTimeSeconds`: externally measured elapsed monotonic time, or null when
  absent. Process uptime is never summed or substituted for scenario wall time.

An admitted request is not necessarily queued. A post-admission SQL/capacity
error, or a crash before queueing, still consumes admission. Wait/rate-limit
events do not. Explicit admissions correlate by `(process_id, boot, request_id)`
and Work/peer; an attempt cannot be both queued and not queued. Request IDs and
uptime may restart in a new boot, while semantic repeat identity spans the full
capture. Terminal/staging events stay bound to the request's originating boot.

`capture` contains `expectedProcesses` and `wallTimeSeconds`. The native harness
must derive the expected inventory from all recipient starts/stops and preserve
logs across boots. Observing a complete retrieval path in the latest overwritten
log cannot establish full capture. The captured elapsed duration must come from
the parent harness monotonic clock across all relevant phases, not node uptime.

Historical `output/hrt32-r1` logs remain valid input with unavailable new metrics.
The baseline cannot acquire semantic identities or admitted counts retrospectively.
Any comparison must retain its failed-first-SQL-gate phase boundary and distinguish
observed measurements from missing data. Leaf occupancy is separately proven by
the graph oracle; this tests-only change neither reconstructs nor invents it.

## Exact tests and RED command

The 14 new `TraceMetricTests.test_*` methods cover legacy unknowns, equal payload
with new nonce/time/Work/selection, every meaningful query dimension, canonical
JSON and tampered digests, partial identity coverage, separate admission counts,
duplicate and cross-scope attribution, boot-scoped request reuse, missing capture
boots, explicit finite wall time, and changed completion identity.

```
python3 scripts/build-storage.py --profile portable-linux --output output/h10-trace-red-002 run python3 -m unittest discover -s tests/evm -p test_history_read_trace_oracle.py -v
```

Expected RED causes: missing `readMetrics` / `captureScope` / `wallTimeSeconds`,
unsupported `capture` argument, and failure to reject tampered semantic identity
or duplicate/misattributed admission. Existing five tests should remain green.
No Cargo, Forge, preparation or native process run is part of this tests-only step.

Revision: 14 new methods; adds failed, rejected and unfinished queued-attempt controls. Original contract and RED evidence remain unchanged.
