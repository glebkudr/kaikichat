# AR1 authenticated finalized-prefix index: test contract

User objective remains full V1. This prerequisite does not close AR-R01 or its
multi-epoch issuer gate. Tests precede production changes at fb78e4e.

Implement a reusable P-256 finalized-history index in the existing encrypted
ProfileStore. Preserve canonical Entry/journal/QC bytes. The existing QC over a
finalized entry commits its hash-chain prefix. Derive indexed operation membership
only from a complete verified prefix rooted in that QC, never from partial pages.
Keep one state row per entry and per operation and a small atomic head record.
Restart revalidates the complete prefix in bounded working memory; live lookup
uses indexed rows and checks the handle's persisted head revision.

Transfer uses one authenticated checkpoint and newest-first pages of at most 7
canonical entries (14 entry/operation rows plus one head/cursor row stay inside
the existing 16-state transaction bound). Validate each page against the previous digest/height; keep
the target unavailable until the chain reaches the correct genesis. Persist the
cursor and rows together, with activation in the final transaction. Resume after
restart. Refuse replacing any active history with an older snapshot. Reject a
different committee/epoch for an existing log; this is not epoch handover.

The prefix's existing finalized digest is its authentication root. This design
does not invent a new Merkle root or claim compact non-membership proofs. Total
initial transfer/revalidation remains O(history); pages, writes and live working
sets are bounded. A local index alone does not authorize a spend or prove it is
current: runtime integration must reconcile it to the actual consensus archive
and validate a bounded unfinalized suffix. WAL/archive rollback protection and
epoch sealing/handover are separate required work.

Tests use the existing fixture's real Commonware P-256 quorum signatures over
opaque operations and real SQLCipher stores. They do not simulate native funding
or claim a live consensus run beyond height 128. New ancestry tests run for both
signature suites and distinguish proof path length <=128 from absolute height.
The engine, service adapter, spend policy and epoch guards remain in place until
their own real runtime/admission tests pass. No mocked QC or weakened quorum.

Files: crates/finalizer/tests/history_index.rs;
crates/finalizer/tests/support/ancestry_cases.rs.

API requested by tests: history::HistoryIndex::open/append/head/checkpoint/lookup/page
and history::HistoryImport::begin/push/remaining. Index methods take the existing
ProfileStore; persisted namespaces start finalizer/history/{log}/, with head,
entry/{20-digit sequence} and operation/{hex id}. Wrong/currently stale handles
fail closed; successful retry retains original checkpoint bytes.

Before production: independent backend-test-critic ACCEPT. Afterwards: focused
tests, full backend and frontend regression, formatting and Clippy. Do not change
the release manifest's 67 tasks/22 E2E or claim full AR1 completion.
