# Independent backend test critic

Reviewer: separately spawned `/root/index_holder_test_critic`, without inherited
context. Read-only static review; no reviewer test execution or production edits.

First verdict **REVISE** before production:

- Changing anchor and revision together did not isolate ordinary revision updates
  under the same longest-lived anchor.
- A nonexistent stored message ID did not prove direction/conversation checks for
  genuine existing records.

The accepted revision keeps the original anchor and explicitly compares signed
anchor bytes, while retaining unresolved-directory, stale-token, preserved-import
and fresh concurrent-import assertions. Cold mutations independently start from
valid stored bytes and point to a missing message, an actual own message in the
same conversation, an actual incoming message in another conversation, or an
altered descriptor commitment. Each requires unchanged SQL and snapshot.

Final verdict **ACCEPT**, received before any production changes. Direct reuse of
`HistoryReference::verify_descriptor`, `verify_indexed_envelope` and the existing
atomic receive helper was accepted; duplicating crypto hostile vectors was not
required. Remaining optional suggestion: assert original JSON field values before
mutation to protect future schema changes. The implemented serde fields currently
match those mutated fields; the gate verifies the resulting failures.

Accepted SHA256:

- `crates/core/tests/support/custody_history_import.rs`:
  `94a98d492dc98aef04e25c1ec8d7fadccd149b706c42b3986be258fc5c6f461f`
- `crates/core/tests/support/custody_history_pointer.rs`:
  `153b3db5fab50b315a57fa7c45ada1ceae37aacf6043f08ceb6c6e9c36a5b280`
- `evidence/reviews/AR3-history-import/TEST_CONTRACT.md`:
  `dade5f8a3962b9a6905a9ac7bf9bc707db6ff552f0db3186c5f19085e75851a3`

The accepted-input baseline-2 against `16e264a` fails compilation with one E0432
and 61 E0599 errors for missing public types/APIs. No runtime assertions ran in
that RED. Baseline-1 is preserved as the earlier reviewed test revision.
