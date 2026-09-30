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
