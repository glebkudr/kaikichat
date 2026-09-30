# L06 bounded checkpoint history — verified increment

Full V1 goal remains active. This increment implements Core history and catch-up, not automatic
network fetching, independent attestors, live chain finality, spend authorization or R=10 custody.
The authoritative contract is spec/postage/checkpoint-history.md.

## Test-first review

L2 expired-history/funding test and six Core business tests were written before production.
Initial RED output contained test API mistakes, corrected before the final RED gate; final
RED fails for missing history APIs. Separate context-free node_test_critic returned FINAL REVISE:
the denied-time oracle could be accidentally satisfied by a later status read, and fresh joining
only used sequence1. Tests now inspect exact persisted SQL immediately after refusal and make
lower-time advancement the first call after reopen. A fresh selected client joins a source
already at sequence10. The reviewer then returned FINAL ACCEPT. A subsequent mechanical Clippy
range-loop correction was separately reviewed: FINAL ACCEPT, unchanged assertions/order/time.
No work proceeded between critic dispatch and its final decision.

Tests exercise actual SQLCipher transactions, real signed certificates and existing real MLS
messages/receipts. They cover expired multi-page catch-up, current funding, fresh joining, legacy
head serving without rewrite, strict anchors/successors, invalid signatures/profile/future/clock,
SQL INSERT/UPDATE rollback, exact restart, pruning, corrupted archive and a real32-key signer set.

## Implementation and evidence

TrustedCheckpoint::authenticate_history reuses complete canonical/signature/profile verification
at min(current time, signed issue time); preliminary typed time extraction is untrusted.
Funding still uses actual time. Core stores exact bounded public certificate history separately
from the existing bounded version1 head. Both rows and the clock share one SQLCipher transaction.
All entries authenticate and strictly chain to the saved head before serving; no skip/reset fallback.

Targeted GREEN: L2 one test and Core15 filtered checkpoint tests; corrected Clippy exit0.
The isolated mutation script copies crates to an APFS scratch workspace and checks compiling
mutants against named business-test failures. All six were killed: future issue hint, funding
expiry bypass, skipped successor, stale anchor, lost denial clock, manual archive atomicity.
Report L06-checkpoint-history-mutations.json contains hashes and empty cleanupErrors.

Full native gate exit0:279 Rust,40 frontend,14 Solidity/256 fuzz, real Anvil funding and five
packaged hidden WKWebView scenarios. Release builds and strict ad-hoc codesign passed; no Apple
notarization claimed. Native active/restored checkpoint screenshots were visually compared with
the preceding committed reference: same readable layout and identical accepted head/lease after
restart. The new catch-up itself is verified through real Core/SQL tests; no peer catch-up E2E claimed.

Linux gate exit0: seven actual outcomes, run ain-nat-82107cb4,
sourceHash97c863dfc2ffce2c349e710031c6390136fd8ba8b1f7a30cacdaee0520060397,
cleanupErrors empty. Separate labelled container inventory was empty after completion.
Daemon/Anvil sourceHash1e63ac02d5a5f8327f17de0c8fce312da173b6673703b0dd9a696c19578f113c;
trusted CLI sourceHash5f79f02d6546bfd080322b908f2890c6afb37ebdc1443b46c6f42f2a1a762d16.
Raw logs retain actual commands/outcomes. No dependencies installed in this increment.

## Remaining product work

Integrate automatic bounded peer fetch/serve, including restart, failed source fallback and
untrusted wire tests. A finite archive can lack an old anchor; additional availability is required
for full long-offline support. Live attestations, registry, admission/spending, actual independent
ciphertext custody and repair, groups/jobs/trust and all E01–E26 acceptance remain open.
