# Test critic decisions and evidence

The existing dedicated `/root/public_postage_test_critic` agent was originally
created without inherited context and reused for this bounded test review.
No production implementation began until R3 FINAL ACCEPT.

- R1: REVISE. Required real queue saturation, preservation of runtime quota
  across policy removal/readdition/book aliases, capability/owner API denials,
  and missing indexed job/corrupted policy recovery controls.
- R2: REVISE. All four additions accepted; the owner configuration denial still
  used a stale revision and the policy denial used an unauthorized conversation.
- R3: FINAL ACCEPT. Actual revision, allowed conversation and exact Unauthorized
  errors removed those alternative causes of refusal. Three manifest hashes
  were verified; reversing the three changes reconstructed R2 exactly.
- After production compiled, R3 runtime revealed that the permission fixture's
  full byte-array authority exceeded the signed document body limit before
  reaching the broker. Nine tests passed and this one setup failed. R4 replaced
  only that request representation with real compact checkpoint/common IDs,
  epoch, operation and registry proof JSON; original history is retained before
  the state snapshot. R4 FINAL ACCEPT confirmed the roughly 12 KB body fits the
  49,152-byte limit, hashes match, and reversing the change reconstructs R3.
  All denial, unchanged-state and same-nonce positive assertions remain intact.

RED before production: R1 had 23 E0432/E0599 missing-API/type diagnostics; R2 also
failed for missing Core APIs/types. No production test stub was used.

Preliminary failed executions remain local ignored logs: the first production
compile found the existing job-market AuthorizationChanges initializer needed
the new optional field; the next run found that state_namespaces supports at
most 64 names. The implementation now follows its own bounded 128-job index and
uses the existing scan only to refuse a missing index when job rows remain. The
store's existing global scan limit was preserved. The final R3 run was 9 PASS /
1 setup failure as described above; it is not accepted as GREEN R4.

Only the final frozen R4 regression results in module-checks.json establish
accepted execution. No prior failure log is used as a successful run.
