# 11. Voluntary external account verification

Google, Telegram, and site/org credentials yield only explicitly accepted claims; funded onboarding is limited to a campaign.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R07, AR-R08, AR-R24.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-O02** (k-of-n attestors): quorum counting ≠ threshold crypto — FROST (RFC 9591); Boldyreva threshold BLS; quorum intersection — Mazières SCP. If they switch to threshold signatures: GG20/CGGMP21.
- **V1-O05/V1-O06** (handoff): RFC 8628 (device authorization flow); RFC 7636 (PKCE); TOFU — Perspectives (USENIX Security 2008).
- **V1-O07** (credential↔campaign dedup): an open niche — pairwise-subject dedup without a single identifier; the closest pattern is Semaphore scoped nullifiers. See research §9.

<a id="v1-o01"></a>
## V1-O01. Implement provider-neutral Credential/TrustProfile

**Type:** backend. **Source cards:** O02, F02. **Position in dependency order:** 70.
**After:** [V1-I01](07-tasks.md#v1-i01), [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Define ExternalCredential, ProviderBinding, and TrustProfile with issuer/subject namespace, owner possession, claims, policy version, and expiry.
2. Separate proven account identity, voluntary trust, and eligibility for a specific funded grant.
3. Persist bindings separately from the root identity; do not use email/username/phone as a stable subject.

**Verifiable scenarios:**

- A profile explicitly accepts only the listed issuer claims and terms.
- A credential of another owner/audience or a mixed policy epoch is not accepted and grants no recovery/mint authority.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There are strict wire vectors, a storage lifecycle, and a broker boundary; login does not change NetworkID.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o02"></a>
## V1-O02. Verify single-issuer and k-of-n attestation

**Type:** backend. **Source cards:** O03. **Position in dependency order:** 71.
**After:** [V1-O01](11-tasks.md#v1-o01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Support an explicit organization 1-of-1 and a separate threshold profile with distinct signer IDs.
2. Bind all signatures to a single challenge, owner, claims digest, policy epoch, and upstream evidence.
3. The transport stake/BFT committee does not automatically appoint login attestors.

**Verifiable scenarios:**

- 1-of-1 works only within the accepted scope; k-of-n requires a real threshold of distinct admissible keys.
- Duplicates, different epochs/issuers, and unconfirmed claims do not add up; an outage is limited to that trust profile.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Both variants pass independent fixtures; the UI can honestly explain their different assumptions.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o03"></a>
## V1-O03. Implement Google JWT/JWKS validation

**Type:** backend. **Source cards:** O01. **Position in dependency order:** 72.
**After:** [V1-O01](11-tasks.md#v1-o01).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Before coding, cross-check the current official Google OAuth/OIDC documentation and the registered native client profile.
2. Verify the signature, admissible issuer, exact audience/authorized party, subject, nonce, expiry, and owner challenge.
3. Cache JWKS for the admissible term/rotation with bounded refresh; do not contact Google for every message.

**Verifiable scenarios:**

- Authentic offline fixtures of the current and rotated key pass with a valid challenge.
- A foreign aud, alg confusion, an expired nonce, a modified JWT, and an unknown key without admissible refresh are rejected.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The provider-specific validator returns only verified claims for O01 and does not issue stamps itself.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o04"></a>
## V1-O04. Wire up the native Google PKCE flow

**Type:** backend. **Source cards:** O01, U02. **Position in dependency order:** 73.
**After:** [V1-O03](11-tasks.md#v1-o03).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Create a one-time state/nonce/PKCE session bound to the local owner and a restricted callback.
2. Open the system auth flow only on user action; handle cancel/retry/timeout without losing the profile.
3. After token validation, persist the credential atomically and delete temporary auth secrets.

**Verifiable scenarios:**

- Real authorization yields an owner-bound credential; canceling and retrying do not change the address.
- An intercepted callback/replayed code, a different state, and a parallel session do not substitute the owner.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is a live Google flow with a valid callback registration; unit fixtures do not close this gate.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A registered Google native OAuth client and an allowed test account.

<a id="v1-o05"></a>
## V1-O05. Implement a site/org gateway with owner-bound handoff

**Type:** backend. **Source cards:** O07. **Position in dependency order:** 74.
**After:** [V1-O02](11-tasks.md#v1-o02).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. In a separate optional gateway application, implement challenge→site/OIDC auth→signed credential→daemon possession.
2. Keep OAuth secrets only on the gateway; allowlist issuer/origin/callback; protect discovery from SSRF.
3. Add a reference site issuer and a dedicated organization-account profile; an outage does not stop the chat.

**Verifiable scenarios:**

- Reference OIDC and an explicit org issuer give claims verifiable for the intended owner.
- A foreign owner, an intercepted handoff, redirect/issuer substitution, and replay do not yield a credential; server tokens do not reach the desktop.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A deployable gateway and reference site work with the shared O01/O02; no specific government system is assumed.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A domain/HTTPS/callback and a registered reference issuer for live handoff; local fixtures are possible in advance.

<a id="v1-o06"></a>
## V1-O06. Implement the Telegram gateway adapter

**Type:** backend. **Source cards:** O08. **Position in dependency order:** 75.
**After:** [V1-O05](11-tasks.md#v1-o05).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Cross-check the current official Telegram Login/OIDC method for the chosen registration.
2. Verify provider evidence in the gateway and bind the handoff to the owner, request digest, origin, expiry, and one-time consumption.
3. Return only a scoped credential; do not include bot/client secrets in the bundle or renderer.

**Verifiable scenarios:**

- A real Telegram login ends with a credential for the original desktop challenge.
- A foreign Telegram subject, an expired/replayed handoff, and owner substitution are rejected; a blocked bot affects only this branch.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The live Telegram flow passes together with negative handoff cases and offline gateway control.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A registered Telegram bot/client, a gateway, and an explicitly allowed test account.

<a id="v1-o07"></a>
## V1-O07. Tie credential eligibility to a funded campaign

**Type:** backend. **Source cards:** O04, O06, L03. **Position in dependency order:** 76.
**After:** [V1-O02](11-tasks.md#v1-o02), [V1-ECO04](10-tasks.md#v1-eco04).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Apply an allowlisted trust-profile/version/provider namespace to a single atomic claim-state.
2. Deduplicate the stable subject across device/wallet/attestor; accept pairwise namespaces only with a proven mapping or a pinned sector/client.
3. Limit linked Google/Telegram accounts to a shared campaign cap; treat unlinked accounts as residual Sybil risk.

**Verifiable scenarios:**

- One subject gets a single entitlement across different wallets/devices.
- Changing salt/version/attestor and cross-provider linking do not double the grant; independent identities are not automatically declared identical.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Eligibility and the actual funded claim are separated; the cross-provider policy is verified in a concurrent flow.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o08"></a>
## V1-O08. Implement refresh/revoke/unlink without losing the address

**Type:** backend. **Source cards:** O05, I03. **Position in dependency order:** 77.
**After:** [V1-O04](11-tasks.md#v1-o04), [V1-O06](11-tasks.md#v1-o06), [V1-O07](11-tasks.md#v1-o07).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Update provider bindings and trust/JWKS in explicit versions and terms.
2. Store unlinking separately from the root, contacts, history, and the used campaign entitlement.
3. On an issuer/key outage, return a scoped failure; do not block stored books or ordinary communication.

**Verifiable scenarios:**

- Unlink/relink and rotation preserve NetworkID, messages, and the already used entitlement.
- A changed display name/email does not grant a new grant; the issuer does not get the ability to recover the root.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The lifecycle is verified for Google, Telegram, and site/org without implicit profile re-creation.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o09"></a>
## V1-O09. Add a clear funded onboarding UI

**Type:** frontend. **Source cards:** O06, U02. **Position in dependency order:** 78.
**After:** [V1-O08](11-tasks.md#v1-o08), [V1-ECO07](10-tasks.md#v1-eco07).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Offer a key-only start and separate voluntary provider/trust branches.
2. Show the admissible campaign, available transport resource, TTL/limits, and gas sponsor state; do not promise cash-out or the same bonus for any login.
3. Explain fund/L2/provider errors separately; keep the transition to the paid path and a working profile.

**Verifiable scenarios:**

- In each allowed campaign branch, a user without a crypto balance sends the first message backed by real collateral.
- Exhausted/cutoff/rejected credentials do not create stamps and do not delete local data.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A headless visual/native flow covers Google/Telegram/org and the failure of each external dependency.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-o10"></a>
## V1-O10. Accept the full provider/onboarding lifecycle

**Type:** verification. **Source cards:** O01, O02, O03, O04, O05, O06, O07, O08, X04. **Position in dependency order:** 79.
**After:** [V1-O09](11-tasks.md#v1-o09).

**Change boundary / entry points:** `Docs/agentic_internet_v1_1_plan/AUTH_AND_ATTESTATION_V1.md`, `crates/protocol-types/src/lib.rs`, `crates/capabilities/src/lib.rs`, `crates/core/src/broker.rs`, `crates/core/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/ChatShell.tsx`, `tests`.

**Implementation plan:**

1. Run live logins of the three branches, 1-of-1 and k-of-n fixtures, the first funded messages, and repeated claims.
2. Check cancel/retry, unlink/relink, rotation, issuer-off, and the fund after cutoff.
3. Save redacted evidence with the actual registrations/domains, not tokens/secrets.

**Verifiable scenarios:**

- All enabled branches really work without test substitution of credentials.
- An unavailable registration/provider stays blocked_by_environment, not PASS and not skipped-required.

**Checks:** AUTH ECONOMY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Each of O01–O08 has its own full-scope check; live results are tied to the candidate revision.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
