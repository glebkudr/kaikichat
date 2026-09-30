# Discovery service

Finds people by their Google or GitHub account and open groups or profiles
by interest (spec: [discovery-v1.md](../../spec/discovery-v1.md), decision:
[V1_DISCOVERY_2026_09_27.md](../../Docs/V1_DISCOVERY_2026_09_27.md)). It is
an index: every card is signed by its author, every binding by this
service; it keeps no handle nor its digest, only a keyed hash of the
digest, and lists no binding.

## What it does

- **Bindings.** A profile's owner opens a login link (`POST /v1/links` with
  a consent signed by the profile's root key, taken once) and signs in with
  Google (a verified address) or GitHub (the login; the numeric id keeps
  one handle per account). The link's page names the profile and the code
  the owner's terminal shows, and the sign-in counts only in the browser
  that confirmed it: a link handed to someone else binds nothing. Free.
  One handle binds one profile, one account one handle, a profile one
  handle of each kind.
- **Lookups.** `POST /v1/lookup`: the SHA-256 of normalized handles, one
  stamp each a day, found or not; the binding is signed for the digest
  asked about.
- **Cards.** `POST /v1/cards`: an open group's or a profile's card for 30
  days, ten stamps, paid once (shown again it is not renewed); publishing
  a new card replaces it; `POST /v1/withdraw` takes it down.
  `POST /v1/search` is free for a searcher showing an active book (a pass
  whose peer is a one-time nonce), 30 a minute per book.
- **Payment.** Stamps and books are checked by a node of ours beside the
  service, over its owner IPC (`redeem_stamps`, `book_status`): a slot spent
  here and elsewhere for another operation is a double spend that blocks
  the book network-wide. Stamps burn; the service earns nothing.

## Running

The node must read the chain (chain flags) and be reachable over its IPC
socket with its owner token.

```bash
AIN_DIR_LISTEN=0.0.0.0:8081 \
AIN_DIR_PUBLIC_URL=https://directory.example.org \
AIN_DIR_DOMAIN=0x… \
AIN_DIR_SIGNING_KEY_FILE=/run/secrets/directory-key \
AIN_DIR_PEPPER_FILE=/run/secrets/directory-pepper \
AIN_DIR_DATABASE=/data/directory.db \
AIN_DIR_GOOGLE_CLIENT_ID=….apps.googleusercontent.com \
AIN_DIR_GOOGLE_CLIENT_SECRET_FILE=/run/secrets/google-client-secret \
AIN_DIR_GITHUB_CLIENT_ID=… AIN_DIR_GITHUB_CLIENT_SECRET_FILE=/run/secrets/github-client-secret \
AIN_DIR_NODE_SOCKET=/data/node/ipc.sock AIN_DIR_NODE_TOKEN_FILE=/run/secrets/node-owner-token \
agentic-directory
```

`AIN_DIR_LINK_TTL_SECS` (900) bounds a consent's and a login link's life.
The Google OAuth client must list `$AIN_DIR_PUBLIC_URL/v1/oauth/google/callback`
and the GitHub OAuth app `$AIN_DIR_PUBLIC_URL/v1/oauth/github/callback`.
Keep the pepper and the database together: without the pepper the bindings
cannot be found.

Clients: `kaiki daemon start --directory URL`, then `kaiki discover …`.
