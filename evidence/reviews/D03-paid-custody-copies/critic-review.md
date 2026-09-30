# Independent backend test review

Reviewer: /root/custody_copy_test_critic; fork_turns=none; read-only.

R1 REVISE: independently validate the original primary beyond a consent hash; negative controls and shared quotas for the new outgoing entry point; two owned operators with independent policy/revisions.
R2 REVISE: the second quota object shared the first ticket's nullifier. Use independent genuine case[2], assert different nullifiers/operations and add sufficient-capacity positive control.
R3 FINAL ACCEPT before production edits.
R4 FINAL ACCEPT before production edits: Python carrier key corrected to the existing authority_snapshot serde spelling; otherwise R3 unchanged.
R5 FINAL ACCEPT during implementation, before further production edits: the restart fixture must drop its old Core before opening the same profile. Five actual failures were ProfileInUse in the fixture, not product permission failures. Scratch Core releases the old file lock before real reopen; assertions unchanged.

This review accepts test quality only. Runtime results are recorded separately.
