# Official MCP stdio messaging adapter

Use official rmcp3.2.0 (current crates.io release verified via cargo search and official SDK source; MSRV1.88 is compatible with this workspace). agentic-mcp is an independent executable in the node package. The existing daemon/core remains the source of identity, scoped authority, state and actual network delivery.

Launch with --credentials pointing to a private regular file, at most4096 bytes, no group/other permissions and no final symlink. JSON schema: version1, ipc path, network domain32, grantId32, ownershipEpoch, signingSeed hex32. Read/parse/signing buffers must not leak to logs or stdout; owner token/master key are not part of this file. stdio is reserved for newline-delimited MCP JSON-RPC. Bound each input line to1MiB before SDK parsing. Native permission provisioning will create these scoped credentials in a later stage.

Expose only inbox.poll, inbox.ack, messages.send with exact JSON schemas and operation IDs where required. Map directly to signed AgentCall inbox_poll/inbox_ack/send_message with cryptographic random nonce and current timestamp, then call proof-only IPC. Forward actual structured results; authority/input/business failures are tool errors rather than fabricated success. Runtime chooses neither a caller principal nor owner branch. No shell/root/export/payment tool exists. Discovery is server capability discovery, not a grant expansion; a listed action may still be denied by the daemon.

Test modern2026-07-28 server/discover with required namespaced per-request metadata separately from legacy2025-11-25 initialize/notifications/initialized. Current results include resultType, legacy results retain their historical form. Real child MCP+daemon processes must complete send/peer receipt/reply/poll, client restart with exact send/poll retry, ack and revoke. Invalid tools/fields, read-only send, public credentials and oversized stdio must not create messages.

Cancellation and disconnect must drop pending IPC calls, and queued commands abandoned before execution must not mutate the core. Once a transaction has committed, cancellation cannot undo it; retry uses the same operation ID. Tests for this boundary precede production implementation. Full jobs/reviews/group tools, subscription delivery, runtime provisioning UI and complete M01/M02/M06 remain open.

References: https://github.com/modelcontextprotocol/rust-sdk ; https://github.com/modelcontextprotocol/rust-sdk/discussions/969 ; local installed rmcp3.2.0 source (model/meta.rs, model.rs, service.rs, handler/server.rs).
