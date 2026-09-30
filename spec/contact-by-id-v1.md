# Contact by ID (V1 phase A, step 4; V1-AF02)

Anyone who knows a profile's network id (`ain1` and the hex SHA-256 of its
root public key) can ask it for a conversation, without an invitation and
while it is offline. The request is paid like a message, so strangers cannot
flood a profile for free, and the recipient's policy decides what a request
becomes. Invitations stay as the other way in.

## The intro mailbox

- The intro mailbox of a network id for a period is
  `H("AIN_INTRO_V1", domain, id digest, period)`, where the id digest is the 32
  bytes the network id spells (`mailbox-swarm::address::intro_mailbox_id`).
  Anyone can compute it from the network id; its swarm, stamps, quorum,
  replication and retention are those of every mailbox. Holders do not tell
  it apart.
- An entry is one of:
  - `0x01 ‖ card`: the owner's intro card (below), in the clear;
  - `0x02 ‖ card id (32) ‖ ephemeral X25519 key (32) ‖ ciphertext`: a request
    sealed to that card.

## The intro card

A document signed by the root key (kind Invitation) with the owner's name, a
last-resort MLS KeyPackage, up to 8 addresses and an X25519 seal key. It is
valid for 30 days; the owner makes a new one when the current one has less
than 23 days left, keeps every card's private keys until 31 days after it
expires (a request read before then is still taken), and publishes the
current card into the intro mailbox of every period (4b). The last-resort
KeyPackage serves every request made with the card.

A reader accepts a card only when its author is the requested network id, it
has not expired, and the KeyPackage is that identity's.

## The request

The requester creates the conversation's MLS group with the card's
KeyPackage and signs a Welcome that names the card. The Welcome goes:
- sealed into the recipient's intro mailbox for the current period, with a
  stamp: HKDF-SHA256 of the X25519 shared secret (salt: both public keys,
  info: `AIN_INTRO_SEAL_V1` and the domain) keys ChaCha20-Poly1305 with the
  card id as associated data. Readers of the mailbox learn nothing about the
  requester, not even its root key; holders see the stamp's book, as for
  every message.
  - The ephemeral key comes from a random seed kept with the request and the
    period, so a retry within a period seals the same envelope and spends no
    new slot, while two requests never share a key.
- directly to the card's addresses, carrying the swarm copy's stamp like any
  paid message (4b).

The requester's delivery ends at the swarm quorum or at the recipient's
receipt. A request is idempotent per operation id.

## Publishing and reading (the node)

- The owner's node publishes its current card into the intro mailbox of
  the current period, stamped, at a quorum of the swarm like a message:
  once per period and card, so a card made mid-period is published in that
  period too. The card carries the addresses the node advertises.
- It reads its own intro mailboxes like conversation mailboxes, every 30
  seconds (requests are not urgent): the current and previous period, closed
  periods since the last one read through caught up a few at a time, back to
  the period before its first card. Every entry meets the policy; entries that are not a request to one
  of its cards are skipped and do not stop the reading.

## Finding a card (the requester's node)

- It reads the id's intro mailboxes from every holder of their swarm, from
  the current period back one period at a time to the retention floor, and
  takes the valid card of that id it finds first (of that period, the one
  that expires last). Entries that are not such a card are skipped. It may
  read a few periods at once; the whole lookup ends after a minute.
- Owner IPC `request_contact {networkId, name, operationId}` answers
  `{conversationId, name}` once the request is made (at once for an
  operation id already used), `card_pending` while it looks (retryable),
  `card_not_found` when no period holds a valid card or its holders did not
  answer in time (final for this call; a later call looks again), and
  `invalid_request` for a malformed id or one's own.

## The recipient's policy

| Mode | A request from a stranger |
|---|---|
| `all` (default) | joins at once, at most `dailyLimit` (default 100, 0–1000) a day; the rest wait |
| `list` | waits unless listed |
| `manual` | waits unless listed |

- In every mode, ids on the list (at most 1000 network ids) join at once and
  do not count against the limit.
- The day is the period (UTC) of the moment the request is read, so a backlog
  read at once still meets one day's limit.
- A request from a profile that is already a contact is ignored.
- Each requester has at most one waiting request; one issued later replaces
  it, one issued earlier is ignored.
- At most 32 requests wait; beyond that a request is dropped.
- The owner lists waiting requests and accepts or rejects each.
- A request that was accepted, rejected or replaced is ignored when it is
  read again (holders keep it for the mailbox's lifetime).
- A waiting request stays until it is accepted or rejected, or its card's
  keys are dropped.
- The direct copy meets the same policy as the sealed one, once. Joining,
  waiting and ignoring all answer it with a receipt, so the requester stops
  retrying; none reveals the policy's decision to the requester.

## Refused

- A card signed by another key, expired, or with another identity's
  KeyPackage, whether it is only opened or used for a request.
- A request sealed to an unknown card, changed in transit, or naming a card
  the Welcome does not match.
- A Welcome from oneself.
- An unpaid request: holders refuse the store, and a node that reads the
  chain refuses the unpaid direct copy (4b).
