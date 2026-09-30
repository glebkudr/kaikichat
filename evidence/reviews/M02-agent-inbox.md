# Durable agent inbox test review

Before production, five real SQLCipher/MLS tests exercised polling, ack cursor, scoped incoming-only pages, finite runtime leases, expiry takeover, idempotency and SQL rollback. Separate core_test_critic returned REVISE for missing scan-only boundary, distinct agent/service isolation and expiry rejection before replacement.

Added a real1000-send history followed by incoming work: the first scan-only page has a lease and advancing cursor, ack permits the next page to reach the incoming message, then polling becomes empty. Separate agentId and serviceId changes each have independent leases/cursors while previous contexts retain live leases. Expired current ack fails before replacement with unchanged persisted state. Also compare owner snapshot/outbox/MLS across poll+ack, replay an already committed ack after expiry, and inject SQL failures on both INSERT and UPDATE.

Seven tests demonstrated RED on missing inbox commands (Unauthorized on first valid poll). FINAL ACCEPT preceded production. The unchanged seven tests pass after implementation; the long-history test uses real owner sends and an actual peer's MLS message. Operation/cursor/lease/replay/fence changes share the existing state transaction; no network/MLS/history/outbox mutation occurs during polling or acknowledgement.

This module establishes durable inbox primitives through the shared broker. It does not establish full M02 acceptance, runtime scheduling or MCP subscriptions.
