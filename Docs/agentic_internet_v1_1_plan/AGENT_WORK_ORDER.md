# Protocol for issuing and accepting work by agents

## Purpose

This file is handed to each implementer together with the card `tasks/<ID>.md`, the related Rxx, and approved upstream contracts. Code generation speed is not limited by an estimate of human time. What is limited is semantic ambiguity, the scope of authority, and the size of an unverified change.

## 1. The atomic task contract

An atomic unit ends with observable behavior: a message is delivered after a restart, a double spend is prevented, Remove is executed, replicas are restored, or an illegal MCP call is rejected. "Created a crate/trait/folder" is not a sufficient product outcome.

The card must contain the source Rxx, prerequisites, the permitted scope of files/APIs, behavior and prohibitions, a specific independent test oracle, a negative/fault scenario, and a reproducible acceptance command. A complex packet may be split into subpackets along known boundaries — parser/vectors, reducer, store adapter, interop, UI integration. Its external contract and release gate do not disappear because of this.

## 2. The order TEST CONTRACT → RED → GREEN → ADVERSARY → MERGE

**Test contract.** A separate author/reviewer describes the inputs, permitted outputs, invariants, failure modes, and the source of expected values. Data from one's own future implementation is not an oracle. Wire vectors are built from the specification or an independent implementation; for distributed state machines there is a reference model.

**RED.** Tests are run against a missing or deliberately incorrect implementation and fail for the expected reason. A compile error can be an early TDD stage, but before acceptance a meaningful failing assertion is required. Commands, output, and commit ID are saved.

**GREEN.** The implementation agent writes a minimally sufficient but not cut-down implementation. It does not weaken auth, quorum, expiry, secret storage, or data restrictions to pass the happy path. A change to the public contract is done separately, with an update of all consumers and review by an independent owner of the specification.

**ADVERSARY.** A separate role attacks the boundaries: input corruption, expiry, replay, role escalation, reorg, interrupted journal, partition, false receipts. Changing the comparator, removing a nonce check, lowering a threshold, or a missing domain check must be caught by the corresponding mutation suite. A found trace is added to the permanent corpus.

**MERGE.** The integration owner runs the packet on real producer adapters in an isolated devnet/profile and checks the black-box result. GREEN on mocks alone does not complete the packet. Scope, tests, limitations, license/dependency changes, compatibility, and traceability are provided.

## 3. Roles do not mean six mandatory people

Roles can be performed by agents in parallel, but all checks must not be merged into one uncontrolled self-review. There must be a specification owner, an independent test author, an implementer, an adversarial reviewer, a completeness critic, and an integrator. For economic/cryptographic boundaries, an independent subject-matter review is mandatory regardless of the number of automated runs.

## 4. The mandate of the completeness critic

The completeness critic receives the original Uxx, the initial request and AMENDMENT-01…05, `REQUIREMENTS.md`, and the plan/implementation diff. Its task is **not to simplify the product but to find the loss of the original wishes**.

It checks: where the mandatory groups are; who repairs data when both clients are off; how a clean client starts without a company; where the Google/Telegram/site login and separate funded eligibility are; how free becomes paid without an upgrade; who pays for repair/relay; why company keys do not control the network; how job completion differs from settlement; how an unconfirmed model stays unconfirmed; where the Tauri security boundary is, the right to a negative review without the executor, public rating with a partial dataset; why a single issuer is acceptable only within its scoped profile; why the V2 economic engine did not become a V1 prerequisite.

The report has the format `requirement → implementation path → positive test → adversarial test → unresolved gap`. The critic must not replace "V1 groups" with "later, because it is hard". It is allowed to show that the chosen technical means does not fulfill the requirement and to demand another means with the same observable outcome.

The security critic has a separate mandate: to block an unsafe implementation and propose a safe path. A found problem must not be resolved by renaming a failure to success or by disabling the check. Neither of them declares production safety based on a single green suite.

## 5. Parallelization without merge chaos

`backlog.json` contains the DAG and owners. `F02` owns wire/types/schema: downstream agents do not edit them ad hoc at the same time. Shared interfaces are changed by a separate contract PR. Independent implementations work in separate branches/worktrees and named network domains. Integration batches are small and contain one observable outcome.

When the prerequisite implementation is not ready yet, independent contract tests, a reference model, UI viewmodels, and consumer implementation on a strictly typed fake adapter are allowed. A fake runtime/chain/verifier does not get into the production genesis and cannot grant the status of real money or verified quality. Late integration GREEN is mandatory.

The meaning of consensus must not be parallelized as two incompatible schemes that are then "glued together". Model checking, vectors, persistence faults, the network adapter, and the implementation of one already approved scheme can be parallelized.

## 6. Test classes and mandatory evidence

| Risk | Minimal set |
|---|---|
| Parser/wire | Golden vectors, independent parser, roundtrip/property, size limits, fuzz malformed inputs |
| Identity/capabilities | Permission matrix, domain/replay tests, revocation/expiry, key leakage fixtures, recovery rollback |
| Persistence | Kill at commit/fsync boundaries, disk full, corrupted record, deterministic replay, idempotent outbox |
| Replication/group | Seeded Byzantine/partition simulator, reference state model, eventual recovery under stated assumptions |
| Postage/L2 | Conservation, concurrent spend, finalized roots, reorg, committee handover, integer rounding, circuit vectors |
| External trust | Google/Telegram/site; proof/session/PKCE/iss/aud/expiry, JWK rotation, exact subject namespace, single/threshold issuer, funding caps, unlink and optionality |
| Reviews/rating | Two receipt signatures; eligibility without result/payment/veto; amendments/replies/replay; deterministic aggregate; partial sampling; Sybil residual risk |
| MCP | Protocol conformance, ACL per tool, prompt injection, cancellation/restart, HTTP origin/audience/session checks |
| Tauri | Real IPC bridge/ACL, XSS/CSP/remote origin, secrets outside webview, keyboard E2E, accessibility/IME, trusted approvals; no test hooks in release |

Machine-readable traces and resource metrics are needed, not just a screenshot of green terminal output. End-to-end scenarios must run from clean state and after an upgrade/rollback attempt.

## 7. Definition of Done of a packet

Observable behavior actually works; positive/negative/fault tests are reproducible; RED/GREEN evidence is saved; the consumer is integrated with real dependencies; there are no undeclared trust roots, new rights, unbounded allocation, or false settlement semantics; stable test IDs and source traceability are updated. Limitations are written next to the promise they constrain.

Task-ID → test names: `<ID>.T01...` for contract tests, `<ID>.N01` for the mandatory negative scenario, `<ID>.F01` for fault, `<ID>.E01` for external acceptance. This is a naming contract for the future implementation, not a claim that such tests already exist.

## 8. Forbidden ways to "speed up" delivery

Do not replace decentralized placement with a single company API; do not move groups out of V1; do not treat a local nullifier as global; do not raise trust from a self-declared model; do not pass off a fixture receipt as a paid contract; do not lower quorum under partition; do not turn Google into a mandatory recovery method; do not substitute the audit of an external library with the number of one's own unit tests; do not promise physical deletion of someone else's copy.

## 9. Execution of untrusted code

Coding tasks and obtained tools run in an explicitly dedicated execution environment. The daemon/desktop/MCP process is not a sandbox. Minimal file/network/secret permissions, CPU/RAM/time limits, and egress control are set outside the prompt. For tasks capable of creating arbitrary code, the VM/microVM profile is separated from the transport identity and the wallet. The working directory contains no root keys; the agent receives only a job-specific capability.

## 10. Rules of revision 1.1

Public reviews are part of the V1 order protocol, not a future reputation market. A04 delivers only the contract/constraints of future delegation and a safe unsupported; writing a full-fledged economic engine is not a way to "close more tasks" in V1. The product scope is not expanded without a new user decision.

For each changed task, the agent reads the corresponding AUTH_AND_ATTESTATION_V1.md or PUBLIC_REVIEWS_PROTOCOL_V1.md. F02 keeps ownership of the canonical wire types, Q02 of the review event semantics, Q04 of the rating reducer, U07 of the Tauri bridge. The shared broker, not the frontend or an LLM prompt, is the source of permissions.

The plan validator checks metadata, links, and mandatory guardrails; it does not prove the correctness of the future implementation. Product RED/GREEN evidence is created anew when each card is executed.
