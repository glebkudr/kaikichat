R20 FINAL REVISE — /root/capacity_test_critic

The two warm-cache tests adequately preserve fresh persisted bytes/metadata/scope and atomic renewal behavior. Blocking: add a bounded churn test with more than 128 distinct genuine bindings in one AppCore lifetime. Observe the actual memoized-entry count through test-only access. Assert at most 128 proofs, superseded wires removed after decoding current stored entries, and successful verification through pruning. Existing persisted floor capacity tests cannot catch an unbounded historical in-memory map.
