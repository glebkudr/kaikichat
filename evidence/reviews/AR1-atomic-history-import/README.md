# Atomic history pages with application state

`HistoryImport::push_with_states` commits caller-verified application records,
canonical history rows and the head in one existing Store transaction. The old
`push` API delegates with no application writes. The combined sixteen-state limit
and history namespace protection remain enforced.

Three independently reviewed new tests pass; all nineteen history-index tests
pass. They use real P-256 finality proofs and encrypted SQLite to check SQL/CAS
rollback, cold resume, exact records/revisions and the existing batch limit.
Opaque test markers do not claim postage authority. The authenticated caller is
implemented and checked in `../AR1-spent-history-import/`; its checks.json records
the shared full regression results. This prerequisite does not close AR1 or V1.
