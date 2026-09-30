# Independent backend test review

Reviewer: `/root/index_holder_test_critic`, separately spawned without inherited
context and reused for this bounded review. It read the new tests, source/helper
context and contract without editing files or running checks. Production stayed
unchanged until ACCEPT.

First verdict: REVISE. The cold-corruption test incorrectly required successful
store construction despite fail-closed constructor validation. Also missing was
a real data-only outgoing state proving that no retained index promise means no
manifest stage/ACK. Both were corrected. Optional improvements tightened quota
to exact serialization accounting and added a fresh original after real MLS epoch
transition. No assertion was removed to accommodate production behavior.

Final verdict: ACCEPT. No mandatory missing scenario within this prerequisite
boundary. Accepted SHA256:

- `crates/core/tests/support/custody_history_candidates.rs`:
  `11714050e6a051ff03ecc8dc3d9d874e9566315e4a3bcb1aecd1716804651881`
- `crates/postage-spend/tests/support/outgoing_index_history.rs`:
  `78772eccabafa23879caf5ed5962c6b2d2b13f3123176449005e894b66ec043f`

Module declarations and the reused history fixture helper's `pub(super)` visibility
change were included. This is static acceptance of tests, not runtime acceptance.
The baseline and candidate reports separately record runtime outcomes.
