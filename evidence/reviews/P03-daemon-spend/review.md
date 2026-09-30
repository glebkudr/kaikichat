# Independent test review

Reviewer: `/root/common_context_test_critic`, originally created without inherited
context and reused with standalone bounded prompts. No production edits occurred
before the final acceptance below.

R1 FINAL REVISE: the new Python Node assigned a method containing a zero-argument
`super()` from an unrelated lexical class. The reviewer independently reproduced
`TypeError` without launching a daemon. Additional business improvements: actual
pair connectivity, authority loss while a SQL effect is blocked, fresh submit after
head replacement, bounded overload, fixed proof image/binary hashes and runner entry.

R2 FINAL ACCEPT: startup explicitly delegates to the shared base Node. Accepted
assertions now check actual isolated TCP pairs, rejection before reconfiguration at
a new head, no commit/ack after role disable despite removing the SQL fault, disabled
restart and subsequent cold recovery with all other peers stopped. Fixed image and
proof binary hashes are checked. This accepts test quality before production, not
the implementation or full V1. Bounded overload remains a separate incomplete gate.

Accepted test inputs: `accepted-inputs.json` (copied after validation). The first Rust
baseline compiled and failed because the requested owner method returned
`unknown_method` instead of `invalid_request` when an unknown `now` field was added.
The full live gate is intentionally run only after implementation; it generates two
fresh, genuine proofs and makes no pre-production success claim.

The contract document was written after implementation to describe the resulting
interface and its explicit remaining limits. It was not a test-first review input.
