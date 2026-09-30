# Durable public postage book wallet V1

Status: tests-first design, not implemented or accepted. Implements the local
wallet boundary of `Docs/V1_POSTAGE_BOOK_2026_09_10.md`; daemon, UI, CLI/MCP and
actual no-prover network delivery require subsequent integration acceptance.

## Reuse and ownership

Reuse the existing SQLCipher custody intent keystore, profile trust, pre-beacon
registry seal verification, random Ed25519 seed/salt generation, zeroizing secret
entries and atomic state CAS. A backwards-compatible stored format discriminator
selects the existing legacy inner commitment or the public outer commitment.
Existing entries without the discriminator retain their legacy meaning. Public
preparation returns the actual public commitment to fund, never the legacy inner
opening. Preparation retries cannot change format, key, salt, epoch or seal.
Public and legacy intent listings are separated; neither returns private seeds.
The existing 32-intent limit applies across formats.

## Funding and accounting

Binding a public book requires the owned public intent, current authenticated
Core postage context and real native MPT funding verification. Count, class and
expiry come exclusively from the verified funded batch. The book state key
contains the issuer domain and actual public funded commitment, rather than a
caller-selected alias. Re-importing an equivalent proof retains the original
proof and all allocations. A new proof cannot repair corrupt saved evidence.

A book reports total, allocated and available tickets and the verified resource
class/funding expiry. Allocated includes signatures already exposed to callers,
even if the network has not yet finalized the spend. A wallet read is not a QC
or proof that an allocated operation was delivered. Current readiness requires
the current Core trust/head/time fence; offline metadata must not claim readiness.

## Reserve before release

The operation digest in the authenticated context is the local idempotency key.
Reserve the next unused index and persist its exact signature/context together
with the incremented allocation count in one SQLCipher transaction, before
returning the signed stamp. A failed INSERT/UPDATE returns no stamp and consumes
no local allocation. The existing profile lifetime OS lock admits one Core/daemon
owner; a competing owner is refused, and its successor resumes the durable state
after the first closes. UI/CLI/MCP requests use that shared daemon, rather than
opening competing wallets. State revisions protect the existing atomic transaction.
Distinct operations never receive the same index from a healthy shared database.

A repeat of the same operation returns the original stamp and allocation,
including after Core closes/reopens or a compatible checkpoint refresh. Changed
statement semantics conflict rather than silently replacing an exposed signature.
A ticket with an exposed signature is not released for another operation merely
because a client cancels or a network request times out. Finalized spend status
continues to come from the existing issuer-global log. Restoring or cloning an
entire old wallet cannot bypass that log; local accounting alone cannot detect
a rollback of all its own durable data.

Funding proof is stored once per book. Each bounded reservation stores the
operation context, ticket index and signature; the canonical count bounds the
number of reservation rows. No independent prover, per-message L2 transaction,
new signing primitive or independent network/finality service is introduced.

## Required tests before production

- Fresh public preparation, actual outer commitment, private-key exclusion,
  idempotency, SQL failure/retry and restart; legacy intent regressions unchanged.
- Restore the independently authored actual-EVM funded public fixture through
  SQLCipher (no production seed override); bind and spend multiple operations,
  verify each signature with an independent receiving Core.
- Exact operation retry, equivalent proof reload and restart preserve the
  allocation count and original signature. Exhaustion refuses another operation.
- Real SQL INSERT/UPDATE failure cannot expose or allocate a signature; retry
  succeeds at the original index. A competing profile owner is refused without
  mutation; after the original owner closes, its successor retries the exact
  stamp and allocates the next index. Actual concurrent UI/CLI/MCP requests are
  an additional daemon integration gate, not claimed by this Core test.
- Wrong/unpaid/late/legacy funding, foreign key/format and native MPT corruption
  cannot create ready balance. Current expiry/stale trust cannot create another
  allocation. A fresh input cannot heal corrupt saved funding or reservation.

Backend test critic must give FINAL ACCEPT before implementation. Full workspace
and frontend regressions follow implementation; actual daemon/no-prover delivery
and UI acceptance remain separately required for the product goal.
