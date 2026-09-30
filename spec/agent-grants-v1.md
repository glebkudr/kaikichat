# Scoped agent access (V1)

The owner of a profile lets another agent read and/or send in chosen
conversations without handing it the profile: a signed grant for the agent's
own runtime key, a private credentials file, and the scoped clients
`agentic-cli` and `agentic-mcp` that sign every call with that key. The owner
creates grants with `kaiki grants` ([owner-cli-v1.md](owner-cli-v1.md#grants))
or in the window's Agents screen ([desktop-gui-v1.md](desktop-gui-v1.md)).
The agent never receives the owner token, the database key or the root key.

Code: `crates/capabilities` (the grant document and its checks),
`crates/core/src/broker.rs`, `inbox.rs`, `runtime_provisioning.rs`,
`agent_failure.rs`, `crates/node/src/ipc.rs`, `runtime_client.rs`,
`messaging_cli.rs`, `mcp.rs`.

## The grant document

A delegation grant is a `SignedDocument` ([wire/signed-document-v1.md](wire/signed-document-v1.md))
of kind `Identity` with a mandatory expiry and the ownership epoch as its
authority epoch. Its body is a definite CBOR array of 16 fields; the first is
the layout version, 3 (no other layout decodes):

`[3, owner_key32, agent_id32, service_id32, subject_key32, parent_document_id32_or_null, device_epoch, service_epoch, actions_sorted_uint_array, resources_sorted_text_array, recipients_sorted_text_array, budget_asset, budget_units, max_data_bytes, remaining_delegation_depth, no_subcontract_bool]`

Other `Identity` bodies are told apart by length and first field: a node
record is `[1, …]` or `[2, …]` ([node-runtime-v1.md](node-runtime-v1.md)), a
signed agent call `[3, …]` with 5 fields (below).

- **Actions.** 1 `read_inbox`, 2 `send_message`. Codes 3, 4 (groups), 7, 8
  (artifacts), 9–12 (reviews) and 13 (postage spending) still decode but no
  product path grants them: the broker issues grants with actions 1 and 2
  only. Codes 5, 6 and 14 and unknown codes fail closed. There is no owner,
  root-signing, export, shell or wallet action.
- **Limits.** Grant wire ≤ 16 KiB, chain of 1–5 grants, ≤ 11 actions, ≤ 32
  resources and ≤ 32 recipients, identifiers 1–160 bytes without control or
  wildcard characters (`*`, `?`), asset 1–96 bytes, delegation depth ≤ 4,
  lifetime 1 second to 30 days. Sets are sorted and unique; decoding is
  canonical and rejects trailing data.
- **Chain.** The root grant is signed by the profile owner and has no parent.
  Each child names its parent's document id, is signed by the parent's
  subject, keeps owner, agent, service and service epoch, and only narrows:
  actions, resources and recipients are subsets, the asset is the same,
  budget and data limit do not grow, expiry does not extend, it is not
  issued before its parent, depth strictly decreases and `no_subcontract`
  stays set. Every subject's device epoch must match the broker's, the leaf
  subject must be the caller, and revoking any ancestor denies the chain.
- **Checks per call.** `GrantChain::authorize` takes an action, an exact
  resource, an optional recipient, units of the asset, a data size and a
  subcontract flag. A recipient is required for `send_message`. Units above
  zero are allowed only for action 13, so a send permission cannot be read
  as spending authority. `prepare_debit` returns the budget state changes
  for every ancestor and an operation binding, for the caller to commit in
  the same transaction as the action; the broker's grants carry
  `budget_units` 0, so sends debit nothing.

## The broker

`AppCore` keeps the registry of grants in the encrypted profile
(`authorization/registry`, at most 1024 grants).

- **Issuing.** `provision_runtime` takes `{operationId, name, agentId,
  serviceId, conversationIds, actions, expiresAt, maxDataBytes}`: 1–32
  direct conversations (group ids are refused), actions `read_inbox` and/or `send_message`,
  `maxDataBytes` 1–48000, a future expiry within the 30 days above. The core
  generates the runtime's signing key and signs a root grant for it with
  resources `conversation:<id>`, the contacts' network ids as recipients,
  asset `transport-credit-v1`, budget 0, depth 0 and `no_subcontract`. The
  grant, the registration and the private provisioning record
  (`authorization/provision/<hash of the operation id>`: the hash of the
  intent, the grant id, the ownership epoch and the seed) commit in one
  transaction. The same operation id with the same
  intent returns the same grant and key, also after a restart; another
  intent under it is `operation_conflict`, and an operation whose grant
  was revoked or expired cannot be provisioned again. A second live grant for the same
  runtime key is refused. `grant_runtime` (a caller-supplied key) remains an
  owner method that the window does not expose.
- **Listing and revoking.** `list_runtimes` returns `{grantId, name,
  principal, agentId, serviceId, conversationIds, actions, expiresAt,
  maxDataBytes, status}` with status `active`, `expired` or `revoked`; no
  seed. `revoke_runtime {grantId}` persists before it answers, affects every
  later call including retries, and an identical reissued grant never comes
  back to life.
- **Agent calls.** A call is a `SignedDocument` of kind `Identity`, signed by
  the runtime key, living at most 30 seconds, without extensions, body
  `[3, grant_id32, method, canonical_request_json, nonce32]` (object keys
  sorted recursively). The broker checks the domain, the ownership epoch, the
  registration and the whole grant chain on every call, then consumes the
  nonce (at most 4096 live nonces; overflow fails closed). The registry
  fence, the nonce and the call's own state changes commit together; a failed
  call consumes nothing.
- **Methods.** `runtime_context` and `snapshot` (the grant's conversations as
  `{id, title, networkId}` with the grant's metadata, ≤ 16 KiB; `snapshot`
  needs `read_inbox`), `send_message {conversationId, operationId, text}`,
  `delivery_get {conversationId, operationId}`, `inbox_poll`, `inbox_ack`.
  Anything else is `unauthorized`. Owner methods are not reachable from a
  proof.
- **Sending.** The text follows the owner's limits (non-blank, ≤ 12 000
  characters and ≤ 48 000 bytes) and the grant's `maxDataBytes`. The
  operation id is namespaced by the runtime key, so an exact retry with a
  fresh proof returns the original message without advancing MLS; a changed
  request under the same id conflicts. The message goes the owner's way:
  MLS, history, outbox and the mailbox swarm.
- **Delivery.** `delivery_get` answers only for this runtime's own send:
  `queued`, or `delivered` once a quorum of the recipient's holders stored it
  or the recipient acknowledged it.

## The inbox

`inbox_poll {conversationId, operationId, limit 1–100, maxBytes 1–48000,
leaseSeconds 1–600}` needs `read_inbox` on that conversation.

- One cursor and at most one live lease per (agent id, service id,
  conversation), shared by that agent's runtimes, under
  `authorization/inbox/…`.
- A page is `{conversationId, items: [{id, author, text, createdAt,
  sequence}], cursor, leaseId, expiresAt, hasMore}`. It scans at most 1000
  stored records, skips the owner's own messages and non-text events, never
  truncates a text and never exceeds `maxBytes` or the grant's limit. When
  the first item does not fit, the answer is `inbox_item_too_large` with
  `requiredBytes`. A page with nothing past the cursor has null `leaseId`
  and `expiresAt`; a page that only skipped records still needs an ack to
  move past them.
- The same operation id returns the same page (with its original expiry),
  also after a restart or an ack; polling never moves the cursor. While a
  lease is live, another poll is `inbox_busy`; after it expires another
  runtime may lease the same messages.
- `inbox_ack {conversationId, leaseId}` moves the cursor once and clears the
  lease; repeating it is safe. An expired or replaced lease is
  `inbox_lease_expired`. Neither call changes MLS, history, the outbox or the
  owner's read state.

## Local IPC

The daemon's Unix socket (in a 0700 directory, mode 0600) takes exactly two
envelopes: the owner's `{token, method, request}` and the agent's
`{proof}` (hex, at most 65 536 decoded bytes). Mixed or unknown fields fail.
Framing, the 1 MiB request and 16 MiB response bounds and the 5-second
deadline are the owner's ([node-runtime-v1.md](node-runtime-v1.md)). A command
whose caller disconnected before it started is dropped; once it started, its
commit stands and a retry with the same operation id returns it.

Failures reach agents as `{error: {code, message, retryable}}` with codes
`unauthorized`, `inbox_busy` (retryable), `inbox_lease_expired`,
`inbox_item_too_large`, `idempotency_conflict`, `unavailable` (retryable) and
`cancelled`; private storage and authorization details stay out.

## Credentials

The daemon writes the runtime's credentials to
`<profile directory>/runtimes/<grantId>.json`: a 0700 directory (not a
symlink), a 0600 file written atomically, at most 4096 bytes, JSON
`{version: 1, ipc, domain, grantId, ownershipEpoch, signingSeed}`. The
owner's answer adds `credentialsPath`, `cliConfig` (`agentic-cli
--credentials PATH`) and `mcpConfig` (`mcpServers.ain-<grantId>` running
`agentic-mcp --credentials PATH`), both beside the daemon binary; no caller
picks the path or the command. If writing the file fails, the grant stays
listed and revocable, and the same operation id writes it again. The
clients accept only a private regular file (no group/other bits) and zero
the seed after use.

## `agentic-cli`

`agentic-cli --credentials FILE COMMAND` prints exactly one JSON envelope,
`{result}` or `{error: {code, message, retryable}}`, and exits 0 (done), 2
(invalid arguments, text or credentials), 3 (final refusal) or 4 (retry
later, daemon unavailable).

- `context`: the grant's metadata and contacts.
- `messages send --to NETWORK_ID --operation-id ID --text-stdin`.
- `delivery get --to NETWORK_ID --operation-id ID`.
- `inbox poll --from NETWORK_ID --operation-id ID [--limit 10] [--max-bytes 4096] [--lease-seconds 30]`.
- `inbox ack --from NETWORK_ID --lease-id ID`.

A network id must name exactly one conversation of the grant; the CLI finds
it through a fresh `runtime_context`. Besides the daemon's codes the CLI
answers `invalid_request` (arguments or text) and `invalid_credentials`,
both with exit 2. The skill for such agents is
[integrations/agent-skill/agentic-messaging/SKILL.md](../integrations/agent-skill/agentic-messaging/SKILL.md).

## `agentic-mcp`

`agentic-mcp --credentials FILE` serves MCP on stdio with the official Rust
SDK `rmcp` 3.2.0 (modern `2026-07-28` discovery and the legacy `2025-11-25`
initialize). Input lines are bounded to 1 MiB before the SDK parses them.

- Tools: `inbox.poll`, `inbox.ack`, `messages.send`, `delivery.get`, with
  exact JSON schemas (conversation ids as 64 hex digits, operation ids 1–128
  characters, text 1–12 000 characters). Only `delivery.get` is marked
  read-only.
- Resource `agentic://runtime`: the `runtime_context` answer, not cached.
- Results are the daemon's structured answers; refusals are tool errors.
  Cancellation or shutdown drops a pending call as `cancelled`; a committed
  call is not undone, and a retry with the same operation id returns it.

## The owner's window

The Agents screen lists grants with their status, creates one from a name,
at least one direct conversation, reading (always) and sending (opt-in), a
lifetime of an hour, a day or a week, and a byte limit, and revokes one by
grant id. A failed attempt retries the same approved request under the same
operation id; any edit starts a new one. A grant is a permission, not a
running agent: the window shows no online status.
