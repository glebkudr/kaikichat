# Scoped messaging CLI — implementation contract

`agentic-cli --credentials FILE` is a standalone process using the same private
credential format, request signing and daemon authorization as MCP. It does not
launch or speak through MCP and never receives the owner token or database key.
The owner grants the runtime access through the existing desktop interface.

Commands:

- `context`: own public NetworkID, current permissions and only allowed contacts.
- `messages send --to NETWORK_ID --operation-id ID --text-stdin`: bounded UTF-8
  text from stdin; keep the operation ID on retry. Acceptance is queued delivery.
- `delivery get --to NETWORK_ID --operation-id ID`: only this authenticated
  principal's send operation; recipient receipt is independent of paid storage.
- `inbox poll --from NETWORK_ID --operation-id ID [--limit N] [--max-bytes N]
  [--lease-seconds N]`: defaults 10 items, 4096 bytes, 30 seconds; returned leases
  follow the existing durable cursor/byte/scope rules.
- `inbox ack --from NETWORK_ID --lease-id ID`: acknowledge the processed lease.

Address lookup uses a fresh signed runtime context. Contacts contain their public
NetworkID as well as their existing internal conversation ID; the caller need not
supply the latter. The actual send/read/ack is still independently authorized by
the daemon. This does not grant access to a new contact, create a conversation from
an arbitrary public ID, or expand the runtime grant. Those E11 onboarding conditions
remain open, as do NAT and complete host-lifecycle acceptance. Delivery status is the
recipient phase only; the mailbox swarm's "stored at a quorum" phase is decided with
its native acceptance ([V1_MAILBOX_SWARM_IMPLEMENTATION.md](../Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md)).

Machine commands write exactly one JSON envelope to stdout: `{result:...}` or
`{error:{code,message,retryable,...}}`. No request/credential/parser contents are
interpolated into diagnostics. Exit codes: 0 success; 2 invalid arguments, local
UTF-8/text bounds or credential file; 3 nonretryable domain refusal; 4 retryable
failure or unavailable daemon. Help/version are ordinary CLI text.

Reuse the existing MCP credential checks (private regular file, bounded size,
identity check across open, secret zeroization), signing and proof-only IPC through
one runtime client module. Preserve MCP cancellation, framing, schemas and modern/
legacy lifecycle. No generic owner RPC escape is exposed.

Tests are real CLI and daemon processes with real MLS messages, recipient receipts,
reply/poll/ack, restart/idempotency, scope/revoke/foreign-principal refusal, input
limits and unavailable-daemon retry. Independent critics accepted the tests before
implementation. Owner provisioning returns a structured `cliConfig` containing the
bundled executable and the same private credentials arguments as MCP; the desktop
renders a quoted command that starts with `context`.

The actual paid CLI gate also passes two sponsored sends, exact retries, budget
refusal and cold recipient recovery after sender/data/index loss. Evidence lives
in `evidence/reviews/AR2-wallet-flow/cli-native-candidate-1.json`. The current native
platform is macOS/Unix IPC; this does not declare E11, three-platform acceptance or
the whole wallet complete. Shared paid durability and cold retirement pass the
real native lifecycle gate. The skill's separately reviewed source is
[`SKILL.md`](../integrations/agent-skill/agentic-messaging/SKILL.md); its packaged
text is included in the signed macOS debug bundle and exposed verbatim by the
native permissions panel. An independent Codex host completed context, send,
exact retry, signed delivery, oversized-reply recovery, full save, ACK and a fresh
empty poll using only the packaged CLI and skill. The separately reviewed
[host evidence](../evidence/reviews/AR2-wallet-flow/skill-host/README.md) covers an
existing granted local contact. New contacts, host/NAT lifecycle and complete E11
remain open.
