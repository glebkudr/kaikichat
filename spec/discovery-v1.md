# Discovery service (V1, before the public testnet)

Finding people by their Google or GitHub account and open groups or
profiles by interest. Decision and rationale:
[Docs/V1_DISCOVERY_2026_09_27.md](../Docs/V1_DISCOVERY_2026_09_27.md),
part 3. The service is an index: every card is signed by its author, every
binding by the service. A card cannot be forged; a binding is the service's
word for an OAuth sign-in, so a client trusts the service it is configured
with, as it trusts that service's TLS name.

## Parts

- `services/directory` (`agentic-directory`): an HTTP service with its own
  SQLite database and Ed25519 key. It signs in Google and GitHub accounts
  (OAuth), keeps bindings and cards, and answers lookups and searches.
- A node of ours beside it, reached over owner IPC, checks the stamps the
  service is paid with and the books its searchers show
  (`redeem_stamps`, `book_status`): a stamp spent here and anywhere else for
  another operation is a `SenderEquivocation`, and the book is blocked
  network-wide.
- The owner CLI (`kaiki discover …`), its MCP tools and the desktop
  window talk to the service over HTTP through one library
  (`agentic_node::discover`); the node signs what is sent (consents, cards,
  passes) and spends the stamps (`discover_*` IPC).
- The daemon names the service: `--directory URL` and `--directory-key HEX`
  (the key the service signs bindings with), which the network's preset
  sets from its `directory` and `directoryKey` fields; the CLI and the
  window ask the daemon (`discover_config`). Before anything is sent the
  service's policy must name this network and, when the daemon names a key,
  that key (`directory_key_mismatch` otherwise). A profile set by hand
  without a key trusts the service's own.

## Signed documents

All are signed documents of the profile's root key (`agentic_protocol`),
kind `Directory` (14), body a CBOR array whose first item names it:

| Body | Meaning |
|---|---|
| `["link-v1", kind, service]` | consent to bind this profile to the account the human signs in with (`kind`: `google` or `github`; `service`: the service's public URL) |
| `["unlink-v1", kind, service]` | drop the binding of `kind` |
| `["group-card-v1", group id, name, about, tags, langs]` | an open group's card, by its owner: `G = group_ref(domain, author, group id)` |
| `["channel-card-v1", group id, name, about, tags, langs]` | a public channel's card, by its owner; searched as `kind: "channel"`, in the same place as a card of its `G` as a group, which it replaces |
| `["profile-card-v1", name, about, tags, langs]` | a profile's card |
| `["withdraw-v1", card id]` | take a card off the index |

- `tags`: at most 8, each 1–32 characters, lowercase letters, digits and
  `-`; `langs`: at most 4 two-letter codes; `name`: the name rules of
  contacts; `about`: at most 500 characters.
- A card's id is the hex SHA-256 of its signed wire.

The service signs a binding with its key, kind `Directory`, body
`["binding-v1", kind, handle digest, network id, issued at]`, the handle
digest being SHA-256 of the normalized handle.

## Handles

- Email (Google): trimmed and lowercased; for `gmail.com` and
  `googlemail.com` the dots and a `+suffix` of the local part are dropped
  and the domain is `gmail.com`. Only a verified address binds.
- GitHub: the login, lowercased. The numeric id is kept too; a renamed
  account binds again.
- One handle binds one profile, one account one handle, and a profile one
  handle of each kind: a new binding replaces the old (a renamed GitHub
  account's old login stops finding it; GitHub may give that login to
  someone else). A binding lasts until it is dropped or replaced.
- The client normalizes a handle and sends its SHA-256 (`handle digest`);
  the service keeps `HMAC-SHA256(pepper, kind ‖ ":" ‖ handle digest)`, the
  profile and the time bound, and the account's keyed hash, never the
  handle or its digest
  (`agentic_protocol::directory::{normalize_handle, handle_digest}`); it
  signs a binding for the digest a paid lookup asks about.
- An address is `local@domain`, the domain of letters, digits, `-` and dots;
  a local part holding `"(),:;<>[\]` or whitespace is none.

## Payment

- A lookup pays one stamp per handle a day, found or not:
  operation `H("AIN_DISCOVER_LOOKUP_V1", domain, kind, handle digest, day)`,
  with `H` the keccak256 digest of the stamp conventions and `day` the UTC
  day (`unix time / 86400`, 8 bytes big-endian). The service takes today's
  and yesterday's: the same request again is answered again those two days
  and spends nothing more; later it must be paid again, so a stamp does not
  watch an address.
- A card pays ten stamps, operations
  `H("AIN_DISCOVER_CARD_V1", domain, card id, i)` for `i` in 0…9. A card is
  paid once: shown again it is answered again (200) and neither renewed,
  nor moved above newer cards, nor brought back once withdrawn or replaced
  (410 `card_ended`). First shown, it must be signed within the link TTL.
- The node makes the same stamp for the same operation again (the same
  slot), and the same card for the same content within ten minutes, so a
  retry spends nothing more.
- At most four books pay one request.
- Stamps burn; the service earns nothing from them.

## HTTP API

Every error is `{"error": code}`: a 4xx status, or 503 when the node does
not answer and 500 for the service's own failure.

| Method | Body | Answer |
|---|---|---|
| `GET /v1/policy` | — | `{domain, key, lookupPrice: 1, cardPrice: 10, cardDays: 30}` |
| `POST /v1/links` | `{consent}` (hex wire) | 201 `{linkId, loginUrl, code, expiresAt}`; free |
| `GET /v1/links/{id}` | — | `{status: pending|linked|denied, reason?}` |
| `GET /v1/links/{id}/login` | — | a page naming the profile and the code, with a button; sets a cookie |
| `POST /v1/links/{id}/login` | form `token` | 303 to Google or GitHub; 403 without the page's cookie |
| `GET /v1/oauth/{google,github}/callback` | — | a page: 200 bound, 403 denied, 400 unknown login (also another browser's) |
| `POST /v1/unlink` | `{consent}` | `{}`; free |
| `POST /v1/lookup` | `{handles: [{kind, digest}], day, stamps: [stamp], grants?}` | `{results: [{kind, digest, binding?}]}`; at most 100 |
| `POST /v1/cards` | `{card, stamps: [stamp × 10], grants?}` | 201 `{id, expiresAt}`; 200 the same for a card already paid |
| `POST /v1/withdraw` | `{withdrawal}` | `{}`; free |
| `GET /v1/cards/{id}` | — | `{id, card, publishedAt, expiresAt}` |
| `POST /v1/search` | `{query, tag?, lang?, kind?, pass}` | `{cards: [{id, card, publishedAt, expiresAt}]}`, at most 50 |

- A stamp on the wire is `{book, index, operation, signature}` in hex.
- `grants` are the identity server's grants of the books the stamps (or
  the search pass) are of, for a node that does not know them yet.
- Refusals: 402 `payment_required` (a stamp missing or for another
  operation; nothing is spent), 402 `stamp_refused` (the node refused it:
  `conflict`, `blocked`, `expired`, `index`, `signature`), 409
  `unknown_book` (retry), 400 `bad_consent` (not a fresh consent to this
  service, or one taken already), 400 `bad_card`, 410 `card_ended`, 401
  `bad_pass`, 401 `book_required`, 400/404 `not_found` (also a card of
  another author), 429 `rate_limited`, 503 `ledger_unavailable` (the node
  does not answer), 400 `invalid_request`.
- A consent (link or unlink) is taken once, within the link TTL (15
  minutes) of its issue.
- The login page names the profile being linked and the code `discover
  link` shows, and asks the human to go on only if they started it
  themselves; it sets a cookie the confirmation and the provider's callback
  must bring. A link handed to someone else, or the provider's page it led
  to, binds nothing in their browser.
- A card lives 30 days from its publication; the same author's card for the
  same group (or profile) replaces the older one.
- The search pass is an `AccessPass` whose `peer` is a random nonce: its book
  must be active at the node, its day today; a nonce is taken once. Searches
  are limited per book (30 a minute); behind a proxy an address says
  nothing. A query is at most 8 words and 200 bytes, a tag or language at
  most 32 bytes.
- There is no listing of bindings and no export of them.

## Node IPC (owner)

| Operation | Request | Answer |
|---|---|---|
| `discover_config` | `{}` | `{url, key}` (`key` null when none is named) or `directory_not_configured` |
| `discover_consent` | `{action: link|unlink, kind, service}` | `{consent}` |
| `discover_stamps` | `{handles: [{kind, digest}]}` | `{day, stamps, grants}` |
| `discover_card` | `{kind: group|profile, groupId?, name, about, tags, langs}` | `{card, id, stamps, grants}` |
| `discover_withdrawal` | `{cardId}` | `{withdrawal}` |
| `discover_pass` | `{}` | `{pass}` or `book_required` |
| `redeem_stamps` | `{stamps: [stamp], grants?}` | `{results: [ok | code]}` |
| `book_status` | `{book, grant?}` | `{state: active|unknown|ended|blocked, key?}` (the book key's account while active) |

`redeem_stamps` needs chain flags: it checks each stamp against its book
(read from the chain meanwhile, or learned from its grant once the grant's
notaries vouch for it; `unknown_book` until then), records its ticket as a
notary does and puts it on record with the ticket's notaries, so a slot
spent elsewhere for another operation is proven there too. `book_status`
lets a granted book in on its issuer's rules alone, as a holder does a pass.

## CLI

```
discover link google|github
discover status --link ID
discover unlink google|github
discover lookup [--email ADDR]… [--github LOGIN]… [--file PATH]
discover publish group --group GROUP --about TEXT [--tag T]… [--lang L]…
discover publish profile --about TEXT [--tag T]… [--lang L]…
discover withdraw --card ID
discover search TEXT [--tag T] [--lang L] [--kind group|profile]
groups follow --card ID | --group-ref G --owner ID --name NAME
```

`discover link` answers the login URL for the human to open and the code
its page shows; `discover status` answers the link's state. `lookup` and
`publish` wait for the service's node to learn a newly bought or granted
book (`unknown_book`) for up to a minute, resending the same request.
`--file` reads a vCard (`EMAIL` lines), a CSV (any column holding
addresses) or one address or `github:LOGIN` per line.
