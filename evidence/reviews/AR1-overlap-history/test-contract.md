# Resume an overlapping validator's incomplete old epoch

A validator selected in both committees may have retained only part of the old
canonical history when it learns the closing QC. It must finish that history
before successor activation, without deleting or rewriting its existing entries,
nullifier rows, original SpendRecord proofs or verification times.

Add an explicit `HistoryImport::begin_extending_with_terminal` entry point. It
authenticates the existing prefix and the newer checkpoint before making the head
pending. It refuses equal/older tips and extension after an existing terminal
entry. Beginning the extension invalidates old index handles and prevents fresh
negative membership checks. Existing begin APIs can resume a pending extension.

Pages descend from the new QC to genesis. They may reuse an existing entry only
when its immutable entry bytes and operation-to-sequence row match exactly. A
different payload, operation or ancestry cannot overwrite the old prefix, even
with a genuine signature quorum. Rejecting a page cannot advance any durable or
cached cursor. Keep the existing page and Store transaction limits.

`SpentHistoryImport::begin` selects this path for a nonempty old prefix that has
not reached the authenticated closing tip. Complete old indexes keep their
existing local verification path and original tip proof. Application records,
index additions and progress commit atomically. Cold restart resumes the same
QC/cursor and never exposes SpentContinuity before all original records and
canonical entries are present. Valid alternative source QCs cannot replace a
validator's original record proof/time.

Tests extend the existing finalizer/history_index and postage/spent_history
suites, using real P-256 quorum signatures, existing paid receipt fixtures and
encrypted SQLite. A disk trigger fails the page crossing between new entries
and the retained prefix. Complete SQL row snapshots check rollback and immutable
bytes/revisions; cold reopens and actual record/index readers check activation.
Conflicting signed chains exercise both reuse of an existing operation with a
changed entry and replacement of its operation at the same sequence.

This is a required overlap prerequisite, not a native multi-epoch pass. Actual
daemon successor lifecycle, peer transfer, public client integration and funded
network execution remain required. Production changes require an independent
backend-test-critic ACCEPT; afterward run backend/frontend regression checks.
