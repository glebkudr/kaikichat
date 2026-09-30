# Independent test review before implementation

Reviewer: `/root/common_context_test_critic`, originally created without inherited
context and reused with standalone descriptions of each bounded module. Production
was held until the review's final decision. Full V1 acceptance was never delegated
to this module's test decision.

## Public Core authority and historical record reader

R1: FINAL REVISE. The snapshot test did not isolate checkpoint signatures from
root/ID mismatch, and record tests did not mutate the journal independently of QC.

R2: FINAL ACCEPT. One original checkpoint signature byte is changed without
changing CBOR/body/time/proofs; one final resource u8 in the original journal is
changed (4 to 5) while preserving the original QC and record metadata. Both retain
original positive controls. Genuine two-chain issuer/committee proofs, no operator
keys, alternate policy/unbound issuer, head renewal, foreign profile, historical
expiry, durable clock rollback and actual SQL write failures are covered.

This foundation was implemented after R2; Core and postage-spend passed 178 tests,
and frontend passed 40. `verify_record` keeps the snapshot's finite historical
committee window and rejects recorded timestamps after local observation. It never
creates current admission or a signer.

## Ordinary daemon client and public ingress

R1: FINAL REVISE. Damaged receipt was rejected by the client before reaching the
new public ingress; the preparation test's authority and context expired together.

R2: FINAL ACCEPT. The library test supplies another profile's opaque permission
with identical committee and still-live local context, followed by a local positive
control. The live gate directly sends the damaged genuine receipt through an
independent nonmember Noise peer to the public receiver, checks no durable
candidate before/after restart, and retains ordinary positive client delivery. It
also tests wrong outer candidate ID, wrong outer nullifier and oversized input.

The malicious result carrier reopens a genuinely selected stopped profile and
serves real Core P256/Noise bindings. Its false result has current metadata/journal
and a genuine independently authenticated 720-byte QC from another committee.
Actual route, served proof and captured submission are required before observing
local rejection. Ordinary service with the same selected identity is the positive
control. SQL request and result failures, crash recovery, verified conflict, offline
reads and actual checkpoint expiry remain required by the full live gate.

Nonblocking suggestions: peer/request capacity boundaries and explicit pending
revocation scenarios. They were not substituted for the required live assertions.

RED was observed before network production: missing `prepare_client`, actual owner
API `unknown_method`, and actual EVM/daemon protocol `UnsupportedProtocols` before
expensive proof generation. Later assertions were not claimed executed at RED.

After R2, new production was implemented; the complete Rust workspace passed 533
tests with zero failures/ignored. Frontend 40, models 7, independent oracles 9,
TypeScript/Vite and workspace Clippy passed. Final live/regression/package results
must be read from their execution reports rather than inferred from this review.

## Historical observation regression after the first full live run

The first full live run failed when a second competing client request tried to
read the genuine winner certified before that request's public snapshot. The
selected response reached the client, but the historical lower authority bound
was incorrectly the client's later observation time. Earlier live stages had
already passed; this failure is retained in `failed-first-live/` and is not a
successful full gate.

R3: FINAL ACCEPT. Before correction, an added test using the original genuine QC
and a separate no-operator profile observing the same checkpoint five seconds later
failed with `Finalizer(Time)`. An isolated lower-bound negative changes only the
recorded timestamp to one second before checkpoint issuance, still within the
original journal and epoch. Original positive controls, upper limits and the
unchanged conflict E2E are retained. The reviewer verified that deleting this new
test block reproduces the prior accepted test hash and that production matched the
failed-live frozen inputs before implementation.

After R3, historical roster authentication uses the signed checkpoint's issuance
as its lower bound. Snapshot certificate authentication still uses observedAt;
current monotonic time, upper bounds and live admission remain unchanged. The
focused regression passed. Full final validation is recorded separately.

## Existing selected-service regression after client GREEN

R4: FINAL ACCEPT of the unchanged existing required finalizer-service test and its
actual RED as the regression contract before correction. No test assertions were
changed. Four actual profiles had seven authenticated effects and live services;
node0 had all three peer connections but only one route, with receiver rateLimited0.
The reviewer confirmed exact frozen hashes and that QUIC had not run after TCP
failed. Root cause was not established; bounded public discovery diagnostics were
accepted as a useful next step. Increasing timeout, limits or weakening authority
checks has no basis. Diagnostic reruns are not a production correction or release
acceptance.

## Cooldown-dependent discovery starvation

Public diagnostic instrumentation reproduced the unchanged TCP failure after
network replacement. The receiver correctly returned unavailable for repeated
wrong peer/key pairs; the healthy missing pairs were never queried before the
original 35-second deadline. Other peers retained all three valid routes. Passing
diagnostic reruns were kept separate from release acceptance.

R5: FINAL ACCEPT before production. Three scheduling tests cover staggered peer
cooldowns, changing input order with duplicate client targets, and one permanently
pending peer. They require all reachable routes within 35 seconds and at most
eighteen actual probes under the existing global two-second and per-peer six-second
pacing. The initial RED was an unresolved new scheduling module, not a simulated
claim that cryptographic verification had executed. The reviewer independently
reproduced the old global cursor finding only one of three routes in this schedule.

The correction canonicalizes and deduplicates candidates, rotates peers, and keeps
each peer's target progress separately. Progress advances only after the existing
bounded proof API queues a request. State is pruned against connected candidates
and reset with the network. Core authority, connection verification, receiver
limits and all original integration assertions remain unchanged. All three new
tests passed; unchanged full TCP/QUIC regression and final-source validation are
recorded separately and remain necessary for acceptance.

Nonblocking review suggestion: a future explicit disconnect/reconnect case with
changed targets could add coverage for scheduling-state cleanup. The three tests
exercise scheduling stimulus; only the real network gate demonstrates the retained
runtime pacing and authentication behavior.

## Test-only lint correction

The corrected production passed the original full TCP/QUIC gate, all 536 Rust
tests, 40 frontend tests and formatting. Clippy then rejected three `unwrap()`
calls in the new scheduling test module. The original failure is retained under
`failed-discovery-clippy/` with its frozen input manifests.

R6: FINAL ACCEPT. The sole change adds `#![allow(clippy::unwrap_used)]` to the
`cfg(test)` module, matching the existing checkpoint scheduling tests. The reviewer
verified that removing this line restores the previous test file and accepted hash
byte for byte, and that all 448 other source and 12 other accepted hashes match.
Panics and every assertion/stimulus are unchanged; production was not edited.
R5 remains accepted. The final validation report records this exact transition
and the corrected Clippy/focused-test outcome without hiding the earlier failure.
