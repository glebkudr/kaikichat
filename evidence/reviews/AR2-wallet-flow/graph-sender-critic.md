# Ordinary graph sender test review

The independent context-free backend-test critic reviewed the new native tests
before production changes and returned **FINAL ACCEPT**. Production remained
SHA256 `1f7f76f357e19fdcc9807871f87f0eec695f8ef59e8ada98f16d9164cb35221c`
during the behavioral RED run.

The first build failed on a test-only SQL serial type (`u64` is not `FromSql`);
it was corrected to `i64`. Native R2 reached real paid CLI publication, last-child
ACK commit failure/cold retry and sender lifecycle faults, then failed the new
root-kind assertion against the old flat publisher. This is RED, not acceptance
of the implementation.

The critic found one blocking oracle omission: an empty sequence-claim set could
escape validation despite existing v2 operation rows. The corrected test builds
an expected mapping from v2 operation rows and unconditionally compares it with
actual sequence claims; duplicates fail and legacy-only state remains valid.

Accepted files and SHA256:

| File | SHA256 |
| --- | --- |
| `tests/evm/history_graph_oracle.py` | `755082c2793b9ba191e7b9c5aba78dcb920c2af06478a5ac0e87e9649bb705ff` |
| `tests/evm/public_history_graph.py` | `8a51445ad06e0364d159e36f49d401a486829aaa5092e09737f711080a96e573` |
| `crates/node/examples/support/history_graph.rs` | `ee176dd185b360f7a66ab54721f891ac353e1d8ff2b410cee40dfb19e969c50e` |
| `tests/evm/public_sender_lifecycle.py` | `93df8fe0d3a9c1ce277eff0292b5ad033aae81158d2a6e594481c0047dce014f` |
| `tests/evm/public_index_recipient.py` | `387fbf39e5f8175fc7928290d04cce0aa6aa661f4a3ab44cfab8006d9809c4fe` |
| `crates/node/examples/postage_spend_store_failure.rs` | `8d9f52ed865866fee1a73f135c9fdb65edf2173061656e6be1b1e570c3fdf360` |

The acceptance concerns two ordinary genuine MLS paid originals, actual ACK
ordering, child commit failure, cold state, paid loss and recovery. It does not
cover >128 simultaneously live paid originals or close the complete V1 goal.
The optional suggestion to retain the full audit in a failure `finally` block
was not required for acceptance. Raw native traces remain local and may contain
credentials; publish only sanitized evidence.
