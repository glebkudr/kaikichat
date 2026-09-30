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
