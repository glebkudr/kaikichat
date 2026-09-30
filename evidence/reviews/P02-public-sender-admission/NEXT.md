# Next execution gate for the automatic public-postage sender

The admission module alone does not implement automatic sending. Continue with
one ordinary-message execution gate that proves native preparation, remote
spend finality, actual selected custody and automatic recipient recovery.

Use the same Core jobs and sponsor identity for owner UI/CLI and signed MCP/CLI
messages. Owner configuration must reuse the existing bounded public authority
adapter; normal messaging grants cannot configure the wallet or inspect other
jobs. Pause/revocation before allocating an unexposed stamp must be checked by
the trusted Core boundary. Preserve already exposed allocations; no retry,
policy update or cancellation may make their indices free again.

For each durable message ID, reuse prepare_public_postage_message so a crash
between its commit and the separate spend-client database cannot allocate twice.
Read an existing spend-client request/result before creating another; preserve
the original operation, candidate and QC across restart. Current checkpoint
refresh cannot be handled by changing a saved request's input hash arbitrarily:
revalidate any authority refresh against the same native candidate/journal.

Generalize the existing custody resolver to the verified SpendCandidate's
placement. The old resolver depends on legacy CustodyAssignment; constructing a
fake legacy-funded assignment for a public book is not a valid adapter. Reuse
existing proof query streams, connection/transport pinning, admission limits and
finite ordinals. Verify selected membership plus the current operator binding
before putting, then independently verify/retain actual returned storage receipts.

Reuse PostageClient, PaidCustody Service, durable outgoing receipts and private
mailbox publication. Bounded concurrent puts can cover the ten selected primaries
with one stamp. A rejected in-memory request ID cannot be polled forever; retries
need bounded attempts/backoff and must first consult durable outgoing receipts.
Treat spend finality, replica count, pointer publication and recipient delivery
as distinct states. Do not expose an owner-set paid/stored boolean as evidence.

The existing live EVM base supplies real owner-generated funded books, four
selected finalizers, 17 registered custodians, independent cryptographic oracles
and self-tested zero-ZK tripwires. The new gate must start from ordinary send
after owner setup. During sender execution the harness must not call prepare,
reserve, request_postage_spend, request_custody_put or publish_mailbox to perform
the daemon's work. Observe read-only status, assert real QC/storage and one
allocation across restart, then stop sender and recover the original ciphertext
through the returning recipient. Keep independent placement/signature checks,
actual disk-failure retry and budget/scope exhaustion controls.

R10 repair, independent durable indexes, admission-expired live copies, epoch
handover, UI funding/balance/queued/error views and three-OS release evidence
remain full-V1 obligations. Existing manual paid-custody gates do not close them.
