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
