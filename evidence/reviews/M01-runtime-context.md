# Authenticated MCP runtime discovery

User goal remains a working V1 desktop, transport and agent product. This slice lets a configured agent discover its allowed conversation IDs and limits without an owner API. It also closes the old broker snapshot's bulk message-history escape around bounded inbox delivery.

Contract under test before production changes:

- Signed runtime_context accepts exactly an empty object. The existing broker verifies the principal, live grant/epochs, expiry, revocation and nonce. Any valid runtime, including send-only, can discover its own scope. No extra business action is granted.
- Returned JSON contains exactly version1, grantId, agentId, serviceId, principal (hex public IDs), expiresAt, maxDataBytes, actions and conversations[{id,title}]. Only this grant's dialogs appear, max32 and max16KiB serialized metadata. This metadata bound is independent of maxDataBytes, which limits message text. No messages, unread counts, history, routes, profile, other grants or credentials.
- Legacy signed snapshot becomes this metadata response with its existing ReadInbox requirement retained. Owner snapshot is unchanged. Message content comes only from bounded inbox.poll/ack.
- Context reads persist the usual registry fence and nonce together before releasing a result. A storage failure returns no metadata and leaves the same signed proof retryable; replay after success is denied. Discovery never creates an inbox lease/cursor or changes message/MLS/outbox state.
- MCP resources/list catalogs a single static agentic://runtime application/json resource; resources/read authenticates afresh on every call. Other URIs fail with invalid params without filesystem access. Current daemon denial returns JSON-RPC -32001 with sanitized structured daemon error and no context. Modern response uses ttlMs0/cacheScope private; legacy initialization remains supported. Three existing tools stay unchanged.
- Cancellation/stdio EOF must close pending signed IPC for resources as it already does for tools. Native packaged E2E obtains the tool conversation ID and data bound from the resource, then performs actual peer send/poll/ack and rejects context reads after UI revoke.

Tests changed: core conversations module registration and shared inbox test helper visibility; new support/runtime_context.rs; node processes.rs legacy content assertion moved to a real inbox poll (original peer-reply oracle retained); node support/mcp_stdio.rs resource flow plus existing cancellation fixture extended; native E2E and raw MCP client.

Separate context-free /root/node_test_critic returned REVISE because an adapter accidentally routed through legacy snapshot could pass dual-action tests but fail send-only discovery. Tests now exercise an independent send-only MCP process for both protocol versions: resource discovery succeeds, inbox read fails, actual peer send succeeds. FINAL ACCEPT followed before production changes.

RED: all3 new core tests compiled and failed Unauthorized for absent runtime_context. Real process integration compiled and failed resource catalog length0 vs1. These are behavioral failures against unchanged production.

GREEN: all41 core tests and17 node process tests pass, including exact metadata allowlist, sender-only read denial, SQL rollback, both MCP versions and resource cancellation/EOF. scripts/check-native.mjs then passed in full:148 Rust,20 frontend, format/Clippy/TypeScript/Vite, hidden packaged WKWebView E2E with MCP-discovered ID/bound, release build, strict/deep signature and driver exclusion. Log /tmp/ain-runtime-context-check.log. Native agent screenshots were opened beside component references; no visual regression. Owned native/MCP/test processes cleaned up. Current release .app contains this implementation. No complete upstream card or full V1 acceptance is claimed by this slice.
