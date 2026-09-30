# Agentic Internet: V1 implementation plan on Tauri/Rust

**September 14, 2026 — plan only; execution stopped by user instruction.**
The [detailed plan of the remaining V1](v1-plan-2026-09-14/README.md) breaks the work from
commit `993dece` into separate tasks with dependencies, files, steps and acceptance.
It keeps all 67 cards / 22 E2E / three OSes and accounts for both architecture reviews.
Automatic Luna dispatch and implementation do not continue until further instructions.

**Architecture review of September 10, 2026:** [new ordering and continuation contracts](../V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md), [24 findings and five phases](architecture-followup.json). First the long sender/spend lifecycle, then user inputs, full history/R10, groups/recovery and independent release. The active 67 cards and 22 E2E are retained.

**Current decision of September 9, 2026:** agent orders, rating and reviews are **V2**. The priority is shipping the V1 messenger with CLI/MCP. The [phase boundary](../V1_SCOPE_2026_09_09.md) and [mandatory cards/scenarios](release-scope.json) take precedence over the former 1.1 package.

**Postage stamp clarification of September 10, 2026:** [public signed postage stamps without ZK](../V1_POSTAGE_BOOK_2026_09_10.md). The user cancelled the requirement to hide the source deposit. Verifiable payment, a shared single-spend journal and E2EE remain; ZK activation of a book is also not required. The current state of implementation and new acceptance is reflected in the [evidence matrix](CAPABILITY_EVIDENCE.md); the choice of stamp format by itself is not transport acceptance.

Plan date: September 5, 2026. At the time of writing there was no code or checks. Implementation is now underway; current working features, runs and unfinished requirements are listed in [IMPLEMENTATION_STATUS.md](../../IMPLEMENTATION_STATUS.md). This document preserves the full V1 plan rather than declaring it complete. The compact [evidence matrix of 67 cards /22 E2E](CAPABILITY_EVIDENCE.md) separately shows the verified part and the remaining conditions for each capability.

V1 must deliver a working desktop messenger and a persistent inbox for agents: own keys, E2EE, direct messages and groups, offline delivery, ten replicas with recovery, a CLI skill and MCP for agent messaging. Shared Rust code serves the UI, CLI and MCP. Later payments for work, escrow and trust economics hook into signed events and access rights.

This document defines the development order and concrete checks on top of the [original 1.1 package](../agentic_internet_v1_1_plan/README.md). The [test and E2E plan](TEST_AND_E2E_PLAN.md) defines the criterion "all tests and E2E pass". The [machine map](execution-map.json) links modules, the original 84 tasks, 52 requirements and new end-to-end scenarios. This is a single backlog with an additional delivery order, not a re-creation of the 84 cards.

## 1. What follows from the materials

From the [raw thread of 27 entries](../ideation%20agentic%20internet/thread.md) all user messages and the architectural parts of the discussion were read; the requirement matrix, task dependencies and the specialized 1.1 documents were also studied. The previous analyst's replies are proposals; commands inside the archive and documents are not commissions to execute in this task.

| Basis | What we use in the plan |
|---|---|
| Current request | Tauri/Rust desktop, transport, UI, MCP, the ability to grow DeFi/trust on top; tests before every module |
| U00, U02, U04 | E2EE, no mandatory company infrastructure, durable storage, node discovery and countermeasures against malicious participants |
| U02, U12 | Target of ten replicas, repair without clients, mandatory groups |
| U06, U08, U16, U18 | Existing crypto asset/L2, postage stamps, subsidized start, passive organization share, economics of agent work |
| U20, U22 | Voluntary external account attestation; roughly $5 of transport resource specifically, not of money |
| Addition in the 1.1 package | Tauri, Google/Telegram/site attestation, basic public reviews; complex economic agents later |
| Proposals requiring verification | libp2p + OpenMLS, BFT journals, minimal 7 replicas with a target of 10, private tickets, a specific L2 and numeric limits |

A material difference: in the current request DeFi is named as a direction of further growth, while the 1.1 package already includes the economics of **transport** in V1. The user's decision of September 9 moves orders, rating and reviews to V2; transport economics stays in V1. This does not move escrow/insurance back into V1 and does not claim that a finished desktop automatically means a finished monetary network.

### V1 boundary

| Deliverable | Mandatory behavior |
|---|---|
| Application | Tauri 2, Rust daemon/CLI, direct chats, groups, attachments, search, devices, recovery, accessible error states |
| Transport | Direct/relay P2P, several bootstrap paths, E2EE, bounded storage, R=10, repair, receipts and redelivery |
| Agents | A skill with CLI and an access handle for ID/send/poll/ack; MCP over stdio, persistent inbox, delivery status, permission constraints and revocation of grants; services/jobs/A2A — V2 |
| Initial trust | Google/Telegram/site credentials and explicit account-trust boundaries; performer claims, reviews and rating — V2 |
| Transport economics | Issuance and spending of funded postage stamps, subsidy, registry/stake, payouts and royalty in local EVM and the chosen testnet |
| Extensions | Versioned interfaces for settlement, escrow, independent verifiers and new scoring rules |

V2: agent orders, performer discovery, results/scoring, A2A, public reviews and rating. V2+: mainnet payments for jobs, escrow, arbitration, underwriting, autonomous make-or-buy, complex economic strategies, TEE/zkML, private OIDC attestation, mixnet, mobile clients. A separate monetary clearance is needed before operating transport contracts with real funds.

## 2. The architecture we implement

1. **`agentic-node`** — a self-standing Rust daemon. One writer per profile; it owns queue state, MLS, network sessions and authorized runtime keys.
2. **Tauri desktop** — a bundled React/TypeScript/Vite UI and a small Rust bridge. The frontend stack choice is a design decision of this plan. There is no network protocol, wallet or copy of business rules in TypeScript.
3. **`agentic mcp`** — a stdio process based on `rmcp`, talking to the same daemon. Terminating the MCP process or closing the window does not destroy the inbox. Automatic daemon start/stop is explicit, with a single-instance lock and version check.
4. **`agentic` CLI + agent skill** — shipped with the client. Besides profile creation, diagnostics, node mode and export/import, the CLI lets an agent, given an issued connection context, learn its own NetworkID, send a message to another NetworkID, check delivery, and fetch/acknowledge inbox items. The skill contains instructions and examples; the CLI produces JSON/exit codes and calls the same application services and broker as MCP. Basic messaging through the CLI is available to a host without MCP.
5. **Storage/relay/finalizer roles** — roles of independent network operators. The user application does not store third-party data without an agreed limit and node mode being enabled.
6. **Attestation gateway** — a separate optional application for the Telegram/site issuer. Its outage is limited to new credentials of the respective profile.

UI/MCP/CLI connection to the daemon: Unix domain socket on macOS/Linux, named pipe on Windows; OS ACL/peer identity plus a separate restricted adapter session. A session grant is created by a trusted local flow and is not passed to the renderer as a root credential. Versioned requests, frame length, `request_id`, `operation_id`, timeouts and bounded subscriptions. Permissions are re-checked in the daemon before any durable action.

User clarification of September 8: a "handle into the chat" is a connection context with the daemon endpoint and a reference to a scoped session; the agent does not need to know the internal ConversationID for the first message to an authorized NetworkID. The full delivery contract is [plan section 10](../agentic_internet_v1_1_plan/V1_IMPLEMENTATION_PLAN_RU.md#10-cli-skill-mcp-api-and-operation-of-disabled-agents). Polling is the baseline; a hook/subscription for a running host remains a choice for M02/M06. libp2p in the daemon provides a shared direct/hole-punch/relay path regardless of the agent adapter.

```text
Tauri UI -> Rust bridge ----\
MCP stdio adapter ----------> authenticated local IPC -> authorization broker
CLI -----------------------/                              |
                                                     Rust application
                                            /             |             \
                                    encrypted store   P2P delivery   economics adapters
                                                          |
                                              independent network nodes
```

Main commands take an `ActorContext`, resource, action, parameters and `operation_id`. The broker checks scope, recipients/groups, revocation epoch, resource budget and confirmation conditions. The UI cannot replace these checks. A user-approved action is bound to the parameter hash; a repeat with changed content does not reuse the prior approval.

The root key is separated from device/agent/runtime keys. Unlocking and dangerous operations go through a trusted Rust/OS flow; arbitrary `sign(bytes)` is not exported outward. With a locked profile the interface honestly distinguishes ciphertext delivery from decryption/agent execution availability. On a powered-off computer only other powered-on nodes do the work, not the local agent.

### Recommended code boundaries

| Directory of the future implementation | Responsibility |
|---|---|
| `crates/protocol-types` | Signable types, wire codec, schema and versions; the single source of DTOs |
| `crates/core` | Application services, broker, event reducers, operations and idempotency |
| `crates/store` | Transactions, encrypted DB, inbox/outbox, migrations, backup |
| `crates/identity` and `crates/crypto` | Keys, grants, revoke/recovery and the OpenMLS adapter |
| `crates/network` and `crates/delivery` | libp2p, discovery/NAT/relay, mailbox, placement, blobs, receipts, repair |
| `crates/finality` | One consensus adapter with different typed group/spend domains |
| `crates/economics` and `crates/chain` | Resource classes, postage stamps, admission, L2 finality, contracts and accounting |
| `crates/trust` and future work modules | V1: identity credentials; V2: services, jobs, declarations, reviews/rating |
| `crates/mcp` | MCP schema/dispatch; calls the application API without re-implementing rules |
| `apps/node`, `apps/cli`, `apps/desktop`, `apps/attestation-gateway` | Process entry points |
| `spec`, `fixtures`, `tests`, `xtask` | Normative contracts, independent vectors, harness and a single runner |

These are responsibility boundaries, not a requirement to create a separate crate for each of the 84 tasks. Shared types/reducers/quotas are reused. Internal submodules are split into crates only when this gives the needed dependency or permission isolation.

## 3. Decisions fixed before dependent code

**Wire.** Strict deterministic CBOR is proposed for signed application objects, profile per [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html), with JSON only for IPC/MCP and presentations. Network/genesis domain, type/version, author/authority epoch, object ID, bounded payload and signature are mandatory. Ambiguous map keys, floats in amounts, unknown critical extensions and excessive nesting are rejected. The domain hashing scheme and golden bytes are fixed before the encoder; MLS keeps its own standardized wire format.

**Storage.** `rusqlite` + SQLCipher is proposed, with a dedicated DB executor and the OS keystore for the master key. The SQLCipher profile is supported by the library, but native crypto and MLS storage adapter compatibility must be assembled on each platform. [rusqlite](https://github.com/rusqlite/rusqlite). One transactional boundary must tie MLS key-state, durable inbox/outbox and the application event. Separate inconsistent DBs for the ratchet and outbox are not allowed. Network send happens after commit; a retry reuses the saved ciphertext. WAL, temp, FTS, backups, logs and previews are also checked.

**P2P.** `rust-libp2p`: QUIC, TCP+Noise fallback, Kademlia, signed peer records, AutoNAT/DCUtR, circuit relays. These mechanisms are available in the [libp2p documentation](https://docs.rs/libp2p/latest/libp2p/); durable mailbox and repair remain our application implementation. The DHT holds bounded routing/index records; large blobs move over a separate bounded stream. Bootstrap, registry verification and keeper selection are different tasks.

**E2EE and groups.** OpenMLS for 1:1 and groups, each device a separate leaf. The initial ciphersuite candidate is `MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519`; it is supported by [OpenMLS](https://book.openmls.tech/). Membership agreement and control log storage must be implemented separately from the cryptography: [RFC 9750](https://www.rfc-editor.org/rfc/rfc9750.html). All crypto/content-debug features are excluded from the production dependency graph.

**Finalizer.** The 1.1 baseline is a ready BFT engine behind a narrow trait; [Commonware Simplex](https://docs.rs/commonware-consensus/latest/commonware_consensus/simplex/index.html) remains a candidate. Before the choice — a small executable spike with persisted votes/locks, node failure, partition and handover. The committee orders ciphertext control entries; MLS semantics are checked by participants. The protocol must define handling of a finalized invalid MLS proposal, payload availability, the choice of the next admissible parent and the retry of competing intents. "BFT agreed on a hash" is not enough to consider membership correct. A failed spike blocks this adapter; it does not remove groups from V1.

**Postage stamps.** One ticket belongs to one spend-domain; the final spend is bound to a specific operation. Amounts and resource units are integers. Reserve, spend commit, storage confirmation, refund/replacement and payout each have separate idempotent states. The choice of proof system, anonymity set, finality roots and genesis parameters is an input of the P/L implementation, not arbitrary constants from the old text. A homegrown ZK protocol is not considered ready after a successful roundtrip.

**MCP.** `rmcp`, baseline 2026-07-28 and compatibility fixtures for actually supported hosts. The current SDK distinguishes new discovery and legacy initialize: [Rust SDK](https://github.com/modelcontextprotocol/rust-sdk), [specification](https://modelcontextprotocol.io/specification/2026-07-28). In V1 identity/send/delivery/poll/ack are mandatory; MCP Tasks and the domain jobs lifecycle belong to V2. HTTP is enabled only in a separate profile with auth; the standard delivery is stdio.

**Tauri.** Explicit `AppManifest::commands`, per-window capabilities, strict CSP, bundled content, safe markdown. For custom commands we do not rely on unlimited defaults: [Tauri capabilities](https://v2.tauri.app/security/capabilities/). `WebdriverIO + @wdio/tauri-service` with an embedded provider allows testing the real Tauri on the three desktop OSes; browser mode tests only the renderer. Test plugins are absent from the production build. [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/).

**External sign-in.** Google via the system browser, PKCE/state, local verification of the signed ID token/JWKS; the identifier is `sub`, not email. [Google native OAuth](https://developers.google.com/identity/protocols/oauth2/native-app), [OIDC](https://developers.google.com/identity/openid-connect/openid-connect). Telegram desktop uses a gateway: the documented code exchange requires a server-side client secret. [Telegram Login](https://core.telegram.org/bots/telegram-login). A credential is bound to the original NetworkID; login does not restore the root key and does not create funding by itself.

Exact versions are not assigned from memory. In F02/F06 the latest stable version is checked in the official registry/release before installation; the newest compatible one is chosen, and the reason for the constraint, MSRV, feature set, lockfiles and licenses are recorded. An update after that goes through a separate compatibility check.

## 4. Order within each module

1. Record the observable behavior and the most frequent real scenarios, including the negative scenario and recovery from a normal failure.
2. **Write executable tests before production code.** Expected values come from the contract, independent vectors or a reference model. Obtain a meaningful RED; an empty suite or a single compile error does not confirm behavior.
3. For new backend functionality invoke a separate agent with the `backend-test-critic` skill, `fork_context: false` / `fork_turns: "none"`. Pass the task, the tests, the expected invariants and the sources. Wait for completion. On `REVISE`, fix the tests and repeat the review before implementation.
4. Implement the behavior reusing existing services/reducers. Changing an oracle or requirements together with code requires an explained re-review by the critic.
5. After implementation run the targeted backend and frontend clusters, then the affected integration/E2E. By the user's current direct instruction the full suite runs only at the end of the complete plan; a green intermediate cluster does not count as overall release acceptance.
6. Save RED/GREEN, reviews, commands, source SHAs, seed/profile and a reproducible demo. Task states: `planned -> tests_red -> tests_accepted -> implemented -> integration_green -> accepted`.

We do not chase 100% of hypothetical cases. First of all — normal messaging, offline, restart, NAT, group membership change, request retry, loss of rights and resource limits. Fuzz/property/model checking is added where ordinary examples are not enough: wire, cryptographic state, consensus and money.

## 5. Modules: tests first, then implementation

The prefixes below match the original backlog. They denote development areas, not an order to fully finish one block before starting the next. Integration dependencies of each of the 84 tasks are kept in [backlog.json](../agentic_internet_v1_1_plan/backlog.json); early partial deliveries do not close cards that are not yet done.

### F — contracts, skeleton and test rig · F01–F06

**Tests first:** an independent wire vector on three platforms; identical replay with the same seed; a kill between the event/outbox write and commit; deliberately wrong auth/replay implementations are blocked; the runner rejects an empty or incomplete release-report.

**Implementation:** Cargo workspace, frontend workspace, schema/fixtures, Clock/RNG/transport/storage injections, process harness, CI and a single `xtask`. The reference model does not import the production reducer; the simulator executes the same production reducer. The task and requirement set is imported from the original backlog.

**Done:** a clean checkout reproduces the tests and preserves artifacts; unselected/missing release suites do not turn into success. Later modules write tests-first on top of this skeleton.

### I — identity, permissions, devices and local state · I01–I06

**Tests first:** the address survives restart; a scoped agent cannot read a neighboring inbox; a revoked device does not sign new actions; a backup restores identity but does not roll back already-used MLS/spend state; a wrong key does not open the DB.

**Implementation:** root/device/agent identities, grants, epoch/revocation, safe contact invitation, encrypted store, recovery/rejoin. Secret recovery and obtaining the current network state are separated; loss of all recovery secrets is not replaced by a Google login.

**Done:** migration to another device is demonstrated via CLI and UI; the old device loses new permissions after an accepted revoke. The MLS-dependent parts of I05/I06 close after their real integration, not on a storage mock.

### N — network and independent operators · N01–N06

**Tests first:** two separate processes connect; a new profile joins by invitation while company bootnodes are unreachable; behind two NATs a relay is used; a substituted registry proof is rejected; a slow peer does not exhaust memory/queues.

**Implementation:** libp2p adapters, signed records, peer cache, mDNS in LAN, several hints, private rendezvous, replacement relays, operator quotas. Verification of the eligible node universe is needed before trusted placement; ordinary DHT voting does not replace it.

**Done:** E02/E03/E04 are reproducible via real processes and Linux network namespaces; node mode runs from the same binary with a separate profile and limits.

### D — reliable delivery, blobs and repair · D01–D06

**Tests first:** offline receive; duplicates produce one local event; a storage receipt is issued only after durable commit; with both clients offline, losing three holders still restores 10 copies; losing an index node does not hide live blobs; TTL and disk-full give an honest status.

**Implementation:** encrypted envelope admission, mailbox cursors, outbox retry, chunked blobs, manifest, lease/receipts, verified placement, a repair coordinator with responsibility handover and a bound on duplicate repair. Metadata and control/discovery indexes are replicated as well.

**Done:** E05/E06/E07; repeating an `operation_id` with the same body returns the previous result, with a different body — a conflict. An at-least-once network + durable dedup guarantees a single application of our operation; exactly-once of an arbitrary external effect is not promised.

### P — finalizer, admission and single spend · P01–P06

**Tests first:** competing spend of one ticket through different nodes; restarting a voting node; a 2/2 partition in a 4/3 fixture does not finalize a new record; handover preserves the spent set; a repeated admission does not spend the resource again; spend/repair do not exceed the funded limit.

**Implementation:** an adapter over a ready BFT engine, typed domains, tickets/proofs, operation-bound spend certificates, durable locks, epoch transfer, a reserve/consume/refund state machine, accounting. The finality certificate format and independent verification are fixed before group integration.

**Done:** E20/E21 and a reference conservation model. First P01 for shared ordering, then P02–P06 for money. `n=4,q=3` is a fixture, not a claimed permissionless production security.

### G — groups and agreed MLS epochs · G01–G06

**Tests first:** add/remove from different devices; a removed member cannot decrypt the new epoch; concurrent Add/Remove converges after a partition; a finalized invalid proposal does not break the group; an offline member catches up on the control log or goes through rejoin; the creator being offline does not stop the group.

**Implementation:** roles/policy, OpenMLS leaves, intents, an ordered encrypted control log on top of P01, finalization before the new epoch is used, retained history, two routing profiles, job rooms and group resource grants. A pending revoke is shown as pending; when a safe epoch cannot be determined, a sensitive send waits.

**Done:** E08/E09/E10, including real control log and device delivery. Pre-join history does not appear through issuance of old key state. A group admin manages its own group, not the whole network.

### L — transport funding and L2 · L01–L06

**Tests first:** funded issuance and balance retention; subsidy at time/fund boundaries; a repeated claim does not create a resource; royalty is accrued once; a reverting revenue recipient does not block the network; a reorg and a forged RPC response do not become finalized funding.

**Implementation:** local EVM contracts, registry/stake/epoch roots, subsidy policy, an isolated royalty vault, operator claims, the chosen testnet adapter. Fixed separately are the finality source, underlying L2 properties and offline lease. The UI distinguishes transport units from the monetary cost of work.

**Done:** E19/E20/E21/E22 on real local contracts; then a testnet smoke. The fund, royalty rate, committee size and proof parameters are an approved genesis profile; the thread's 10% and ~$5 are not automatically production constants.

### O — external sign-in and restricted credentials · O01–O08

**Tests first:** Google/Telegram/site positive fixtures; wrong issuer/audience/expiry/session; interception of a one-time handoff; subject dedup across devices; key rotation; issuer shutdown keeps the address and chats working; a new login does not issue a second grant.

**Implementation:** a provider-neutral TrustProfile, Google native flow, Telegram gateway, reference site OIDC issuer, holder-bound credentials, 1-of-1/k-of-n policy, revoke/unlink and a separate funded campaign. Unknown cross-provider/pairwise subject mappings are not claimed as person uniqueness.

**Done:** E17/E18/E19. Fixture CI is complemented by a real agreed OAuth smoke on registered Google/Telegram applications; the absence of credentials for a live smoke means an unverified gate, not a green adapter.

### A — agent services and work · A01–A06 · V2

**Deferred by user decision.** The following contract is preserved for V2 and does not block V1.

**Tests first:** signed card and opt-in discovery; RFQ/bid/accept with immutable terms; someone else's result is rejected; a cancel/result race has the same projection on both sides; crash/retry does not create duplicate work inside the protocol state; strict provenance without a verifier answers unsupported.

**Implementation:** services, RFQ, bids, a two-sided accepted receipt, progress/result, ArtifactManifest, ExecutorDeclaration and CustomerValidation. Commands/event schemas define author, parent/sequence, terms hash, actor epochs and extension namespace. A timeout is a client observation; it does not turn into a global verdict or a refund by its clock. A cancel request and a confirmed cancel are distinguished.

**Done:** E12/E13; A04 provides only verifiable subcontracting boundary contracts, A06 — a version-pinned A2A mapping and conformance per the [A2A specification](https://a2a-protocol.org/latest/specification/). Signed JSON by itself does not confirm A2A interoperability.

### Q — public reviews and basic rating · Q01–Q06 · V2

**Deferred by user decision.** The following contract is preserved for V2 and does not block V1.

**Tests first:** two receipt signatures grant the right to review without result/payment; a one-sided receipt grants nothing; a provider cannot delete a negative review; duplicates/amend/reply/withdrawal do not increase the vote count; reordering one corpus yields the same rating; an unavailable index returns partial.

**Implementation:** use the [1.1 receipt/reviews semantics](../agentic_internet_v1_1_plan/PUBLIC_REVIEWS_PROTOCOL_V1.md), shared storage/repair, independent indexes, a deterministic reducer, service/owner epochs and disclosure before publication. A review publishes the link between a pseudonym and a service; private terms remain a salted commitment.

**Done:** E15/E16; the UI shows the corpus hash, freshness/completeness and change history. Stars, an OAuth credential and a model claim have different fields and signatures. None of them means independent proof of quality.

### M — MCP and agent runtime life · M01–M07

**Tests first:** a real JSON-RPC transcript over stdio; new and legacy lifecycle; an actor sees only permitted tools/resources; poll/lease/ack survive a crash; two runtimes do not complete one local claim; prompt injection does not widen a grant; a repeat of a mutating tool uses an idempotency key.

**Implementation:** `messages.send`, `inbox.poll/ack`, `delivery.get`, `groups.*`, `budget.quote/get`. `services.*`, `jobs.*`, job `artifacts.*`, `reviews.*` and `ratings.get` — V2. A single dispatcher calls the application API; the CLI and UI go through the same broker. Domain errors are typed, retryable ones are marked explicitly. The tool schema version is included in compatibility fixtures.

**Done for V1:** E11/E14 with an independent test MCP client; E12/E13 — V2. A durable inbox is not an autonomous LLM launch: a live host loop picks up work and calls its own runtime. Execution of third-party code lives in an external sandbox adapter, not in MCP/daemon.

### U — desktop and everyday UX · U01–U08

**Tests first:** onboarding with an own key; send/retry/offline states; `7/10 degraded` separate from delivered/read; revoking permissions; a real Tauri command denied with a wrong session; safe rendering of malicious markdown; sending and reading only under the current grant. Publishing a review — V2.

**Implementation:** screens "Profile and devices", "Chats", "Groups", "Agents and permissions", "Network resources". The "Orders" and "Reviews" screens — V2. Virtualized history, bounded uploads, search, notifications, account lock, diagnostics, backup/restore and keyboard accessibility. Raw protocol IDs and stack traces are available in diagnostics, not in the main user flow.

**Done for V1:** E01/E08/E14/E23/E24; E15 — V2. The real bridge is checked plus, by eye, screenshots of every key state; the original mockup, if one appears, is compared with the result at the same time. Checks run headless/in an isolated runner without grabbing the user's focus.

### X — integration and release · X01–X07

**Tests first:** the release runner rejects missing/skipped/failed required suites; a broken auth/quorum invariant breaks the release; missing live/platform evidence is visible; a compatible upgrade preserves history; company shutdown does not destroy transport.

**Implementation:** bring together the 22 mandatory V1 scenarios from release-scope.json, platform builds/installers, config/genesis manifests, interop, SBOM, secret-safe diagnostics, source/artifact digests and the release-report. Installer signing/notarization and monetary contract verification have separate evidence.

**Done:** all mandatory suites are green on a single source revision and fixed profiles. Documentation structure checks do not count as product tests. A prepared testnet release and clearance of real money have different statuses.

## 6. Integration milestones

Further implementation order is set by AR1–AR5 from the [architecture follow-up](../V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md). C0–C6 below are retained as historical integration criteria, not as the current development queue.

| Milestone | Verifiable outcome | What the demo actually needs |
|---|---|---|
| C0 · reproducible base | Contracts, RED tests, harness and a minimal Tauri/daemon/MCP handshake | F01/F02, basic F03/F04/F06 and I01/I02; backend-test-critic before production features |
| C1 · first end-to-end conversation | Tauri Alice → real Rust/P2P → Bob; an agent reads/sends a message via MCP; restart preserves the queue | I04/I05, N01, D01/D02, early U07/U01 and M01/M02; a separate dev genesis with a bounded fixture resource grant |
| C2 · autonomous transport | Ten replicas, both clients offline, recovery after losses, independent bootstrap/NAT | N02–N06, L01/L02 for a verifiable registry, D03–D06; the quota/accounting contract is already fixed |
| C3 · groups | Agreed Add/Remove, multiple devices, rejoin, creator offline | P01, G01–G04, I03/I06; group fanout tests per the contract, the final G05 economics close in C5 |
| C4 · agent cycle · V2 | Signed order → result → client validation → public review; MCP and UI on one core | A/Q/M/U integrate gradually along the original DAG; payment/provenance are not simulated in statuses |
| C5 · full transport resource and trust onboarding | Real postage stamps/subsidy/spend/royalty, Google/Telegram/site, funded repair, eventual autonomy from L2 | All L/P/O, final D/G/M/U integration gates in V1-scope, local EVM and testnet |
| C6 · V1 release | All 67 active cards accepted in V1-scope, 22 mandatory E2E green, installers and release evidence | X01–X06 in V1-scope, all supported OSes, independent security review; X07 — V2 |

C1–C3 are intermediate V1 demonstrations; C4 is moved to V2. The V1 release proceeds toward C5/C6 without depending on C4. A dev fixture grant is not a real postage stamp and is not allowed in a public genesis.

The historical DAG is retained for traceability. The effective V1 dependencies and integration order without V2 cards are in release-scope.json. The riskiest chains are verified before mass development of consumers: `registry → placement → finalizer`, `MLS storage → crash-safe outbox`, `ticket proof → spend persistence/handover`, `Tauri/MCP → broker`. Calendar estimates are reasonable only after C0 and these spikes; the number 84 does not equal the number of equally complex sprints.

## 7. Interfaces for the next economy

V1 creates working events and boundaries that future implementations can hook into:

| Contract | What exists now | What hooks in later |
|---|---|---|
| `AdmissionPolicy` / `ResourceQuote` | Numeric limits on bytes/TTL/fanout/repair and idempotent consumption | New resource classes and economic policies without changing E2EE |
| `SettlementAdapter` | A separate namespace and `unsupported` status when job settlement is absent | Escrow, settlement proofs, arbitration outcomes |
| `EvidenceStatement` / `Verifier` | Executor declaration, client validation, methodology and author | Independent TEE/zkML/verifier records |
| `TrustProfile` / `CredentialVerifier` | Signed scoped credentials and a visible issuer | Other trust networks and more private proofs |
| `ReviewEvent` / `RatingPolicy` | Immutable source events and a locally reproducible projection | Other scoring policies with their own policy IDs |
| `DelegationGrant` | Recipients/data scopes, budget, deadline, depth/no-subcontract | Economic performer choice and bounded subcontracting |

`JobID`, `ServiceID`, identity epochs, event hashes and the genesis domain are kept in extensions. A later financial adapter must not change the mailbox address, ciphertext format or already-signed reviews. An unknown critical extension is rejected explicitly; an unsupported settlement does not turn into `paid`.

## 8. How the goal closes

Planning is complete when the module order, independent checks, E2E, the V1/V2 boundary and the sources of decisions are agreed. **The implementation goal "all tests and E2E pass" closes only at C6**, after fulfilling the [acceptance plan](TEST_AND_E2E_PLAN.md). At the time of this document: product tests executed — **0**, product E2E executed — **0**.

A separate portability defect was found and fixed in the source package: AppleDouble sidecars `._*.md` were counted as task cards. Before the fix the existing suite gave 38/39; a regression test was added, RED shown, then a shared source filter for inventory/manifest was introduced. The result is **40/40 documentation checks**. These are not messenger tests. The result and provenance are in [VALIDATION.md](VALIDATION.md).
