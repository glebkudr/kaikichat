# Identity server

Grants coins to accounts verified by Google or GitHub (the app says “Log in
via Google or GitHub. Up to 10 thousand messages every month.”). It only mints: it never takes part
in sending, storage or bootstrap, and the network works without it. Scope and
decisions: [V1_AGENT_FIRST_SCOPE_2026_09_25.md](../../Docs/V1_AGENT_FIRST_SCOPE_2026_09_25.md).

## Flow

1. The agent's CLI signs a `ClaimRequest` with its stamp-book key (secp256k1)
   and posts it to `POST /v1/claims`; the response carries `claimId`,
   `loginUrl` and `expiresAt`. A retried request returns the same claim.
2. The human opens `loginUrl`: a page to choose Google or GitHub
   (`/v1/claims/{id}/login/google`, `/login/github`), or Google straight away
   when GitHub is not configured. Both are authorization code flows with a
   single-use `state` and PKCE S256 (Google also a `nonce`); a login started
   for one provider is honoured only at that provider's callback. A claim
   yields at most one grant.
3. Google redirects to `/v1/oauth/google/callback`: the server exchanges the
   code at Google's token endpoint over TLS and checks `iss`, `aud`, `exp`,
   `nonce`, `sub` and `email_verified` of the returned ID token. GitHub
   redirects to `/v1/oauth/github/callback`: the server exchanges the code over
   TLS, reads `/user` and `/user/emails` with the token and requires a primary
   verified email.
4. One grant per account per claim interval: a Google account by its `sub`, a
   GitHub account by its numeric id (never its login or email, which change),
   each stored only as a SHA-256 in its own namespace. Grants are numbered per UTC day and never exceed the day's cap:
   `(serial + 1) * bookSize <= capCoins`. The account check, the serial and the
   claim outcome are one SQLite transaction.
5. The CLI polls `GET /v1/claims/{id}`: `pending`, `granted` with a signed
   [grant book](../../crates/grant-book/src/lib.rs), or `denied` with a reason.

Denial reasons: `email_not_verified`, `invalid_identity_token`,
`already_claimed`, `account_banned`, `daily_cap_reached`, `issuer_inactive`,
`claim_expired`.
A cancelled sign-in, a failed code exchange or unreadable network rules keep
the claim `pending` until its TTL, so the link can be opened again.

`GET /v1/policy` returns the domain, the issuer account, today's cap, book
size, maximum validity and grants issued today.

## Penalties

[The decision](../../Docs/V1_IDENTITY_PENALTIES_2026_09_30.md): an account
proven to spend a grant twice gets no grant for 90 days, and its grants still
in force are revoked; holders take no new stamps of them. A grant got after
that ban spent twice bans the account for good.

- `POST /v1/reports` with `{"grant": …, "first": stamp, "second": stamp}`
  (stamps as the directory's HTTP API shows them: `book`, `index`,
  `operation`, `signature` in plain hex). The grant must be signed by this
  server's key for its domain (else `400 not_ours`), and the two stamps must
  be one slot of its book signed twice by its book key for different
  operations (else `400 no_proof`). A first offence bans the grant's
  account for 90 days from the report, a grant got after the ban spent twice
  bans it for good; either revokes every grant of it still in force. A proof
  about a grant from before the ban changes nothing. A grant issued before
  the server linked grants to accounts is revoked alone; another grant under
  the same id, signed with the key outside a claim, is `400 not_ours`. The
  answer is `{"banned": bool, "revoked": n}` (`banned`: the account is
  banned now); a repeated report revokes nothing more.
- `GET /v1/revocations?after=N`: `{"revocations": [...], "last": M}`, at most
  256 in order after the `N`-th, only of grants that have not ended. A revocation is signed by the issuer key
  over `H("AIN_GRANT_REVOCATION_V1", domain, server, book id, expiry,
  revoked_at)` (`GrantRevocation` in the grant-book crate).

Holders that know the server report what they see and read the revocations
every 10 minutes (`serve --identity-server`).

## Network rules

Holders accept a grant only under the rules of the `GrantIssuer` contract
(`contracts/src/GrantIssuer.sol`) for the grant's day: active issuer key,
book size, `serial < cap(day) / bookSize`, domain, day and expiry. The cold
owner key changes the cap at most once per UTC day, by at most 10x, effective
the next day. Anyone can revoke an issuer key with two different grants it
signed for one day and serial (`reportEquivocation`); the key stops from the
next UTC day, so grants already issued today stay valid. The cold owner can
stop a key from today with `revokeIssuer`.

Operations follow from this: run a single instance per issuer key, keep the
database (it holds the per-day serial counter) and never restore it to an
earlier state. Reusing a serial is exactly the proof that revokes the key.
The database also holds which account holds which grant, the bans and the
revocations.

## Running

```bash
AIN_ID_LISTEN=0.0.0.0:8080 \
AIN_ID_PUBLIC_URL=https://id.example.org \
AIN_ID_DOMAIN=0x… \
AIN_ID_SIGNING_KEY_FILE=/run/secrets/issuer-key \
AIN_ID_DATABASE=/data/identity.db \
AIN_ID_GOOGLE_CLIENT_ID=….apps.googleusercontent.com \
AIN_ID_GOOGLE_CLIENT_SECRET_FILE=/run/secrets/google-client-secret \
AIN_ID_CAP_COINS=100000 AIN_ID_BOOK_SIZE=50 AIN_ID_MAX_VALIDITY_DAYS=30 \
agentic-identity-server
```

Optional: `AIN_ID_GRANT_VALIDITY_DAYS` (30), `AIN_ID_CLAIM_INTERVAL_DAYS` (30),
`AIN_ID_CLAIM_TTL_SECS` (900). The Google OAuth client must list
`$AIN_ID_PUBLIC_URL/v1/oauth/google/callback` as a redirect URI.

GitHub is offered when `AIN_ID_GITHUB_CLIENT_ID` and
`AIN_ID_GITHUB_CLIENT_SECRET_FILE` are set; the GitHub OAuth app's callback
URL is `$AIN_ID_PUBLIC_URL/v1/oauth/github/callback`.

“Up to 10 thousand messages every month” is `AIN_ID_BOOK_SIZE=10000` with
`AIN_ID_CLAIM_INTERVAL_DAYS=30`, and the same book size in `GrantIssuer`.

## Not yet wired

- Reading `GrantIssuer` over RPC: the binary mirrors the contract's cap, book
  size and maximum validity from the environment (`StaticRules`); keep them
  equal to the contract or holders reject the grants. Holders themselves read
  the contract (`crates/node/src/chain.rs`).
- Registering each grant with the network notary on its day: the node that
  collects the grant puts it on record with its notaries when it adds it
  (`coins_claim` reads the claim every 5 s), so a grant signed in for in a
  day's last seconds can be first seen the next day and refused. The server
  registering grants itself would close that gap.
- The node's side (`coins_claim` over owner IPC, `serve --identity-server`)
  is wired; the owner CLI command comes with phase A's CLI.
