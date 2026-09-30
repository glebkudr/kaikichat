# Core receive ordering from conversation creation

Status: Core admission implemented after independent test acceptance; the required
130-original Core recovery passes. [Checks and native scope](https://github.com/glebkudr/kaikichat/blob/7563f614931f26e7dd1148a5c5053bb1e1537847/evidence/reviews/AR2-wallet-flow/RECEIVE_ADMISSION.md).
Ordinary graph traversal and explicit product gap/recovery outcomes remain open.

The previous R19 gate failed on 130 simultaneously live originals with arrival
offsets 1..129,0. Use the staged contiguous MLS operation in the shared Core application
reducer from initial conversation creation/Welcome onward. Both initiator and
recipient persist that policy atomically with the new contact, MLS snapshot and
Welcome/outbox. Every application path, including jobs, direct authenticated
transport, custody envelopes and v1/v2 imports, uses the same policy. Receipts and
exact already-stored message retries retain existing semantics.

A valid future application returns `CoreError::Crypto(ReceiveGap)`, with no ACK,
message, MLS advancement, import claim or completed traversal cursor. Retained
history plus absence of an import row continues to mean pending after restart;
the sender's existing outbox remains responsible for direct retries until ACK.
No unbounded receiver ciphertext queue or historical MLS snapshot archive is
introduced. Receive callbacks do not synchronously drain an epoch. Ordinary
bounded workers still need to schedule retries and surface gaps at the product
boundary; this Core step is not that network integration gate.

Preserve actual arrival offsets 1..129,0 in the required test. The first 129
attempts must now report a gap without mutation, including a cold reopen and a
direct-delivery attempt. Once offset0 arrives, fresh reference-bound retries
recover all130 exact original IDs/text/authors, in constant per-import writes.
Keep cold dedup, stale root tokens, SQL rollback and bounded leaf proof checks.
An expired earlier paid reference does not permit silently skipping its MLS key:
later live imports stay pending until the missing original arrives by a still
authorized route or an explicit recovery/epoch outcome handles the gap. Custody
sequence/leaf ordinal is not MLS generation; do not infer crypto order from it.

The application state becomes version2 when it contains this policy. Version2
contacts explicitly name `contiguous` or `legacy_bounded`; missing/unknown policy
is invalid. New empty profiles use version2. A version1 profile remains readable
without writes and retains bounded legacy receive semantics. Creating a new
contact on it atomically migrates existing contacts to explicit legacy policy
and protects the new contact. Old version1-only readers reject version2, rather
than ignoring the new invariant and consuming skipped keys behind it.

An existing unguarded contact cannot be upgraded by accepting a v2 history root
mid-ratchet: it may already contain skipped/evicted keys. Root admission and leaf
use return a distinct `HistoryRecoveryRequired` with all prior state unchanged.
Old v1 retrieval and existing messages remain available. No owner switch can
falsely certify that old ratchet as contiguous. A subsequent recovery/rejoin flow
must expose its recoverable range and gaps before replacing the old epoch.

Test both initial roles, ordinary direct ACK/retry, cold policies, an actual
legacy out-of-order receive, safe migration with SQL failure, missing/unknown
version2 policies, and the modified custody/import cases. Existing real MLS
epoch tests remain: this policy is retained across authorized fresh epochs, but
does not by itself provide a control log, first offline Welcome or recovery past
the three-epoch bound. The full 67-card/22-E2E/three-platform scope stays open.
