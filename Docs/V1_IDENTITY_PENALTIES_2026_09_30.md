# Identity penalty for double-spending a grant — September 30, 2026

**Status:** user decision from 30.09.2026. Clarifies “Violation
proofs” and “Penalty” in
[V1_STORAGE_REDESIGN_2026_09_24.md](V1_STORAGE_REDESIGN_2026_09_24.md):
there is no slashing of bonds.

## Decision

Network nodes are permissionless and unbonded (the 0.0001 ETH per unit is a
registration fee); there is nothing to take from them. The only stake is the
identity, for which the server issues free coins. For a proven double spend
of a grant book:

1. All active grant books of the identity are revoked: holders no longer
   accept new stamps from them. Already stored messages remain — they are
   the recipients' correspondence, and a copy on someone else's holder cannot
   be deleted.
2. The identity server issues no grants to it for 90 days. Double-spending a
   grant received after this ban means a permanent ban.

Escalation rather than a permanent ban right away (the owner's decision after
review): an honest person can also turn out to be the author of a proof. An
agent profile restored from a backup or copied to a second machine re-signs
the same slots with different operations. And a claim opened with someone
else's key via a sent link binds the grant to whoever logged in. Such a
person loses three months of free messages, not everything; they can always
buy stamps.

Identity does not penalize node operators: they do not go through it. The
network still excludes an equivocating holder on proof.

## Trigger

Only a self-contained proof — two signatures of the grant book key on one
slot with different operations (`SenderEquivocation`) together with the
grant itself, signed by that server. Complaints, spam, and the like are not
a trigger.

## Identity server

- When issuing a grant, the server records the mapping “grant → account
  hash” (`sha256(sub)`).
- `POST /v1/reports` — a grant and two stamps of one slot of its book with
  different operations (stamps in the catalog HTTP-API form). The server
  verifies its own grant signature and both signatures of the book key.
  Otherwise — `400 not_ours` or `400 no_proof`, and nothing changes.
- Proven double spend: the server revokes all grants of the account whose
  term has not expired and bans the account for 90 days; if the grant was
  received after the ban — forever. A proof about a grant issued before the
  ban changes nothing — neither the ban term nor the revocations.
  A grant issued before this mapping existed (old testnet grants) is revoked
  alone, without a ban. A grant under the same id signed by the server key
  outside an application (test runs) is `400 not_ours`. The response is
  `{"banned": bool, "revoked": n}`; a repeat changes nothing.
- `GET /v1/revocations?after=N` — revocations of unexpired grants in order,
  at most 256 at a time, with the number of the last one. A revocation is the
  issuer key's signature over
  `H("AIN_GRANT_REVOCATION_V1", domain, server, grant id, expiry, revoked_at)`.
- A new application from a banned account is rejected with `account_banned`.

## Nodes

- A holder that learns of a grant book double spend (itself or via the
  network) queues a report and sends it to its identity server; if the server
  is unavailable, it retries once a minute. An unanswered report is deleted
  when the grant expires.
- A holder with a known identity server (`--identity-server` or the network
  preset) reads its revocations every 10 minutes, from the beginning after
  each start, and immediately after the response to its own report: the
  server may have just banned an identity. This way the reporting holders
  block its grants within seconds, the others within 10 minutes.
- A revocation signed by the grant issuer blocks the book just like a
  double-spend proof: no new stamps, and copies of what was paid for before
  the revocation keep being repaired. A revocation for a grant the holder
  does not yet know is kept until the grant's term ends and applied once the
  holder learns the grant.
- Revocations are not passed between nodes: proof pages have a strict
  format, and old nodes would not accept a new field. A node without an
  identity server blocks only books with a double-spend proof.
- Testnet nodes get `--identity-server`.

## What this achieves

With the current settings (a 30-day grant, one per 30 days) an identity
rarely has more than one live grant, so the main penalty is the ban.
Revocation also closes the previous gap: a holder accepts a double-spend
proof only for a book it already knows, whereas a revocation it stores in
advance and applies when the grant reaches it.
