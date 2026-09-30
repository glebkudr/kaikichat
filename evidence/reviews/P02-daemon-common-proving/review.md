# Separate backend test review

Reviewer: `/root/common_context_test_critic`, initially spawned without inherited
context and reused for this bounded review. Production baseline: e424029.

FINAL ACCEPT preceded production. The accepted package covers actual common-policy
proving, the fixed verifier, the independent complete-journal oracle, foreign
verification without a sender wallet, owned-ticket failures, shared capacity and
kind conflicts, cancellation/reaping, daemon SIGKILL and finite current-authority
expiry. Replay helpers appear only in negative late-output tests.

The reviewer checked that both callbacks preserve the previous daemon regression
suite. New head mutations run after those checks. Future certificates seven and
eight use genuinely mined EVM headers with strict sequence/predecessor links and
matching registry proofs; auxiliary Unix socket paths fit the macOS limit.

The baseline process run exited 101: two existing cases passed and the new case
failed because start_common_postage_proof returned unknown_method. The Python
capability probe likewise exited 1 on unknown_method before starting the expensive
chain/proof scenario. These establish missing-interface RED, not execution of the
later cryptographic cases on the baseline.

Nonblocking limitations: sixteen-record eviction is covered by the reused pool's
existing tests rather than isolated again for the new job kind. The mismatched
funding fixture also has a shorter resource-class lifetime, so it does not isolate
foreign ownership alone; the absent owned operation case does isolate wallet
lookup. The finite 1800-second authority lease accommodates the two previously
observed 300–370-second proofs plus bounded checks, but does not guarantee success
on slower hardware. Per-proof waits remain 1200 seconds. Remaining lease logging
was suggested for diagnostics, without changing the accepted input files.

Exact accepted file hashes are retained in accepted-inputs.json. Raw RED and GREEN
execution logs stay in ignored output/postage-common-proving; validation records
their hashes and actual outcomes separately.
