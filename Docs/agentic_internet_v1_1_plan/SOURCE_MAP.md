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
