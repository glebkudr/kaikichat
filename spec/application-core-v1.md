# Application core (V1)

`AppCore` (`crates/core`) is the trusted Rust service behind every
conversation: it joins the signed documents
([wire/signed-document-v1.md](wire/signed-document-v1.md)), the encrypted
profile store ([store-v1.md](store-v1.md)) and staged MLS
([mls-adapter-v1.md](mls-adapter-v1.md)), and commits every change of crypto
state together with the work it causes. The daemon runs it
([node-runtime-v1.md](node-runtime-v1.md)); owners reach it through owner
IPC, agents only through the broker ([agent-grants-v1.md](agent-grants-v1.md)).
This file covers the profile, direct contacts and text messages; contacts by
id, groups and channels build on it: [contact-by-id-v1.md](contact-by-id-v1.md),
[groups-v1.md](groups-v1.md).

## Profile

- `AppCore::new(ProfileStore, domain)` loads a profile. The store's root key
  exists from the first open; the identity has no name until
  `create_profile(name)` saves the name and the initial MLS state together.
  The same name again is idempotent; another name is `profile_exists`.
- A network id is `ain1` followed by the hex SHA-256 of the root public key.
- Names: 1–80 characters, at most 320 UTF-8 bytes, no control characters,
  not blank.

## Packets

Every packet is a signed document of this network, root epoch 0, whose body
is a definite CBOR array opening with its type:

| Type | Body | Document kind |
|---|---|---|
| 1 Invitation | `[1, name, key_package, addresses, nonce32]` | Invitation |
| 2 Welcome | `[2, group32, invitation_id32, name, welcome]` | GroupControl |
| 3 Application | `[3, group32, mls_private_message]` | Message |
| 4 Receipt | `[4, group32, accepted_message_id32]` | Message |
| 5 Intro card | `[5, name, key_package, addresses, seal_key]` ([contact-by-id-v1.md](contact-by-id-v1.md)) | Invitation |
| 6 Group Welcome, 7 Group commit, 8 Group tree, 9 Group application | [groups-v1.md](groups-v1.md) | GroupControl |
| 10 Channel keys | [Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md](../Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md), part 10c | GroupControl |

The MLS AAD of an application message is the CBOR array
`["AgenticInternet/message/v1", domain, group]`. The signed author must be
the expected peer root *and* the authenticated MLS sender: neither the outer
signature nor the MLS credential alone establishes it. A message id is the
SHA-256 of its signed wire.

## Invitations

- An invitation is `ain-invite1:` and the hex of a signed Invitation packet
  living at most 7 days. Its KeyPackage credential is the issuer's network
  id; the outer root signature attests the MLS signature key. At most 8
  addresses of at most 256 bytes. Issuing it stores the invitation id and
  the new KeyPackage state together.
- Importing one checks signature, network, lifetime, the KeyPackage and its
  binding to the root, then prepares a new group and its MLS add and stores
  the contact, the MLS state and the signed Welcome in the outbox in one
  transaction. The same invitation again returns the existing conversation;
  a self-invitation, or an expired, foreign or altered one, changes nothing.
- A Welcome naming an invitation the receiver issued is accepted only while
  that invitation is unexpired and unused, for the expected group, with both
  member credentials and the root-attested sender matching; a Welcome for any
  other id is the answer to an intro card and follows the receiver's contact
  policy ([contact-by-id-v1.md](contact-by-id-v1.md)). Consuming the
  invitation, the contact, the MLS state and the control deduplication
  commit together. A repeated
  delivery of the accepted Welcome is idempotent; another Welcome for a
  consumed invitation is refused. An arbitrary message never creates a
  contact.

## Sending and receiving

- `send_message(conversation, text, operation_id, now)`: text non-blank, at
  most 12 000 characters and 48 000 bytes. The request hash is computed by
  the core from actor, network, conversation and text, never supplied by a
  client. MLS state, history and outbox commit together. The same operation
  with the same request returns the original message, also after a restart
  or a lost answer; a changed text or recipient under it is a conflict and
  advances nothing.
- A message's delivery is `{phase, replicas, target}`: phase `queued` while
  its outbox entry waits, `delivered` once the recipient's signed receipt or
  a quorum of its mailbox holders acknowledged it. `replicas` and `target`
  are fixed at 0 and 10 and carry no information.
- `receive(wire, now)` returns an optional reply. A duplicate delivery
  returns a fresh receipt for the stored original and changes nothing else.
  A receipt removes the outbox entry only when the expected recipient signed
  it for an existing outgoing message of that conversation; receipts need no
  receipt. Neither receiving nor sending marks a message read; only an
  explicit read event does, and the owner's own messages never count as
  unread. Control packets use the same journal and outbox but never appear as
  chat text.
- Message order and gaps follow [mls-adapter-v1.md](mls-adapter-v1.md#receive-order).

## Views

`snapshot()` serializes the camelCase DTOs the desktop and CLI read
(identity, network status, conversations with messages and delivery); the
desktop's bounded views are in [desktop-history-v1.md](desktop-history-v1.md).
`outbox(limit)` returns pending wire with its destination root and
addresses.

## Tests

`crates/core/tests/conversations.rs` and its `support/` modules: two
profiles over real signed and encrypted wire (invitation, Welcome, text,
reply, receipt), restart, duplicates, retries and conflicts, foreign,
altered and expired invitations, a valid signature from the wrong root
around another sender's MLS message, receipts bound to the exact message
and peer, and real SQLCipher failures before an outgoing commit and while
accepting a Welcome.
