# Ordinary client chosen-epoch lineage

An ordinary client can prepare a genuine public postage receipt in a successor
without becoming an operator or downloading validators' complete spent records.
It must first authenticate every chosen transition from epoch 1 and still hold
current Core public authority for the exact issuer/code/policy/committee.

`EpochLineage::import(core, store, closing, now)` retains one authenticated public
choice after establishing the source lineage; a valid later closing alone cannot
skip missing predecessors. Genuine registry-epoch skips chosen directly by a QC
are allowed. Conflicting choices from one predecessor fail without replacing the
first original ClosingRecord. Retry preserves its QC/time and retained rows.

`EpochLineage::open(core, store, target_snapshot, now)` cold-authenticates all
chosen links with bounded memory. `prepare_client_with_lineage(authority, core,
store, lineage, context, receipt, now)` requires an owner-bound, unchanged lineage
and the same current receipt/fence checks as the first-epoch client path. Wrong
store, target, Core owner, revoked roster or expired authority must fail. Legacy
prepare_client and SpendSession::new retain their non-first-epoch guards.

Public lineage is not SpentContinuity: it has no original SpendRecords or index
and can never authorize successor voting. Reuse existing verified closing and
context APIs. A validator store written by the preceding implementation already
has authenticated chosen-link metadata in postage/handover. Both partial and
complete imports must support public lineage cold-open without fabricating a new
spent-state capability or requiring an incompatible migration. Warm lineage
checks bind the revision of the retained link used to create the capability.

The public choice marker is committed after the original closing. An interrupted
marker write may leave public evidence but cannot expose an authorized lineage;
retry after the SQL failure must resume and retain the first proof. Cold open
must reject corrupted QC even if a public completion marker still exists.

Tests use existing genuine three-epoch fixture QCs and receipts, encrypted SQLite,
actual Core public profiles with no operator key, cold restart and SQL faults.
The epoch-2 closing fixture is historical evidence, not a claim of live consensus;
actual epoch-2/3 application-policy execution is separately tested. Ordinary-peer
bootstrap and funded multi-node consensus remain mandatory integration work.

Obtain independent backend-test-critic ACCEPT before lineage production changes.
Run backend and frontend regression gates after implementation.
