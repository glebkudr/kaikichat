# Public message preparation — accepted Core dependency

Eight new tests and all eight prior public-wallet tests pass. The combined
Core and daemon regression passes **444 tests, zero failed/ignored**, across
9 nonempty suites. All 59 frontend tests, workspace
fmt/Clippy, TypeScript and Vite pass. All five reviewed R5 inputs and all
445 frozen source files remain unchanged.

`AppCore::prepare_public_postage_message` derives the paid operation from the
exact saved original MLS custody envelope. The envelope change, canonical
message association, wallet allocation and exact signature reservation commit
together through the existing checkpoint + SQLCipher transaction. No signature
is returned on a write failure. Retrying preserves the envelope and signature
while returning the current book balance and rechecking live trust. A changed
retention, expired preparation, corrupted association or stale checkpoint cannot
silently renew the message or consume another ticket.

Evidence uses actual MLS and actual-EVM funding fixtures, an independent receiver
Core, direct acknowledgement/deduplication, multiple messages/exhaustion, restored
aliases, compatible checkpoint refresh and current expiry, restart and genuine
SQL INSERT/UPDATE failures. Original resource/class bounds are preserved. Native
signing and envelope validation are reused; no new dependency or cryptography.

Review history is in `critic-review.md`; exact inputs and command results are
recorded in JSON. Raw local execution logs are ignored by Git under repository
storage rules. The failed compilation and earlier RED logs are retained locally,
not reported as successful checks. A filename caveat is documented in the review.

This does **not** wire a new daemon command or run a new live-EVM/packaged-app
gate. Automatic sender scheduling, shared UI/CLI/MCP wallet policy, R10, repair,
durable indexes, handover and remaining V1 acceptance are still required.
The next integration uses this Core entry point through the common daemon and
the real public paid-MLS network scenario.
