# Daemon service test review

Baseline: ed9bfd712b37333ce149f92f93201873f0784b71. This is a test-first P01
integration increment; neither P01 nor the full V1 goal is complete.

Independent `backend-test-critic`, without context fork: **FINAL ACCEPT**.
The initial REVISE identified three false-positive risks. The accepted revisions
read the actual durable Marshal cursor, observe automatically renewed route bindings
and their use, and reset the report before fingerprinting. The critic confirmed all
three blockers resolved and found no remaining material blockers. Production code
was unchanged when this verdict was received.

Reviewed SHA256:

- Runner: 2cc5f16698bcb6bc3c2cdfc12a70814cd0ce0676d8e11196c0b27e04d916c552
- Oracle tests: 2dfb8da0a53dd9d3de37bf1041da415e75fdd2db6d235d1fe98875454f70a572
- Contract: fc0be907334401ce75bdfa781991543449f92131ade9928312f16a2c4e06270f
- Rust fixture: bda8021bb608bf32fb291cfad8c58cd6280a09b64710901c77ba8a8a77a30da5
- Shared carrier: 03a05aa76bca43953bd5dce05d0e3d86760c7160945aec72d6f91941634c86fb
- Cursor generator: e69294b69daca4ed094fd3662addbe8362a72758472ddaacfd3a9f6a034f8e24
- Real cursor corpus: 2c2af29e78bea3aab049ec6b6916d3c8037be9d69027c044d0892ed7a0b18b3a

Six independent oracle tests pass, including real four-process metadata and P256
certificate verification. Full runner RED is cargo101 for the absent
`FinalizerRegistration` and `run_with_finalizers` APIs. The report has passed:false.
Actual scope-to-scope packet replay and domain admission remain additional open gates.
