# Bounded state-name traversal for retained custody

The Store now exposes `state_namespaces_between(after, through, limit)` for the
binary-ordered interval `(after, through]`, with a page limit of 1–64 names.
The query uses the existing namespace primary-key range and selects only names,
without loading payloads or materializing all matching rows. It changes no schema
or stored state. The previous prefix discovery API retains its all-or-error
contract.

This is a live range, not an immutable snapshot. The caller owns scope, upper
bound and continuation; a new key behind the cursor will not be revisited.
Expiry integration must enforce its admission/clock ordering before using this
primitive. The API alone does not implement paid retention or expiry cleanup.

Independent context-free backend-test-critic accepted the three new tests before
production changes. RED contains only 11 missing-method compilation errors.
The accepted tests cover 257 SQLCipher rows, equal deadlines, pages of 19,
continuation after reopening, exclusion of later scheduled work, literal bounds,
missing cursor rows, invalid requests and exact preservation of persisted bytes.
The critic's optional live insertion-inside-range test was not added; the explicit
API contract makes no snapshot claim. Index use follows from the implementation's
primary-key range query; the functional tests do not measure query cost.

Targeted results: **10 backend tests**, **26 chat frontend tests**, Store all-target
Clippy, formatting and whitespace checks pass. [Checks and focused source hashes](state-range-checks.json)
record these results. Raw logs remain in `output/ar2-wallet-flow/`.

This is a prerequisite for the [retained lifecycle](../../../spec/custody-retained-lifecycle-v2.md).
Outgoing/incoming paid evidence still uses the earlier whole-document stores.
No new native acceptance or whole V1 card is claimed.
