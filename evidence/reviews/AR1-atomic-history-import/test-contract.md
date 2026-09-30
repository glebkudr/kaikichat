# Atomic application state during historical import

Prerequisite for authenticated postage spent-state handover, under the unchanged
67-card / 22-E2E / three-platform V1 goal. This change does not enable epoch > 1
spending or claim that generic application bytes constitute verified SpendRecords.

HistoryImport must expose `push_with_states(store, page, states)` and preserve
`push` as its empty-application-state specialization. The caller verifies domain
records before providing StateChanges; the finalizer validates canonical history.
One existing Store transaction commits the whole page, application state and head.
Neither the cached cursor nor the durable prefix advances on any failure.

The Store's existing 16-state limit stays unchanged. Four entries plus four
application records, one application cursor and the history head fit it. Seven
entries plus one application state exactly fill it. Oversized combined batches
must reject without writes. Application states cannot write the reserved
`finalizer/history/` namespace, including another log's history.

Tests use real P-256 finality proofs, actual encrypted SQLite and independent
owners. They cover multi-page cold resumption; simultaneous last-page activation
and an application-ready record; SQL faults in both application and history
writes; CAS conflict; malformed canonical pages; reserved namespaces; and the
existing batch limit. Exact persisted bytes, row revisions, membership and cursor
are checked after failures and restart. Markers deliberately stand for opaque
application state, not postage proofs; actual authenticated spent-state import
remains the next layer.

Production changes may begin only after an independent backend-test-critic accepts
the new tests. Run backend/frontend checks and preserve their actual evidence.
