# Access by book (V1)

Holders serve the mailbox protocol only to registry units and to peers that
showed a pass of an active stamp book. Reads, directory pulls, proofs and
notary requests are free, but not free for anyone: a peer id costs nothing,
a book costs a purchase or a Google or GitHub account. Decision and
rationale: [Docs/V1_DISCOVERY_2026_09_27.md](../Docs/V1_DISCOVERY_2026_09_27.md),
part 1. Admission of other peers and protocols:
[node-capacity-v1.md](node-capacity-v1.md).

## The pass

- An active book is a book of the profile (bought, or granted by the
  identity server) that has not ended and that the node has not blocked by a
  double-spend proof. A book with nothing left to spend is still active.
- `AccessPass {book, peer, day, signature}`: `peer` is the node's Ed25519
  transport key, `day` the UTC day (mailbox period), the signature the book
  key's over
  `keccak256(keccak256("AIN_ACCESS_V1") ‖ domain ‖ book ‖ peer ‖ day)` with
  the stamp conventions (secp256k1, low s, v ∈ {27, 28})
  (`agentic_mailbox_swarm::access`).
- A holder accepts a pass when its book is known and active, it names the
  showing peer, it is signed by the book's key, and its day is the holder's
  or next to it. It lets the peer in until an hour after the pass's day, or
  the book's end if sooner, in memory only.
- A granted book the holder does not know is let in on the grant shown with
  the pass, under its issuer's rules (`GrantIssuer`), before its notaries saw
  it; it pays for nothing until they vouch for it. A grant that breaks the
  rules is refused as `grant`.
- A pass accepted before its book was blocked lasts to its end; the next one
  is refused.

## The mailbox protocol

- `Access {credential}`, the credential a pass (`Pass {pass, grant?}`) or a
  unit's own signed record (`Unit {record}`), answered `Access {until}` or
  refused: `bad_pass` (not the peer's, not the book key's, another day, a
  record of another peer), `unknown_book` (read from the chain meanwhile;
  retry), `grant_pending`, `grant`, `book_expired`, `blocked`,
  `unknown_unit`, `malformed`.
- A unit record is taken when its commitment is an active unit of the
  registry and its transport key is the showing peer's; it lets the unit in
  until an hour past the day, and the record joins the directory.
- From a peer that is neither a unit (listed in the directory or introduced)
  nor let in by a pass, only `Access` is taken; every other request is
  refused `access_required`. `Summaries` are taken from units only.
- Nodes without chain flags take everyone, as they take unpaid messages.

## Admission

Decided before a request's bytes are read (`processing::Gate`):

| Peer | Admitted | Limits |
|---|---|---|
| unit | yes | the shared processing budget, as before |
| let in by a pass | yes | 30 requests a second per book, shared by all its peers; when more books ask than the node's 256 a second serve, each gets an even share (by the books that asked in the previous second) |
| anyone else | only to show a credential | 2 a second per peer, 8 per IP (loopback: per peer only), 64 in all; outside the shared budget |

- A bad credential (`bad_pass`, `grant`, `malformed`) keeps the peer and its
  address out for five minutes (loopback: the peer only).
- An unknown book is read from the chain at most once a minute per peer and
  per address.
- At most 1024 peers per book are let in at once; a newer pass replaces the
  oldest.

## The client

- A node shows its credential before a request to a peer that has not let
  it in today, and the request waits for the answer: a unit shows its record
  while the registry lists it, others a pass of the active book that lasts
  longest (`AppCore::mailbox_access`). A unit sends straight away (peers
  know it from the directory) and introduces itself on `access_required`.
- A refused credential fails the waiting requests like a transport failure
  (each lane retries on its own schedule) and is shown again after 5 s
  (`unknown_book`, `grant_pending`), 5 minutes (`bad_pass`, `grant`,
  `malformed`) or a minute (the rest).
- A request refused `access_required` (the peer restarted, or never knew
  this unit) waits and goes again behind the credential, shown a second
  later so the requests already on their way do not use up the small path.
- A node that is not a unit and has no active book sends holders nothing
  (`access: book_required` locally). Direct delivery while it is online is
  unaffected: the sender pays for it.
- Clients start at most 30 requests a second per holder.
- Diagnostics: `node_info.mailboxSwarm.access` (`credential`: `book`,
  `unit` or `none`; `accepted`, `waiting`, `shown`, `refused`) and
  `node_info.accessGate`.
