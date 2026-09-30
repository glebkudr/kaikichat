# MCP stdio adapter test review

Reviewer: separate backend-test-critic /root/node_test_critic, originally started without fork context.

Tests preceded production: empty agentic-mcp executable made four black-box tests RED; queued IPC test failed compilation only on missing Command.execute. Critic REVISE: keep stdin open while checking public credential startup refusal; cancellation must preserve a usable MCP process. Both fixed, ACCEPT before implementation. Invalid limit and unknown fields separated, all stdout lines checked, stderr retained across restarts.

After implementation, real macOS accepted test socket inherited nonblocking mode; fixed fixture with set_nonblocking(false), keeping 3s socket deadline and 2s EOF oracle. Audit drains remaining stdout on shutdown. Critic ACCEPT for fixture corrections. No oracle weakened.

Validation: scripts/check.sh PASS, 138 Rust +15 frontend tests, strict Clippy/format/TypeScript, production Vite build. Modern and legacy clients are independent raw JSON-RPC subprocess fixtures, with actual TCP/MLS daemons and SQLCipher. Cancellation and EOF close signed IPC; owner queued-call test proves disconnected command cannot create a profile and connected command can.

This is messaging MCP evidence, not completion of M01/M02/M06 or V1. Runtime provisioning UI, groups/jobs/reviews, subscription and packaged MCP integration remain pending.
