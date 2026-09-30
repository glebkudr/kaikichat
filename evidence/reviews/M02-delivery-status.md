# Scoped delivery status for submitted agent operations

V1 requires delivery.get in addition to asynchronous messages.send. The agent should inspect the accepted operation without resending its text, including after restart. This is metadata about its own previously submitted work; no general message-history permission is introduced.

Contract before production:

- Signed delivery_get and MCP delivery.get accept exactly conversationId and operationId. Both remain bounded strings; owner/profile/principal/message body fields are rejected. Current SendMessage authority for the real conversation/recipient is required, with no new debit. Runtime principal + original operation ID select the same namespace as messages.send. Owner/sibling operation IDs are inaccessible; the requested conversation must match the stored operation even if both dialogs are granted.
- Response is exactly conversationId, operationId, messageId, delivery{phase,replicas,target}. Reuses the UI's current delivery calculation: queued while durable outbox exists, delivered after signed recipient receipt. Replicas remain truthfully0/target10 until independent replica storage is implemented. Delivery does not imply10stored copies.
- Unknown operation, changed conversation, read-only grant, wrong signature and revocation produce opaque denial. Reads consume a fresh nonce/fence together before releasing a result; no history/outbox/MLS/budget changes. Failed storage commit releases no result and keeps the same proof retryable.
- Store exposes a read-only operation_message lookup through its existing durable operations table, usable after restart and after outbox acknowledgment. No schema migration or new dependency.
- MCP tool is annotated read-only/idempotent. Existing tool catalog assertions grow from3 to4. Real modern/legacy clients discover conversation through the resource, send while peer offline, observe queued, restart sender, then observe actual receipt after receiver restarts. Another runtime cannot inspect the operation. Native packaged E2E uses delivery.get before continuing and checks UI revoke denies it.

Tests first: store operation lookup, core scoped lifecycle and real SQL rollback, node MCP process lifecycle, native product assertions. RED: store API absent; core methods compile and deny unknown delivery_get; actual node MCP catalog lacks delivery.get.

Separate context-free /root/node_test_critic returned REVISE: initial read-only case lacked an existing operation. Corrected test gives the same prior sender a new live ReadInbox-only grant, confirms runtime_context works, then denies its existing operation with unchanged state. Tests also compare all business states/budget after successful reads and assert MCP idempotentHint. FINAL ACCEPT before production changes.

GREEN: all25 store/43 core/21 process tests pass. Complete scripts/check-native.mjs PASS:155 Rust/20 frontend, format/Clippy/TypeScript/Vite, packaged hidden native E2E where MCP observes an actual signed delivery receipt and UI revoke denies delivery lookup, release build, strict/deep codesign and driver exclusion. Log /tmp/ain-delivery-status-check.log. Current native screenshot viewed beside retained component reference; no visual regression. Owned test processes cleaned up. Updated release includes the four-tool MCP adapter. Full V1 remains unfinished.
