# Independent backend test review

Baseline: 6c44814. Reviewer: `/root/common_context_test_critic`, created without
inherited context and reused with a standalone bounded request. No production code
was changed before FINAL ACCEPT.

Initial verdict REVISE required two additions: a valid cached-context request whose
clock write fails, and structurally valid stored evidence containing a certificate
or proof from the wrong chain/root. The revised package covers both, preserves all
original checks and adds exact valid_until assertions before and after renewal.
The critic returned FINAL ACCEPT with no remaining blockers. All seven reviewed
inputs were frozen before implementation.

The revised RED run exits 101 with only the three missing interfaces:
RetainedPostageContextInput, retain_postage_context and retained_postage_context.
This is a compiler-level missing-interface baseline, not execution of the later
eight scenarios. Their first execution after implementation passed all eight.

The public capacity fixture has 33 actual contract-created epochs, genuine signed
successors and eth_getProof evidence. An independent existing CLI checked 99
registry cases during generation. All 33 epoch policies remain live at the final
head even though the first head's lease expired; exactly the first policy expires
at the reuse checkpoint. All heads and reuse proofs come from actual EVM state.

Earlier fixture authoring failures exposed the 1800-second helper lease limit,
state-root changes across empty blocks, and a wrong network domain for Core tests.
These were corrected before the final clean RED and critic review. No production
fallback, synthetic state-root proof or weakened capacity assertion was introduced.
