# 09. Common client API and agent operation

The UI, the shipped CLI, and MCP show the same data and verify the current limited rights.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R09, AR-R10, AR-R15, AR-R16, AR-R17, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-M02** (leases/fencing between runtimes, zombie takeover): fencing token (Kleppmann); Chubby (OSDI 2006); Gray–Cheriton leases (SOSP 1989); Zab epochs; SEDA admission control.

<a id="v1-m01"></a>
## V1-M01. Finish the common delivery/read/durability model

**Type:** backend. **Source cards:** M02, D03, U01. **Position in dependency order:** 57.
**After:** [V1-A04](03-tasks.md#v1-a04), [V1-W04](04-tasks.md#v1-w04).

**Change boundary / entry points:** `crates/core/src/broker.rs`, `crates/core/src/inbox.rs`, `crates/core/src/message_delivery.rs`, `crates/node/src/messaging_cli.rs`, `crates/node/src/mcp.rs`, `crates/node/src/runtime_client.rs`, `apps/desktop/src/AgentPanel.tsx`, `integrations`.

**Implementation plan:**

1. Cross-check the already persisted orthogonal funding/delivery/storage/discovery/work projections in all adapters.
2. Add the missing durable read/ack lifecycle per actor and inbox without duplicating business logic in TS.
3. Persist the acceptable state "delivered, persisted 4/10" and an explanation of partial/gap.

**Verifiable scenarios:**

- After a restart, CLI/MCP/desktop return a single snapshot for an exact message ID.
- A read ACK does not turn into an R10 storage proof; a storage receipt does not mean the recipient has read it.

**Checks:** AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A single persisted projection and a stable typed contract cover the entire messaging lifecycle.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-m02"></a>
## V1-M02. Implement inbox leases and outbox fencing for multiple runtimes

**Type:** backend. **Source cards:** M05, M02, I03. **Position in dependency order:** 58.
**After:** [V1-M01](09-tasks.md#v1-m01), [V1-I01](07-tasks.md#v1-i01).

**Change boundary / entry points:** `crates/core/src/broker.rs`, `crates/core/src/inbox.rs`, `crates/core/src/message_delivery.rs`, `crates/node/src/messaging_cli.rs`, `crates/node/src/mcp.rs`, `crates/node/src/runtime_client.rs`, `apps/desktop/src/AgentPanel.tsx`, `integrations`.

**Implementation plan:**

1. Use the existing poll/ack and runtime grant IDs for a bounded lease of a single message/stream.
2. Fence the outgoing actions of the old runtime after takeover; deduct shared messaging budgets atomically.
3. Survive pause/revoke/restart without losing the unread inbox; job execution/cancel/result remain V2.

**Verifiable scenarios:**

- Two runtimes do not ack each other's leased inbox and do not repeat a single fenced send.
- A lease owner crash gives an allowed takeover after the deadline; the old actor does not come back via retry.

**Checks:** AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A host/runtime change preserves message delivery and budget; external side effects are not declared exactly-once.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-m03"></a>
## V1-M03. Align the default V1 API/tool catalog

**Type:** backend. **Source cards:** M01, M06, U04, F02. **Position in dependency order:** 59.
**After:** [V1-M02](09-tasks.md#v1-m02), [V1-A05](03-tasks.md#v1-a05).

**Change boundary / entry points:** `crates/core/src/broker.rs`, `crates/core/src/inbox.rs`, `crates/core/src/message_delivery.rs`, `crates/node/src/messaging_cli.rs`, `crates/node/src/mcp.rs`, `crates/node/src/runtime_client.rs`, `apps/desktop/src/AgentPanel.tsx`, `integrations`.

**Implementation plan:**

1. Keep messaging, grants, wallet/network/recovery in the default UI/CLI/MCP; hide V2 jobs/services/reviews behind the existing feature/capability boundary.
2. Simplify only duplicate DTOs and adapters around the shared typed service; do not reorganize the entire workspace.
3. Fix the supported same-UID host trust model and the scope limits of the delegated handle.

**Verifiable scenarios:**

- Default tools are available by grant and do not offer V2 operations.
- Historical V2 readers/tests are preserved; a forged scoped request does not get root capability.

**Checks:** AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The default build matches the V1 scope; same-UID processes are not declared isolated by a sandbox that does not exist.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-m04"></a>
## V1-M04. Finish the grants panel and host provisioning

**Type:** frontend. **Source cards:** U04, M01, M06. **Position in dependency order:** 60.
**After:** [V1-M03](09-tasks.md#v1-m03).

**Change boundary / entry points:** `crates/core/src/broker.rs`, `crates/core/src/inbox.rs`, `crates/core/src/message_delivery.rs`, `crates/node/src/messaging_cli.rs`, `crates/node/src/mcp.rs`, `crates/node/src/runtime_client.rs`, `apps/desktop/src/AgentPanel.tsx`, `integrations`.

**Implementation plan:**

1. Show runtime identity, contacts/groups/methods, limit, and term before issuing a connection context.
2. Provisioning returns real packaged CLI/MCP paths and a private handle available to the selected host.
3. Revoke/change the limit through the backend; incoming message texts cannot change the approval by themselves.

**Verifiable scenarios:**

- The user connects and revokes a runtime, sees the old actor denied after revoke.
- Changed action parameters do not reuse the previous approval; secrets do not end up in renderer logs.

**Checks:** AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is a component/native UI flow and a visual check; the limits match the broker.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-m05"></a>
## V1-M05. Verify the shipped CLI skill and MCP on a real host

**Type:** verification. **Source cards:** M01, M02, M05, M06, X05. **Position in dependency order:** 61.
**After:** [V1-M04](09-tasks.md#v1-m04), [V1-W04](04-tasks.md#v1-w04), [V1-G09](08-tasks.md#v1-g09).

**Change boundary / entry points:** `crates/core/src/broker.rs`, `crates/core/src/inbox.rs`, `crates/core/src/message_delivery.rs`, `crates/node/src/messaging_cli.rs`, `crates/node/src/mcp.rs`, `crates/node/src/runtime_client.rs`, `apps/desktop/src/AgentPanel.tsx`, `integrations`.

**Implementation plan:**

1. From the installed artifact, run identity/send/delivery/poll/ack with the issued context and without dev APIs; include a new contact.
2. Repeat with an independent MCP wire client: negotiation, tools/resources, bad scopes, stdout protocol only.
3. Run a real host smoke test with restart/revoke and NAT/relay; if the skill changes, separately check instruction quality per AGENTS.

**Verifiable scenarios:**

- The CLI works without MCP; the agent receives and acks a reply after restart.
- Malicious content does not expand scopes or provoke a forbidden send; an incompatible version is explicitly refused.

**Checks:** AGENT FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A real-host transcript and black-box conformance are obtained, including first contact; a deterministic fixture alone does not close this gate.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** An available real agent host and a separate test conversation; outgoing messages only to explicitly allowed test recipients.
