# Incomplete old history at a committee overlap

An overlapping validator retaining a shorter old canonical prefix can now import
through the authenticated closing without deleting or rewriting its existing
history. The previous implementation treated every nonempty index as already
complete and rejected this case at import start.

The explicit extending import authenticates old and new checkpoints, retains
immutable rows and disables negative membership until all entries reach genesis.
Only exact existing entry bytes and operation mappings can be reused. New rows,
original spend records and progress commit together; failures leave the whole
page unchanged. Complete local history keeps its earlier path and original tip
QC. No transaction limit, page limit or persisted format changed.

Four new tests cover overlapping history, SQL rollback at the boundary, cold
restart, original QC/time preservation and conflicting genuine signed chains.
The focused suites pass 22 finalizer history tests and 8 postage history tests.
The independent critic accepted tests before production; see
[contract](test-contract.md), [RED evidence](red-checks.json) and
[review](test-review.md).

The full workspace passes **871 Rust tests** with zero failures or ignored tests
in 59 nonempty suites. **59 frontend tests**, TypeScript/Vite, 19 model tests and
workspace Clippy/fmt also pass. All 576 source fingerprints stayed unchanged
during the full workspace run. [Check results](checks.json),
[source manifest](validated-inputs.json) and [documentation checks](documentation-check.json)
record the exact inputs and limits. Raw run logs remain in managed local output.

This is a library prerequisite. Actual daemon successor startup, ordinary-peer
transfer, client network lineage and funded multi-epoch consensus remain open;
see the [next integration contract](../AR1-successor-spending/NEXT.md).
AR-R01 and full V1 remain open: 67 mandatory cards, 22 E2E and three platforms.
