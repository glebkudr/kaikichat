# Agentic Internet · Full V1 plan · revision 1.1

September 5, 2026. 84 planned packets, 52 requirements. Tauri, Google/Telegram/organization attestation and public reviews are in V1; complex economic agents are V2+. This is a plan, not a product implementation/audit.


---

# File: CHANGELOG_V1_1.md

# Plan changes 1.0 → 1.1

**Date:** September 5, 2026. This is a revision of the V1 document, not a move of the product to V2.

| Area | Was | Became |
|---|---|---|
| UI | Proposed egui/eframe | Approved Tauri 2 + Rust bridge + independent daemon; U07 security and U08 reviews UX |
| Google attestors | Mandatory independent threshold tied to the committee infrastructure | Voluntary TrustProfile: organizational 1-of-1 or k-of-n; no transport authority appears |
| Trusted networks | Google-specific credential | Provider-neutral credential; Google, Telegram, organizational/site OIDC; no specific state IdP is invented |
| Telegram | Not included | O08 browser/gateway login, holder-bound handoff, secrets outside the desktop |
| Result | Evidence/provenance and client-side validation, emphasis on future verifiers | Explicit ExecutorDeclaration + CustomerValidation, without fake independent attestation |
| Reputation | Mostly future scoring adapters | V1 public review for an accepted order, replies/amendments, distributed storage, and a basic rating |
| Right to review | Not defined | Two-sided AcceptedOrderReceipt; no dependence on result/payment/subsequent consent of the executor |
| Economic agents | Make-or-buy/subcontracting engine in V1 | A04 contract-only; autonomous economic strategies and complex calculations are V2+ |
| Checks | 72 tasks / 44 requirements | 84 tasks / 52 requirements, updated DAG/cards/release gates; previous IDs preserved |

## New packets

O07 — organizational issuer/site gateway; O08 — Telegram. Q01–Q06 — receipt/right, events/replies, distributed publication, rating reducer, privacy/filter policy, adversarial tests. M07 — MCP review tools. U07 — Tauri security boundary; U08 — review/rating UI. X07 — independent end-to-end acceptance of public reviews.

## What is preserved

Rust headless core, E2EE, groups, ten replicas and repair without clients, NAT/relay, transport postage stamps and their backing, single-spend semantics, subsidy schedule, royalty without authority, one existing L2, MCP/A2A, and mandatory test-first. Transport economics was not moved to V2 along with the economic agents.

## Priority and migration

SOURCE_AMENDMENT_2026_09_05.md records the new requirements. All previous R01–R44 and task IDs are preserved; R23 explicitly changed phase. The new R45–R52 extend the matrix. The old full-plan is not required to execute the new revision. One consistent current version is stored in the ZIP; the provenance of the previous archive is recorded by SHA-256.

## Checks of this revision

Before changing the data/validator, 17 revision-contract tests were added; they produced RED on the 1.0 plan. Then the data and validator guardrails were updated. The final commands and logs are in README and *_GREEN.txt. The checks concern the plan only; no product messenger tests were run.


---

# File: V1_IMPLEMENTATION_PLAN_RU.md

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


---

# File: AUTH_AND_ATTESTATION_V1.md

# V1 · Trusted external networks and authorization

## 1. One model, different sources of trust

One's own cryptographic identity remains the root of ownership. An external login confirms a limited statement bound to a NetworkID. Google, Telegram, a site account, and an organization certificate are different claims with different grounds, not the same boolean trusted flag.

```text
NetworkID + signed one-time challenge
    → browser authentication / the organization verifies the ground
    → provider adapter / optional website gateway
    → a single issuer or k-of-n per the selected TrustProfile
    → ExternalCredential
    → local acceptance of the claim
    → separately: eligibility in a paid campaign
    → regular transport postage stamps
```

### Types

`TrustProfile`: profile_id/version; provider_kind; upstream_issuer; subject_namespace and its stability; accepted client IDs/audiences; permitted algorithms/keys; allowed claims and assurance labels; attestor signer set/k; TTL/freshness; revocation/key-rotation policy; rules for binding to the network key; disclosures and endpoint configuration. There are no secrets in the published profile.

`ExternalCredential`: profile_id/version; attesting_issuer; upstream_provider; subject_reference/commitment with explicit linkability; holder NetworkID; issued_at/expires_at; scope; the proven ground; credential_id; signature/signature set. The credential carries no root authority and grants no right to unlimited postage stamps.

`ProviderBinding`: a local link of one's own identity to an external account. Binding/unbinding and network recovery are different commands. `GrantPolicy` separately accepts the profile/version and sets campaign/provider/network caps, the fund, and the sponsor budget.

## 2. A single issuer is acceptable

The initial issuer can be the project site, another trusted organization, or a government organization. The client explicitly chooses the profile. For 1-of-1, it is honestly shown who is able to issue a false statement/deny login. For k-of-n, distinct signer IDs, a common policy version, and the threshold are verified. The number of processes does not prove the independence of organizations.

The set of login attestors is not appointed automatically by the transport's stake/BFT committee. Issuer key rotation follows the accepted profile version; the user does not get a new root of trust from an arbitrary network message. Issuer compromise is limited to its claims and the allocated grant budget. It does not grant access to E2EE, the authority to revoke a NetworkID, deletion of public reviews, global mint, or transport upgrade.

When the single issuer is down, **new attestation of its profile is unavailable**. Chat, groups, accepted orders, reviews, one's own keys, and already backed postage stamps keep working within the usual limits of the protocol. This is an acceptable centralization of a voluntary entry point, not a hidden dependency of the transport.

## 3. Google

The existing Google native flow is preserved: system browser, PKCE/state, verification of the signed ID token and cacheable JWKS. The Google `sub`, not the email, is used as the provider subject [S09, S10]. The credential can be signed by an organizational issuer or a selected set; each profile discloses who will see the original confirmation. A Gmail-specific policy requests only the necessary additional claims and does not assume a person's age/uniqueness.

## 4. Telegram

The technical baseline is the current Telegram Login/OIDC, not the old iframe widget and not Mini App initData. The official documentation describes the Authorization Code Flow with PKCE, callback registration via BotFather, server-side exchange with a client secret, and verification of the signed ID token [S16].

The proposed desktop chain:

```text
Tauri → the Rust daemon creates the owner challenge
    → system browser to the selected website gateway
    → Telegram login
    → registered HTTPS callback gateway
    → server-side token exchange and verification of provider assertions
    → credential for the original NetworkID
    → one-time handoff handle / authenticated polling
    → the daemon proves possession and receives the credential
```

The client secret and bot token belong to the gateway; they are absent from the Rust installer, frontend bundle, deep link, or MCP. The handoff handle itself is not a bearer right to the identity: obtaining the result requires a signature of the original key holder, binding to a session/request digest, and a limited validity period. The gateway maintains TLS, one-time use, and crash-safe state; interception of the callback/handle does not allow binding the account to another NetworkID.

`state` and PKCE protect the agreed flow. Where the provider genuinely supports a signed nonce, it is verified; the absence of a nonce in a specific flow must not be hidden behind an invented provider-signed binding. For the basic site-mediated chain, the binding to the owner is an explicit statement of the trusted gateway based on its authenticated session. This is part of the disclosed trust model.

A minimal profile is requested; the phone number, display profile, and the right to write via the bot are not needed by default. Username/phone are not used for deduplication. The live gate verifies the real supported claims, algorithms, callback configuration, and identifier stability for the selected bot/client settings. The scheme does not automatically assume that `sub` is the same across any applications.

## 5. Organization and site OAuth

V1 delivers a reference website issuer and a configurable OIDC adapter, not just a trait. For plain OAuth without OIDC, a provider-specific verified way to obtain the identity is needed; an access token by itself does not authenticate the user [S19]. An organization can issue its own credential after its own procedure, but must state what exactly it verified.

Connecting a government issuer will require its real interface, application registration, allowed claims, and access. Until such an issuer is named, the example is implemented as a test/reference identity provider with explicit labeling. The label "government organization" is not a cryptographic or legal proof of its participation.

Discovery/JWKS endpoints are selected from the accepted profile: SSRF, arbitrary URLs from the incoming token, algorithm confusion, and unbounded downloads are forbidden. Credentials are verified by the transport without a provider call for every message.

## 6. Deduplication is not human uniqueness

The key of a single campaign entitlement is based on campaign + verified provider subject namespace + canonical subject. A wallet, device, another attestor's signature, and a new version of the same profile do not create a new entitlement. Aud is verified as the token recipient but is not used to artificially multiply an already provably common subject.

OIDC can have public and pairwise subject identifiers [S19]. Therefore cross-application deduplication is allowed only with a confirmed common subject scope, a single campaign client/sector, or an accepted mapping. Different Google/Telegram identities are not merged by email/username/phone; there is no proof that "this is one person".

For a single campaign, it is proposed not to sum grants of linked providers on one NetworkID. Several independent accounts/NetworkIDs can still claim different permitted entitlements; damage is limited by the pre-established fund and caps. Login of all three types is part of V1; the size and availability of a specific grant depend on a separately funded policy. Neither a successful login nor a political/organizational name grants money automatically.

## 7. Mandatory tests

Positive fixtures for Google/Telegram/site; negative signature/issuer/audience/expiry/nonce/session/PKCE; replay across owner/network/profile/campaign; key rotation and gateway loss; one subject across wallets/attestors; unknown pairwise mapping; cross-provider stacking; absence of secrets in bundles/URI/logs; unlink without losing the address; provider offline without stopping the independent network.

Product tests have not been executed by this document yet. Cards O01–O08, U02, and X04 define their acceptance; only a real smoke test can confirm a specific live OAuth configuration.


---

# File: PUBLIC_REVIEWS_PROTOCOL_V1.md

# V1 · Basic protocol for orders, public reviews, and rating

## 1. What the protocol attests

V1 attests the signing keys, their authority, and the link of events to a specific order. The executor signs `ExecutorDeclaration`: "under such terms I got such a result". The customer signs `CustomerValidation` and/or a public `ReviewEvent`: their checks and impression. These are different statements.

Neither the executor's signature, nor successful client tests, nor five stars prove which model/hardware the executor used inside. Independent TEE/zkML/verifier/arbitration markets are V2+. Machine-verifiable result criteria are available already, but this is precisely a result check with a named author/methodology.

## 2. The right to review arises on order acceptance

```text
ServiceCard → RFQ → Bid / terms → two-sided Accept
                                      │
                           AcceptedOrderReceipt
                                      │
                         ReviewRight at the customer
                                      │
        Result / Timeout / Cancel / No response — any of these branches
                                      │
                      Review → Amendment / Withdrawal
                                      │
                          Reply + RatingSnapshot
```

A basic standard order is always reviewable after acceptance. The executor cannot disable this with a `no_negative_reviews`/`private_reputation` flag, demand payment, first deliver a result, or approve the review one more time. The client gets the right as soon as both signatures exist; the review can be about the process, non-performance, or the final result. An early rating is not called proof of completed work.

An unaccepted RFQ does not give a verified-order review. The justification of the right is the two-sided receipt, not the client's one-sided claim "I ordered". The client stores the receipt reliably until the UI declares the order accepted. This does not require escrow, proof of payment, or a complex economic agent.

## 3. Receipt and bounded disclosure

The minimal publicly verifiable `AcceptedOrderReceipt` contains:

```text
protocol_domain, protocol_version, order_id
customer_id, provider_id, service_id
customer/provider ownership_epoch, service_epoch
salted_private_terms_commitment
public_review_policy_version
acceptance_context and proofs of authority to accept
customer_signature, provider_signature
```

The specific timestamp/checkpoint and signature suites are fixed by F02. The private terms commitment uses a sufficient random salt; a hash of a short guessable assignment without a salt is not considered confidentiality. The order ID has sufficient randomness and domain separation. Repeated acceptance of the same receipt is the same deal, not a new right.

The prompt, inputs, full price/terms, correspondence, and results remain private unless the client separately decided to disclose them. Both sides sign the minimal header and consent to its publication in the review in advance. The receipt is not broadcast publicly automatically for every order: it is published by the author together with the review. This preserves private messaging, but **the public review itself discloses the link between the customer's pseudonym and the service**.

In V1, a standard order has no hidden provider veto for the sake of privacy. The user sees the disclosure before acceptance; they may decline such an order or not publish a review. Any future fully anonymous proof of an order is a separate profile, not a promise of the current scheme.

## 4. Events and rights

`ReviewRight` is logically derived from a valid receipt and does not require a separate issuer. It is bound to the customer identity/authority, not to the current OAuth account or wallet. Delegating publication to an agent runtime is a separate narrow capability; permission to read an order does not mean permission for public text.

`ReviewEvent` contains an order/receipt reference, author, service+owner epochs, version, rating 1–5, bounded UTF-8 text, declared outcome, revision, prev_event, network domain, and signature. The declared outcome is the author's statement. If needed, agreed job-state evidence is attached separately, without passing off an unconfirmed timeout as an objective verdict.

One effective review per `(order_id, customer_id)`. `amend` is the next signed version; `withdraw` is a signed removal of the current rating; `reply` is a separate message from the executor, not a second vote. The executor does not get the right to change/remove the client's rating. History is kept under the paid retention policy; withdrawal does not promise to physically erase replicas or readers' memory.

If one customer key signed incompatible edit branches, all valid branches are kept as evidence. The normative draft reducer: root revision=0; the parent belongs to the same review and revision=parent+1; among complete valid branches, the head with the maximum `(revision, canonical_event_id)` in a fixed byte order is selected. Missing parents are pending, not a new independent vote. The specific revision order/limits are fixed by vectors before code. The author can change their own opinion but cannot get extra votes.

The right check takes into account accepted identity/revocation epochs and historical proofs, not only the current display name. A change of service owner does not erase old reviews; by default, the rating of the new owner/service epoch is shown separately with the available history.

## 5. Publication without the executor's server

A public review bundle consists of the receipt, the minimal history for verification, and the signed event. It is stored openly as deliberately public data; private correspondence remains E2EE. Sizes, signatures, and proof eligibility are checked before expensive indexing.

The publisher pays for transport/storage within the selected class; a pre-limited sponsor is possible. Storage/repair/TTL use the existing D/P/N primitives, not a free eternal obligation of the network. The receipt, text, amendments, and discoverability pointers are replicated; expiry and availability are shown separately.

The index is addressed by service/owner epoch and served by several independent peer paths. It does not have to belong to the executor. The client verifies signatures and merges answers. Censorship of one index does not prevent finding the review if an available honest path exists; absolute availability when all copies/TTL are gone is not promised.

Publicity does not mean a globally complete journal. When completeness is unknown, the API returns `partial/unknown_completeness`, the list of queried sources/cursors, freshness, and the corpus hash. Two clients with different observations may see different numbers; an identical corpus always gives an identical result.

## 6. Basic rating

This is a reproducible representation of reviews, not a global consensus about true quality. The baseline policy does not require a Google/Telegram credential, does not weight votes by money, and gives no automatic weight by trust level.

For a specific `ServiceID + service_epoch + provider ownership_epoch`:

```text
eligible = valid receipts + authorship + permitted rights
current = one effective review version per order/customer
active = current without withdrawal, with a permitted score
count = number of active
sum = sum of integer scores
histogram = number of scores 1, 2, 3, 4, 5
mean = sum / count if count > 0; otherwise no_rating
```

The answer includes rating_policy_version, dataset_hash, sources/cursors, freshness/retention, completeness, count, and distinct customer count. Display rounding is fixed separately from the exact sum/count. A reply does not change the rating; an amendment replaces the old contribution; repeated delivery/copies do not increase count.

A known self-review of an OwnerID is excluded from the basic rating. Different colluding identities can sign a fictitious order. One right per order does not solve Sybil, bought reviews, wash-trading, or identity change. V1 prevents forgery of rights, double voting, and hidden sample substitution, but does not claim manipulation is impossible. Hardened trust/economic policies are future derivatives that do not change the raw signed evidence.

## 7. Moderation, security, and UX

Text is untrusted data. HTML/scripts are not executed; external media are not loaded automatically. The publication preview shows the fields being disclosed and binds the confirmation to a specific action hash. Neither a task result nor prompt injection can grant itself the `review.publish` scope.

Local mute/filter/report is allowed, but it does not erase the event in the network and does not disguise a filtered aggregate as the basic result over the same dataset. The operator's storage/content policy is disclosed separately. In V1 there is no global rating administrator who "fixes the truth".

The service screen distinguishes: declared characteristics; confirmed account control; the number of confirmed orders for which reviews were found; the reviews and their sampling; the executor's statements; client checks; the absence of independent provenance. An unavailable index is not "0 reviews".

## 8. MCP and acceptance

`reviews.eligibility`, `reviews.publish`, `reviews.amend`, `reviews.withdraw`, `reviews.reply`, `reviews.list`, `reviews.get`, `ratings.get`. The tools use the same core as Tauri. Reading and publishing are different grants; a separate trusted flow is applied for human confirmation.

Mandatory demonstration: the parties accept an order, the executor disappears without a result/payment, the client publishes a negative review from their receipt, an independent client finds it and computes the rating. Then redelivery, two concurrent edits, a provider reply, a withdrawal, a vanished index, a key change, and a partial corpus do not break rights/deduplication/honesty of statuses.

Cards Q01–Q06, M07, U08, and X07 are V1 implementation. They do not depend on the presence of the V2 economic engine. This document does not declare product code and tests ready.


---

# File: BACKLOG.md

# Backlog V1 · revision 1.1

84 atomic packets; 52 requirements. All tasks are planned. A04 is contract-only for V1, not a mandatory V2 engine implementation. In the table, dependencies mean readiness for integration acceptance, not a ban on writing independent tests early.

| ID | Observable outcome | Depends on | Scope |
|---|---|---|---|
| [F01](tasks/F01.md) · Verifiable specification of wishes and prohibition of requirement loss | Traceability distinguishes the original archive and AMENDMENT-01…05; blocks the loss of Tauri, Google/Telegram/organization trust, public reviews or the return of a complex economic engine into mandatory V1. | — | V1-implementation |
| [F02](tasks/F02.md) · Canonical wire protocol and version compatibility | One parser/encoder with size limits, signature domains, integer amounts and stable golden vectors for identity, envelope, job, evidence, credential and postage. Added TrustProfile, ProviderBinding, ExecutorDeclaration, AcceptedOrderReceipt, ReviewRight, ReviewEvent, ReviewReply and RatingSnapshot; independent vectors cover their signatures and versions. | F01 | V1-implementation |
| [F03](tasks/F03.md) · Deterministic simulator of network, time and failures | The same core state machines run with seeded RNG, a virtual clock, and partition, reorder, loss, Byzantine peer and crash/restart models. | F02 | V1-implementation |
| [F04](tasks/F04.md) · Transactional state journal and reliable local queue | Durable outbox/inbox, events, cursors and epochs are saved atomically; side effects are executed through a transactional outbox. | F02, F03 | V1-implementation |
| [F05](tasks/F05.md) · Executable model of threats, committees and resource economics | The models verify quorum intersection, the grinding/Sybil splitting threat, the subsidy cap and cost accounting before the economic protocols are implemented. | F01, F02, F03 | V1-implementation |
| [F06](tasks/F06.md) · Independent test oracles and delivery pipeline | CI runs contract, property, model, fuzz, mutation and platform smoke tests; separates security-critical gates and stores RED/GREEN evidence. | F01, F02, F03, F04 | V1-implementation |
| [I01](tasks/I01.md) · Local owner, permanent address and key storage | Root identity and NetworkID creation is independent of Google/chain; keys are protected by the system keychain and an encrypted store. | F02, F04 | V1-implementation |
| [I02](tasks/I02.md) · Delegation to devices, agents and runtime with limits | The owner signs limited grants to agent/device/runtime: methods, inbox, recipients, budgets, TTL, subcontracting depth and epochs. | I01, F02 | V1-implementation |
| [I03](tasks/I03.md) · Revocation, epoch change, new owner and recovery | A signed history of ownership/device/service epochs makes owner change and revocation verifiable; recovery requires a pre-selected recovery right. | I02, F04 | V1-implementation |
| [I04](tasks/I04.md) · Contacts, invitations and safe first message | One-time invite/prekey packets and closed contacts allow establishing a chat; the public contact endpoint is quota-limited and does not publish the contact list. | I02, F02 | V1-implementation |
| [I05](tasks/I05.md) · MLS client for personal chat and multiple devices | The MLS profile implements 1:1 as a device group; KeyPackage/Welcome/commit/application messages have a strict lifecycle and binding to identity. | I02, I04, F06 | V1-implementation |
| [I06](tasks/I06.md) · Encrypted backup, migration and rollback protection | Export/import of identity, contacts and history has a version, integrity and recovery policy; migration does not clone a dangerously active MLS and spend state. | I03, I05, F04 | V1-implementation |
| [N01](tasks/N01.md) · Authenticated P2P channel and backpressure | libp2p QUIC and a fallback TCP+Noise transport bind PeerID to a signed NodeRecord; streams are limited by size and concurrency. | F02, I02 | V1-implementation |
| [N02](tasks/N02.md) · First launch without a single bootstrap server | The client combines built-in diverse seed hints, saved peers, invitations, local discovery and verifiable chain/rendezvous hints. | N01, I04, F02 | V1-implementation |
| [N03](tasks/N03.md) · Partial Kademlia routing and secret mailbox rendezvous | A bounded routing table serves rotating secret mailbox keys and optional service records; manifest discovery does not require a full registry. | N02, I04 | V1-implementation |
| [N04](tasks/N04.md) · NAT traversal and decentralized relays | AutoNAT/DCUtR/Circuit Relay v2 provide a direct or proxied encrypted route; a relay consumes a bounded resource and is replaceable. | N01, N02 | V1-implementation |
| [N05](tasks/N05.md) · Verifiable node set and safe placement selection | The algorithm uses an authenticated snapshot and provable sampling over fixed stake units, diversity hints and committed-before-beacon assignment. | N02, L02, F05 | V1-implementation |
| [N06](tasks/N06.md) · Operator mode: resources, quotas and signed offers | A headless/desktop operator publishes allowed services, capacity/prices/TTL, stores ciphertext and respects the disk/network/CPU budget. | N01, F04, F05 | V1-implementation |
| [D01](tasks/D01.md) · Verifiable envelope and cheap admission protection | The envelope binds ciphertext, service class, mailbox capability, expiry and spend certificate; validation order bounds CPU/memory DoS. | F02, N01, N06, I05 | V1-implementation |
| [D02](tasks/D02.md) · Storage, retrieval and delivery lifecycle | Store/get/ack and a durable recipient cursor implement at-least-once transport and message/operation-level idempotency. | D01, F04 | V1-implementation |
| [D03](tasks/D03.md) · Ten replicas, storage certificates and honest durability status | The network collects storage receipts from selected nodes; distinguishes quorum spend, min accepted copies and target replicas; brings the state to R=10. | D02, N05 | V1-implementation |
| [D04](tasks/D04.md) · Autonomous loss detection and repair without owners | Distributed leases, probes and repair responsibilities restore missing copies; TTL expiry and quotas bound the obligation. | D03, N06, F03 | V1-implementation |
| [D05](tasks/D05.md) · Durable indexes, cursors and long-offline recovery | A replicable index/control log makes messages and commits discoverable after relay change, mailbox rotation and runtime change; no endless time-slot scanning. | D04, N03, F04 | V1-implementation |
| [D06](tasks/D06.md) · Attachments, protected fanout, TTL and garbage collection | Encrypted chunk+manifest transfer is resumable; read/repair capabilities are separated; GC accounts for leases and the signed expiry policy. | D05, I02 | V1-implementation |
| [G01](tasks/G01.md) · Group with roles, invitations and devices | Create/invite/join/leave/remove and role policy work on top of MLS; a human member and their device leaves are not mixed up. | I05, I03, D02 | V1-implementation |
| [G02](tasks/G02.md) · Decentralized ordering of the private group log | Baseline: OpenMLS + a separate BFT order of encrypted control entries; keepers see commitments/permitted routing metadata, not MLS secrets. de-MLS is explored as a possible replacement, not as a ready guarantee. | G01, P01, D03 | V1-implementation |
| [G03](tasks/G03.md) · Competing MLS commits, removal and safe epoch change | A commit is built relative to the final parent; the losing intent is re-evaluated/rebased without rolling back an already merged MLS epoch; a new application message is encrypted only into the accepted epoch. | G02, I05, F03 | V1-implementation |
| [G04](tasks/G04.md) · Long offline, control log recovery and safe rejoin | An offline device receives the stored control log or goes through a new permitted rejoin; old private MLS keys are not published for recovery convenience. | G03, D05, I06 | V1-implementation |
| [G05](tasks/G05.md) · Two group privacy profiles and paid fanout | Shared-group-storage and private-recipient-pointers share E2EE semantics but differ in routing structure, resource cost and exposed metadata. | G03, P05, D06 | V1-implementation |
| [G06](tasks/G06.md) · Organizations, task rooms and delegated group budget | A permanent organization group and a temporary job room have member, archiving and shared postage budget policies without handing out a shared master key. | G05, A04, I02 | V1-implementation |
| [L01](tasks/L01.md) · Backed issuance of transport postage stamps on a single EVM L2 | The contract accepts a native asset, records the paid class/count of resource tickets and commitments; chain/genesis/issuer-root are part of the proof domain. | F02, F05, I02 | V1-implementation |
| [L02](tasks/L02.md) · Stake registry, epochs, root membership and randomness binding | Node registration, bond/unbond delays and epoch snapshots provide a verifiable participant set without downloading the whole registry; the seed is bound to a future agreed source. | L01, F05 | V1-implementation |
| [L03](tasks/L03.md) · SubsidyVault: time, shared fund and grant entitlement | The paid subsidy uses the same stamp format, a time-based schedule and global/epoch/credential limits; the sponsor pays for a bounded claim transaction. A separate optional EntryBond admits anonymous backed entry: entitlement is bound to lock_id, amount, term and withdrawal delay, not to the number of wallets. | L01, F05, I02 | V1-implementation |
| [L04](tasks/L04.md) · Non-governing organization share and economic immutability | An immutable splitter routes the agreed fee share to the recipient; pull payments and recipient failure do not block service. | L01, L03, F05 | V1-implementation |
| [L05](tasks/L05.md) · Operator payments and correct bounds of service proofs | Batch payout for backed transport services verifies receipts and quotas; slashing is allowed only for a formalizable violation. | L04, P05, D04 | V1-implementation |
| [L06](tasks/L06.md) · Working L2 adapter: finality, reorg and independent sources | The adapter verifies the accepted finality/checkpoint profile, caches receipts/roots and survives an RPC change without moving plaintext on chain. | L02, L03, L04, L05, P04 | V1-implementation |
| [P01](tasks/P01.md) · Reusable BFT finalizer for bounded protocol logs | A proven consensus engine finalizes app-defined entries with durable quorum certificates; memberships and leases are set by L2 snapshots, not by the company. | F03, F04, F05, N01, N05 | V1-implementation |
| [P02](tasks/P02.md) · Private verifiable postage stamp and bounded spend domains | A finalized funded commitment issues bounded one-time tickets of a fixed resource class; the proof hides the chosen deposit within the real anonymity set. | L01, P01, F06 | V1-implementation |
| [P03](tasks/P03.md) · Atomic spend and admission certificate without double-spend | One ticket is allowed exactly one shard/epoch/lease; consensus atomically reserves the nullifier and issues a certificate for a specific operation commitment. | P01, P02, D02 | V1-implementation |
| [P04](tasks/P04.md) · Epoch survival, reconfiguration and eventual autonomy from L2 | Spent state is handed to the next committee with a verifiable checkpoint and continuity; without a handover proof old tickets are not reissued. | P03, L02, F03 | V1-implementation |
| [P05](tasks/P05.md) · Resource pricing, reservation and shared repair budget | The stamp class covers the specified bytes×TTL×replicas, permitted fanout, control overhead and repair allowance; payment is verified before durability is promised. | P03, D03, D06, N06, I02 | V1-implementation |
| [P06](tasks/P06.md) · Attacks on stamps, privacy and backing as a shared regression suite | A separate adversary checks double-spend, front-running, circuit edge cases, grinding, deanonymization fixtures and artificial operator loops. | P04, P05, L05 | V1-implementation |
| [O01](tasks/O01.md) · Native Google OAuth with local JWT verification | A system browser flow with PKCE/state/nonce binds login to the controlled owner key; the JWT is verified by iss/sub/aud/exp and cacheable Google JWKS. | I01, F02, F06 | V1-implementation |
| [O02](tasks/O02.md) · Common ExternalCredential and TrustProfile for external networks | A versioned TrustProfile describes the upstream issuer, subject namespace, accepted audiences, verifiable claims, signer set/k, TTL/revocation and an independent grant policy. ExternalCredential binds an assertion to a NetworkID; there is no Google code in the network type. | F02, I02 | V1-implementation |
| [O03](tasks/O03.md) · Bounded attesters: a single organization issuer or k-of-n | A single site/organization issuer or a selected k-of-n set verifies provider evidence and the owner challenge and issues a scoped credential. V1 allows explicit 1-of-1; the transport BFT/stake registry does not appoint login attesters automatically. | O02, I02, F06 | V1-implementation |
| [O04](tasks/O04.md) · Provider-scoped grant: deduplication, campaigns and cross-provider limits | A GrantClaim is bound to the campaign and a confirmed subject namespace. Wallet/device/attestor/client change without repeated entitlement where subject stability is proven; for a pairwise identity one campaign client/sector or a verifiable mapping is allowed. Unrelated Google/Telegram identities are not declared one person. | O02, O03, L03, P03 | V1-implementation |
| [O05](tasks/O05.md) · Freshness, revocation and unlinking of external providers without losing the address | Google/Telegram/site bindings have an independent lifecycle. Refresh/revoke/unlink do not change the NetworkID and do not reset campaign spent state; profiles and JWKS update within their explicit policy boundaries. | O04, I03 | V1-implementation |
| [O06](tasks/O06.md) · Free start under an explicitly accepted trust profile | Google, Telegram and organization credentials connect to a separate allowlisted funded campaign policy. Having a login does not guarantee a grant; when the fund exists the sponsor covers limited claim gas and the first transport tickets. | O04, O05, O01, O07, O08, L03, P05 | V1-implementation |
| [O07](tasks/O07.md) · Organization issuer and site OAuth/OIDC gateway | A deployable site gateway accepts a challenge from the daemon, runs OIDC or the organization's own authentication and issues a credential with provable claims. It also fits an explicitly trusted government organization; connecting a specific government system is not assumed without its configuration/access. | O02, O03, I02 | V1-implementation |
| [O08](tasks/O08.md) · Telegram Login/OIDC with safe desktop handoff | The Telegram adapter uses the system browser and a registered website callback. The OAuth exchange goes to the gateway with the client secret; signed provider claims are bound to the owner challenge. Tauri receives only a one-time handoff handle; the credential is issued after proof of possession. | O02, O03, O07 | V1-implementation |
| [A01](tasks/A01.md) · Signed service cards and decentralized discovery | ServiceCard binds owner/agent/service/runtime epochs, skills, endpoint, price terms and provenance claims; publication is optional. The card separates claimed properties from reviews and contains a review-discovery namespace without a pointer to a single executor directory. | I02, I04, N03 | V1-implementation |
| [A02](tasks/A02.md) · Signed job offers and immutable terms | RFQ/bid/accept fix an immutable terms_hash, service/owner epochs, criteria, price/asset, transport budget and deadline. An accepted standard order contains a bilaterally signed minimal AcceptedOrderReceipt with the customer's mandatory right to a public review; raw inputs/outputs are not published. | F02, I02, D02 | V1-implementation |
| [A03](tasks/A03.md) · Async work, result, validation and separate payment status | The async job reducer separates provider progress/ExecutorDeclaration, customer acceptance/validation, timeout/cancel and settlement. A missing result or dispute does not take away the previously arisen review right. | A02, D05, D06 | V1-implementation |
| [A04](tasks/A04.md) · Future delegation contract without an economic engine in V1 | V1 validates no_subcontract, data scopes, budget/depth and versioned extensions; unsupported automatic subcontract/settlement is explicitly rejected. Built-in make-or-buy, price optimization and subcontract chains are implemented in V2+ and are not a release prerequisite. | A03, I02, P05 | V1-contract-only |
| [A05](tasks/A05.md) · Executor declaration and customer validation without false attestation | ExecutorDeclaration and CustomerValidation are distinct signed types. The former contains order/terms/result digest and the claimed model/runtime; the latter records the customer's own checks. Independent provenance/settlement is stored only through an explicit adapter; a missing verifier answers unsupported. | A03, I03 | V1-implementation |
| [A06](tasks/A06.md) · A2A adapter with verifiable compatibility | The adapter maps the agreed Agent Card/Task/Message/Artifact and async states between A2A and the native E2EE job transport. | A01, A03, A04, F06 | V1-implementation |
| [Q01](tasks/Q01.md) · Confirmed order and the customer's independent review right | An AcceptedOrderReceipt with customer/provider signatures creates a non-transferable ReviewRight verifiable by any node. The receipt contains domain/order_id/customer/provider/service+epochs, a salted terms commitment and a public disclosure policy; signer authority is bound to the acceptance epoch. | A02, I02, F04 | V1-implementation |
| [Q02](tasks/Q02.md) · Signed reviews, amendments, publication withdrawal and executor replies | A ReviewEvent rates a specific service on a 1–5 scale and contains text, receipt proof and an optional declared outcome. Amendments/withdrawal/replies are new signed events; one effective customer review per order/customer, and a provider reply does not change the customer's score. | Q01, A05, F02 | V1-implementation |
| [Q03](tasks/Q03.md) · Distributed publication and discovery of reviews | Public review bundles and content-addressed events are placed under a paid replication/repair/TTL policy. The service+owner epoch index has independent keepers; any client verifies signatures/receipts and merges sources. A publisher is not obliged to use the executor directory. | Q02, D05, D06, N03, P05 | V1-implementation |
| [Q04](tasks/Q04.md) · Reproducible rating over a known review set | A pure versioned reducer builds count, sum, histogram[1..5], the mean as a rational number and distinct customer count over the effective eligible reviews. RatingSnapshot includes ServiceID/service+owner epochs, policy_version, dataset hash, source cursors and incomplete status. | Q02, Q03, A01 | V1-implementation |
| [Q05](tasks/Q05.md) · Review publicness, privacy preview and local filters | Before publication the specific disclosed fields and the hash of the review being signed are shown. Automatic uploading of private artifacts, JWT, emails/phone, decrypted orders is forbidden. Local block/filter/report policies change visibility but do not forge the signature/history/base aggregate. | Q02, Q03, I02 | V1-implementation |
| [Q06](tasks/Q06.md) · Adversarial review suite: Sybil, replay and censorship | An independent oracle attacks entitlement, duplicate/amendment accounting, cross-epoch replay, censorship and UI exaggerations. With colluding wash-orders it shows the residual risk instead of a false Sybil-proof rating. | Q01, Q02, Q03, Q04, Q05, F06 | V1-implementation |
| [M01](tasks/M01.md) · Local MCP server with narrow authority | stdio MCP uses the official Rust SDK and a single shared authorization broker; tool schemas separate read, send, approve and delegated spending. The common identity.get API returns one's own NetworkID, permitted rights and separate AgentID/RuntimeID if assigned; the CLI uses the same access boundary. | I02, F02, F06 | V1-implementation |
| [M02](tasks/M02.md) · Inbox/outbox, acknowledgment and wake-up of a running runtime | Tools send/poll/ack, group create/invite/members/send and an allowed subscription provide a durable cursor, lease, dedup, pagination and backpressure; the launching process is responsible for LLM wake-up. CLI and MCP call the common messages.send by the permitted recipient's public NetworkID; address resolution and opening the first conversation are done by the daemon under the contact policy. Polling is the baseline; the specific hook/subscription for a running host is chosen before implementation and preserves the durable queue. | M01, D05, P05, G05 | V1-implementation |
| [M03](tasks/M03.md) · MCP for jobs, artifacts and dangerous-action approval | Tools jobs.* manage the signed order lifecycle, return a durable AcceptedOrderReceipt, ExecutorDeclaration and the customer's own checks. The broker limits budget/data/roles; there is no built-in economic engine. | M01, A03, A04, A05 | V1-implementation |
| [M04](tasks/M04.md) · Service discovery and evidence reading via MCP without data leaks | Paged tools/resources return cards, executor declarations, credentials and RatingSnapshot with dataset/policy/source/partial metadata; untrusted text stays data. | M01, A01, A05, Q04 | V1-implementation |
| [M05](tasks/M05.md) · Multiple runtimes of one agent and safe switching | Runtime leases, grants and outbox fencing prevent simultaneous inconsistent execution of one task and exceeding the shared budget. | M02, M03, I03 | V1-implementation |
| [M06](tasks/M06.md) · Shipped agent skill with CLI, MCP integration and optional HTTP | A working client config, an executable conformance scenario and a stdio-first integration are shipped; optional Streamable HTTP listens on loopback with auth/origin checks. A ready agent SKILL.md, connection context examples and working CLI commands identity/send/delivery/poll/ack with versioned JSON, exit codes and stderr diagnostics are mandatory. A host without MCP handles basic messaging using only the skill and the issued handle; core rules and the broker are reused. | M02, M03, M04, M05, A06, M07 | V1-implementation |
| [M07](tasks/M07.md) · MCP for reviews, replies and rating | reviews.eligibility/publish/amend/withdraw/reply/list/get and ratings.get call the same Rust broker/reducers as desktop. The read scope is separate from publication; automatic publication requires a pre-issued exact grant or owner confirmation. | M01, M03, Q04, Q05 | V1-implementation |
| [U01](tasks/U01.md) · Tauri desktop: profile, contacts and personal messaging | A Tauri 2 shell with a bundled web frontend calls a typed Rust bridge → authenticated daemon IPC → common broker. The UI shows messages/durability/read-state; the daemon keeps running after the window is closed per the setting. | I01, D05, F04, U07 | V1-implementation |
| [U02](tasks/U02.md) · Onboarding: own keys, Google, Telegram and organization trust | Tauri UX offers an independent profile/purchase, Google, Telegram and a site/organization issuer. It shows whom the user trusts (1-of-1 or k-of-n), which claims are disclosed and separately eligibility/grant/transport budget. | U01, O06, P05, L04 | V1-implementation |
| [U03](tasks/U03.md) · Full group UX, device management and privacy profile | The UI supports roles, invitations, device leaves, pending/final membership, offline catch-up and routing privacy selection with a price explanation. | U01, G04, G05, G06 | V1-implementation |
| [U04](tasks/U04.md) · Panel of agents, jobs, trust and permissions | The Tauri panel shows runtime grants, service cards, signed orders, ExecutorDeclaration, customer checks and a separate settlement state. Approval is bound to a specific action hash; no economic decision engine is shipped. | U01, M03, M04, A05 | V1-implementation |
| [U05](tasks/U05.md) · Daily operation: attachments, search, notifications and recovery | Local search, private notifications, safe export/backup, diagnostics, quota and operator settings make the client a standalone product. | U01, D06, I06, M05 | V1-implementation |
| [U06](tasks/U06.md) · Installable builds, headless CLI and voluntary updates | Tauri installers for macOS/Linux/Windows include the Rust bridge and a headless daemon/CLI/MCP; lockfiles/SBOM cover Rust and frontend. The production bundle contains no OAuth client secrets, dev server or WebDriver test plugins; updates are voluntary. The shipment also includes an agent skill with CLI instructions and examples: connecting requires neither project sources nor a dev toolchain. | U02, U03, U04, U05, F06, N06, U08 | V1-implementation |
| [U07](tasks/U07.md) · Hardened Tauri webview → Rust → daemon boundary | A Tauri skeleton with an explicit AppManifest/command ACL, allowlisted capabilities, CSP, origin/window checks and typed IPC connects to the common broker. Runtime secrets and the root signer live outside the webview; remote content is not loaded into a privileged view. | F02, F06, I02, F04 | V1-implementation |
| [U08](tasks/U08.md) · Tauri UX for public reviews and service rating | In an accepted order a review is available even on executor timeout/refusal. The UI provides 1–5 stars/text, footprint preview, amendments/withdrawal/replies and history. The service card shows rating count, distinct customers, epochs, dataset freshness/partial and claimed-vs-verified labels. | U04, Q04, Q05, M07 | V1-implementation |
| [X01](tasks/X01.md) · End-to-end transport chaos and shutdown of company resources | An independent black-box suite verifies fresh bootstrap, real NATs, offline delivery, 10→7→10 repair and bounded storage across several operators. | U06, D04, N04, L06, P06 | V1-implementation |
| [X02](tasks/X02.md) · End-to-end group and privacy security | An external attacker checks forked membership, remove/rejoin, multi-device setups, stale keys and the claimed metadata privacy bounds. | U03, G04, G05, I06, P06 | V1-implementation |
| [X03](tasks/X03.md) · End-to-end economic security and autonomy from L2 | A separate suite attacks spent-state, committee handover, finality, treasury compromise, royalty and resource conservation. | L06, P06, U02 | V1-implementation |
| [X04](tasks/X04.md) · End-to-end Google/Telegram/site onboarding and bounded trust | The suite verifies the Google, Telegram and organization/site adapters, single-issuer/k-of-n profiles, optionality, issuer/key lifecycle, claim dedup and the subsidy schedule. | O06, U02, L06 | V1-implementation |
| [X05](tasks/X05.md) · Three end-to-end agent work scenarios and malicious content | An MCP customer and an independent executor complete OCR, coding and inference service jobs: signed receipt, ExecutorDeclaration, customer checks and a public review. An additional scenario is an accepted but unfulfilled order with a negative review. | M06, G06, U04, A06, M07, U08 | V1-implementation |
| [X06](tasks/X06.md) · Independent release gate and readiness for the next economic phase | The release manifest separately lists V1 code/testnet/mainnet-transport gates and V2 modules. Tauri, provider-neutral attestation with Google/Telegram/site, basic jobs and public reviews/rating are mandatory; the absence of an economic engine does not block V1. | X01, X02, X03, X04, X05, X07 | V1-implementation |
| [X07](tasks/X07.md) · End-to-end review protocol without the company or executor veto | Three independent clients verify the full cycle accepted order → negative review → provider reply → rating discovery after the company/executor are disconnected. Privacy, the customer's right, bounded retention and partial views are verified via public APIs. | Q06, M07, U08, L06 | V1-implementation |


---

# File: REQUIREMENTS.md

# Requirements matrix · revision 1.1

52 requirements. CURRENT is the original request; Uxx/Axx are 27 messages of the source archive; AMENDMENT-01…05 is the user's addition of September 5. Old IDs are kept, later decisions take priority. R23 is now contract-now/future-implementation; public reviews did not move together with economic agents.

| ID | Requirement | Sources | Scope | Packets |
|---|---|---|---|---|
| R01 | Transport independence; optional centralized trust profiles | U00, U08, AMENDMENT-02 | V1 | F01, N02, N04, G02, L04, L06, O03, O07, Q03, U06, X01, X06, X07 |
| R02 | Tauri desktop, independent Rust core, CLI with skill, and MCP | CURRENT, AMENDMENT-01, FOLLOWUP-2026-09-08 | V1 | F06, M01, M06, U01, U05, U06, U07, X06 |
| R03 | End-to-end encryption and authentication | U00, A13 | V1 | F02, I01, I04, I05, N01, D01, G01, G02, G03, G04, X02 |
| R04 | Stable addresses and the Owner / Agent / Service / Runtime separation | U20, A19, AMENDMENT-02, AMENDMENT-03 | V1 | F04, I01, I02, I03, I06, O01, O02, O05, O08, A01, A05, M05, U01, U05, X04 |
| R05 | Direct messages with offline delivery | U00, U02 | V1 | F04, I04, I05, N03, D01, D02, D03, D05, P03, A03, M02, U01, X01 |
| R06 | Ten replicas and autonomous recovery | U02 | V1 | F03, F05, N05, N06, D03, D04, D05, G04, G05, P05, Q03, X01 |
| R07 | Malicious nodes, Sybil, and eclipse | U04 | V1 | F03, F05, N01, N03, N05, D01, D03, D04, L02, P01, P06, X01, X03 |
| R08 | First launch, several bootstrap paths, an incomplete network table | U04, U06 | V1 | I04, N02, N03, N05, L02, X01 |
| R09 | Operation behind NAT and interchangeable relays | A05, A11 | V1 | N01, N04, N06, X01 |
| R10 | Groups as a mandatory feature, roles, and devices | U12, A13 | V1 | F01, I05, G01, G02, G03, G04, G05, G06, M02, U03, X02 |
| R11 | MLS epoch agreement and offline member recovery | A13 | V1 | F03, I05, D05, G02, G03, G04, P01, U03, X02 |
| R12 | Metadata, private mailboxes, and two group modes | A05, A13, A19 | V1+future | I04, N03, N04, D06, G02, G05, P02, P06, O05, Q05, U03, X02 |
| R13 | One existing EVM L2 for the economic control plane | U06, U18, A19 | V1 | F02, N05, L01, L02, L06, X03 |
| R14 | Postage stamps, resource cost, and absence of double spending | U08, A07, A17 | V1 | F02, F03, F05, D01, D03, G05, L01, L05, P01, P02, P03, P04, P05, P06, O04, Q03, U02, X03 |
| R15 | A free start and a smooth time-based transition without an upgrade | U16, A17 | V1 | F05, L03, O06, U02, X04 |
| R16 | Revenue for the supporting organization without administrative rights | U08, A09 | V1 | F05, L04, L05, X03 |
| R17 | Full agent operation via the CLI with skill and MCP | CURRENT, U14, A15, FOLLOWUP-2026-09-08 | V1 | F04, I02, D02, D05, A03, A04, A06, M01, M02, M03, M04, M05, M06, M07, U04, X05 |
| R18 | A2A interoperability without replacing the transport | A15, A19 | V1 | F02, F06, A01, A06, M04, M06, X05 |
| R19 | Basic service order: terms, acceptance, result, and customer assessment | U18, A19, AMENDMENT-04 | V1 | A02, A03, A04, A06, Q01, M03, U04, X05 |
| R20 | Escrow, arbitration, insurance, and a full DeFi labor economy | U18, A19, CURRENT, AMENDMENT-04, AMENDMENT-05 | contract-now/future-implementation | A02, A03, A05, X05, X06 |
| R21 | Separation of account trust, executor statements, and public rating | U18, U20, A19, AMENDMENT-04 | V1 | I03, O02, A01, A05, Q04, M04, U04, X05 |
| R22 | Declared model and the absence of fake independent attestation | U18, A19, AMENDMENT-04, AMENDMENT-05 | V1+future | A01, A05, M04, U04, X05 |
| R23 | Future make-or-buy; verifiable delegation boundaries now | U18, A19, AMENDMENT-05 | contract-now/future-implementation | I02, G06, A02, A04, M03, M05, U04, X05 |
| R24 | Permanent agent organizations and temporary job rooms | A15, A19 | V1 | G01, G05, G06, P05, A04, M02, U03, X02, X05 |
| R25 | Independent login: own keys or Google/Telegram/organization trust | U20, U22, AMENDMENT-02, AMENDMENT-03 | V1 | F01, I01, L03, P02, O01, O02, O03, O04, O05, O06, O07, O08, U02, X04 |
| R26 | Cacheable external attestation without provider calls per message | U20, A24, AMENDMENT-02, AMENDMENT-03 | V1 | O01, O02, O03, O05, O07, O08, X04 |
| R27 | Deduplication within a provider namespace and the limits of cross-provider uniqueness | U16, U20, A24, AMENDMENT-02, AMENDMENT-03 | V1 | F05, L03, P02, P03, P06, O04, O05, O06, O08, X04 |
| R28 | ~$5 — transport postage stamps, not money and not job payment | U22, A17, A24 | V1 | L01, L03, L05, P02, P05, P06, O04, O06, U02, X03, X04 |
| R29 | Further evolution without rewriting the transport | CURRENT, A19 | V1 | F02, I03, L01, L06, O02, A02, A03, A05, A06, M01, X06 |
| R30 | Optional executor discovery and customer privacy | U18, A19, AMENDMENT-04 | V1 | I04, N03, A01, Q01, Q03, Q05, M04, X07 |
| R31 | Separation of node, agent, validator, and trust issuer roles | U08, A19, A24 | V1 | I02, I03, N06, G06, L02, L04, O03, A05 |
| R32 | Continued operation when the L2 is unavailable | A19 | V1 | F03, L06, P01, P04, X03 |
| R33 | TTL, bounded resources, deletion, and honest statuses | U02, A17 | V1 | F04, N06, D01, D02, D04, D05, D06, G04, P03, P05, Q03, M02, U05, X01 |
| R34 | Job artifacts and attachments | A13, A15, A19 | V1 | D02, D05, D06, G05, A03, M03, U05 |
| R35 | A usable Tauri client with orders and public reviews | CURRENT, A13, AMENDMENT-01, AMENDMENT-04 | V1 | I06, D06, G01, U01, U02, U03, U04, U05, U06, U07, U08 |
| R36 | Test-first as the mandatory development order | CURRENT | V1 | F01, F02, F03, F04, F06, G03, P01, P06, Q06, U07, X02, X06 |
| R37 | Atomicity and high parallelism without artificial cuts | CURRENT | V1 | F01, F06, U06, X06 |
| R38 | Key isolation and protection from prompt injection / unsafe execution | A15, A19, CURRENT | V1 | F04, F06, I01, I02, I03, I06, O01, O05, O07, O08, A04, Q02, Q05, M01, M03, M04, M05, M06, M07, U01, U04, U05, U07, U08, X05, X07 |
| R39 | Versioning, compatibility, and voluntary updates | U08, A09, CURRENT | V1 | F02, F06, I06, L04, L06, P04, U06, X06 |
| R40 | Operator economics without a fake proof-of-useful-inference | U14, A15, A19 | V1 | F05, N06, D04, L05, X03 |
| R41 | Spendable capabilities and a free entry without hidden gas purchase | U20, U22, A19 | V1 | I02, G06, L03, P05, O06, A04, M01, M03, M05, U02, U04, X04 |
| R42 | Verification by real agent scenarios | U14, U18, A19, AMENDMENT-04, AMENDMENT-05 | V1 | M06, X05 |
| R43 | Measurable resource and operational constraints | U02, U04, U16, CURRENT | V1 | F05, N01, N04, N05, N06, D03, D06, L02, L05, P05, P06, O06, X01, X06 |
| R44 | External dependencies and the limits of the decentralization claim | U00, U08, U20, A24, AMENDMENT-02 | V1 | F01, F05, N02, N05, L02, L04, L06, P01, P04, O03, O05, O07, U06, X01, X03, X04, X06 |
| R45 | Tauri 2 instead of egui/eframe | AMENDMENT-01 | V1 | F01, F06, O01, U01, U02, U04, U05, U06, U07, U08, X06 |
| R46 | Extensible trust profiles; a limited set or a single issuer | AMENDMENT-02 | V1 | F01, F02, F05, L03, O01, O02, O03, O04, O05, O06, O07, O08, U02, X04, X06 |
| R47 | Telegram authorization in V1 | AMENDMENT-03 | V1 | F01, F06, L03, O02, O04, O05, O06, O08, M06, U02, U06, X04, X06 |
| R48 | ExecutorDeclaration is not objective attestation of the result | AMENDMENT-04 | V1 | F02, A01, A02, A03, A05, Q02, Q04, M03, M04, U04, X05, X06 |
| R49 | The customer's right to a public review arises from the accepted order | AMENDMENT-04 | V1 | F01, F02, F04, F06, I02, A02, A03, A05, A06, Q01, Q02, Q06, M03, M06, M07, U08, X05, X06, X07 |
| R50 | Public signed reviews, amendments, and replies | AMENDMENT-04 | V1 | F02, F04, F06, I02, I03, D05, Q02, Q03, Q04, Q05, Q06, M06, M07, U06, U08, X05, X06, X07 |
| R51 | A reproducible basic rating, without promises of protection from all manipulation | AMENDMENT-04 | V1 | F02, F05, A01, Q04, Q05, Q06, M04, M07, U08, X06, X07 |
| R52 | Phase boundary: orders and reviews in V1, complex economic agents in V2+ | AMENDMENT-05 | V1 | F01, F05, G06, A02, A03, A04, A05, A06, Q01, Q06, M03, M05, U04, X05, X06, X07 |

## Acceptance criteria

### R01 · Transport independence; optional centralized trust profiles

The company going offline does not stop the independent transport, groups, previously issued postage stamps, and orders. An optional organization/site issuer may temporarily stop new attestation of its profile; it does not control addresses, keys, reviews, or the network.

### R02 · Tauri desktop, independent Rust core, CLI with skill, and MCP

Tauri 2 desktop on macOS/Linux/Windows uses the shared Rust daemon over a narrow IPC. Headless and MCP work without a window; the webview stores no network keys or OAuth secrets. An executable CLI and an agent skill with instructions/examples ship with the client; the issued scoped connection context is sufficient for agent messaging without MCP support on the host.

### R03 · End-to-end encryption and authentication

Intermediaries, committees, and the company get no plaintext or correspondence keys; sender/device keys are verified; epoch changes test forward secrecy and security recovery.

### R04 · Stable addresses and the Owner / Agent / Service / Runtime separation

Moving a device, changing the runtime, and unlinking Google, Telegram, or the organization's issuer do not change the NetworkID; selling/changing the owner does not unconditionally inherit old trust statements.

### R05 · Direct messages with offline delivery

The sender may go offline after the storage certificate; the recipient reads later; repeated delivery does not repeat the application action.

### R06 · Ten replicas and autonomous recovery

The R=10 test profile restores replicas after failures without the sender and recipient given a source, an honest quorum, available independent nodes, and a paid resource.

### R07 · Malicious nodes, Sybil, and eclipse

Attack cost, operator independence, grinding, and correlated failures are modeled; an arbitrary ten keys are not passed off as ten independent operators.

### R08 · First launch, several bootstrap paths, an incomplete network table

A clean client discovers the network without a single domain and a full participant list; sample responses are verified against an authenticated snapshot.

### R09 · Operation behind NAT and interchangeable relays

QUIC and the fallback TCP route, hole punching, and limited independent relays pass real NAT tests; a relay does not decrypt messages.

### R10 · Groups as a mandatory feature, roles, and devices

Creation, invitation, joining, leaving, removal, roles, and several devices per member are implemented, not moved to V2.

### R11 · MLS epoch agreement and offline member recovery

Competing Add/Remove/Update do not create two final membership states; a removed member cannot read data after finalized removal.

### R12 · Metadata, private mailboxes, and two group modes

V1 provides secret rotating addresses, padding, private pointers, and an explicit description of leaks; strong anonymity against a global observer and a mixnet are a separate future phase.

### R13 · One existing EVM L2 for the economic control plane

No own L1/PoW/speculative token and no transaction per message; chain_id, genesis, and contract addresses are part of signature and proof domains.

### R14 · Postage stamps, resource cost, and absence of double spending

A postage stamp is backed by a budget, bound to a resource class and a single spend domain; a race between nodes allows at most one final spend.

### R15 · A free start and a smooth time-based transition without an upgrade

One fixed subsidy function, a shared reserve, and limits work across epoch and time boundaries; previously paid postage stamps remain valid until their own expiry.

### R16 · Revenue for the supporting organization without administrative rights

Compromising the share recipient gives no mint, pause, blacklist, upgrade, change of fees, committee compositions, or keys; the share applies to an explicitly chosen fee base, not silently to the entire job GMV.

### R17 · Full agent operation via the CLI with skill and MCP

The agent discovers a service, receives messages/tasks, sends a result, and reads the allowed status via MCP; an offline agent gets a durable inbox, not a fake push into a powered-off model. Using the supplied skill and CLI, the agent uses the access handle to learn its NetworkID, writes to an allowed NetworkID without a known ConversationID, receives/acknowledges a reply, and restores the inbox after a restart. The same scenario runs behind NAT over libp2p, including relay-only and relay failure; a hook remains an optional addition to durable polling.

### R18 · A2A interoperability without replacing the transport

A conformant adapter supports version negotiation, Agent Card, Task, Artifact, and the agreed states; the own binary transport is not automatically declared A2A-compatible.

### R19 · Basic service order: terms, acceptance, result, and customer assessment

RFQ/bid/accept/progress/result/cancel/timeout are signed. A two-sided accepted order gives the client the right to a public review without requiring a result, payment, or new consent from the executor. A confirmed order, the executor's declaration, the customer's check, and settlement are different states.

### R20 · Escrow, arbitration, insurance, and a full DeFi labor economy

V1 fixes the types and negative checks of future settlement. Autonomous economic strategies, escrow, insurance, arbitrator and assessor markets are V2+. Public reviews and a simple rating are not among the deferred features.

### R21 · Separation of account trust, executor statements, and public rating

V1 stores scoped identity credentials, the executor's signed statement, customer checks, and public reviews of confirmed orders. The rating describes a set of reviews, not universal truth/honesty or proven model execution.

### R22 · Declared model and the absence of fake independent attestation

In V1 the executor signs the ExecutorDeclaration and binds it to the job/terms/result digest. The customer assesses the available result; the review does not prove internal execution. A strict external provenance-verifier without an implementation returns unsupported; TEE/zkML and verifier markets are V2+.

### R23 · Future make-or-buy; verifiable delegation boundaries now

V1 stores and enforces no_subcontract/data/budget/depth restrictions and fails closed for unsupported extensions, but does not ship a built-in autonomous make-or-buy/subcontracting engine. Complex economic agents and subcontracting calculations are V2+.

### R24 · Permanent agent organizations and temporary job rooms

Groups are used both as organizations and as a job room with limited participants and a shared postage stamp budget without a shared root key.

### R25 · Independent login: own keys or Google/Telegram/organization trust

Anonymous purchase/backed path and voluntary external credentials work. Authorization is not the same as a grant entitlement: campaigns separately accept the issuer/profile and set backed limits. One's own address does not depend on the provider.

### R26 · Cacheable external attestation without provider calls per message

Google native/OAuth and the Telegram/site gateway verify the signature, issuer, audience, challenge, and validity periods. Nodes verify the scoped credential against the accepted profile and do not call Google/Telegram on forwarding; an OAuth access token by itself does not authenticate identity.

### R27 · Deduplication within a provider namespace and the limits of cross-provider uniqueness

Changing the wallet, device, attestor, or application within a confirmed subject namespace does not create a new entitlement. For pairwise sub, cross-application uniqueness is not assumed without a verifiable mapping/a single campaign client. Google and Telegram are not declared to be one person; stacking is limited by preset campaign/network/profile caps.

### R28 · ~$5 — transport postage stamps, not money and not job payment

There is no user cash-out, transfer of a subsidy entitlement, or direct payment of labor with a grant; the denomination is expressed in resources, and the monetary equivalent is labeled as an estimate.

### R29 · Further evolution without rewriting the transport

Messages, job terms, evidence, settlement, and credentials are versioned; a new market/validator does not change E2EE and storage; test adapters are isolated by network domain.

### R30 · Optional executor discovery and customer privacy

Service cards are opt-in; private order data is encrypted. Publishing a review discloses only the pre-signed minimal receipt and the client's chosen text; there is no mandatory global list of all orders/clients. The client is warned about the public link with the service.

### R31 · Separation of node, agent, validator, and trust issuer roles

The rights of one role do not grant another; a permissionless node gets no plaintext; job quality is not determined by the treasury owner.

### R32 · Continued operation when the L2 is unavailable

Delivery and spending of existing postage stamps continue within the current lease and known final roots; after the safe limit, new admissions stop while stored data keeps being served.

### R33 · TTL, bounded resources, deletion, and honest statuses

The UI distinguishes the local queue, storage, degradation, and reading; TTL and quota are measurable; deletion does not promise to erase a copy already decrypted by the recipient.

### R34 · Job artifacts and attachments

Encrypted chunks/manifests, resumable transfer, and capability-based access work without centralized object storage; recovery extends to manifests and service pointers.

### R35 · A usable Tauri client with orders and public reviews

Contacts, correspondence, groups, devices, notifications, search, backup, and settings are available in Tauri. For an order, one can write/edit/withdraw a review, read the reply, the rating, the sample size, and completeness limits.

### R36 · Test-first as the mandatory development order

Every card has an independent specification, positive/negative checks, a fault scenario, and RED→GREEN evidence; product tests are not declared completed in this plan.

### R37 · Atomicity and high parallelism without artificial cuts

Every packet delivers observable behavior and has ownership boundaries and dependencies; the dependencies reflect causality, not the calendar or team size.

### R38 · Key isolation and protection from prompt injection / unsafe execution

MCP has no root-sign, arbitrary shell, or wallet without limits; external results are untrusted data; code execution requires a separate sandbox/VM and is not performed by the daemon process.

### R39 · Versioning, compatibility, and voluntary updates

There is no mandatory auto-update or company kill-switch; clients can reject an incompatible profile, migrate in an agreed way, and import data into a fork.

### R40 · Operator economics without a fake proof-of-useful-inference

Inference remains a job, not a Sybil-proof/consensus; payouts account for backed service and the debatable limits of observability and do not equate the availability of ten keys with ten physical disks.

### R41 · Spendable capabilities and a free entry without hidden gas purchase

A grant can be arranged at a limited sponsor's expense; the agent spends only the delegated budget, and paid excess requires the owner's permission.

### R42 · Verification by real agent scenarios

An MCP client and an executor go through OCR/coding/inference orders, an offline period, the ExecutorDeclaration signature, and a customer review. A failed/unperformed accepted job can also be reviewed; a complex economic engine is not needed.

### R43 · Measurable resource and operational constraints

Benchmark results, quorum risks, replication overhead, and operator income/spending are published as measurements; the initial parameters are labeled as proposed.

### R44 · External dependencies and the limits of the decentralization claim

L2 governance, bootstrap, the OAuth gateway, JWK roots, the attestor threshold/single issuer, and operator concentration are disclosed. A single issuer is acceptable for a voluntary trust profile but is not passed off as an independent quorum and does not control the transport.

### R45 · Tauri 2 instead of egui/eframe

The Tauri webview uses a packaged frontend and an explicit command ACL; the privileged Rust bridge talks to the shared broker. OAuth opens in the system browser. XSS/remote content do not get shell, root-sign, arbitrary fs, JWT, or client secrets.

### R46 · Extensible trust profiles; a limited set or a single issuer

One ExternalCredential/TrustProfile format supports Google, Telegram, site OAuth/OIDC, and attestation by a trusted organization, including a government one. The 1-of-1 mode is allowed and explicitly disclosed; k-of-n does not require a new transport. Claims, issuer keys, expiry, revocation, and funding are bounded by the profile.

### R47 · Telegram authorization in V1

The user goes through Telegram Login/OIDC in a browser via the configured gateway and binds the confirmation to their own NetworkID. The bot/client secret stays only on the gateway; the exact provider subject, audience, one-time challenge, and safe callback are verified. Unlinking preserves the address.

### R48 · ExecutorDeclaration is not objective attestation of the result

The executor signs that they performed the specific order and received the result digest. The customer may attach their own checks and review, but the protocol attests authorship and the link, not internal execution/quality. Without an external verifier, the independently verified status is unavailable.

### R49 · The customer's right to a public review arises from the accepted order

The AcceptedOrderReceipt contains both parties' signatures and the minimal public review commitment. Having received it, the customer may publish a review regardless of the executor's subsequent participation, consent, result, and payment. A random RFQ, someone else's order, or a self-signed receipt grant no right.

### R50 · Public signed reviews, amendments, and replies

One effective review per order/customer with a 1–5 rating and text; amendments/withdrawals/replies are signed events with history. The executor cannot veto/erase a negative. Publication and indexing are distributed; storage is paid; TTL and incompleteness are disclosed.

### R51 · A reproducible basic rating, without promises of protection from all manipulation

A versioned reducer deduplicates reviews and computes count/histogram/sum over the known dataset, service/owner epochs, and an explicit policy. It returns a dataset hash, freshness, sources, and a partial status; Sybil/wash-orders are not declared solved. OAuth gives no extra weight by default.

### R52 · Phase boundary: orders and reviews in V1, complex economic agents in V2+

Basic jobs, ExecutorDeclaration, public reviews/rating, MCP, and Tauri UX are mandatory for the V1 release. The absence of autonomous make-or-buy, underwriting, arbitration markets, or independent verifiers does not block V1; the interfaces and prohibitions of false statuses are present.


---

# File: DECISIONS_AND_VERIFY_GATES.md

# Architectural decisions and verifiable blocking conditions

This is not a set of vague "research later" items. Each decision has a proposed baseline, a bounded check, and a forbidden shortcut. If the baseline does not pass the check, the technical means changes; the original requirement does not disappear. The product release stays closed until the corresponding gate is met.

## ADR-01 · Wire, cryptography, and supply chain

**Baseline:** a canonical binary profile with independent vectors; mature signatures/AEAD/MLS ciphersuite from supported libraries; keys separated by role, epoch, and network domain. The specific algorithm IDs, encoding, proof backend, and exact versions are fixed after the compatibility matrix, before the first product-code consumer.

**Gate F02/F06/I05/P02:** cross-platform canonical hashes, reference verifier, malformed vectors, side-channel/memory handling review, currency of security advisories, license/SBOM. For ZK — circuit constraints, trusted setup/ceremony assumptions or a transparent alternative, reproducible proving/verifying keys, domain binding, soundness review, and measured local proving cost. A single company party must not have a hidden setup trapdoor.

**Forbidden:** a homemade cipher, an unnoticed ciphersuite change, an `accept_all` verifier behind a production flag, a new salt as a way to bypass the nullifier, "library audited → our composed protocol is automatically audited".

## ADR-02 · Group ordering

**Baseline:** OpenMLS + an independent BFT-finalized encrypted control log. This avoids basing V1 on the unproven maturity of a specific raw de-MLS implementation. The committee receives only the necessary ciphertext/commitment/authorization envelope. The participants themselves verify the MLS semantics and never merge a knowingly incorrect commit.

**Gate G02/G03:** an adversary generates concurrent Add/Remove/Update, withholding payload, epoch mismatch, malicious sequencing, stale proposals, and partition. Safety: a single accepted roster/epoch under the assumptions; post-removal secrets are not disclosed. Liveness: after the prerequisites are restored, the group continues operating without its creator.

**Alternative:** VAC de-MLS behind the same adapter contract only after conformance to the fixed specification/commit and independent review. A README and raw specification do not replace a proof [S04].

**Forbidden:** a single company sequencer; "pick the lowest hash and roll back already used keys"; agreeing on membership through several contradictory UI views; moving groups to V2.

## ADR-03 · Double-spend and committee sizing

**Baseline:** a ticket with a single spend-domain and a BFT spent-log; the signature/certificate binds a specific operation commitment. A chain tx per message is not needed, but an agreed spend journal is.

**Gate F05/P01/P03/P04:** quorum intersection, <=f Byzantine model, durable votes/locks, concurrent conflicting spend, node restore, committee reconfiguration, lease expiry, no double refund, and source-of-time. Committee capture by stake, grinding assignments, correlated operators, and admission time/cost are assessed. The number of validators is not derived from the number of storage replicas.

**Parameters:** 4/3 is the minimal test fixture, not a production choice. The specific n/q, epochs, unbond delay, number of shards, repair margin, and admission concurrency are outputs of the risk/benchmark gate. There is no obligation to shard into thousands of committees right away.

**Forbidden:** a local nullifier set instead of finality; lowering quorum under partition; "a new epoch means a clean spent base"; an early refund while an old storage certificate can still be executed.

## ADR-04 · Replication and economic proofs

**Baseline:** ten target nodes in the test profile, a paid finite retention/repair allowance, repair by surviving operators on their own, authenticated manifests and indexes. The storage threshold and the consensus threshold are different parameters.

**Gate D03/D04/L05/X01:** two users offline, failure/lying of some holders, recovery on new nodes, resource growth, and bounded payouts. Operator concentration, correlated failures, and the on-demand fetching attack to pass a challenge are measured separately.

**Forbidden:** treating a retrieval proof as proof of ten physically independent copies; slashing for a single ping timeout; paying twice for one repair; promising infinite repair from a finite budget; passing off 7/10 as achieving 10/10.

## ADR-05 · One L2 and verifiable finality

**Development baseline:** local EVM and Base Sepolia as the proposed test profile. The production L2 is a separate fixed decision, not mandatory loyalty to Base. The transport does not use the chain as message storage. Domain separation allows another voluntary deployment without a bridge.

**Gate L02/L06/P04:** source of authenticated finalized roots, chain-specific finality, root proof verification or an honestly described trusted-checkpoint profile, reorg handling, RPC substitution, sequencer/governance dependency, data availability, and emergency upgrade assumptions. Multiple RPCs mean availability/comparison of answers, not a light client by themselves.

**Offline boundary:** an existing lease + current roots + an honest quorum. Unlimited offline issuance or an unknown epoch are forbidden. An expired lease means stop new admissions; ongoing serving of stored data continues.

**Forbidden:** "a contract without an owner, therefore nobody can change anything in the underlying L2". For example, the current Base security documents describe governance/security council authority [S14].

## ADR-06 · Royalty without hidden control

**Baseline:** a fixed share of postage stamp revenue with an unambiguous accounting event; the royalty is not a share of all agent GMV. 10% is a number proposed in the conversation; the final genesis parameter is chosen explicitly. The operator reserve is calculated after the royalty and is not deducted a second time on payouts.

**Gate L04/X03:** the full call graph, all write capabilities, and dependency contracts; a malicious/reverting recipient; treasury compromise; the impossibility of pause/mint/blacklist/upgrade/committee change/JWK root; a disabled company updater does not affect an already installed compatible client.

**Forbidden:** an admin proxy in an "auxiliary" contract, a mandatory signed config from the company server, or a policy service with the authority to disable users. Compromise of a downloaded malicious release remains a supply-chain threat; default opt-in updates and independent builds limit this separate risk.

## ADR-07 · Subsidy, EntryBond, and transition smoothness

**Baseline:** one finite paid fund, an immutable time schedule, overall/per-epoch limits, finite entitlement, TTL, and sponsored claim. Accepted external-trust branches do not require a user bond; the specific grant depends on the campaign. An anonymous user can simply buy postage stamps; an optional EntryBond gives a limited backed right under a separate policy.

**Gate L03/O06/F05:** boundary time/rounding/overflow; pool exhaustion; multiple wallets; repeated bond lock/unlock; one lock_id does not create new claims; the minimal lock period covers the agreed responsibility; capital farming is modeled by opportunity cost and limits. A bond is not proof of human uniqueness. The model must not allow promised spending without matching funding.

**Forbidden:** giving "one-time $5" daily forever, resetting entitlement by changing wallets, unbacked free issue, hidden buy-gas before the free first message, forcibly changing the rules after genesis. If the fund is exhausted early, the UI reports it: a mathematically smooth function by itself does not make a finite reserve infinite.

## ADR-08 · Provider-neutral trust instead of a mandatory independent Google quorum

**Decision AMENDMENT-02/03:** the common TrustProfile supports Google, Telegram, and site/organization OAuth/OIDC. A single issuer (1-of-1) is acceptable as a voluntary profile; k-of-n is also supported. The mode is shown explicitly to the client. A transport BFT/stake registry is not required to determine login issuers.

**Gate O01–O08/X04:** verification of provider evidence, exact subject namespace, audience, validity period, challenge/holder binding, JWK provenance, scopes, revocation, and funding policy. An ordinary access token or a "government" flag does not authenticate arbitrary claims. A specific government IdP must have real interface/configuration/access, otherwise it remains an example, not a completed integration.

**Forbidden:** putting a raw JWT into the shared transport/chain, calling Google/Telegram on every message, declaring OAuth proof of personhood, or requiring an independent quorum for an explicitly accepted single-issuer profile.

## ADR-09 · Gateway, roots, and bounded issuer authority

**Baseline:** Google native flow; Telegram browser → website gateway → signed credential → holder-bound one-time handoff. The official Telegram server exchange uses a client secret [S16]; it is stored only at the gateway. There are no secrets in Tauri/installer/MCP. A site issuer/reference OIDC adapter is part of V1.

Root identity is not derived from the provider subject and is not automatically recovered from an external login alone. Profile controls are limited to credentials and a pre-funded campaign. An issuer outage can stop new attestation of that profile, not chat/orders/reviews/previously issued postage stamps.

**Gate:** forged/expired JWKS, key rotation, invalid audiences/algorithms, session swapping, handoff interception, SSRF, blocked OAuth app/bot, unlink/relink, and spent entitlement. Substituting an issuer namespace/client/attestor/version does not create a new entitlement. Pairwise OIDC identifiers and cross-provider uniqueness are not assumed without a provable mapping/a restricted campaign namespace [S19].

**Forbidden:** a secret in the frontend, a callback with a bearer JWT, the fallback "any issuer signature is trusted", a single issuer as network admin, or forgetting claims after unlink. A more private ZK-OIDC is V2+, not a reason to postpone a working login.

## ADR-10 · Executor declaration, customer review, and separate payment

**Decision AMENDMENT-04:** V1 implements ExecutorDeclaration, CustomerValidation, and a public ReviewEvent. The signature confirms the author/link, but not the actual internal model or objective quality. The customer review is not renamed to an independent attestation.

**Gate A02/A03/A05/Q01–Q06/X05/X07:** an immutable terms/result digest; the public right from a two-sided receipt; result/payment/provider approval are not a condition for the review. The executor may reply but not veto/delete the client's negative review. Strict external provenance without a verifier answers unsupported.

**Forbidden:** paid=true from JSON; rating→verified-model; executor signature→independent attestation; transfer of previous reputation to a new owner without epochs; negative review→automatic fine/money back.

## ADR-11 · V1 — basic order/public rating; economic engines — V2+

**Mandatory now:** Tauri, groups/repair/postage, Google/Telegram/site profiles, MCP/A2A, signed job lifecycle, ExecutorDeclaration, public reviews, and a simple reproducible representation of the rating.

**In V2+:** autonomous make-or-buy and complex economic orchestration/subcontracting calculations, escrow, insurance/underwriting, arbitration markets, guild collateral, economic scoring agents, TEE/zkML. A04 is now V1-contract-only: a testable interface/constraints and safe unsupported, not an engine implementation. An external runtime can use the basic job API on its own without a mandatory built-in decision engine.

**Gate F01/X06:** no mandatory V1 item is deferred for speed; no explicitly deferred economic engine returns as a release dependency. The original requirements are preserved with a changed phase and a reference to the AMENDMENT.

## ADR-12 · Original numeric examples do not silently become the specification

The original conversation mentioned ~$5, 10% royalty, a bond of about $2, 90+270 days of subsidy, 14 days TTL, a $500k fund, and other illustrations. The meaning of the mechanisms was approved, not the entire numeric configuration. In this plan, target replicas=10 reproduces the user's example as the main test profile; min=7, 100 group members, and 3 devices are newly proposed test baselines.

Before genesis, resource classes, retention tiers, repair allowance, subsidy budget/schedule, grant caps, royalty base/rate, bond/withdrawal delays, committee sizing, and the L2 finality/lease safety bound are needed. They are chosen by models/benchmarks, fixed in test vectors, and not changed by a hidden company config.

## Analyst proposals that were replaced or constrained

| Idea | Correction |
|---|---|
| The nearest ten DHT nodes decide storage | Routing is separate; placement from a verifiable eligible universe with an attack model |
| One nullifier is enough | A single spend-domain, agreement, persistence, and handover are needed |
| MLS/de-MLS automatically solve all groups | E2EE, consensus, the availability control log, and recovery are separate verifiable layers |
| For speed, MVP 1:1 only | Contradicts the subsequent U12; groups are in V1 |
| Smooth stake across keys | Without an anti-splitting model it can amplify Sybil; it is tested and not accepted silently |
| Replica/availability proven by a node signature | The receipt proves a claim; independence of physical storage is a separate assumption |
| Free postage stamps become an economy by themselves | Someone funds their real cost, including repair/relay |
| No direct cash-out — no abuse | Key sale and colluding operators are possible; total damage is bounded and measured |
| Google provides a permanent trusted address | The address is one's own; Google gives a scoped historical/fresh credential |
| ZK hides the email and solves everything | Dedup, issuer roots, audience, nonce, setup, and the privacy set are still needed |
| A single trust score | A set of scoped evidence with different sources/confidence/epochs |
| Inference as PoW | In this architecture inference is a service, not a Sybil-defense/consensus instrument |

## ADR-13 · Tauri 2 and a real security boundary

**Approved by the user:** Tauri instead of the earlier egui recommendation. The Rust daemon stays headless. Packaged frontend → explicit command ACL/AppManifest → Rust bridge → authenticated daemon → shared broker. Custom commands do not rely on safe defaults [S15].

**Gate U07/F06/U06:** XSS/remote content/iframe, capability union, Origin/window/profile confusion, tampered budget, confirmation race, arbitrary invoke. No arbitrary shell/fs/root-sign and OAuth secrets in the renderer. CSP and sanitization complement but do not replace the broker [S17]. Direct tests of the real bridge are mandatory.

**Platform gate:** WebdriverIO embedded tests are available in the current Tauri guidance for the three desktop OSes; the ordinary standalone tauri-driver has different platform coverage [S18]. Test driver plugins and IPC mocks are absent in production. If live automation has not been performed on a platform, the gate is not considered passed.

## ADR-14 · ReviewRight from an accepted order, not from payment/result

**Baseline:** a two-sided minimal AcceptedOrderReceipt with pre-agreed publicity. The client keeps both signatures until accepted is declared. A standard order always grants the right to review; late provider approval, a successful result, and payment proof are not needed. A negative review about non-performance is allowed.

**Gate Q01/Q02:** RFQ-only/someone else's receipt/self-only signature grants no right; the signed header does not change; private terms are salted; a review is not carried over to another service/network/owner epoch. The author fixes/withdraws their own review; the provider can only reply. Competing revisions have a fixed deterministic reducer and a visible history.

**Forbidden:** making all private terms public, silently disabling reviewability, or using a provider signature issued only after a good rating. Details are in PUBLIC_REVIEWS_PROTOCOL_V1.md.

## ADR-15 · Rating without a global censor and without invented Sybil resistance

**Baseline:** signed public bundles + independent replicated indexes; one active score per order/customer; policy-versioned count/sum/histogram, dataset hash, and partial metadata. The aggregate is computed locally by any client over the observable corpus, not certified by an organization as world truth.

**Gate Q03–Q06/M07/U08/X07:** censorship of one index; an identical corpus after reorder/duplicates; replies/edits/withdrawals; owner change; bounded storage/repair funding; prompt injection/privacy preview. An honest path/source/paid TTL are prerequisites of availability.

**Forbidden:** "five stars prove honesty", silent OAuth weight, public ratings as free eternal storage, absence of observations as confirmed absence of negatives. Colluding identities/wash-orders remain a known risk. Future scoring policies do not rewrite raw signed reviews.


---

# File: AGENT_WORK_ORDER.md

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


---

# File: DAG.md

# DAG · causal fronts of integration readiness

Revision 1.1: 84 packets, no calendar speed estimates. Tests/contracts may be written before dependencies are implemented; final acceptance requires real producers. A04 is V1 contract-only; the V2 economic engine is not part of the graph.

| Front | Packets |
|---|---|
| 1 | F01 |
| 2 | F02 |
| 3 | F03 |
| 4 | F04, F05 |
| 5 | F06, I01 |
| 6 | I02, O01 |
| 7 | I03, I04, L01, M01, N01, O02, U07 |
| 8 | I05, L02, L03, N02, N06, O03 |
| 9 | D01, I06, L04, N03, N04, N05, O07 |
| 10 | A01, D02, O08, P01 |
| 11 | A02, D03, G01, P02 |
| 12 | D04, G02, P03, Q01 |
| 13 | D05, G03, O04, P04 |
| 14 | D06, G04, O05, U01 |
| 15 | A03, P05 |
| 16 | A04, A05, G05, L05, O06 |
| 17 | A06, G06, L06, M02, M03, P06, Q02, U02 |
| 18 | M05, Q03, U03, X03, X04 |
| 19 | Q04, Q05, U05, X02 |
| 20 | M04, M07, Q06 |
| 21 | M06, U04 |
| 22 | U08 |
| 23 | U06, X05, X07 |
| 24 | X01 |
| 25 | X06 |

Main new chains:

```text
F02/I02 → O02/O03 → O07 → O08 → O06 → U02 → X04
F02/F06/I02/F04 → U07 → U01 → U04 → U08
A02 → Q01 → Q02 → Q03 → Q04/Q05 → Q06
Q04/Q05 → M07 → U08 → X07 → X06
```

This is an overview; the full source of truth is depends_on in backlog.json. U07 does not require the economic engine, O03 does not require BFT/stake transport, Q01 does not require result/settlement. Boundary complexity is covered by separate tests, not by cutting down the product.


---

# File: SOURCE_MAP.md

# Source map and provenance of decisions

## User archive

Source: `ideation agentic internet.zip`, files `thread.md`, `thread.txt`, `messages.json`. 27 messages were read. The archive SHA-256 is recorded in `metadata.json`. A duplicate copy of the entire private archive is not included in the packet; short source references are used.

Indexing starts from zero: U00 is the user's first message, A01 is the analyst's reply, and so on. CURRENT is the original request about V1/Rust/MCP/test-first/atomicity. AMENDMENT-01…05 are the five normalized decisions from the user's new message of September 5; the verbatim text is preserved in SOURCE_AMENDMENT_2026_09_05.md.

| Messages | Content used in the plan |
|---|---|
| U00 / A01 | Full independence from company servers; E2EE; distinguishing a physical node from a privileged server |
| U02 / A03 | The ten-replica example, holder failure, and the need for automatic replacement |
| U04 / A05 | Malicious/Sybil nodes, bootstrap, a dynamic network without a full table on the client |
| U06 / A07 | Using an existing crypto network/asset, on-chain control plane and off-chain payload |
| U08 / A09 | Postage stamps and the share of the supporting organization without authority to control the network |
| U10 / A11 | Architectural synthesis and the earlier project review; a liveness review is not proof of library readiness in 2026 |
| U12 / A13 | Groups are mandatory; MLS, membership agreement, offline log, devices, privacy, group budget |
| U14 / A15 | A network for agents, persistent inbox, MCP/A2A, inference as a service rather than a ready proof of work |
| U16 / A17 | Paid initial subsidy, fixed transition time, finite fund, caps/TTL, anonymous bond route |
| U18 / A19 | Agent outsourcing; jobs, evidence, provenance models, escrow/validators/underwriting, separation of roles, and a shared L2 |
| U20 / U22 | Google/external trust instead of a bond; ~$5 specifically of transport postage stamps; local login and a persistent address without the whole network querying Google |
| A21 | A promise to research Google, but not a finished technical implementation |
| U23 / A24 | Handoff with a list of open Google questions: JWT claims, uniqueness, privacy, issuer models, recovery, and attacks |
| U25 / A26 | Archiving; not standalone requirements for the transport protocol |

Analyst proposals are marked design/synthesis in `requirements.json`. Explicit user requirements and the current V1 boundary take precedence. Changed/rejected assumptions are listed in `DECISIONS_AND_VERIFY_GATES.md`.

## Delivery clarification of September 8, 2026

`FOLLOWUP-2026-09-08` is a new direct user message: "this thing will be delivered as a client + skill with a cli for the agent. The agent is given a handle into the chat, and it pulls it. It gets methods like learn your own ID/write to another ID and so on. For output, we will think up polling or a hook. It will dive behind NAT with libp2p". It refines R02/R17 and section 10 of the main plan. The composition of the connection context, method names, and the CLI JSON contract are a design concretization of the scenario; the final choice of hook is not claimed as a user decision.

## Verified primary technical sources

The original S01–S14 are kept from revision 1.0 with an access date of September 4, 2026; they are not declared re-verified in this update. S09/S10 and the new S15–S19 were verified on September 5, 2026. A source confirms the specific narrow fact/interface; it is not an audit of our proposed composite protocol. Versions/commit hashes must be re-fixed in the dependency gate before implementation.

| ID | Primary source | Why it is used |
|---|---|---|
| S01 | [rust-libp2p](https://docs.rs/libp2p/), [Kademlia](https://libp2p.io/docs/kademlia-dht/), [DCUtR](https://libp2p.io/docs/dcutr/), [relay](https://libp2p.io/docs/circuit-relay/) | Available networking mechanisms; our own durability/placement on top of them is not implied automatically |
| S02 | [RFC 9420 — MLS](https://www.rfc-editor.org/rfc/rfc9420.html), [RFC 9750 — MLS Architecture](https://www.rfc-editor.org/rfc/rfc9750.html) | Group E2EE, epochs, architectural boundaries of the delivery service |
| S03 | [OpenMLS book](https://book.openmls.tech/) | The Rust MLS implementation and the integration surface |
| S04 | [VAC de-MLS](https://github.com/vacp2p/de-mls), [raw off-chain consensus specification](https://github.com/logos-co/logos-lips/blob/master/docs/anoncomms/raw/decentralized-mls-offchain-consensus.md) | A real candidate library and the need to verify the raw protocol; not conflated with other DMLS projects |
| S05 | [Commonware Simplex](https://docs.rs/commonware-consensus/latest/commonware_consensus/simplex/index.html), [runtime](https://commonware.xyz/blogs/commonware-runtime), [repository](https://github.com/commonwarexyz/monorepo) | The BFT primitive and deterministic runtime as candidates, not a ready implementation of our spend/group protocol |
| S06 | [MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28) | The currently declared specification profile, tools/resources, and capability negotiation |
| S07 | [Official Rust SDK rmcp](https://github.com/modelcontextprotocol/rust-sdk), [MCP security guidance](https://modelcontextprotocol.io/docs/draft/tutorials/security/security_best_practices) | The SDK declares 2026-07-28 and compatibility with 2025-11-25; discover and legacy initialize are different lifecycles; the security guidance is partially draft |
| S08 | [A2A specification](https://a2a-protocol.org/latest/specification/) | Agent Cards, Tasks, Artifacts, and versioned interoperability |
| S09 | [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect) | Stable sub, issuer/audience/expiry/nonce, local verification and JWKS caching |
| S10 | [OAuth for iOS & Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app) | Native OAuth flow and PKCE |
| S11 | [Google OAuth 2.0 policies](https://developers.google.com/identity/protocols/oauth2/policies) | Restrictions on embedded user agents and the need for a matching app policy/configuration; not permission for any subsidy business model |
| S12 | [Sui zkLogin](https://docs.sui.io/sui-stack/zklogin-integration/zklogin) | A precedent of OIDC→ZK and explicit JWK/salt/validator assumptions; not an argument to port the project to Sui or copy address derivation |
| S13 | [ERC/EIP-8004](https://eips.ethereum.org/EIPS/eip-8004) | Future compatibility of identity/reputation/validation records, without a promise of universal truth about quality |
| S14 | [Base security council](https://docs.base.org/specifications/security/security-council-for-base) | Immutability of our own contract does not remove the authority of the underlying L2 governance |
| S15 | [Tauri capabilities](https://v2.tauri.app/security/capabilities/) | Explicit custom-command ACL/AppManifest, capabilities, and webview restrictions; replaces the withdrawn eframe baseline |
| S16 | [Telegram Login](https://core.telegram.org/bots/telegram-login) | A modern OIDC flow, registered callback, server-side client secret, signed ID tokens; not the old widget |
| S17 | [Tauri security](https://v2.tauri.app/security/), [CSP](https://v2.tauri.app/security/csp/) | The frontend/core boundary, defense in depth; not an automatic proof of application security |
| S18 | [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/) | The embedded WebdriverIO provider for desktop OSes; the difference from direct standalone tauri-driver |
| S19 | [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html) | Identity claims, public/pairwise subjects, the distinction between OAuth access and OIDC authentication |

## Where the source ends and the design decision begins

R=10, groups, transport postage stamps, transport independence, and test-first are preserved from the original request. Tauri, Telegram, the organizational issuer, and public reviews in V1 are new direct wishes. The later wish for V2 economic agents replaces the former V1 make-or-buy baseline.

Min replicas=7, the BFT spent-log, and OpenMLS+finalized control log are preserved as proposals of plan 1.0, not proven ready components. The receipt-based review right, stars 1–5, signed amendments/replies, local deterministic rating, single-issuer gateway handoff, and the decomposition into 84 tasks are concretizations of this revision; correctness must be confirmed by independent tests.

The found pages confirm only narrow properties of external interfaces. Checking the live Telegram discovery endpoint in this session did not produce a readable answer; profile/claims/key configuration must be confirmed by a real O08/X04 smoke test. The official Telegram page is sufficient for choosing the documented baseline but is not an executed OAuth test.


---

# File: SOURCE_AMENDMENT_2026_09_05.md

# User amendment of September 5, 2026

Plan version 1.1 updates the version 1.0 documents. This is an additional message outside the 27 messages of the original archive; their Uxx/Axx numbering is preserved.

## Verbatim remarks

> - Let's have Tauri do the UI
> - a limited number of Google attestors -- okay. Trusted networks can be relayed through attestation. In the first version this can be the government organization itself/oauth on the site
> - by the way, I would also like to add Telegram authorization
>
> In essence, only the executor can attest the result. Perhaps, to start, we simply need a public rating mechanism here, i.e., for an ordered service the customer has the right to write a review. This is the level of a basic service order protocol (public reviews), while complex economic agents on top of that are already v2 and beyond.
>
> update the documents per these remarks

## Normalization for traceability

| ID | Decision | Requirement linkage |
|---|---|---|
| AMENDMENT-01 | Tauri instead of the proposed egui/eframe; the Rust daemon is preserved | R02, R35, R45 |
| AMENDMENT-02 | Generalized attestation of trusted networks; a limited set and an organizational issuer/site OAuth are acceptable | R01, R25–R27, R44, R46 |
| AMENDMENT-03 | Telegram authorization is included in V1 alongside Google | R25–R27, R47 |
| AMENDMENT-04 | The executor declares the result; the client gets a public review/rating for their order at the protocol level | R19, R21–R22, R30, R35, R42, R48–R51 |
| AMENDMENT-05 | Complex economic agents are V2+; the basic order and review are V1 | R20, R23, R52 |

A government organization is an acceptable example of an explicitly trusted issuer, not a claim that a specific government system is connected. No unnamed provider is considered already integrated. Allowing a single issuer is treated as a limited voluntary trust-profile, not as a change to the decentralized transport requirement.

The protocol refinements of this update — the two-sided receipt, the absence of veto after order acceptance, 1–5 stars, amendments/replies, and deterministic rating — are design decisions for implementing the wish, not verbatim quotes from the user.


---

# File: tasks/F01.md

# F01 · Verifiable specification of wishes and prohibition of requirement loss

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R10, R25, R36, R37, R44, R45, R46, R47, R49, R52.  
**Integration GREEN dependencies:** none.  
**Ownership:** `spec/requirements; tools/traceability`.

## Observable outcome

Traceability distinguishes the original archive and AMENDMENT-01…05; blocks the loss of Tauri, Google/Telegram/organization trust, public reviews or the return of a complex economic engine into mandatory V1.

## Tests first

1. Full coverage passes; a removed requirement, an unknown task-id and a wrong phase are rejected.
2. A later direct user wish takes priority over a previous analyst recommendation; no old Rxx disappears.

**Negative test:** An attempt to mark an unverified feature as implemented or to assign mandatory groups to later is rejected.

**Failure/race:** A modified/truncated source digest is detected.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The plan-check command prints uncovered=0 only for plan coverage, separately from implementation coverage.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F01.Txx`; negative: `F01.N01`; fault: `F01.F01`; black-box: `F01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/F02.md

# F02 · Canonical wire protocol and version compatibility

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R13, R14, R18, R29, R36, R39, R46, R48, R49, R50, R51.  
**Integration GREEN dependencies:** F01.  
**Ownership:** `crates/protocol-types; spec/wire; fixtures/wire`.

## Observable outcome

One parser/encoder with size limits, signature domains, integer amounts and stable golden vectors for identity, envelope, job, evidence, credential and postage. Added TrustProfile, ProviderBinding, ExecutorDeclaration, AcceptedOrderReceipt, ReviewRight, ReviewEvent, ReviewReply and RatingSnapshot; independent vectors cover their signatures and versions.

## Tests first

1. Rust and an independent minimal reference-parser read the same vectors.
2. Property tests verify canonical roundtrip, unknown optional fields, versions, domain separation.

**Negative test:** Duplicate keys, ambiguous encoding, overflows, NaN, wrong chain/genesis and oversized frames are rejected before expensive cryptography.

**Failure/race:** Corrupted/truncated packets and a decompression bomb do not cause panic or unbounded allocation.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

CLI decode/verify produces identical hashes on macOS, Linux and Windows.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F02.Txx`; negative: `F02.N01`; fault: `F02.F01`; black-box: `F02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/F03.md

# F03 · Deterministic simulator of network, time and failures

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R07, R11, R14, R32, R36.  
**Integration GREEN dependencies:** F02.  
**Ownership:** `crates/test-runtime; tests/simulation`.

## Observable outcome

The same core state machines run with seeded RNG, a virtual clock, and partition, reorder, loss, Byzantine peer and crash/restart models.

## Tests first

1. Two runs with the same seed produce the same event trace and digest.
2. A deliberately faulty reference implementation loses/duplicates a message and is caught by the oracle.

**Negative test:** A test cannot silently use a wall clock, nondeterministic RNG or a real network client.

**Failure/race:** Partition+reorder+crash is minimized to a reproducible failing trace.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The simulate --seed N command reproduces the failure and exports the trace for another agent.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F03.Txx`; negative: `F03.N01`; fault: `F03.F01`; black-box: `F03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/F04.md

# F04 · Transactional state journal and reliable local queue

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R05, R17, R33, R36, R38, R49, R50.  
**Integration GREEN dependencies:** F02, F03.  
**Ownership:** `crates/state-store; crates/domain-reducers`.

## Observable outcome

Durable outbox/inbox, events, cursors and epochs are saved atomically; side effects are executed through a transactional outbox.

## Tests first

1. The reducer model and journal replay produce the same state.
2. An idempotency key with the same request returns the previous result; a different payload with the same key is rejected.
3. AcceptedOrderReceipt, the review right and the review-event outbox survive a crash; the receipt is not lost when the executor fails.

**Negative test:** Lowering an epoch/counter and importing corrupted state do not permit re-signing/re-spending.

**Failure/race:** SIGKILL at every commit/fsync boundary, disk full and partial write do not produce a false durable-ack.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The process accepts a send, crashes and after restart either continues it or explicitly returns a rejection, without losing confirmed data.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F04.Txx`; negative: `F04.N01`; fault: `F04.F01`; black-box: `F04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/F05.md

# F05 · Executable model of threats, committees and resource economics

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R07, R14, R15, R16, R27, R40, R43, R44, R46, R51, R52.  
**Integration GREEN dependencies:** F01, F02, F03.  
**Ownership:** `spec/models; tools/risk-simulator`.

## Observable outcome

The models verify quorum intersection, the grinding/Sybil splitting threat, the subsidy cap and cost accounting before the economic protocols are implemented.

## Tests first

1. The model checker finds no two final spend/MLS epochs in the accepted model with <=f Byzantine.
2. Simulations compare correlated/independent operators and splitting stake across keys.
3. The model separates the price of a transport Sybil attack from rating manipulation via bilateral wash-orders; OAuth does not become proof of personhood.

**Negative test:** A model without a global spend domain and with sqrt(stake) weight on an arbitrary key yields a reproducible counterexample.

**Failure/race:** A long partition preserves safety but predictably stops liveness without a quorum.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The report shows assumptions and the probability/cost of compromise instead of choosing the committee size by aesthetics.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F05.Txx`; negative: `F05.N01`; fault: `F05.F01`; black-box: `F05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/F06.md

# F06 · Independent test oracles and delivery pipeline

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R18, R36, R37, R38, R39, R45, R47, R49, R50.  
**Integration GREEN dependencies:** F01, F02, F03, F04.  
**Ownership:** `ci; tests/acceptance; tools/conformance`.

## Observable outcome

CI runs contract, property, model, fuzz, mutation and platform smoke tests; separates security-critical gates and stores RED/GREEN evidence.

## Tests first

1. Deliberately broken auth/quota/replay implementations are blocked by the pipeline.
2. Wire compatibility and MCP/A2A transcripts are reproducible and versioned.
3. Tauri E2E and bridge tests run separately from renderer mocks; test WebDriver plugins/IPC bypass are absent from the production build.

**Negative test:** Changing a reference test at the same time as the implementation requires separate approval; a skipped security gate blocks the release.

**Failure/race:** A worker crash, a corrupted artifact and a mismatched lockfile make the build unfit for release.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A single command reproduces the package check on a clean checkout and shows dependency/artifact sources.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `F06.Txx`; negative: `F06.N01`; fault: `F06.F01`; black-box: `F06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I01.md

# I01 · Local owner, permanent address and key storage

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R04, R25, R38.  
**Integration GREEN dependencies:** F02, F04.  
**Ownership:** `crates/identity-owner; crates/key-vault`.

## Observable outcome

Root identity and NetworkID creation is independent of Google/chain; keys are protected by the system keychain and an encrypted store.

## Tests first

1. One root yields the same address across platforms; Google/email/wallet do not participate in its derivation.
2. A locked vault forbids signatures; a delegated public identity remains verifiable.

**Negative test:** The root secret value does not end up in RPC, MCP, logs, crash dump fixtures or the clipboard without an explicit export.

**Failure/race:** An unavailable keychain/corrupted vault does not silently create a new identity.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

CLI create/lock/unlock shows the permanent address without an internet connection.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I01.Txx`; negative: `I01.N01`; fault: `I01.F01`; black-box: `I01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I02.md

# I02 · Delegation to devices, agents and runtime with limits

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R17, R23, R31, R38, R41, R49, R50.  
**Integration GREEN dependencies:** I01, F02.  
**Ownership:** `crates/capabilities`.

## Observable outcome

The owner signs limited grants to agent/device/runtime: methods, inbox, recipients, budgets, TTL, subcontracting depth and epochs.

## Tests first

1. A correct grant chain permits only the specified actions; all restrictions are inherited by intersection.
2. Two concurrent spends of one delegated budget do not exceed the limit.
3. review.publish/amend/withdraw and provider.reply are separate capabilities; the customer's right is not granted to a runtime of another role.

**Negative test:** Runtime cannot grant rights wider than its own, become Owner or reuse a capability in another network.

**Failure/race:** Clock skew and a stale grant move a dangerous action to deny/approval, not allow.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two runtimes of one agent have different rights; one sends a message but cannot export keys or buy work beyond the budget.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I02.Txx`; negative: `I02.N01`; fault: `I02.F01`; black-box: `I02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I03.md

# I03 · Revocation, epoch change, new owner and recovery

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R21, R29, R31, R38, R50.  
**Integration GREEN dependencies:** I02, F04.  
**Ownership:** `crates/identity-lifecycle`.

## Observable outcome

A signed history of ownership/device/service epochs makes owner change and revocation verifiable; recovery requires a pre-selected recovery right.

## Tests first

1. The old runtime loses new authorities after a revoke is accepted; the receipt keeps the old owner epoch.
2. A new service version does not automatically inherit the old version's proof.
3. Key rotation preserves a verifiable revocation history, but a review is not inherited by the new owner as their own achievement.

**Negative test:** A Google JWT, the company and a single member of the recovery set by themselves do not re-sign the root.

**Failure/race:** Conflicting recovery attempts and a partition have an explicit final order/freeze policy.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A stolen device is disabled; a new one continues the same address; the UI shows the revocation freshness boundary.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I03.Txx`; negative: `I03.N01`; fault: `I03.F01`; black-box: `I03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I04.md

# I04 · Contacts, invitations and safe first message

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R05, R08, R12, R30.  
**Integration GREEN dependencies:** I02, F02.  
**Ownership:** `crates/contact-handshake`.

## Observable outcome

One-time invite/prekey packets and closed contacts allow establishing a chat; the public contact endpoint is quota-limited and does not publish the contact list.

## Tests first

1. Invite verifies the signature, network, expiry and possession challenge.
2. A first contact and a confirmed contact use different budgets/rights.

**Negative test:** Invitation replay, prekey substitution and mass unsolicited requests do not create a trusted chat.

**Failure/race:** A removed/expired prekey leads to a safe handshake retry, not a plaintext fallback.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two new profiles exchange an invitation out of band and establish a private channel without the company.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I04.Txx`; negative: `I04.N01`; fault: `I04.F01`; black-box: `I04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I05.md

# I05 · MLS client for personal chat and multiple devices

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R05, R10, R11.  
**Integration GREEN dependencies:** I02, I04, F06.  
**Ownership:** `crates/crypto-mls`.

## Observable outcome

The MLS profile implements 1:1 as a device group; KeyPackage/Welcome/commit/application messages have a strict lifecycle and binding to identity.

## Tests first

1. Library vectors and independent interoperability fixtures confirm encryption/decryption.
2. Multiple devices receive messages under the permitted history policy; FS/PCS fixtures verify past traffic after old keys are removed and future secrecy after an honest Update with new entropy. The locally stored plaintext history is a separate object of protection.

**Negative test:** Substituted leaf identity, KeyPackage replay, wrong epoch and unknown ciphersuite are not accepted.

**Failure/race:** A crash between commit and state write does not repeat one-time key operations.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two owners with three devices exchange E2EE messages; an intermediary sees only the limited envelope.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I05.Txx`; negative: `I05.N01`; fault: `I05.F01`; black-box: `I05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/I06.md

# I06 · Encrypted backup, migration and rollback protection

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R35, R38, R39.  
**Integration GREEN dependencies:** I03, I05, F04.  
**Ownership:** `crates/backup-restore`.

## Observable outcome

Export/import of identity, contacts and history has a version, integrity and recovery policy; migration does not clone a dangerously active MLS and spend state.

## Tests first

1. A backup restores the NetworkID; a new runtime gets a new device identity/epoch.
2. An unsupported version first goes through migration preview and verification.

**Negative test:** An old backup does not restore revoked rights and does not reset an already spent budget.

**Failure/race:** An interrupted import does not corrupt the active profile; loss of the network anti-rollback proof forbids unsafe continuation.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Migration to another computer preserves contacts/address, performs a safe rejoin and shows the restored-history limit.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `I06.Txx`; negative: `I06.N01`; fault: `I06.F01`; black-box: `I06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N01.md

# N01 · Authenticated P2P channel and backpressure

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R07, R09, R43.  
**Integration GREEN dependencies:** F02, I02.  
**Ownership:** `crates/p2p-transport`.

## Observable outcome

libp2p QUIC and a fallback TCP+Noise transport bind PeerID to a signed NodeRecord; streams are limited by size and concurrency.

## Tests first

1. Both transports transmit identical frames and correctly negotiate versions.
2. A slow reader and priority queues do not block review/group management.

**Negative test:** An invalid NodeRecord signature, oversize and stream flood are rejected before expensive processing.

**Failure/race:** QUIC→TCP fallback, a dropped connection and half-open do not duplicate the application action.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two headless processes exchange signed encrypted envelopes over different routes.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N01.Txx`; negative: `N01.N01`; fault: `N01.F01`; black-box: `N01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N02.md

# N02 · First launch without a single bootstrap server

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R08, R44.  
**Integration GREEN dependencies:** N01, I04, F02.  
**Ownership:** `crates/bootstrap`.

## Observable outcome

The client combines built-in diverse seed hints, saved peers, invitations, local discovery and verifiable chain/rendezvous hints.

## Tests first

1. A clean profile starts with any company-owned seeds disabled.
2. Unavailable DNS does not break direct IP/multiaddr/contact hints.

**Negative test:** A single fake seed group does not get the right to rewrite genesis or a trusted checkpoint.

**Failure/race:** All network hints unavailable: an honest actionable bootstrap-needed, not an infinite spin and not an invented network.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A new client joins through an independent contact and confirms the network's identity.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N02.Txx`; negative: `N02.N01`; fault: `N02.F01`; black-box: `N02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N03.md

# N03 · Partial Kademlia routing and secret mailbox rendezvous

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R07, R08, R12, R30.  
**Integration GREEN dependencies:** N02, I04.  
**Ownership:** `crates/routing-mailboxes`.

## Observable outcome

A bounded routing table serves rotating secret mailbox keys and optional service records; manifest discovery does not require a full registry.

## Tests first

1. Bounded lookup on a growing synthetic network keeps the local routing-state limit.
2. Rotation period overlap gives a bounded read window for an offline recipient.

**Negative test:** Unsigned/stale records, targeted eclipse and mass service spam writes do not substitute a contact.

**Failure/race:** Route rebuild and loss of the nearest peers find another path/replica.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The CLI recovers the storage address via the secret rendezvous without downloading the whole network table.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N03.Txx`; negative: `N03.N01`; fault: `N03.F01`; black-box: `N03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N04.md

# N04 · NAT traversal and decentralized relays

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R09, R12, R43.  
**Integration GREEN dependencies:** N01, N02.  
**Ownership:** `crates/nat-relay`.

## Observable outcome

AutoNAT/DCUtR/Circuit Relay v2 provide a direct or proxied encrypted route; a relay consumes a bounded resource and is replaceable.

## Tests first

1. The NAT/firewall matrix includes direct success, hole-punch success and relay-only success.
2. Relay bytes, reservation TTL and connection count are accounted for.

**Negative test:** The relay does not get application keys; a reservation flood does not displace all legitimate streams without limits.

**Failure/race:** The active relay shuts down: reconnect picks another with idempotent replay.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two desktops behind NAT send messages; shutting down the company relay does not stop them while an independent relay is available.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N04.Txx`; negative: `N04.N01`; fault: `N04.F01`; black-box: `N04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N05.md

# N05 · Verifiable node set and safe placement selection

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R07, R08, R13, R43, R44.  
**Integration GREEN dependencies:** N02, L02, F05.  
**Ownership:** `crates/node-selection`.

## Observable outcome

The algorithm uses an authenticated snapshot and provable sampling over fixed stake units, diversity hints and committed-before-beacon assignment.

## Tests first

1. Merkle/index proofs tie the sample to the full known root/count, not to an imposed peer subset.
2. Splitting stake across keys does not raise the expected share; grinding is measured and bounded by epoch rules.

**Negative test:** Fake ASN/operator assertions do not count as cryptographic proof of independence.

**Failure/race:** Disappearance of selected nodes triggers a deterministic replacement policy with a known safety bound.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

For a seed/snapshot two clients obtain the same admissible placement and an explanation of all trust assumptions.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N05.Txx`; negative: `N05.N01`; fault: `N05.F01`; black-box: `N05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/N06.md

# N06 · Operator mode: resources, quotas and signed offers

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R09, R31, R33, R40, R43.  
**Integration GREEN dependencies:** N01, F04, F05.  
**Ownership:** `crates/node-operator`.

## Observable outcome

A headless/desktop operator publishes allowed services, capacity/prices/TTL, stores ciphertext and respects the disk/network/CPU budget.

## Tests first

1. Admission never exceeds the specified bytes/leases; resources are released after expiry.
2. Disabling operator mode leaves the client usable for ordinary messaging.

**Negative test:** The operator does not get access to the user profile's keys even when running on the same computer.

**Failure/race:** Disk full, I/O error and graceful shutdown produce correct failure/drain/repair hints.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A node with a small quota accepts a paid object, refuses beyond the limit and shows measured usage.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `N06.Txx`; negative: `N06.N01`; fault: `N06.F01`; black-box: `N06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D01.md

# D01 · Verifiable envelope and cheap admission protection

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R05, R07, R14, R33.  
**Integration GREEN dependencies:** F02, N01, N06, I05.  
**Ownership:** `crates/envelope-admission`.

## Observable outcome

The envelope binds ciphertext, service class, mailbox capability, expiry and spend certificate; validation order bounds CPU/memory DoS.

## Tests first

1. A valid envelope is accepted only with the full set of checks and consistent resource bounds.
2. Unit fixtures use a separate test network; the unmodified production path does not accept a fake verifier.

**Negative test:** Wrong size, hash, signature, capability, epoch or voucher forbid writes and a receipt.

**Failure/race:** An attacker sends millions of cheap invalid frames: worker slots remain for valid traffic.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

CLI admission explains the rejection without leaking content and without burning a postage stamp for an invalid admission.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D01.Txx`; negative: `D01.N01`; fault: `D01.F01`; black-box: `D01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D02.md

# D02 · Storage, retrieval and delivery lifecycle

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R17, R33, R34.  
**Integration GREEN dependencies:** D01, F04.  
**Ownership:** `crates/mailbox-store`.

## Observable outcome

Store/get/ack and a durable recipient cursor implement at-least-once transport and message/operation-level idempotency.

## Tests first

1. Only fsync+validated state admits a durable storage receipt.
2. Repeating send/get/ack does not repeat the user-visible message or the application effect.

**Negative test:** Delivered does not mean read; a node receipt does not mean recipient acknowledgment.

**Failure/race:** Crash before/after ack, transfer interruption and recipient restart recover the correct status.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The sender shuts down after storage, the recipient reads later and receives a single message.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D02.Txx`; negative: `D02.N01`; fault: `D02.F01`; black-box: `D02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D03.md

# D03 · Ten replicas, storage certificates and honest durability status

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R06, R07, R14, R43.  
**Integration GREEN dependencies:** D02, N05.  
**Ownership:** `crates/replication-placement`.

## Observable outcome

The network collects storage receipts from selected nodes; distinguishes quorum spend, min accepted copies and target replicas; brings the state to R=10.

## Tests first

1. The target=10/min=7 profile shows degraded until the full target and confirms real retrieval tests.
2. Manifest, ciphertext and repair metadata have their own durable placement.

**Negative test:** Ten signatures of one operator are not displayed as ten provably independent copies.

**Failure/race:** Some nodes confirm and then lie: retrieval detects the discrepancy and initiates replacement.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

After sending the UI shows 7/10, then 10/10; the original sender is not needed for additional copies.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D03.Txx`; negative: `D03.N01`; fault: `D03.F01`; black-box: `D03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D04.md

# D04 · Autonomous loss detection and repair without owners

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R07, R33, R40.  
**Integration GREEN dependencies:** D03, N06, F03.  
**Ownership:** `crates/replication-repair`.

## Observable outcome

Distributed leases, probes and repair responsibilities restore missing copies; TTL expiry and quotas bound the obligation.

## Tests first

1. With sender/recipient offline and three failures, surviving nodes create new copies and update the manifest.
2. Repeated repair assignments do not cause double payment or infinite copying.

**Negative test:** A missed ping is not sufficient proof for slashing.

**Failure/race:** Partition/heal, lying source, false alarm and replacement failure keep at least the available correct copies.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

10→7→10 replicas are restored autonomously; inability to repair is shown as degraded with a reason.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D04.Txx`; negative: `D04.N01`; fault: `D04.F01`; black-box: `D04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D05.md

# D05 · Durable indexes, cursors and long-offline recovery

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R06, R11, R17, R33, R34, R50.  
**Integration GREEN dependencies:** D04, N03, F04.  
**Ownership:** `crates/mailbox-index`.

## Observable outcome

A replicable index/control log makes messages and commits discoverable after relay change, mailbox rotation and runtime change; no endless time-slot scanning.

## Tests first

1. Catch-up over a durable cursor reproduces gaps, duplicates and out-of-order without losing messages.
2. The index and manifest are restored even after losing the original storage leader.

**Negative test:** A forged cursor, unavailable retained range and truncated journal are not presented as full synchronization.

**Failure/race:** The receiver is offline for a month of virtual time; after individual TTLs expire it receives an exact gap report.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A profile on a new runtime finds the available history and explicit unrecoverable intervals.

## Outside the packet

Do not expand the public API and authority beyond the task contract. The public review index is implemented separately in Q03; shared durability primitives are reused.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D05.Txx`; negative: `D05.N01`; fault: `D05.F01`; black-box: `D05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/D06.md

# D06 · Attachments, protected fanout, TTL and garbage collection

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R12, R33, R34, R35, R43.  
**Integration GREEN dependencies:** D05, I02.  
**Ownership:** `crates/blob-artifacts; crates/retention`.

## Observable outcome

Encrypted chunk+manifest transfer is resumable; read/repair capabilities are separated; GC accounts for leases and the signed expiry policy.

## Tests first

1. A file of several chunks is readable after some keepers fail; each chunk's hash and the manifest are verified.
2. Reads by permitted devices fit within the paid fanout.

**Negative test:** A third-party recipient cannot burn the quota with repeated reads; traversal/decompression/oversize are forbidden.

**Failure/race:** Partial download, TTL expiry mid-transfer and crash during GC produce deterministic state.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

An interrupted artifact transfer resumes; deletion is shown as local/policy-based, without a promise to erase someone else's plaintext.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `D06.Txx`; negative: `D06.N01`; fault: `D06.F01`; black-box: `D06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G01.md

# G01 · Group with roles, invitations and devices

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R10, R24, R35.  
**Integration GREEN dependencies:** I05, I03, D02.  
**Ownership:** `crates/group-domain`.

## Observable outcome

Create/invite/join/leave/remove and role policy work on top of MLS; a human member and their device leaves are not mixed up.

## Tests first

1. The permission matrix covers owner/admin/member, invitations and a new device joining.
2. The signed operation intent is bound to group_id, the current epoch and authorities.

**Negative test:** A former admin, someone else's device and another group's invitation do not change membership.

**Failure/race:** Join is interrupted before/after Welcome: no unregistered reading device.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Three owners create a group, add a device and remove a member while seeing a comprehensible roster.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G01.Txx`; negative: `G01.N01`; fault: `G01.F01`; black-box: `G01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G02.md

# G02 · Decentralized ordering of the private group log

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R03, R10, R11, R12.  
**Integration GREEN dependencies:** G01, P01, D03.  
**Ownership:** `crates/group-finalization`.

## Observable outcome

Baseline: OpenMLS + a separate BFT order of encrypted control entries; keepers see commitments/permitted routing metadata, not MLS secrets. de-MLS is explored as a possible replacement, not as a ready guarantee.

## Tests first

1. Two concurrent proposals have one final order; clients validate and apply each entry identically.
2. An invalid MLS proposal is a rejected/no-op action, not a reason to merge two divergent key states.

**Negative test:** A raw draft/library does not count as proof of safety; a single group admin does not become a mandatory online sequencer.

**Failure/race:** A withheld commit payload is not finalized without proven storage; leader censorship/partition changes liveness but not finality.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The group changes membership with the creator offline and one faulty validator within the admissible model.

## Outside the packet

Finalize only the necessary control plane; do not pass every application message through global consensus or L2.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G02.Txx`; negative: `G02.N01`; fault: `G02.F01`; black-box: `G02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G03.md

# G03 · Competing MLS commits, removal and safe epoch change

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R10, R11, R36.  
**Integration GREEN dependencies:** G02, I05, F03.  
**Ownership:** `crates/group-epoch-machine`.

## Observable outcome

A commit is built relative to the final parent; the losing intent is re-evaluated/rebased without rolling back an already merged MLS epoch; a new application message is encrypted only into the accepted epoch.

## Tests first

1. Add+Remove, Remove+Update and two device joins converge to one permitted state.
2. A removed device cannot decrypt messages after a finalized Remove.

**Negative test:** New sensitive data must not be sent into the old epoch after locally accepting the removal; no key reuse after rollback.

**Failure/race:** A split network and a malicious steward do not create two final rosters; the UI shows a pending change rather than a fake instant removal.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Simultaneous addition and removal end with identical epoch/roster on all honest online clients.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G03.Txx`; negative: `G03.N01`; fault: `G03.F01`; black-box: `G03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G04.md

# G04 · Long offline, control log recovery and safe rejoin

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R06, R10, R11, R33.  
**Integration GREEN dependencies:** G03, D05, I06.  
**Ownership:** `crates/group-catchup`.

## Observable outcome

An offline device receives the stored control log or goes through a new permitted rejoin; old private MLS keys are not published for recovery convenience.

## Tests first

1. A full retained log restores the current epoch; an incomplete one gives an explicit rejoin-required.
2. A new member does not get the history by default; a permitted history share is a separate action.

**Negative test:** A forged snapshot with someone else's roster or an epoch rollback is rejected.

**Failure/race:** Loss of a control-log replica while the recipient is offline is repaired; loss of all retained records is not masked.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A device returns after many membership changes and safely continues in the group.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G04.Txx`; negative: `G04.N01`; fault: `G04.F01`; black-box: `G04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G05.md

# G05 · Two group privacy profiles and paid fanout

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R10, R12, R14, R24, R34.  
**Integration GREEN dependencies:** G03, P05, D06.  
**Ownership:** `crates/group-routing`.

## Observable outcome

Shared-group-storage and private-recipient-pointers share E2EE semantics but differ in routing structure, resource cost and exposed metadata.

## Tests first

1. One ciphertext is distributed to permitted members; pointers contain no open roster.
2. Reading by multiple devices counts against the pre-agreed fanout.

**Negative test:** Private pointers are not presented as protection against global correlation of identical blob/time.

**Failure/race:** Some pointers/manifest are lost: the remaining ones and repair restore delivery without widening access.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

In the UI switching the profile shows the price and specific leaks; both configurations deliver the same group text.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G05.Txx`; negative: `G05.N01`; fault: `G05.F01`; black-box: `G05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/G06.md

# G06 · Organizations, task rooms and delegated group budget

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R10, R23, R24, R31, R41, R52.  
**Integration GREEN dependencies:** G05, A04, I02.  
**Ownership:** `crates/agent-groups`.

## Observable outcome

A permanent organization group and a temporary job room have member, archiving and shared postage budget policies without handing out a shared master key.

## Tests first

1. Two agents spend different grant limits from a shared budget without overspend.
2. A subcontractor gets only their room and permitted artifacts.

**Negative test:** A job room member does not get the organization history, treasury or owner authorities.

**Failure/race:** Executor revocation and a budget broker restart do not return the already spent limit.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The customer creates a temporary room, invites an executor, receives the result and closes access to new messages.

## Outside the packet

Do not expand the public API and authority beyond the task contract. Automatic economic strategy and multi-party subcontract settlement are not part of V1; authority limits remain mandatory.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `G06.Txx`; negative: `G06.N01`; fault: `G06.F01`; black-box: `G06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L01.md

# L01 · Backed issuance of transport postage stamps on a single EVM L2

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R13, R14, R28, R29.  
**Integration GREEN dependencies:** F02, F05, I02.  
**Ownership:** `contracts/PostageIssuer; crates/l2-types`.

## Observable outcome

The contract accepts a native asset, records the paid class/count of resource tickets and commitments; chain/genesis/issuer-root are part of the proof domain.

## Tests first

1. Conservation tests: no more than the paid/permitted resource can be issued, rounding is always bounded.
2. Local-EVM and testnet use the same bytecode configuration with different genesis domains.

**Negative test:** A test receipt, another network, a substituted issuer and a reentrant call do not issue production stamps.

**Failure/race:** A reorg before finality does not create an available funded balance.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Native testnet payment yields verifiable commitments; test assets are explicitly not called operators' real funds.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L01.Txx`; negative: `L01.N01`; fault: `L01.F01`; black-box: `L01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L02.md

# L02 · Stake registry, epochs, root membership and randomness binding

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R07, R08, R13, R31, R43, R44.  
**Integration GREEN dependencies:** L01, F05.  
**Ownership:** `contracts/NodeRegistry; contracts/EpochAnchor; crates/registry-proofs`.

## Observable outcome

Node registration, bond/unbond delays and epoch snapshots provide a verifiable participant set without downloading the whole registry; the seed is bound to a future agreed source.

## Tests first

1. The contract and reference model compute root/count and the active stake snapshot identically.
2. New stake does not change an already committed assignment; an early unbond does not remove a known commitment.

**Negative test:** Blockhash is not declared an unbiased random number; admissible influence and withholding are analyzed.

**Failure/race:** L1/L2 reorg, stale proof and snapshot delay are not accepted as a final new epoch.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The client verifies the chosen operator's proof against a specific finalized checkpoint.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L02.Txx`; negative: `L02.N01`; fault: `L02.F01`; black-box: `L02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L03.md

# L03 · SubsidyVault: time, shared fund and grant entitlement

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R15, R25, R27, R28, R41, R46, R47.  
**Integration GREEN dependencies:** L01, F05, I02.  
**Ownership:** `contracts/SubsidyVault; contracts/EntryBond; crates/subsidy-curve`.

## Observable outcome

The paid subsidy uses the same stamp format, a time-based schedule and global/epoch/credential limits; the sponsor pays for a bounded claim transaction. A separate optional EntryBond admits anonymous backed entry: entitlement is bound to lock_id, amount, term and withdrawal delay, not to the number of wallets.

## Tests first

1. Property tests verify bounded spend, monotonic decay, continuity/rounding and all epoch boundaries.
2. One credential yields a finite entitlement; an EntryBond cannot be withdrawn before the given lock/commitment ends or reused as a new right. Capital-farming of several bonds is modeled as residual economic risk.
3. A campaign admits only pinned trust-profile/version and provider namespaces; a login without an accepted eligibility policy yields no grant. Cross-provider stacking and different pairwise namespaces have explicit separate caps.

**Negative test:** A new wallet, a new owner secret, someone else's campaign and a repeated claim do not double one entitlement.

**Failure/race:** An empty fund causes an explicit subsidy refusal, not unbacked mint; purchased stamps keep working.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

In virtual time the free coverage smoothly declines to zero without an admin transaction/upgrade.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L03.Txx`; negative: `L03.N01`; fault: `L03.F01`; black-box: `L03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L04.md

# L04 · Non-governing organization share and economic immutability

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R16, R31, R39, R44.  
**Integration GREEN dependencies:** L01, L03, F05.  
**Ownership:** `contracts/FeeSplitter; contracts/RevenueVault`.

## Observable outcome

An immutable splitter routes the agreed fee share to the recipient; pull payments and recipient failure do not block service.

## Tests first

1. A full role/call-graph proves the absence of owner/proxy/pause/blacklist/arbitrary-call and rule changes by this address.
2. The fee base, rounding and repeated payout are verified by conservation invariants.

**Negative test:** Capturing the revenue key does not change the issuer, grants, validators, protocol version or fee schedule.

**Failure/race:** The recipient reverts/is unavailable: other flows work, its share accumulates for withdrawal.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The compromise-treasury test gets only the right to withdraw what is due, not to stop the network.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L04.Txx`; negative: `L04.N01`; fault: `L04.F01`; black-box: `L04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L05.md

# L05 · Operator payments and correct bounds of service proofs

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R14, R16, R28, R40, R43.  
**Integration GREEN dependencies:** L04, P05, D04.  
**Ownership:** `contracts/OperatorClaims; crates/service-receipts`.

## Observable outcome

Batch payout for backed transport services verifies receipts and quotas; slashing is allowed only for a formalizable violation.

## Tests first

1. Total payout does not exceed the backed budget minus the agreed share; a repeated proof is not paid twice.
2. One real delivery+repair scenario is decomposed into resources and payouts.

**Negative test:** A missed ping does not prove malicious intent; correlated retrieval does not prove a separate physical disk; inference does not grant the right to mint.

**Failure/race:** A recipient/operator conspiracy on fictitious traffic is modeled: the residual subsidy laundering risk is measured and bounded by the fund/limits.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The operator receives a testnet claim for a known expenditure; the report separately shows observed service and unproven properties.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L05.Txx`; negative: `L05.N01`; fault: `L05.F01`; black-box: `L05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/L06.md

# L06 · Working L2 adapter: finality, reorg and independent sources

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R13, R29, R32, R39, R44.  
**Integration GREEN dependencies:** L02, L03, L04, L05, P04.  
**Ownership:** `crates/l2-adapter`.

## Observable outcome

The adapter verifies the accepted finality/checkpoint profile, caches receipts/roots and survives an RPC change without moving plaintext on chain.

## Tests first

1. Replaying chain events restores a single state under a reorg within the permitted window.
2. Several RPCs telling the same lie do not count as a cryptographic light client.

**Negative test:** A receipt without verified domain/finality does not move budget/work into funded/settled.

**Failure/race:** The company RPC is down, one independent RPC lies, L2 is unavailable: safe offline bounds apply.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The node continues existing transport leases and honestly blocks an unproven new mint/settlement.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `L06.Txx`; negative: `L06.N01`; fault: `L06.F01`; black-box: `L06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P01.md

# P01 · Reusable BFT finalizer for bounded protocol logs

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R07, R11, R14, R32, R36, R44.  
**Integration GREEN dependencies:** F03, F04, F05, N01, N05.  
**Ownership:** `crates/finalizer`.

## Observable outcome

A proven consensus engine finalizes app-defined entries with durable quorum certificates; memberships and leases are set by L2 snapshots, not by the company.

## Tests first

1. 3f+1 / 2f+1 safety with <=f Byzantine is verified by model and simulation.
2. The external validity hook rejects an unauthorized state transition before voting.

**Negative test:** Two conflicting certificates for one sequence are not accepted; the quorum is not reduced for availability.

**Failure/race:** Partition, leader equivocation, withholding and crash/restart preserve safety and restore liveness when the assumptions hold.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Four test validators survive one Byzantine; this is a fixture, not an automatically chosen production quorum.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P01.Txx`; negative: `P01.N01`; fault: `P01.F01`; black-box: `P01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P02.md

# P02 · Private verifiable postage stamp and bounded spend domains

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R12, R14, R25, R27, R28.  
**Integration GREEN dependencies:** L01, P01, F06.  
**Ownership:** `crates/postage-proof; spec/postage-circuit`.

## Observable outcome

A finalized funded commitment issues bounded one-time tickets of a fixed resource class; the proof hides the chosen deposit within the real anonymity set.

## Tests first

1. Independent vectors verify membership, ticket index range, nullifier, domain/expiry and the authorization binding to the request.
2. The number of accepted tickets does not exceed the paid count; prover inputs do not change the dedup domain.

**Negative test:** A new salt, a different wallet, a different shard and proof replay do not create an extra spend; a fake ZK verifier is forbidden outside the test genesis.

**Failure/race:** Interrupting local proving does not reveal the seed or lose an already finalized stamp.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A local prover and a third-party verifier confirm the right to send without Google JWT/email and without revealing the original deposit.

## Outside the packet

Do not invent cryptographic primitives; the scheme, setup and backend are chosen in an ADR with independent review. Do not promise protection against timing analysis and a small anonymity set.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P02.Txx`; negative: `P02.N01`; fault: `P02.F01`; black-box: `P02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P03.md

# P03 · Atomic spend and admission certificate without double-spend

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R14, R27, R33.  
**Integration GREEN dependencies:** P01, P02, D02.  
**Ownership:** `crates/postage-spend`.

## Observable outcome

One ticket is allowed exactly one shard/epoch/lease; consensus atomically reserves the nullifier and issues a certificate for a specific operation commitment.

## Tests first

1. An identical retry returns the same certificate; a different payload with the same nullifier gets no admission.
2. Concurrent spends to different storage nodes/shards end with at most one final spend.

**Negative test:** A local spent set at one node does not count as global protection; incompatible spend domains are rejected.

**Failure/race:** A crash between reserve/store/commit/cancel yields no second certificate and no unbacked refund.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A hundred concurrent attempts to spend one stamp produce one permitted effect and verifiable rejections for the rest.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P03.Txx`; negative: `P03.N01`; fault: `P03.F01`; black-box: `P03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P04.md

# P04 · Epoch survival, reconfiguration and eventual autonomy from L2

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R14, R32, R39, R44.  
**Integration GREEN dependencies:** P03, L02, F03.  
**Ownership:** `crates/spend-handover`.

## Observable outcome

Spent state is handed to the next committee with a verifiable checkpoint and continuity; without a handover proof old tickets are not reissued.

## Tests first

1. An epoch transition does not reset nullifiers; the new committee accepts only the full agreed state.
2. During an L2 outage the active lease keeps spending; after hard expiry new admissions stop.

**Negative test:** The fallback 'no quorum — allow locally' is absent; losing a checkpoint does not create a fresh balance.

**Failure/race:** Old/new committee partition, rollback storage and ambiguous time verify safety fail-closed.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Disabling L2 for a test interval does not interfere with delivery in the active lease; at its end reading stored data remains available while new spends stop.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P04.Txx`; negative: `P04.N01`; fault: `P04.F01`; black-box: `P04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P05.md

# P05 · Resource pricing, reservation and shared repair budget

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R06, R14, R24, R28, R33, R41, R43.  
**Integration GREEN dependencies:** P03, D03, D06, N06, I02.  
**Ownership:** `crates/resource-accounting`.

## Observable outcome

The stamp class covers the specified bytes×TTL×replicas, permitted fanout, control overhead and repair allowance; payment is verified before durability is promised.

## Tests first

1. Conservation/property tests link admission, reservation, successful storage and correct cancellation.
2. The prepaid repair profile restores three lost copies with the sender offline.

**Negative test:** A million repeated reads, a repair loop and a shared group budget do not bypass quota/delegation.

**Failure/race:** Repair allowance exhaustion, a sudden price increase for a new class and partial failure give an honest status without hidden debt.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

UI/CLI shows the price before sending, staged spending and the remainder; one retry does not burn a second ticket.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P05.Txx`; negative: `P05.N01`; fault: `P05.F01`; black-box: `P05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/P06.md

# P06 · Attacks on stamps, privacy and backing as a shared regression suite

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R07, R12, R14, R27, R28, R36, R43.  
**Integration GREEN dependencies:** P04, P05, L05.  
**Ownership:** `tests/postage-adversarial`.

## Observable outcome

A separate adversary checks double-spend, front-running, circuit edge cases, grinding, deanonymization fixtures and artificial operator loops.

## Tests first

1. Mutation tests catch a removed domain check, wrong ticket binding and finality bypass.
2. The budget oracle reconciles mint/claims/fees/expiry across all traces.

**Negative test:** Self-transfer, fake work and colluding storage do not grant a direct user cash-out; residual operator laundering is not hidden.

**Failure/race:** Thousands of seeded partition/crash/reorg traces preserve asset/resource conservation within the model.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The report reproduces the worst found trace and explicitly separates cryptographic privacy from statistical leaks.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `P06.Txx`; negative: `P06.N01`; fault: `P06.F01`; black-box: `P06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O01.md

# O01 · Native Google OAuth with local JWT verification

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R25, R26, R38, R45, R46.  
**Integration GREEN dependencies:** I01, F02, F06.  
**Ownership:** `crates/auth-google; crates/oauth-native`.

## Observable outcome

A system browser flow with PKCE/state/nonce binds login to the controlled owner key; the JWT is verified by iss/sub/aud/exp and cacheable Google JWKS.

## Tests first

1. Fixtures verify both documented iss, the exact sub, the allowed aud, nonce and timing.
2. The JWKS cache follows expiry/rotation and does not call Google for every message.

**Negative test:** Browser callback substitution, an ID token of another OAuth app, a stale token and a missing owner challenge are forbidden.

**Failure/race:** Google unavailable, user cancelled login, client_id blocked or JWKS rotated: local identity is not lost.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A consented live smoke test goes through the system browser; CI uses an explicitly labeled test OIDC provider.

## Outside the packet

Do not put provider credentials in the Tauri webview; do not require Google for NetworkID ownership or root recovery.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O01.Txx`; negative: `O01.N01`; fault: `O01.F01`; black-box: `O01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O02.md

# O02 · Common ExternalCredential and TrustProfile for external networks

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R21, R25, R26, R29, R46, R47.  
**Integration GREEN dependencies:** F02, I02.  
**Ownership:** `crates/credential-types; spec/trust-profiles`.

## Observable outcome

A versioned TrustProfile describes the upstream issuer, subject namespace, accepted audiences, verifiable claims, signer set/k, TTL/revocation and an independent grant policy. ExternalCredential binds an assertion to a NetworkID; there is no Google code in the network type.

## Tests first

1. The same verifier contract accepts valid Google/Telegram/organization fixtures and preserves issuer/source/type without strengthening claims.
2. 1-of-1 and k-of-n modes are explicitly distinct; changing signer/client/version does not create a new subject within the pre-agreed campaign namespace.

**Negative test:** The name 'government organization', an OAuth access token or a pretty domain by themselves do not create personhood/model/unlimited trust.

**Failure/race:** An expired/revoked profile changes acceptance of new credentials but not NetworkID ownership, old reviews or lawfully paid stamps.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A new test issuer plugs in as a profile/adapter without changing the wire transport and is displayed with its real authorities.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O02.Txx`; negative: `O02.N01`; fault: `O02.F01`; black-box: `O02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O03.md

# O03 · Bounded attesters: a single organization issuer or k-of-n

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R25, R26, R31, R44, R46.  
**Integration GREEN dependencies:** O02, I02, F06.  
**Ownership:** `crates/credential-attestors; spec/attestor-profiles`.

## Observable outcome

A single site/organization issuer or a selected k-of-n set verifies provider evidence and the owner challenge and issues a scoped credential. V1 allows explicit 1-of-1; the transport BFT/stake registry does not appoint login attesters automatically.

## Tests first

1. A 1-of-1 profile issues and verifies a credential only in a voluntarily accepted scope; k-of-n requires the necessary number of distinct valid signer IDs.
2. Signatures from different policy epochs/issuers are not mixed in a threshold; the upstream JWK checkpoint, version and validity periods are verified.

**Negative test:** An issuer may assert only the accepted claims of its profile; it cannot change a NetworkID, read E2EE, delete reviews, mint without funding or control consensus.

**Failure/race:** An unavailable/compromised issuer stops or compromises only its own trust/grant branch within the fund; the rest of the network does not depend on it.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

One organization issuer and a separate k-of-n fixture are launched; the client shows the difference in trust assumptions without masking centralization.

## Outside the packet

Do not declare a single organization issuer decentralized and do not require an independent quorum where the user voluntarily chose a single-issuer profile.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O03.Txx`; negative: `O03.N01`; fault: `O03.F01`; black-box: `O03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O04.md

# O04 · Provider-scoped grant: deduplication, campaigns and cross-provider limits

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R14, R25, R27, R28, R46, R47.  
**Integration GREEN dependencies:** O02, O03, L03, P03.  
**Ownership:** `crates/credential-claims`.

## Observable outcome

A GrantClaim is bound to the campaign and a confirmed subject namespace. Wallet/device/attestor/client change without repeated entitlement where subject stability is proven; for a pairwise identity one campaign client/sector or a verifiable mapping is allowed. Unrelated Google/Telegram identities are not declared one person.

## Tests first

1. One Google subject across accepted client IDs, wallets and attesters yields one entitlement.
2. Telegram/site subject fixtures verify public/pairwise namespace policy; an unconfirmed cross-client mapping does not expand the subsidy.
3. Linking several providers to one NetworkID does not increase the fixed campaign cap; different NetworkIDs/real accounts remain a bounded Sybil risk.

**Negative test:** One must not change the campaign/issuer namespace/salt/profile version to re-spend the same right; cash-out from login is not allowed.

**Failure/race:** Two simultaneous claims to different gateways/attesters are processed by one consistent claim state without double issuance.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The claims table explains which provider/profile granted the right, which limits fired and why global human uniqueness is not promised.

## Outside the packet

A hash subject is a pseudonym, not absolute anonymity; subjects of different providers are not declared one physical human.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O04.Txx`; negative: `O04.N01`; fault: `O04.F01`; black-box: `O04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O05.md

# O05 · Freshness, revocation and unlinking of external providers without losing the address

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R12, R25, R26, R27, R38, R44, R46, R47.  
**Integration GREEN dependencies:** O04, I03.  
**Ownership:** `crates/credential-lifecycle`.

## Observable outcome

Google/Telegram/site bindings have an independent lifecycle. Refresh/revoke/unlink do not change the NetworkID and do not reset campaign spent state; profiles and JWKS update within their explicit policy boundaries.

## Tests first

1. Email/username/phone are not used as a stable subject; changing display attributes does not yield a new grant.
2. Unlink/relink, attester change and key rotation preserve the address, correspondence, reviews and the used right.

**Negative test:** A single issuer does not get recovery authority over the root; unreliable cross-provider linking and account-age are rejected.

**Failure/race:** JWKS rotation, a blocked bot/OAuth client and an offline issuer cause a scoped failure, not a network-wide block.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user unlinks all external accounts and continues with the same key-based address; the UI shows credential history and current freshness.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O05.Txx`; negative: `O05.N01`; fault: `O05.F01`; black-box: `O05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O06.md

# O06 · Free start under an explicitly accepted trust profile

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R15, R25, R27, R28, R41, R43, R46, R47.  
**Integration GREEN dependencies:** O04, O05, O01, O07, O08, L03, P05.  
**Ownership:** `crates/onboarding-credit; tests/oidc-abuse`.

## Observable outcome

Google, Telegram and organization credentials connect to a separate allowlisted funded campaign policy. Having a login does not guarantee a grant; when the fund exists the sponsor covers limited claim gas and the first transport tickets.

## Tests first

1. For each enabled devnet campaign branch a user without a crypto balance sends the first message; the issuer itself does not create unbacked stamps.
2. Global/campaign/provider/network caps and time-decay hold across several linked and unlinked accounts.

**Negative test:** The UI does not promise an identical grant for any OAuth, cash-out or unlimited stacking of Google+Telegram.

**Failure/race:** Fund, sponsor, gateway or L2 unavailable: the error explains the specific branch and preserves the working profile/old stamps.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Three devnet profiles show the real payer, remaining resource budget and the optionality of external trust.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O06.Txx`; negative: `O06.N01`; fault: `O06.F01`; black-box: `O06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O07.md

# O07 · Organization issuer and site OAuth/OIDC gateway

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R25, R26, R38, R44, R46.  
**Integration GREEN dependencies:** O02, O03, I02.  
**Ownership:** `apps/attestation-gateway; crates/auth-organization`.

## Observable outcome

A deployable site gateway accepts a challenge from the daemon, runs OIDC or the organization's own authentication and issues a credential with provable claims. It also fits an explicitly trusted government organization; connecting a specific government system is not assumed without its configuration/access.

## Tests first

1. A local test OIDC issuer and a native site-account profile pass the full challenge → auth → signed credential → verify.
2. The OAuth secret is stored only in the gateway; the session is bound to the owner public key, request digest, origin and expiry; the daemon proves possession.

**Negative test:** An access token/site account does not turn into a government ID; issuer substitution, SSRF via discovery and callback swapping are rejected.

**Failure/race:** The gateway crashes after callback: a one-time encrypted handoff/poll issues the credential only to the challenge owner; a retry does not create a new grant.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

An independent operator deploys their own site issuer, the user voluntarily accepts the profile, then disables it and continues chatting.

## Outside the packet

Do not implement integration with an unnamed government IdP; V1 ships a working generic adapter and reference issuer, not a fictitious government approval.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O07.Txx`; negative: `O07.N01`; fault: `O07.F01`; black-box: `O07.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/O08.md

# O08 · Telegram Login/OIDC with safe desktop handoff

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R25, R26, R27, R38, R46, R47.  
**Integration GREEN dependencies:** O02, O03, O07.  
**Ownership:** `crates/auth-telegram; apps/attestation-gateway/telegram`.

## Observable outcome

The Telegram adapter uses the system browser and a registered website callback. The OAuth exchange goes to the gateway with the client secret; signed provider claims are bound to the owner challenge. Tauri receives only a one-time handoff handle; the credential is issued after proof of possession.

## Tests first

1. Official-flow fixtures cover issuer/audience/signature/expiry, state/PKCE, replay, code/redirect substitution and owner binding; only permitted algorithms.
2. The default scope is minimal; no automatic phone/profile/bot-write request. The exact sub in a verified namespace is used, not username/phone.
3. A live smoke test on a consented account verifies the BotFather config and the real callback; unconfirmed provider specifics are gated, not guessed.

**Negative test:** Client secret/bot token/JWT do not end up in the frontend bundle, URI, crash log or MCP. Telegram Mini App initData and the legacy widget are not accepted as OIDC without a separate adapter.

**Failure/race:** Bot blocked, gateway restart, stolen handoff handle, repeated callback and JWKS rotation cause scoped failure without a NetworkID takeover.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The Telegram button in desktop creates a binding to one's own NetworkID; subsequent unlinking does not break messages and does not return the entitlement.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `O08.Txx`; negative: `O08.N01`; fault: `O08.F01`; black-box: `O08.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A01.md

# A01 · Signed service cards and decentralized discovery

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R18, R21, R22, R30, R48, R51.  
**Integration GREEN dependencies:** I02, I04, N03.  
**Ownership:** `crates/service-directory`.

## Observable outcome

ServiceCard binds owner/agent/service/runtime epochs, skills, endpoint, price terms and provenance claims; publication is optional. The card separates claimed properties from reviews and contains a review-discovery namespace without a pointer to a single executor directory.

## Tests first

1. The card's signature and validity are verified; a private card is found only by capability/invite.
2. The service epoch distinguishes a changed model or executor version.

**Negative test:** A model=Fable claim without attestation stays self-declared; a DHT record does not raise trust.

**Failure/race:** Runtime/multiaddr change and loss of directory replicas do not change the agent's address.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

MCP/CLI finds two executors without a mandatory company directory and shows verifiable claims separately from advertising.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A01.Txx`; negative: `A01.N01`; fault: `A01.F01`; black-box: `A01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A02.md

# A02 · Signed job offers and immutable terms

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R19, R20, R23, R29, R48, R49, R52.  
**Integration GREEN dependencies:** F02, I02, D02.  
**Ownership:** `crates/job-contracts`.

## Observable outcome

RFQ/bid/accept fix an immutable terms_hash, service/owner epochs, criteria, price/asset, transport budget and deadline. An accepted standard order contains a bilaterally signed minimal AcceptedOrderReceipt with the customer's mandatory right to a public review; raw inputs/outputs are not published.

## Tests first

1. Both sides sign one terms_hash and review disclosure header; changing price/service/terms requires a new order_id/acceptance.
2. The review right cannot be turned off by a provider flag, made dependent on payment/completion or obtained from a single RFQ.
3. Money amount and transport budget are separate; the public header contains a salted private terms commitment, not the prompt itself.

**Negative test:** A sent 'paid' text and a self-signed settlement receipt do not create funded status.

**Failure/race:** Concurrent accept/cancel is deterministic; the client considers the order accepted only after durable storage of both signatures and the review receipt.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The customer accepts one bid and stores a receipt usable for independent review publication without revealing private materials.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A02.Txx`; negative: `A02.N01`; fault: `A02.F01`; black-box: `A02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A03.md

# A03 · Async work, result, validation and separate payment status

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R17, R19, R20, R29, R34, R48, R49, R52.  
**Integration GREEN dependencies:** A02, D05, D06.  
**Ownership:** `crates/job-runtime-state`.

## Observable outcome

The async job reducer separates provider progress/ExecutorDeclaration, customer acceptance/validation, timeout/cancel and settlement. A missing result or dispute does not take away the previously arisen review right.

## Tests first

1. An executor offline period, heartbeat lease and repeated result do not duplicate completion.
2. Artifact hash and validation receipt are checked against the original acceptance criteria.
3. ExecutorDeclaration binds order/terms/output digest; it only claims execution. After a timeout/non-delivery the customer keeps review eligibility.

**Negative test:** A repeated result does not repeat the side effect; a self-signed result does not make payment, independent provenance or customer satisfaction confirmed.

**Failure/race:** A crash after execution but before sending the result restores a single result reference and continues delivery.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The executor goes offline, returns and delivers the artifact; the customer independently records acceptance of the result.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A03.Txx`; negative: `A03.N01`; fault: `A03.F01`; black-box: `A03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A04.md

# A04 · Future delegation contract without an economic engine in V1

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-contract-only.  
**Requirements:** R17, R19, R23, R24, R38, R41, R52.  
**Integration GREEN dependencies:** A03, I02, P05.  
**Ownership:** `crates/job-policy; spec/future-economic-agents`.

## Observable outcome

V1 validates no_subcontract, data scopes, budget/depth and versioned extensions; unsupported automatic subcontract/settlement is explicitly rejected. Built-in make-or-buy, price optimization and subcontract chains are implemented in V2+ and are not a release prerequisite.

## Tests first

1. Terms with no_subcontract/data scope/budget pass the public parser and broker; a forbidden extension grants no additional rights.
2. A request for automatic make-or-buy/subcontract settlement returns unsupported without spending; an ordinary order via MCP works independently.

**Negative test:** One must not mask an economic engine call with a fictitious success or extend agent access through unknown terms.

**Failure/race:** Repeat/restart/incompatible extension version do not create money, new subjobs or a budget bypass.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A separate contract test demonstrates the future engine's plug-in seam and a safe failure; V1 passes a basic order without the engine.

## Outside the packet

V1 does not implement autonomous make-or-buy, economic optimization, underwriting or multi-party subcontract settlement.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A04.Txx`; negative: `A04.N01`; fault: `A04.F01`; black-box: `A04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A05.md

# A05 · Executor declaration and customer validation without false attestation

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R20, R21, R22, R29, R31, R48, R49, R52.  
**Integration GREEN dependencies:** A03, I03.  
**Ownership:** `crates/work-evidence; crates/verifier-dispatch`.

## Observable outcome

ExecutorDeclaration and CustomerValidation are distinct signed types. The former contains order/terms/result digest and the claimed model/runtime; the latter records the customer's own checks. Independent provenance/settlement is stored only through an explicit adapter; a missing verifier answers unsupported.

## Tests first

1. The executor's signature binds the result to the immutable terms; changing digest/epoch/service breaks verification.
2. Customer tests, a public review, a Google/Telegram credential and an executor claim cannot yield independently_verified_model.
3. A strict external-provenance order without a verifier is rejected before execution; an ordinary result/declaration order passes.

**Negative test:** The word verified, a high rating, a fingerprint and a self-signature do not turn into proof of internal execution or payment.

**Failure/race:** The runtime disappears after the declaration: the signed statement and the customer receipt remain available; the absence of a third party does not block the review.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A single screen/API separately shows 'claimed by executor', 'validated by customer', 'review' and 'no independent proof'.

## Outside the packet

TEE/zkML, independent judges/assessors and economic agents in V2+; V1 does not promise to objectively attest an arbitrary result.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A05.Txx`; negative: `A05.N01`; fault: `A05.F01`; black-box: `A05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/A06.md

# A06 · A2A adapter with verifiable compatibility

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R18, R19, R29, R49, R52.  
**Integration GREEN dependencies:** A01, A03, A04, F06.  
**Ownership:** `crates/a2a-adapter`.

## Observable outcome

The adapter maps the agreed Agent Card/Task/Message/Artifact and async states between A2A and the native E2EE job transport.

## Tests first

1. Official wire fixtures and an independent client pass negotiation and lifecycle.
2. Unmappable extensions are explicitly declared, not lost on roundtrip.
3. A2A task completion does not automatically grant a review right: the adapter must form a bilateral receipt or state the absence of a confirmed protocol order.

**Negative test:** Private fields/credential secrets do not end up in the public Agent Card; a transport envelope is not labeled A2A by itself.

**Failure/race:** Reconnect, task-id mapping collision and duplicate events preserve idempotency.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

An external A2A client interacts with the agent through a user-run adapter without a company server.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `A06.Txx`; negative: `A06.N01`; fault: `A06.F01`; black-box: `A06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q01.md

# Q01 · Confirmed order and the customer's independent review right

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R19, R30, R49, R52.  
**Integration GREEN dependencies:** A02, I02, F04.  
**Ownership:** `crates/order-receipts; spec/review-right`.

## Observable outcome

An AcceptedOrderReceipt with customer/provider signatures creates a non-transferable ReviewRight verifiable by any node. The receipt contains domain/order_id/customer/provider/service+epochs, a salted terms commitment and a public disclosure policy; signer authority is bound to the acceptance epoch.

## Tests first

1. Another client does not get the right; replay across orders/services/networks and public header modification break verification.
2. The right is available immediately after bilateral acceptance and a durable receipt; result/payment/provider approval are not required.
3. Only the minimal receipt is published; private terms/artifacts cannot be recovered from an unsalted low-entropy hash.

**Negative test:** An RFQ without acceptance, a fake provider signer, cancellation before acceptance and a self-only receipt have no verified-order eligibility.

**Failure/race:** Executor offline/deleted the card/closed the service after acceptance: the customer independently presents the receipt and publishes a negative review.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

CLI verify-review-right verifies the order without polling the executor and returns exactly the fact of a confirmed order, not payment/quality.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q01.Txx`; negative: `Q01.N01`; fault: `Q01.F01`; black-box: `Q01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q02.md

# Q02 · Signed reviews, amendments, publication withdrawal and executor replies

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R38, R48, R49, R50.  
**Integration GREEN dependencies:** Q01, A05, F02.  
**Ownership:** `crates/review-events; spec/review-events`.

## Observable outcome

A ReviewEvent rates a specific service on a 1–5 scale and contains text, receipt proof and an optional declared outcome. Amendments/withdrawal/replies are new signed events; one effective customer review per order/customer, and a provider reply does not change the customer's score.

## Tests first

1. Create/amend/withdraw/reply verify roles, domain, limits and authorization epoch; repeating one event is idempotent.
2. Amendments form a verifiable prev_event/revision+1 chain; concurrent branches are visible and have a deterministic tie-break by canonical event_id.
3. Failure, a missing result, refusal/timeout are allowed in a review of an accepted order; the author's outcome is not called an objective verdict.

**Negative test:** The executor does not edit/delete the customer's review; HTML/script/oversized text are not executed; a reply does not add a second score.

**Failure/race:** Concurrent edits from two devices and reorder converge to one version given the same event set; a missing parent stays pending until received.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The customer writes a negative review, amends it; the executor replies; another customer reproduces the history without a central moderator.

## Outside the packet

Withdrawal is a new record, not guaranteed erasure of other people's copies. There is no automatic refund/penalty from the rating.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q02.Txx`; negative: `Q02.N01`; fault: `Q02.F01`; black-box: `Q02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q03.md

# Q03 · Distributed publication and discovery of reviews

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R06, R14, R30, R33, R50.  
**Integration GREEN dependencies:** Q02, D05, D06, N03, P05.  
**Ownership:** `crates/review-store; crates/review-index`.

## Observable outcome

Public review bundles and content-addressed events are placed under a paid replication/repair/TTL policy. The service+owner epoch index has independent keepers; any client verifies signatures/receipts and merges sources. A publisher is not obliged to use the executor directory.

## Tests first

1. Reviews are discovered via several peer/index paths after the author and executor go offline; repair restores the available paid copies.
2. The index stores bounded pointers with receipt verification; stale/duplicate/forged records do not add extra scores.

**Negative test:** An executor directory outage, the company or a negative score do not grant the right to delete a valid review from other people's copies; publication does not reveal raw order input.

**Failure/race:** Censorship by one index, partition, loss of copies and TTL expiry show partial/freshness/retention; unavailability does not turn into 'no reviews'.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A third independent client finds the published negative review after provider and company hosts go offline.

## Outside the packet

No promise of eternal free storage, a complete worldwide list or censor-resistance in the absence of an honest path/funding.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q03.Txx`; negative: `Q03.N01`; fault: `Q03.F01`; black-box: `Q03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q04.md

# Q04 · Reproducible rating over a known review set

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R21, R48, R50, R51.  
**Integration GREEN dependencies:** Q02, Q03, A01.  
**Ownership:** `crates/rating-reducer; fixtures/ratings`.

## Observable outcome

A pure versioned reducer builds count, sum, histogram[1..5], the mean as a rational number and distinct customer count over the effective eligible reviews. RatingSnapshot includes ServiceID/service+owner epochs, policy_version, dataset hash, source cursors and incomplete status.

## Tests first

1. Event permutation, duplicates and replicas do not change the result; an amendment replaces the score, a withdrawal excludes the current score, a reply is not counted.
2. The same corpus yields the same dataset hash/result on all platforms; zero reviews means no_rating, not zero stars.
3. An owner/service change shows separate epochs; a Google/Telegram credential gives no weight multiplier.

**Negative test:** Someone else's orders, out-of-range scores, a self-review by one identity and unverified receipts are excluded; wash-orders by different colluding identities are not declared solved.

**Failure/race:** A hidden or stale index and different event sets yield explicitly different partial snapshots; the reducer does not claim global consensus.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two independent clients import one corpus and get the same rating; the user sees the number of orders, customers, period and limitations.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q04.Txx`; negative: `Q04.N01`; fault: `Q04.F01`; black-box: `Q04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q05.md

# Q05 · Review publicness, privacy preview and local filters

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R12, R30, R38, R50, R51.  
**Integration GREEN dependencies:** Q02, Q03, I02.  
**Ownership:** `crates/review-policy; spec/review-privacy`.

## Observable outcome

Before publication the specific disclosed fields and the hash of the review being signed are shown. Automatic uploading of private artifacts, JWT, emails/phone, decrypted orders is forbidden. Local block/filter/report policies change visibility but do not forge the signature/history/base aggregate.

## Tests first

1. Draft/private text does not leave the device until an allowed publish; the preview and the signature are bound by one immutable action hash.
2. A locally hidden review is marked as filtered; the base observed aggregate and the filtered view are not presented as one set.

**Negative test:** Prompt injection from a result does not get review.publish or private attachment disclosure; an arbitrary external link does not trigger a download/code.

**Failure/race:** A race between editing and confirmation requires renewed consent; a crash after publish displays the durable status without false deletion.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user sees the minimal public footprint, publishes the review and understands that withdrawal does not erase already received copies.

## Outside the packet

There is no central global deletion/censor. Operators' legal policies are disclosed separately, not passed off as network-wide cryptographic truth.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q05.Txx`; negative: `Q05.N01`; fault: `Q05.F01`; black-box: `Q05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/Q06.md

# Q06 · Adversarial review suite: Sybil, replay and censorship

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R36, R49, R50, R51, R52.  
**Integration GREEN dependencies:** Q01, Q02, Q03, Q04, Q05, F06.  
**Ownership:** `tests/reviews/adversarial; spec/models/reviews`.

## Observable outcome

An independent oracle attacks entitlement, duplicate/amendment accounting, cross-epoch replay, censorship and UI exaggerations. With colluding wash-orders it shows the residual risk instead of a false Sybil-proof rating.

## Tests first

1. Removing the second-signature/author/order/service epoch check or dedup is caught by the mutation corpus.
2. One order with 100 repeated events does not give 100 votes; the customer can leave a negative review when the executor refuses/is offline.

**Negative test:** The rating does not turn a self-declared model into independently verified and does not trigger a penalty/settlement; a missing complete set is not hidden.

**Failure/race:** Partition+concurrent edits+lying index+key rotation converge after heal given the same corpus; a smaller set stays partial.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A machine-readable adversarial report is published with the caught attacks and the explicitly unfixed possibility of collusion between different identities.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `Q06.Txx`; negative: `Q06.N01`; fault: `Q06.F01`; black-box: `Q06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M01.md

# M01 · Local MCP server with narrow authority

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R17, R29, R38, R41.  
**Integration GREEN dependencies:** I02, F02, F06.  
**Ownership:** `crates/mcp-server; crates/local-ipc`.

## Observable outcome

stdio MCP uses the official Rust SDK and a single shared authorization broker; tool schemas separate read, send, approve and delegated spending. The common identity.get API returns one's own NetworkID, permitted rights and separate AgentID/RuntimeID if assigned; the CLI uses the same access boundary.

## Tests first

1. The official client passes discovery/version negotiation for the new profile and separately the legacy initialize of the supported previous profile; permitted calls are equally limited.
2. Each tool requires a specific principal/capability; stdout contains only MCP frames.
3. Within a scoped connection context identity.get returns the public NetworkID and one's own actor context without keys or other people's grants; RuntimeID/ConversationID do not substitute the recipient address.

**Negative test:** No root-sign, export-secrets, arbitrary-shell or unrestricted wallet RPC.

**Failure/race:** An invalid frame, someone else's local principal and a cancelled call do not leave an authorized background operation.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The agent gets the list of available tools and reads only its own inbox.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M01.Txx`; negative: `M01.N01`; fault: `M01.F01`; black-box: `M01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M02.md

# M02 · Inbox/outbox, acknowledgment and wake-up of a running runtime

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R05, R10, R17, R24, R33.  
**Integration GREEN dependencies:** M01, D05, P05, G05.  
**Ownership:** `crates/mcp-messaging`.

## Observable outcome

Tools send/poll/ack, group create/invite/members/send and an allowed subscription provide a durable cursor, lease, dedup, pagination and backpressure; the launching process is responsible for LLM wake-up. CLI and MCP call the common messages.send by the permitted recipient's public NetworkID; address resolution and opening the first conversation are done by the daemon under the contact policy. Polling is the baseline; the specific hook/subscription for a running host is chosen before implementation and preserves the durable queue.

## Tests first

1. An MCP client restart continues the inbox without losing unacknowledged messages.
2. Send with the same idempotency_key returns the previous result.
3. A first message to a permitted NetworkID works without a pre-created ConversationID: the recipient sees the text, replies; CLI poll/ack checks content, MessageID and recovery after restart.

**Negative test:** A subscription is not declared as delivery to a powered-off process; an unread event does not execute the agent by itself. A scoped handle does not allow sending to another recipient outside the grant; session expiry or revocation forbids further CLI and MCP calls.

**Failure/race:** stdio disconnect, a slow client and cancel await do not lose events or budget.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A Hermes-like runtime shuts down, then poll fetches the accumulated work and acknowledges exactly the processed events.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M02.Txx`; negative: `M02.N01`; fault: `M02.F01`; black-box: `M02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M03.md

# M03 · MCP for jobs, artifacts and dangerous-action approval

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R19, R23, R34, R38, R41, R48, R49, R52.  
**Integration GREEN dependencies:** M01, A03, A04, A05.  
**Ownership:** `crates/mcp-jobs`.

## Observable outcome

Tools jobs.* manage the signed order lifecycle, return a durable AcceptedOrderReceipt, ExecutorDeclaration and the customer's own checks. The broker limits budget/data/roles; there is no built-in economic engine.

## Tests first

1. The approval preview is bound to the exact action hash/price/recipient/expiry.
2. Fetching an artifact verifies job participation and the granted data scope.
3. The receipt can be fetched again after restart; jobs.validate means the customer's claim/check, not independent attestation.

**Negative test:** Prompt injection in a result/document does not confirm a purchase, read someone else's inbox or run code by the daemon process.

**Failure/race:** Repeated approval, changing parameters after preview and a cancel race do not bypass the limit.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The agent accepts a limited task, sends the result and stops at the request for a separate payment right.

## Outside the packet

Do not expand the public API and authority beyond the task contract. Automatic economic strategy and multi-party subcontract settlement are not part of V1; authority limits remain mandatory.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M03.Txx`; negative: `M03.N01`; fault: `M03.F01`; black-box: `M03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M04.md

# M04 · Service discovery and evidence reading via MCP without data leaks

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R18, R21, R22, R30, R38, R48, R51.  
**Integration GREEN dependencies:** M01, A01, A05, Q04.  
**Ownership:** `crates/mcp-discovery`.

## Observable outcome

Paged tools/resources return cards, executor declarations, credentials and RatingSnapshot with dataset/policy/source/partial metadata; untrusted text stays data.

## Tests first

1. The skills/provenance filter does not raise the card's credibility and respects the local privacy policy.
2. Finding a private service requires an invite capability.

**Negative test:** An external card does not become a system instruction and does not reveal members of closed groups.

**Failure/race:** A malicious directory peer serves a large/cyclic/stale object: a bounded response with a refusal.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The agent reads confirmed orders/reviews separately from advertising claims and can refuse unsupported strict provenance.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M04.Txx`; negative: `M04.N01`; fault: `M04.F01`; black-box: `M04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M05.md

# M05 · Multiple runtimes of one agent and safe switching

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R17, R23, R38, R41, R52.  
**Integration GREEN dependencies:** M02, M03, I03.  
**Ownership:** `crates/agent-runtime-leases`.

## Observable outcome

Runtime leases, grants and outbox fencing prevent simultaneous inconsistent execution of one task and exceeding the shared budget.

## Tests first

1. Two runtimes compete for a job lease: one executes, the other observes/waits.
2. Revoke/epoch bump cuts off the old runtime's new rights.

**Negative test:** An old lease and a restored backup do not get the right to repeat an external effect.

**Failure/race:** Lease owner crash, clock skew and split brain do not lead to an exactly-once promise where the external service does not support it.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The agent moves from a laptop to a headless node keeping the address and queue; possible external duplicate effects are handled explicitly with idempotency keys.

## Outside the packet

Do not expand the public API and authority beyond the task contract. Automatic economic strategy and multi-party subcontract settlement are not part of V1; authority limits remain mandatory.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M05.Txx`; negative: `M05.N01`; fault: `M05.F01`; black-box: `M05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M06.md

# M06 · Shipped agent skill with CLI, MCP integration and optional HTTP

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R17, R18, R38, R42, R47, R49, R50.  
**Integration GREEN dependencies:** M02, M03, M04, M05, A06, M07.  
**Ownership:** `integrations/mcp; crates/mcp-http; integrations/agent-skill; apps/cli (agent adapter)`.

## Observable outcome

A working client config, an executable conformance scenario and a stdio-first integration are shipped; optional Streamable HTTP listens on loopback with auth/origin checks. A ready agent SKILL.md, connection context examples and working CLI commands identity/send/delivery/poll/ack with versioned JSON, exit codes and stderr diagnostics are mandatory. A host without MCP handles basic messaging using only the skill and the issued handle; core rules and the broker are reused.

## Tests first

1. A client of the current MCP version and the supported previous profile passes the whole workflow.
2. HTTP tokens are bound to audience/session and the same capabilities as stdio.
3. The shipped integration shows reviews.publish/list/rating and a separate permission for public publication.
4. A black-box host with the shipped skill and connection context via CLI learns its ID, writes to another NetworkID, reads and acknowledges the reply, continues after restart; JSON/exit codes are checked by an external process without internal APIs.
5. A separate smoke test with a real agent host confirms the skill instructions suffice for CLI messaging; a deterministic fixture alone does not prove this.

**Negative test:** DNS rebinding, someone else's Origin, token passthrough and session hijack do not allow calling tools.

**Failure/race:** A daemon/client restart preserves tasks; an incompatible protocol gives a comprehensible refusal.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user connects their Hermes/MCP runtime by configuration, sends work and receives the result without manually calling internal APIs. Additionally an agent host without MCP receives the skill and a scoped handle, passes ID → send → poll → ack via the shipped CLI; the joint acceptance of E11/E03 checks the same exchange behind NAT.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M06.Txx`; negative: `M06.N01`; fault: `M06.F01`; black-box: `M06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/M07.md

# M07 · MCP for reviews, replies and rating

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R38, R49, R50, R51.  
**Integration GREEN dependencies:** M01, M03, Q04, Q05.  
**Ownership:** `crates/mcp-reviews`.

## Observable outcome

reviews.eligibility/publish/amend/withdraw/reply/list/get and ratings.get call the same Rust broker/reducers as desktop. The read scope is separate from publication; automatic publication requires a pre-issued exact grant or owner confirmation.

## Tests first

1. A client with its own order receipt publishes a review; the provider may reply; another runtime/buyer gets a refusal.
2. Calls are idempotent; schemas return the public footprint, known dataset and the rating's incompleteness.

**Negative test:** An incoming review/artifact with an instruction to give five stars or disclose the order does not get the publish scope.

**Failure/race:** Disconnect/restart after publish does not create a new score and restores event status/cursor.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

An MCP customer goes through order → executor declaration → own score → public review; an independent agent reads the same rating.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `M07.Txx`; negative: `M07.N01`; fault: `M07.F01`; black-box: `M07.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U01.md

# U01 · Tauri desktop: profile, contacts and personal messaging

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R04, R05, R35, R38, R45.  
**Integration GREEN dependencies:** I01, D05, F04, U07.  
**Ownership:** `apps/desktop/src; crates/desktop-viewmodels`.

## Observable outcome

A Tauri 2 shell with a bundled web frontend calls a typed Rust bridge → authenticated daemon IPC → common broker. The UI shows messages/durability/read-state; the daemon keeps running after the window is closed per the setting.

## Tests first

1. GUI acceptance creates a profile, a contact, sends and reads a message after restart.
2. Closing the window does not stop headless delivery when that mode is selected.

**Negative test:** A webview without the required capability or another local user does not get access to profile data; neither root keys nor OAuth secrets are stored in the renderer.

**Failure/race:** A UI crash does not corrupt the journal; an unavailable daemon gives a recoverable state.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Two desktop clients converse without a developer console or centralized backend.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U01.Txx`; negative: `U01.N01`; fault: `U01.F01`; black-box: `U01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U02.md

# U02 · Onboarding: own keys, Google, Telegram and organization trust

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R14, R15, R25, R28, R35, R41, R45, R46, R47.  
**Integration GREEN dependencies:** U01, O06, P05, L04.  
**Ownership:** `apps/desktop/onboarding; apps/desktop/budget`.

## Observable outcome

Tauri UX offers an independent profile/purchase, Google, Telegram and a site/organization issuer. It shows whom the user trusts (1-of-1 or k-of-n), which claims are disclosed and separately eligibility/grant/transport budget.

## Tests first

1. Each login button opens the system browser; callback/credential are bound to the current profile and a one-time challenge.
2. Login success without a funded campaign does not show money; binding, fresh credential and grant are separate statuses.

**Negative test:** No secrets in frontend/URL and no claims that 'the government guarantees quality' from a site account; automatic grant stacking is not allowed.

**Failure/race:** Provider/gateway/sponsor/L2 unavailability does not destroy the profile or break ordinary key-based login.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user logs in via Telegram or Google, keeps their own NetworkID and unlinks the external account without losing history.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U02.Txx`; negative: `U02.N01`; fault: `U02.F01`; black-box: `U02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U03.md

# U03 · Full group UX, device management and privacy profile

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R10, R11, R12, R24, R35.  
**Integration GREEN dependencies:** U01, G04, G05, G06.  
**Ownership:** `apps/desktop/groups`.

## Observable outcome

The UI supports roles, invitations, device leaves, pending/final membership, offline catch-up and routing privacy selection with a price explanation.

## Tests first

1. An automated GUI scenario creates a group, invites, removes and restores a device.
2. Remove pending and Remove final are displayed differently.

**Negative test:** No promise of instant removal during a partition or full anonymity of private pointers.

**Failure/race:** Control log loss and rejoin-required do not hang forever: there is a safe comprehensible flow.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The group is used by humans and agents with the same access model.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U03.Txx`; negative: `U03.N01`; fault: `U03.F01`; black-box: `U03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U04.md

# U04 · Panel of agents, jobs, trust and permissions

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R19, R21, R22, R23, R35, R38, R41, R45, R48, R52.  
**Integration GREEN dependencies:** U01, M03, M04, A05.  
**Ownership:** `apps/desktop/agents`.

## Observable outcome

The Tauri panel shows runtime grants, service cards, signed orders, ExecutorDeclaration, customer checks and a separate settlement state. Approval is bound to a specific action hash; no economic decision engine is shipped.

## Tests first

1. An approve click signs exactly the displayed immutable action hash.
2. The trust view shows source/type/epoch/freshness, and the payment view a separate settlement status.

**Negative test:** A 'model verified' card without a verifier does not render a green guarantee; a completed task does not look paid.

**Failure/race:** The offer's parameters changed during approval: a new confirmation is required.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user or an external agent selects an executor, accepts terms and sees the result without a mandatory make-or-buy/insurance engine.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U04.Txx`; negative: `U04.N01`; fault: `U04.F01`; black-box: `U04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U05.md

# U05 · Daily operation: attachments, search, notifications and recovery

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R04, R33, R34, R35, R38, R45.  
**Integration GREEN dependencies:** U01, D06, I06, M05.  
**Ownership:** `apps/desktop/history; apps/desktop/settings`.

## Observable outcome

Local search, private notifications, safe export/backup, diagnostics, quota and operator settings make the client a standalone product.

## Tests first

1. Search works offline over locally decrypted history; locked mode hides plaintext previews.
2. An attachment opens only after explicit consent, never executes automatically.
3. Review/credential data in history does not bypass sanitization, locked previews or secrets-safe diagnostics.

**Negative test:** Diagnostic export and crash reports do not include keys, JWT, plaintext or the private roster without a deliberate separate choice.

**Failure/race:** Disk full/backup failure and an unread message after restart have a comprehensible recoverable UX.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user migrates the profile and continues chatting with full control over local resources.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U05.Txx`; negative: `U05.N01`; fault: `U05.F01`; black-box: `U05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U06.md

# U06 · Installable builds, headless CLI and voluntary updates

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R02, R35, R37, R39, R44, R45, R47, R50.  
**Integration GREEN dependencies:** U02, U03, U04, U05, F06, N06, U08.  
**Ownership:** `packaging; apps/cli; ci/release`.

## Observable outcome

Tauri installers for macOS/Linux/Windows include the Rust bridge and a headless daemon/CLI/MCP; lockfiles/SBOM cover Rust and frontend. The production bundle contains no OAuth client secrets, dev server or WebDriver test plugins; updates are voluntary. The shipment also includes an agent skill with CLI instructions and examples: connecting requires neither project sources nor a dev toolchain.

## Tests first

1. A clean install and upgrade/rollback-compatible migration pass the platform matrix.
2. Auto-update can be disabled; an outdated but compatible client continues on the network.
3. Release artifact scanning rules out test IPC bypass, an open WebDriver port, a built-in client secret and a remote frontend dependency.
4. A clean install contains the compatible client, CLI and SKILL.md; the commands and paths described in the skill are available in the release artifact. The E11/E24 acceptance uses exactly the shipped files.

**Negative test:** The company cannot revoke a network identity via the update endpoint; a malicious downloaded update is not masked by a promise of absolute safety.

**Failure/race:** Release server and signing service unavailable: already installed clients and independent builds keep working.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

An independent developer builds a compatible client from sources and connects without the company's permission.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U06.Txx`; negative: `U06.N01`; fault: `U06.F01`; black-box: `U06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U07.md

# U07 · Hardened Tauri webview → Rust → daemon boundary

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R02, R35, R36, R38, R45.  
**Integration GREEN dependencies:** F02, F06, I02, F04.  
**Ownership:** `apps/desktop/src-tauri; apps/desktop/ipc; tests/tauri-security`.

## Observable outcome

A Tauri skeleton with an explicit AppManifest/command ACL, allowlisted capabilities, CSP, origin/window checks and typed IPC connects to the common broker. Runtime secrets and the root signer live outside the webview; remote content is not loaded into a privileged view.

## Tests first

1. The allowed/forbidden commands matrix per window/profile is verified on a real Tauri bridge, including custom commands.
2. XSS/HTML/iframe/remote navigation fixtures do not invoke shell/fs/root-sign and do not bypass confirmation; Rust re-validates frontend budget values.
3. A compromised renderer is limited to the granted rights; sensitive approval is performed by a separate trusted Rust/OS flow with an unchanged action hash.

**Negative test:** One must not grant generic invoke/arbitrary RPC/command execution to all webviews, enable wildcard remote capability or store JWT/client secret in JS.

**Failure/race:** UI crash/reload and daemon restart require a new authorized session without losing the durable outbox; stale approvals are invalid.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

A black-box Tauri test performs an allowed request to the daemon and attacks a forbidden one over the same IPC; the release build contains no test hooks.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U07.Txx`; negative: `U07.N01`; fault: `U07.F01`; black-box: `U07.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/U08.md

# U08 · Tauri UX for public reviews and service rating

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R35, R38, R45, R49, R50, R51.  
**Integration GREEN dependencies:** U04, Q04, Q05, M07.  
**Ownership:** `apps/desktop/reviews; apps/desktop/services`.

## Observable outcome

In an accepted order a review is available even on executor timeout/refusal. The UI provides 1–5 stars/text, footprint preview, amendments/withdrawal/replies and history. The service card shows rating count, distinct customers, epochs, dataset freshness/partial and claimed-vs-verified labels.

## Tests first

1. A keyboard E2E publishes a negative review without result/payment/provider approval, amends it and reads the reply from another client.
2. An empty sample shows no_rating; an offline/partial index is not shown as a credible zero reviews.

**Negative test:** The executor does not get a button for global deletion of the customer review; five stars do not paint an independently verified model.

**Failure/race:** Pressing publish repeats after a crash without duplicating the score; text changes after preview require new confirmation.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user goes through a full order → result or timeout → public review → rating on an independent Tauri client.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `U08.Txx`; negative: `U08.N01`; fault: `U08.F01`; black-box: `U08.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X01.md

# X01 · End-to-end transport chaos and shutdown of company resources

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R05, R06, R07, R08, R09, R33, R43, R44.  
**Integration GREEN dependencies:** U06, D04, N04, L06, P06.  
**Ownership:** `tests/e2e/transport-chaos`.

## Observable outcome

An independent black-box suite verifies fresh bootstrap, real NATs, offline delivery, 10→7→10 repair and bounded storage across several operators.

## Tests first

1. All company seeds/RPC/relay/update hosts are disabled; the remaining independent paths deliver the claimed functions.
2. Sender/recipient are offline during repair; the recipient later reads the correct plaintext.

**Negative test:** Insufficient operator independence and the absence of an honest surviving source are marked by the suite as unmet preconditions, not a passed test.

**Failure/race:** Mixed crash/lying nodes/packet loss/disk pressure are reproduced by trace.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The public report contains the actual topology, operators, failures, latency and durability without the artificial phrase '100% decentralised'.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X01.Txx`; negative: `X01.N01`; fault: `X01.F01`; black-box: `X01.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X02.md

# X02 · End-to-end group and privacy security

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R03, R10, R11, R12, R24, R36.  
**Integration GREEN dependencies:** U03, G04, G05, I06, P06.  
**Ownership:** `tests/e2e/group-security`.

## Observable outcome

An external attacker checks forked membership, remove/rejoin, multi-device setups, stale keys and the claimed metadata privacy bounds.

## Tests first

1. The accepted baseline of 100 members/up to 3 devices each passes group churn without divergence of the final roster.
2. A removed member cannot read subsequent application messages.

**Negative test:** Packet capture shows no plaintext; absence of correlation is not declared proven just because one trace looks random.

**Failure/race:** A partition coincides with concurrent Remove/Add and loss of a control-log keeper; after heal the clients converge safely.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

One reproducible video/trace shows the full user group flow and the corresponding cryptographic assertions.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X02.Txx`; negative: `X02.N01`; fault: `X02.F01`; black-box: `X02.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X03.md

# X03 · End-to-end economic security and autonomy from L2

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R07, R13, R14, R16, R28, R32, R40, R44.  
**Integration GREEN dependencies:** L06, P06, U02.  
**Ownership:** `tests/e2e/economic-security`.

## Observable outcome

A separate suite attacks spent-state, committee handover, finality, treasury compromise, royalty and resource conservation.

## Tests first

1. One ticket is attacked from different nodes/shards simultaneously; at most one certificate.
2. A long L2 outage is tested before/after lease expiry; reading stored data remains possible.

**Negative test:** Compromising the company payout key does not affect trust/mint/committees; a mock receipt does not make settlement real.

**Failure/race:** Reorg+committee partition+restart do not create a new backed balance from an old one.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The report shows the exact offline boundary, active assumptions, backing and fail-closed reasons.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X03.Txx`; negative: `X03.N01`; fault: `X03.F01`; black-box: `X03.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X04.md

# X04 · End-to-end Google/Telegram/site onboarding and bounded trust

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R04, R15, R25, R26, R27, R28, R41, R44, R46, R47.  
**Integration GREEN dependencies:** O06, U02, L06.  
**Ownership:** `tests/e2e/onboarding`.

## Observable outcome

The suite verifies the Google, Telegram and organization/site adapters, single-issuer/k-of-n profiles, optionality, issuer/key lifecycle, claim dedup and the subsidy schedule.

## Tests first

1. Consented live smoke tests for Google and Telegram verify the real browser/gateway chain; the site issuer passes the full reference integration. No specific unnamed government IdP is claimed as integrated.
2. The same subject namespace across wallets/apps/attesters is deduplicated; different provider/pairwise namespaces are not declared one person.
3. A virtual calendar passes the subsidy schedule and funding caps without an upgrade.

**Negative test:** An unavailable single issuer honestly blocks only new attestation of its profile; no fictitious independent quorum is required and no hidden fallback is enabled.

**Failure/race:** Google/gateway/bot down, JWKS rotated or the fund exhausted: identity/stamps/messages/reviews persist under their own conditions.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The user links Google or Telegram, gets resources under an available campaign, disables the provider and keeps working with the same address.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X04.Txx`; negative: `X04.N01`; fault: `X04.F01`; black-box: `X04.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X05.md

# X05 · Three end-to-end agent work scenarios and malicious content

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R17, R18, R19, R20, R21, R22, R23, R24, R38, R42, R48, R49, R50, R52.  
**Integration GREEN dependencies:** M06, G06, U04, A06, M07, U08.  
**Ownership:** `tests/e2e/agent-work; integrations/examples`.

## Observable outcome

An MCP customer and an independent executor complete OCR, coding and inference service jobs: signed receipt, ExecutorDeclaration, customer checks and a public review. An additional scenario is an accepted but unfulfilled order with a negative review.

## Tests first

1. The customer verifies the OCR/coding outcome itself; the inference model remains claimed by the executor; strict independent provenance without a verifier is rejected.
2. After success/timeout/reject following acceptance the review right remains available without settlement or an economic engine.
3. E11/E03: an agent with the shipped skill and CLI over a scoped handle learns its NetworkID, writes to another ID, receives/acks the reply and survives a restart behind NAT, including relay-only and relay failure; then grant revocation blocks calls.

**Negative test:** A document/message contains a prompt injection to transfer money/export keys: the broker forbids the action.

**Failure/race:** The executor crashes after computing, then returns; result/review retries do not duplicate the operation/score. If the executor disappears permanently, the customer publishes the review independently.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The full workflow is visible in Tauri and MCP; work declaration, customer validation, review and payment are distinguished. V2 engines are not loaded. A simple chat via skill + CLI without MCP is also mandatory: own ID → message to another ID → reply/poll/ack, together with the N04/E03 NAT matrix and the U06 release shipment.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X05.Txx`; negative: `X05.N01`; fault: `X05.F01`; black-box: `X05.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X06.md

# X06 · Independent release gate and readiness for the next economic phase

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R02, R20, R29, R36, R37, R39, R43, R44, R45, R46, R47, R48, R49, R50, R51, R52.  
**Integration GREEN dependencies:** X01, X02, X03, X04, X05, X07.  
**Ownership:** `release-gates; docs/security-report`.

## Observable outcome

The release manifest separately lists V1 code/testnet/mainnet-transport gates and V2 modules. Tauri, provider-neutral attestation with Google/Telegram/site, basic jobs and public reviews/rating are mandatory; the absence of an economic engine does not block V1.

## Tests first

1. Traceability accounts for AMENDMENT-01…05: groups/repair/postage are not lost, new reviews and Telegram are not deferred, the single issuer is honestly bounded.
2. Old compatible clients handle an unknown critical review/auth extension safely; the V2 verifier connects only through a versioned contract.

**Negative test:** Neither a green plan check, nor a self-declared result, nor a high rating, nor a single issuer gives false production/security/quality assurance.

**Failure/race:** A total organization shutdown, loss of its payout key and release CDN unavailability do not stop the now-independent network.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

Installers, a reproducible devnet, sources, test reports, threat assumptions and test contract addresses/parameters are published; product gates stay red until actually met.

## Outside the packet

Do not expand the public API and authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X06.Txx`; negative: `X06.N01`; fault: `X06.F01`; black-box: `X06.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.


---

# File: tasks/X07.md

# X07 · End-to-end review protocol without the company or executor veto

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** V1-implementation.  
**Requirements:** R01, R30, R38, R49, R50, R51, R52.  
**Integration GREEN dependencies:** Q06, M07, U08, L06.  
**Ownership:** `tests/e2e/public-reviews`.

## Observable outcome

Three independent clients verify the full cycle accepted order → negative review → provider reply → rating discovery after the company/executor are disconnected. Privacy, the customer's right, bounded retention and partial views are verified via public APIs.

## Tests first

1. A customer without a result and without monetary settlement publishes a valid review; an outsider cannot revoke someone else's order or add a vote.
2. Tauri and MCP get the same aggregate for the same dataset; amendments/withdrawal/replies follow versioned rules.

**Negative test:** Two colluding identities can create a wash-order; the suite explicitly does not mark the rating Sybil-proof and does not turn OAuth into a quality guarantee.

**Failure/race:** Two indexes censor the negative review, a third is available; the reviewer keeps the receipt, paid replicas are restored under the agreed preconditions.

Before implementation: independent approval of behavior, wire/vectors and forbidden transitions.

## Acceptance demo

The release report contains a reproducible trace of autonomous publication, discovery, partial state, privacy footprint and known manipulation.

## Outside the packet

Do not expand authority beyond the task contract.

## Definition of Done

- Tests are written and RED demonstrated before the main implementation.
- GREEN includes negative checks and reproduction of the fault scenario.
- The result is observable via CLI/API/GUI or an executable validator, not only through an internal unit test.
- Approved tests are not changed without a separate review; mutation/fuzz are applied according to risk.
- Traceability, fixture corpus, compatibility, limits and the threat model are updated.

Contract/tests: `X07.Txx`; negative: `X07.N01`; fault: `X07.F01`; black-box: `X07.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.
