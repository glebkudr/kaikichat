# Agentic Internet · V1 implementation plan

**Date:** September 5, 2026. **Plan revision:** 1.1; the product phase remains V1.  
**Purpose:** a delivery specification for parallel test-first development by agents. This is a plan, not a report of a finished product or a completed audit.  
**Materials:** 27 messages of the original archive + one new AMENDMENT-01…05 message; 52 normalized requirements; 84 atomic packets. Full cards are in `tasks/`, the index is `BACKLOG.md`, the machine graph is `backlog.json`.

## 1. What exactly must be achieved

V1 is a **standalone decentralized network of encrypted delivery for humans and agents**, shipped as a Tauri 2 desktop application with a Rust core, a window-independent daemon/CLI, a ready agent skill for the CLI, and a local MCP interface. Direct messages, groups, offline delivery, distributed storage with recovery, budgeted postage stamps, Google/Telegram/organization onboarding via limited trust profiles, basic service orders, and public reviews/rating already work in it. The next phase adds a full job economy on top of the existing verifiable events instead of rewriting the transport.

This is not "a chat window with HTTP requests to our backend", not "an MCP wrapper over a centralized inbox", and not "all complex features later". At the same time, V1 does not pass off future escrow, insurance, or proof of the exact model as an already implemented DeFi exchange.

The new AMENDMENT-01…05 amendment takes precedence over plan version 1.0: Tauri is mandatory; a single organizational issuer is acceptable; Telegram and public reviews are included in V1; complex economic agents are V2+.

The original current request takes precedence over the analyst's early proposal to limit the MVP to 1:1 messaging. After message U12, **groups are mandatory**. U22 unambiguously defines the meaning of ~$5: a transport resource, not wallet money. U20 requires a working alternative login without a user bond, not just a trait for an imaginary OAuth plugin.

### Delivery boundary

| Layer | What is actually implemented in V1 | What is not declared ready |
|---|---|---|
| Desktop/core | Tauri 2 + Rust bridge, independent Rust daemon, daemon, CLI, local storage, recovery, messages, groups | Mobile apps, calls, million-member public channels |
| P2P | Discovery, NAT/relay, E2EE, private mailbox, bounded offline storage, R=10 profile, repair | Infinite retention and availability without resources or honest nodes |
| Agents | Skill with CLI, access handle, own ID/send/poll/ack; MCP, A2A, cards, signed jobs, ExecutorDeclaration, client checks, public reviews/rating, scopes/budgets | Objective attestation of any work, built-in complex economic strategies |
| Transport economics | Real contracts and verifier implementations in local EVM and one testnet: postage stamp issue/spend, subsidy, registry/stake, payout, royalty | Unbacked "free" resources; claims about operator income from testnet tokens |
| External trust | Google, Telegram, and organizational/site profiles; 1-of-1 or k-of-n; scoped claims and separately funded grants | An external provider as root identity, human uniqueness, or a quality guarantee |
| Job economics | Basic order, executor declaration, public review, and transparent rating; contracts of future extensions | Autonomous make-or-buy, economic scoring agents, escrow/underwriting/arbitrage, guild treasury, TEE/zkML |

**Releasing V1 code and a test network is not the same as mainnet-ready.** Within the same plan there is a separate monetary release gate: audit, operator independence, fund financing, a fixed L2 security profile, and economic parameters. The required security must not be lowered for a green "V1 done" mark.

## 2. Architecture and process boundaries

```text
Tauri 2 web frontend → Rust bridge        Agent host (Hermes / another MCP host)
              |                                     |
   authenticated local IPC                 stdio MCP / optional loopback HTTP
              |                                     |
              +----------- Authorization broker ----+
                                      |
                          Rust daemon / common core
                                      |
  identity + capabilities + deterministic reducers + encrypted durable journal
             |                      |                      |
       MLS conversations      jobs / declarations / reviews       postage / credentials
             |                      |                      |
        encrypted envelopes + mailbox / blobs + replicated control indexes
                                      |
           libp2p: QUIC / TCP+Noise / Kademlia / NAT / independent relays
                                      |
            independent storage, relay, spend-finalization operators
                                      |
               one EVM L2: registry / roots / funding / claims / royalty
```

Desktop, MCP, and CLI call the same core commands and go through one authorization broker. There must be no bypass of the kind "the GUI checks the limit, MCP calls an internal unrestricted method". The root signer and user confirmation are separated from the LLM process/authority. Code submitted by an executor is not run by the daemon process; a separate sandbox/VM is the responsibility of the execution adapter and the runtime owner.

A storage node, relay, agent owner, job executor, quality assessor, and credential attestor are different roles. One binary can operate in several roles, but this does not merge their keys, data access, and budgets. An ordinary user is not obliged to share disk and computation; enabling node mode is voluntary. Independent available operators must exist: physically, storage and relay are executed by computers even when there is no mandatory company server.

### Proposed technology pillars

| Area | Baseline choice | Mandatory check |
|---|---|---|
| Desktop | Tauri 2, packaged frontend, Rust bridge → daemon | Explicit AppManifest/ACL, CSP, absence of secrets in the webview, real bridge/E2E on the platforms [S15, S17, S18] |
| I/O | Tokio, trait injections of time/RNG/network/storage | Production and simulation run the same state machines; the simulator is not another "correct" product |
| P2P | rust-libp2p | The library provides networking mechanisms, but not our durable replication protocol [S01] |
| E2EE | OpenMLS; 1:1 and groups on a common MLS device model | MLS vectors, state after a crash, concurrent membership, interop [S02, S03] |
| Control plane ordering | BFT engine behind a narrow finalizer trait; Commonware Simplex is a candidate | Do not write a new consensus from scratch without need; verify persistence, external validity, and reconfiguration [S05] |
| MCP | The official `rmcp`, the 2026-07-28 profile and the declared backward compatibility | The current discovery lifecycle and legacy initialize are verified separately, not mixed [S06, S07] |
| A2A | An external adapter per the specification version, pinned fixtures | The presence of an Agent Card JSON by itself does not mean compatibility [S08] |
| Local state | A transactional SQLite-like model with WAL/journal and encryption | WAL, temp, the search index, backups, and notification previews are also part of the leakage model |
| Chain | One EVM L2; local EVM → Base Sepolia as the proposed test profile | The mainnet L2 is chosen by a separate ADR on finality, custody/admin risks, cost, and verifiability, without a mandatory bridge [S14] |

### Tauri boundary

The webview is not a trusted holder of root keys, OAuth secrets, or an unlimited wallet. It contains only the bundled UI; external HTML/iframes get no IPC. Custom commands are listed in the AppManifest and limited by an allowlist, then Rust re-checks scopes/budget/parameters in the shared broker. A Tauri capability does not replace core authorization. Sensitive approvals are performed by a trusted Rust/OS flow with an immutable action hash, not by arbitrary LLM text [S15, S17].

The Google/Telegram/site login opens in the system browser. E2E covers the real bridge and, separately, the renderer models. The current Tauri documentation suggests WebdriverIO with the embedded provider for macOS/Linux/Windows; test plugins and debug IPC must be absent from the production installer [S18].

All versions and commit hashes are fixed after the compatibility gate. There is no requirement here to always take `latest` or a claim that any library update is safe. The specific signature scheme, MLS ciphersuite, canonical wire encoding, ZK backend, and trusted setup must pass ADR-01 before product code that uses these values.

## 3. Entities and trust boundaries

`NetworkID` belongs to a self-created cryptographic identity. `OwnerID` controls rights. `AgentID` is a stable logical agent. `ServiceID` is a specific service and its version. `RuntimeID/DeviceID` is the current process/device. Signatures distinguish `ownership_epoch`, `device_epoch`, `service_epoch`, and `protocol_domain`.

The address must not be tied to an email, OAuth client_id, or a specific wallet such that changing them breaks correspondence. Cryptographic forward secrecy concerns traffic after destruction of old key material; stored decrypted local history is protected separately and does not disappear from a single MLS Update. Google, Telegram, or an organization give a scoped assertion and do not become the key to the network. Transferring an identity is not the same as cloning MLS/spend-state from an old backup: a new device goes through safe registration and rejoin, and spent nullifiers are not forgotten.

Minimal long-term types:

```text
IdentityDocument / DelegationGrant / Revocation / RecoveryStatement
PeerRecord / EpochSnapshot / AssignmentProof
Envelope / StorageIntent / StorageReceipt / DurabilityState / MailboxIndex
GroupIntent / GroupControlEntry / GroupEpochCertificate
PostageClass / FundedCommitment / TicketProof / SpendCertificate / ServiceReceipt
TrustProfile / ProviderBinding / ExternalCredential / GrantClaim / SubsidyEntitlement
ServiceCard / JobTerms / JobEvent / ArtifactManifest / ExecutorDeclaration / CustomerValidation
AcceptedOrderReceipt / ReviewRight / ReviewEvent / ReviewReply / RatingSnapshot
EvidenceStatement / SettlementReference / VerifierResult
```

All signed types have unambiguous serialization, domain/version/expiry, a size limit, and an authority context. Amounts are integer minimal units with an asset/chain namespace. Floating point is forbidden in financial limits and the subsidy. Unknown critical extensions are rejected; an unknown optional extension may be preserved without raising authority.

## 4. Delivery: exact reliability semantics

Message lifecycle:

```text
local-queued → postage-reserved → minimum-storage-confirmed
             → target-replicated → recipient-durable → application-acknowledged
```

These are different events. "Stored on at least seven nodes" does not mean "the recipient read it". "Ten signatures" does not prove ten physical disks or independent operators. The UI shows the actual profile and degree of confirmation.

The proposed test baseline is `R_target=10`, `R_accept_min=7`. The numbers differ from the spend/consensus committee size: seven storage receipts are not a BFT quorum by default. With a smaller confirmed target, delivery may be accepted, but the status is degraded until the target is reached. The user must also see the option of a stricter mode "do not consider it stored until 10/10".

Not only ciphertext is replicated, but also the manifest, discoverability pointers, control log, and necessary repair metadata. Otherwise, after a node dies the bytes formally remain, but nobody knows how to find them or decrypt them with the correct epoch.

Repair is performed by network operators, not the sender. Lease/responsibility and replacement rules prevent an infinite copy storm. Reads for the recipient, replication reads, and audit challenges have different scopes and budgets. The promised responsibility volume is paid first: bytes, retention, number of replicas, fanout, and a set repair margin. A finite postage stamp does not fund infinite adversarial churn. When repair is impossible, the status degrades; the protocol does not invent new free resources.

Liveness guarantees have explicit prerequisites: a correct surviving copy exists, the needed honest quorum is available, there is connectivity and new nodes with paid capacity. Loss of all replicas or TTL expiry is irreversible without a separate backup. Recovery after a partition is eventual, not a promise of instant detection of any failure.

### Discovery and addressing

Several bootstrap hints, a peer cache, invitations, and alternative ways to reach a chain checkpoint rule out a single company domain. Kademlia solves routing but not the selection of trusted holders. The operator sample is verified against an authenticated registry snapshot/root/count; a peer response must not silently substitute the universe with its own Sybil subset.

A private mailbox uses a secret rendezvous and bounded rotation; a bounded lookahead/retained index allows catching up after a long offline period without enumerating infinite time slots. The first-contact endpoint has a separate limited anti-spam budget, consent, and a prekey/invitation handshake. The public executor directory is opt-in and is not a list of all users.

### Sybil and placement

Stake is a participation cost, not a certificate of honesty. Equal stake units and commit-before-beacon assignment are checked against splitting and grinding. `sqrt(stake)` or a per-key cap must not be used as a ready Sybil defense: one owner can split stake. ASN/provider/operator diversity are useful heuristics with disclosed limitations, not cryptographically proven identities. Committee sizing relies on risk simulation, not on a nice coincidence with R=10.

## 5. Groups: MLS plus separate finality

MLS provides the group E2EE model, but distributed delivery and agreement on competing membership changes require an explicit protocol policy [S02]. VAC de-MLS already contains a Rust implementation in that direction, but the associated specification is marked `raw`; the integration cannot be considered automatically proven [S04].

**V1 baseline:** OpenMLS + a decentralized finalizer of the encrypted control log. The BFT infrastructure is reused, but group data/keys remain client-side. The finalizer orders available ciphertext entries with valid sequencing capabilities; clients verify the MLS semantics themselves. An invalid proposal does not force a client to merge an incorrect state. Raw member identities must not be published to the committee without need; routing metadata and the recognizability of the group itself are part of the privacy disclosure.

One final parent/order wins. Losing Add/Remove/Update intents are re-evaluated against the accepted state; an already merged MLS epoch is not rolled back and keys are not reused. Application data does not pass through a single global sequencer or the L2, however the sender must know the accepted MLS epoch. A pending revoke is not called instantly applied: when the new epoch is unknown, sensitive sending stops according to policy.

Owner/admin/member policy, devices as MLS leaves, invitations, leaving, exclusion, concurrency, long offline periods, control log retention, and safe rejoin are mandatory. History is not given to a new member by default. When the retained control log is unavailable, old private keys must not be handed out via a snapshot for convenience.

Two routing profiles: shared encrypted group storage and separate private pointers to recipients. The latter hides part of the addressing structure but does not make the same blob and read times uncorrelatable for a global observer. Onion/mixnet is the next transport specialization with the same envelope contract, not a false V1 promise.

## 6. Postage stamps: a minimal real economy, not an `is_paid` flag

### Why an agreed spend-state is needed

A signature on a postage stamp and a `nullifier` allow detecting a replay, but **do not reconcile two simultaneous spends on different nodes**. Without a single verifiable spend-domain, both nodes may see the same nullifier for the first time and accept it.

Proposed baseline: a funded commitment gives limited tickets of a known resource class. Each ticket has a deterministic spend-domain, shard/epoch/lease, and a proof of validity. The operators of that domain maintain a BFT spent-log. The final spend certificate binds the nullifier to the hash of a specific operation and its resource envelope. A postage stamp cannot move to another shard or be "restored" after a committee change with an empty spent set.

**This is an additional distributed off-chain journal, and its complexity is not hidden.** It does not issue its own currency and does not replace the existing L2. The message payload is stored outside the consensus log, and recording a spend does not require an L2 transaction per message. Shard count/committee sizing are enabled only after an appropriate model; the baseline can start with a minimal number of independent spend domains, not with a promise of infinite scaling.

The BFT profile used is `n=3f+1`, `q=2f+1`; production `n` is chosen by a security/cost ADR. The 4/3 fixture is a single-Byzantine test, not a public-network security recommendation. External validity, partial synchrony, malicious withholding, persistence, and reconfiguration are part of the acceptance of the chosen engine [S05].

### Postage stamp privacy

The proof must hide the specific paid commitment within the available anonymity set, but prove funded membership, a valid ticket index/count, resource class, expiry, domain, and a correct nullifier. Parameters that allow issuing an extra nullifier by changing the user's salt are unacceptable. The circuit/backend/setup is chosen separately with vectors, audit gates, and a reference verifier; this plan makes no claim that a new scheme is already proven.

The funding wallet, message identity, and public ServiceID are not required to be linked. At the same time, a small anonymity set, amount/class, issuance time, and network addresses give correlations. V1 shows the limitations rather than writing "anonymous" on every ZK button.

### Payment, storage, and failures

Reserve, store, and consume/cancel are defined by one model. A StorageIntent has a short bounded duty until finality, while a long lease appears only in the intended state. Refund/replacement is a separate one-time backed right, never the deletion of an old nullifier from the spent-state. One cannot simultaneously pay the operator, refund the user, and keep the promise to store the object for free for the whole TTL.

The storage obligation covers pre-selected costs. An example class is a size bucket × retention × target replicas plus an allowed fanout/repair allowance. A changing price of new classes does not rewrite an already paid TTL. Normal retries of transport requests use the same operation id/certificate and are not billed as new work.

### Unavailable L2

Until the current proven lease expires, existing spends may continue with an available honest committee and known final roots. Creating an unproven new mint, a new subsidy entitlement, or a final monetary settlement is not allowed. After hard expiry, new admissions fail closed; reading previously stored objects remains available under their policy. Infinite autonomy together with global double-spend protection is not promised.

## 7. Subsidy and the share recipient

The anonymous branch allows ordinary purchase without an external credential and a separate optional EntryBond: the finite grant is bound to lock_id/amount/duration, and repeated withdrawal/deposit does not reuse the old entitlement. Farming backed bonds remains an economic risk modeled by cost of capital and limits, not a solved human uniqueness.

A postage stamp is the same regardless of who paid for its issuance: the user or the SubsidyVault. A free period means **a different payer**, not an unbacked promise of disk/traffic by operators.

The proposed policy shape, without the analyst's example numbers treated as approved:

```text
subsidy_fraction(t) = 1                           when t ≤ t_plateau
                     (t_end - t)/(t_end-t_plateau) when t_plateau < t < t_end
                     0                           when t ≥ t_end

allowed_grant ≤ min(remaining_fund,
                    remaining_epoch_cap,
                    remaining_credential_entitlement,
                    policy_resource_cap)
```

The implementation uses integer fixed-point arithmetic with conservative rounding. It must be separately decided whether the full limited package is issued at once or shares of one finite entitlement per epoch; a one-time ~$5 grant must not accidentally turn into infinite ~$5 every day. Package TTL, epoch boundaries, the sponsor gas budget, and source binding are tested.

The total fund is finite. When it is exhausted, the nominally smooth curve does not guarantee a smooth user price: the service must honestly show the end of the available subsidy. A sponsor adding funds is allowed only within the immutable policy; it must not allow arbitrarily extending the promotional period contrary to the genesis schedule. Anti-Sybil rules must not be changed retroactively for existing entitlements.

The grant is not withdrawn by the user in ETH/USDC, does not pay the executor's reward, and is not a transferable entitlement. Selling an account or a private key is technically possible; a colluding operator may try to turn paid fictitious traffic into income. Therefore "no cash-out button" does not prove the absence of subsidy laundering. The fund, expiry, scopes, rate limits, and economics simulation bound the damage; this is explicitly a residual risk.

For royalty, the basic option is an agreed share of gross revenue **from transport postage stamps**, accounted once at issuance/the corresponding actual sale. For example, 10% is only a discussed reference point, not the final figure. The operator reserve and the organization's share, including subsidized purchases, must be factored into the price in advance. The share is not silently applied to the full cost of someone else's agent work. The specific base and accrual moment are fixed in L04 before code, and L05 does not deduct the same share a second time.

The contracts have no owner/proxy upgrade/pause/blacklist/arbitrary-call. The recipient can only withdraw the accrued share. Its `revert` does not block other operations: pull-payment/isolated accumulation. A fork/new voluntary deployment is the way to change the rules. The immutability of our contracts does not cancel the governance/upgrades of the chosen L2; that is a separate public dependency [S14].

## 8. External trust: Google, Telegram, and organizational issuers

V1 implements `TrustProfile`/`ExternalCredential`, which separately define the upstream provider, attesting issuer, namespace subject, scopes/claims, signer set/k, TTL/revocation, and grant policy. A limited set of attestors **and one explicitly trusted organization/site issuer** are acceptable. A government organization is a possible issuer with a real available integration, not a promise of an already working government IdP.

The Google native flow is preserved. Telegram goes through the system browser and a website gateway: the official server-side OAuth exchange requires a client secret, which does not fit in the desktop. The one-time handoff and possession of the original network key bind the result to the NetworkID [S09, S10, S16]. A reference organizational OIDC/site issuer is also part of the delivery.

The provider does not own the address, does not read correspondence, and cannot delete reviews. A shut-down single issuer can stop **new attestation of its voluntary profile**, but not the independent transport or the operation of existing keys. There is no mandatory independent quorum for a profile the user consciously accepts as 1-of-1.

Login, credential, and grant are three different states. A funded campaign explicitly accepts the profile/version and bounds the resource; a successful Telegram/Google/site login by itself does not create money. One provably common subject does not give a repeated entitlement via wallets/devices/attestors; pairwise identifiers and cross-provider uniqueness require a separate policy [S19]. It is not assumed that Google and Telegram automatically identify the same person.

The full specification of flows, the gateway secret boundary, dedup, and live gates is in `AUTH_AND_ATTESTATION_V1.md`. More private ZK-OIDC/OPRF adapters remain V2+. Unlinking any provider preserves the NetworkID and does not return a used grant.

## 9. Basic service protocol and public reputation

V1 conveys RFQ/bid/accepted terms/progress/result/cancel/timeout. The executor signs `ExecutorDeclaration`; the client performs its own checks and may leave a review. The signature attests the author of the statement, not the actual internal model or objective quality. Strict independent provenance without a really available verifier is rejected, not simulated.

### An order creates the right to review

The two-sided `AcceptedOrderReceipt` records the minimal public header, service/owner epochs, and a salted private terms commitment. After receiving both signatures, the client is entitled to publish a review **without a result, payment, or further consent of the executor**. An unaccepted RFQ grants no right. A provider cannot disable a negative review or erase it by closing the service.

The public receipt contains no private input/output and appears on the public network only when the review is published. Before acceptance, both sides see the minimal disclosure. The client is explained that publishing a review discloses the link between their pseudonym and the service; E2EE of the order does not mean an anonymous review.

### Reviews and rating

One effective review per order/customer; a 1–5 scale, bounded text, amendments/withdrawal as new signed events, a separate executor reply. Replication of text, receipt, and the index uses paid storage/repair/TTL. The executor directory is not a mandatory intermediary.

A versioned reducer computes count/sum/histogram and mean over the known corpus with a dataset hash, sources, epochs, freshness, and an honest partial status. Retries, copies, and replies do not give extra votes. An identical set of events gives an identical rating; global completeness and immunity to manipulation by colluding accounts are not promised. Google/Telegram trust is not an automatic review weight.

### The economic agent boundary

V1 provides safe budgets, grants, no_subcontract/data/depth restrictions, and contracts of future extensions. **Built-in make-or-buy and complex economic agents are V2+.** An external agent can already choose a service and order it via MCP, but the transport release does not depend on building its own economic decision engine.

Escrow, insurance, arbitration/judges, economic assessor markets, collective collateral, and independent proof of execution remain V2+. Basic reviews/rating are not deferred along with them. Full semantics and tests are in `PUBLIC_REVIEWS_PROTOCOL_V1.md`; EIP-8004 remains a possible future adapter, not an on-chain review requirement in V1 [S13].

## 10. CLI, skill, MCP API, and operation of disabled agents

### The "client + skill with CLI" delivery scenario

The user clarification of September 8, 2026 (`FOLLOWUP-2026-09-08`) is part of V1 and refines R02/R17. Along with the installable Tauri client, a working CLI and a ready agent skill (`SKILL.md`, command descriptions, connection and incoming-processing examples) are delivered. An agent able to call the CLI must go through the whole scenario with this skill; MCP support on its host is not a condition for basic messaging.

Through a trusted local flow, the user issues the agent an **access handle to the chat**: a named connection context with the local daemon address and a reference to a limited session/grant. The skill explains how to pass this context to the CLI. The authority defines the available chats/recipients, actions, duration, and limits; the daemon checks them on every call. The agent does not need to set up libp2p or obtain the profile's root key.

Mandatory operations via the CLI and the common application API:

| Operation | Observable outcome |
|---|---|
| `identity.get` | Own public `NetworkID`, available rights, and separately `AgentID`/`RuntimeID` if assigned; the internal chat ID does not replace the network address |
| `messages.send(to: NetworkID, …)` | Sending to the public ID of an allowed recipient, including a first conversation without a known `ConversationID`; the daemon resolves the address and creates the conversation under the current contact policy |
| `delivery.get` | `MessageID`/`operation_id` and a verifiable status; acceptance into the local queue differs from delivery to the recipient |
| `inbox.poll`, `inbox.ack` | Receiving incoming messages, cursor/lease, and acknowledgment of processed ones; a retry after a failure does not lose an unacknowledged message |

The CLI has versioned JSON output, clear exit codes, and a separate stderr for diagnostics. The CLI and MCP use one broker and the same core operations. Durable polling remains the baseline of the already accepted plan; a specific hook/subscription for a running host may be chosen in M02/M06. A hook does not replace the queue and recovery after disconnection, and starting/waking the model is done by its host.

Behind NAT, the Rust daemon works over libp2p: reachability discovery via AutoNAT, a direct connection/hole punching attempt via DCUtR, and fallback through independent Circuit Relay v2. When a direct path is impossible, an encrypted route through a relay remains. The same path serves messages from the UI, CLI, and MCP.

**Mandatory acceptance M01/M02/M06/U06/X05, together with N04:** clean install of the client and the skill → issue a connection context → the agent, using only the supplied CLI, learns its ID → writes to another ID → receives a reply and acknowledges it → restarts and reads the accumulated inbox. The scenario runs between two nodes behind NAT, including relay-only and replacement of a failed relay; revoking the grant blocks further requests. Detailed steps are E11 together with E03 in the [E2E plan](../agentic_internet_v1_execution_plan/TEST_AND_E2E_PLAN.md). These executable checks are written first, then the implementation of the corresponding modules.

### MCP method families

Proposed tool families, with specific JSON Schemas and tests in M01–M07:

| Family | Baseline methods | Mandatory boundary |
|---|---|---|
| Identity | `identity.get` | Own NetworkID and the allowed actor context; no secret export |
| Messaging | `inbox.poll`, `inbox.ack`, `messages.send`, `delivery.get` | Cursor/lease/idempotency, only allowed inboxes/recipients |
| Groups | `groups.create`, `groups.invite`, `groups.send`, `groups.members` | Roles, MLS epoch, a separate management grant |
| Discovery | `services.publish`, `services.search`, `services.get` | Opt-in publicity, signed cards, bounded responses |
| Jobs | `jobs.rfq`, `jobs.bid`, `jobs.accept`, `jobs.progress`, `jobs.result`, `jobs.cancel`, `jobs.validate` | Signed terms, external approval policy, immutable action hash |
| Artifacts | `artifacts.put`, `artifacts.get` | Job/data capability, limits, no auto-execution |
| Trust/budget | `evidence.query`, `budget.get`, `budget.quote` | Declarations/credentials/customer checks are separate; no unrestricted payment |
| Reviews | `reviews.eligibility`, `reviews.publish`, `reviews.amend`, `reviews.withdraw`, `reviews.reply`, `reviews.list`, `reviews.get`, `ratings.get` | Receipt-based right; a separate publish grant, privacy preview, and partial dataset |

When the model is off, it does no computation. The daemon keeps the job; the running host loop polls/subscribes to the inbox and wakes an available agent runtime. An MCP subscription is a delivery mechanism for a working client, not a separate scheduler of the LLM's life. After a crash, lease, cursor, and status are preserved; a repeated result does not cause a repeated charge/action.

stdio is used by default. Optional Streamable HTTP is limited to loopback, passes auth/Origin/audience/session checks, and uses the same broker. The agent-facing MCP does not export the root signer, shell, arbitrary file access, or an unlimited wallet budget. An external card/message/artifact is always untrusted data, even when signed by a real user. The fact that the source is authenticated does not make the instructions in its text safe [S06, S07].

## 11. Test-first organization

Before the main implementation of a packet, the public behavior, test oracle, wire vectors, allowed states, and prohibitions are fixed. RED is demonstrated first. Then a separate implementation agent brings the contract to GREEN. Review of the contract itself is not assigned solely to the implementation author: the same misconception in code and tests must not pass acceptance.

Different levels of evidence are needed: unit/state reducer, property-based, protocol vectors/reference implementation, model checking, deterministic fault simulation, fuzzing of parsers/circuit inputs, mutation of authority/amount/epoch checks, process-crash integration, real NAT/network tests, and GUI/MCP black-box flows. Line coverage is useful but does not replace invariant checking.

Mandatory safety invariants: nobody reads someone else's plaintext; untrusted input does not raise authority; spending does not exceed the backed resource; one nullifier has no two final spends; membership has no two final states in the accepted model; a message replay does not create a repeated application operation without an explicit idempotency policy. Liveness is verified only together with the prerequisites that provide it.

Fake chains/providers/proofs are needed for unit/CI but are isolated by a separate genesis and are not allowed in real verifier dispatch. No mock must silently create a paid, funded, verified, or mainnet-ready UI status. All found failing traces are added to the permanent regression corpus.

The full order, roles, and atomicity criteria are in `AGENT_WORK_ORDER.md`. There are no judgments like "this takes long, so we drop groups/Google/Telegram/reviews" in the plan. The speed of agent coding is used for parallel implementations, adversarial corpora, reference models, and smaller integration batches, not for abandoning review trust boundaries.

## 12. Decomposition and integration order

All 84 deliveries are listed in `BACKLOG.md`. A04 is V1 contract-only: the future economic engine is not among the integration prerequisites. In `tasks/<ID>.md`, each has an outcome, an ownership scope, dependencies, tests-first, a negative and a fault test, a black-box demo, non-goals, and a Definition of Done. `backlog.json` is suitable for handing tasks to a scheduler.

| Block | Notable deliveries |
|---|---|
| F | Traceability validator; canonical wire; simulator; crash-safe journal; threat/economic models; independent CI |
| I | Root identity; grants; revoke/recovery; first contact; MLS devices; backup/migration |
| N | P2P streams; independent bootstrap; private routing; NAT/relay; verified placement; operator mode |
| D | Admission; durable delivery; ten replicas; autonomous repair; retained indexes; artifacts/TTL |
| G | Roles/invites; distributed control order; concurrent epochs; offline rejoin; privacy/fanout; agent rooms |
| L | Issuer; registry/epochs; subsidy; nonprivileged royalty; operator claims; finality adapter |
| P | BFT finalizer; private tickets; atomic spend; handover/outage; resource accounting; economic adversary suite |
| O · 8 | Google; shared credentials; single/threshold issuers; campaign dedup; unlink; funded onboarding; site issuer; Telegram |
| A · 6 | Cards; signed terms/receipt; async jobs; future delegation contract; executor/customer statements; A2A |
| Q · 6 | Review right; signed events/replies; public storage/index; rating reducer; privacy/filters; adversarial suite |
| M · 7 | Safe MCP; messaging; jobs; discovery/evidence/rating; runtime leases; integrations; review tools |
| U · 8 | Tauri chats; multi-provider onboarding; groups; agent console; everyday UX; installers; IPC security; reviews UX |
| X · 7 | Transport; groups; transport economy; multi-provider auth; jobs; release gate; public reviews |

The first fixes are F01/F02, then F03/F04/F05 and working identity/network seams. The riskiest causal path: registry/snapshot → finalizer → private postage + spend safety → replication/accounting → group finalization → end-to-end release. UI view models, OAuth fixtures, job reducers, and interoperability tests can be prepared earlier on approved contracts. `DAG.md` shows automatically computed readiness fronts; these are **not calendar sprints**.

A graph front reflects readiness for integration GREEN. There is no need to wait for all dependency crates to be implemented before another agent writes independent red contract tests and a view model: F02 exists for that. At the same time, final acceptance of a consumer cannot be counted on mocks alone.

## 13. Full release criteria

The seven X packets come together into a mandatory demonstration:

**Transport:** a new client joins without company resources; two clients behind NAT exchange messages; sender and recipient shut down; some holders disappear/lie; surviving nodes restore R=10; the recipient later gets the available history.

**Groups:** several devices per member, concurrent Add/Remove, partition, loss of a control-log replica, and return of an offline client do not break the agreed roster and the secrecy of subsequent epochs. The proposed test profile is 100 members, up to three devices each. This is a test target, not a measured limit of the finished implementation.

**Economics:** an attacker spends one postage stamp simultaneously on different nodes, retries after crash/reconfiguration, and provokes a reorg. The spend is not doubled. An L2 outage passes until hard lease expiry; afterwards new admissions stop safely. The share recipient is compromised and cannot control the network.

**External login/subsidy:** Google and Telegram pass a real consented smoke test; the reference site issuer gets a full integration. The single issuer is honestly labeled; shutting down the gateway does not stop the network. Dedup verifies the proven subject namespace, stacking, and caps; the schedule works without an upgrade. An unnamed government provider is not considered ready.

**Agents:** from the "client + skill with CLI" delivery, the agent uses the issued handle to learn its ID, writes to another ID, and gets a reply behind NAT, including after a restart. Then an MCP customer and executor go through OCR/coding/inference jobs with ExecutorDeclaration and client checks; there is no fake independent attestation. Runtime crash and prompt injection do not bypass the broker. A built-in economic engine is not required.

**Reviews:** after an accepted order, the executor shuts down without a result/payment. The client publishes a negative review; an independent client finds it; amendment/reply/withdrawal do not give extra votes. A malicious index, different corpora, and TTL give honest partial/freshness statuses, not a hidden zero reviews.

**Operations:** installable builds, a reproducible devnet, secrets-safe diagnostics, backward compatibility, SBOM, source/build manifest, and an independent security review. Stake/holder/attestor concentration, source of finality, remaining anonymity leakage, and funding are mandatory in the release report.

## 14. What is deliberately left to the next phase

Autonomous make-or-buy/economic strategy agents, multi-level economic orchestration of subcontracting, native mainnet job escrow, multilateral settlements/subcontractor payments, independent arbitrators, underwriting/insurance, guild collateral and treasury, economic reputation/scoring marketplaces, TEE verifiers and zkML, private ZK-OIDC, large public channels/erasure coding, strong metadata privacy/mixnet, and mobile clients. Each module must use the types/evidence/capabilities defined today, and its absence is not hidden in the UI.

**Not deferred:** Tauri, groups, Google/Telegram/site attestation, the backed grant path under the accepted campaign, spending of transport postage stamps, autonomous repair, safe MCP, basic orders, ExecutorDeclaration, public reviews, and a simple rating. This is the minimal coherent V1 for exactly the discussed product.

## 15. Limits of confidence of this document

The plan structure has been verified: task/requirement links, presence of negative/fault tests, acyclicity, and coverage of the original wishes by the plan. Product code, the test network, a cryptographic audit, operator profitability, and actual decentralization are **not verified and not claimed ready**. The private postage stamp scheme, group finality, the source of JWK roots, the Tauri boundary, receipt/review semantics, and the L2 security profile have mandatory gates in `DECISIONS_AND_VERIFY_GATES.md`; these are concrete verifiable engineering work packages, not excuses to drop requirements.

Sources and differences from the analyst's early proposals are in `SOURCE_MAP.md` and `DECISIONS_AND_VERIFY_GATES.md`.
