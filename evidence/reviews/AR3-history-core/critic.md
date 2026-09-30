# Independent test review

The existing context-free `/root/index_holder_test_critic` reviewed this bounded
slice separately, using the backend-test-critic skill. It inspected tests and
related real helpers/production contracts; it neither modified files nor ran
builds. Production stayed at `743e1221bedbac999f267f25839c648a0a3b1cd0` until final
ACCEPT and the current-input compile RED completed.

First verdict: **REVISE**. Add explicit preservation of all non-history state and
fetch cursors after successful accept. Isolate an authentically signed higher
revision with earlier issuance from ordinary revision/equivocation failures, then
show the correctly timed same revision succeeds. Check getters before issuance.

Second verdict: **ACCEPT**, after those assertions were added. The critic also
accepted reuse of the existing paid-store immutable-reference rule in the crypto
verified type: the existing genuine changed-hash test and new cross-anchor
omission test cover the common policy without duplicating fixture signing logic.

Final additional delta: exact retry at a short reference's expiry while its
longer anchor remains live. Verdict **ACCEPT**. The critic confirmed this models
a lost response, retains exact publication/SQL bytes, and does not require Bob's
independent read clock to match Alice's later retry clock.

Accepted SHA256:

- `crates/core/tests/support/custody_history.rs`:
  `c6f6ca22a98cbf3c24b3dd8689b1d4a0084a7150e879a5f542d6dfedf0f33d4b`
- `crates/core/tests/support/custody_progress.rs`:
  `e0d9016fa516b7af9a401614a9294081bfc965f5bf2dd372889d02a3a61e4e3f`
- `crates/core/tests/support/custody_index_progress.rs`:
  `2bf77ae123b9949fb408f2ac0ae57e00fc5247dcc8602d9f6dac4fa69726e38d`

Baseline-1 and baseline-2 belong to earlier test revisions. Baseline-3 is the
accepted final input: 1 E0432 and 44 E0599 for absent API/types, with no changed
inputs during the run. This is compilation RED, not a runtime behavioral failure.
