# Shared broker and atomic retry review

Scope: encrypted runtime registry, signed scoped calls, per-call current authorization, durable nonce replay checks, isolated operation IDs, shared MLS/history/outbox send reducer, owner revocation. This is core integration, not complete MCP/native agent management.

Tests preceded production. Separate `/root/core_test_critic` (originally fork_context=false) returned REVISE: follow-up MLS decrypt did not prove no generation advance; revoked committed send retry and cross-principal operation IDs were missing. Revised tests compare persisted state bytes/revisions, original wire and all state after failure/reopen, deny old send retries after revoke, and deliver independent messages for owner and two runtimes with the same operation ID. Added nonce-content replay and invalid registration cases. Store tests cover explicit retry-only state selection and rollback of an earlier fence when a later nonce write fails.

FINAL ACCEPT before production. Store RED independently confirmed the absent commit_outgoing_with_retry_states API; core RED confirmed absent broker types/methods. A subsequent mechanical tuple borrow correction (StateValue zeroizes on Drop) received separate FINAL ACCEPT; expected state bytes and revisions were unchanged.

Core suite now28 tests (7 broker additions); store suite now24 tests (2 additions). Full backend/frontend verification is recorded in IMPLEMENTATION_STATUS.md after the run. Remaining IPC/MCP, pagination, cancellation and native permissions have no acceptance claim here.
