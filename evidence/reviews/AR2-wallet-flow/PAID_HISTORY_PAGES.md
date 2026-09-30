# Paid index holder page retention

[Targeted validation](paid-history-pages-checks.json) passes: 106 backend
scenarios (50 paid-index, 54 paid-custody, two Runtime) and 31 frontend tests;
postage-spend/node all-target Clippy and formatting pass. All 124 focused source
hashes were verified after the checks. Eight new tests received independent
acceptance before production. A later fixture-only correction was accepted too:
the expiry setup had attempted admission before a descriptor's signed issuance.
Earlier RED and failed runtime logs remain recorded. Regression counts overlap
previous reports; they do not add independent product or native coverage.

## Scope

This implements [one operator's retention/read contract](../../../spec/index-history-pages-storage-v2.md)
for the existing signed leaf/branch/root codec. The actual paid descriptor,
funding, QC, historical trust, operator placement and receipt remain prerequisites.
A returned commitment acknowledges one committed body. It does not acknowledge
the entire referenced graph, establish root consistency, publish a pointer, or
mark a message delivered. An absent child remains a recovery gap.

Each admitted anchor has one 32 KiB allowance for unique signed history wires,
including the legacy slot and all retained page versions. Pinning the identical
legacy leaf shares that logical allowance; its two physical encodings both count
toward global storage quota. Retained pages cannot be replaced or evicted while
their original paid expiry remains live. A mutable v1 update cannot free pinned
bytes or bypass the allowance. Exhaustion is an explicit failed admission.

The page map is part of the existing operation-addressed SQL row. Its full compact
JSON encoding is charged alongside portable index evidence and locations. This
keeps a single bounded mutation and uses the existing body digest, counters, CAS,
clock and expiry transaction. It does not introduce an unbounded lifetime map,
new payment, fabricated sponsor, independent GC queue or new transport.

## Test contract

Eight scenarios use the existing independently funded public-index fixture,
real finalized spend records, actual operator admission and encrypted SQL state:

- Persist leaves, a branch and both roots, reopen holder and recipient trust,
  and read exact portable bundles after the short admission context expires.
  Check actual child links and old-to-new consistency. A root stored before its
  children does not turn a missing branch into a complete graph.
- Refuse unadmitted anchors, another admitted anchor, wrong typed purpose,
  damaged signatures, foreign direction, altered commitment fields and missing
  historical trust, including exact retry after trusted reads. Recipient scope,
  target and original sequence range apply.
- Enforce exact global map accounting at the one-byte boundary; roll back body,
  entry and head SQL failures. Later paid admissions share occupied quota.
  Lowered admission limits preserve existing reads and exact retries.
- Require the full portable read budget and successful read-clock commit before
  releasing positive or negative results. Cold retry works; clock rollback fails.
- Fill one actual paid anchor with signed root/leaf versions and the pinned
  legacy leaf. Refuse bytes above the shared allowance through either API, while
  every previously retained version survives cold reads and exact retries.
- Retain two genuine roots at the same revision without selecting a current head;
  consistency verification still rejects their substitution. Both survive restart.
- Corrupt one page's signature while keeping its map key, requested hash, local
  body digest and quota counters consistent. Fail reads/updates while another live
  anchor remains readable. Exact restoration restores all SQL rows and the page.
- Fail and retry expiry cleanup; collect only the expired anchor, preserve the
  longer one and its quota, and refuse lease renewal through page retry.

## Remaining whole capability

Ordinary workers still use the v1 flat directory. Next integrate typed network
requests and bounded publication/traversal state with actual page acknowledgments,
root/pointer fences and atomic per-reference recipient imports. Then run the
ordinary >128 paid native loss/recovery gate, including disjoint rosters, shorter
leases, missing/corrupt pages and cold sender/recipient restarts.

A long-lived anchor cannot fund unlimited roots after shorter new leases. The
network path must retain an explicit pending/limit state and preserve older live
publication promises. Additional funding requires a separate evidenced contract;
this storage component does not manufacture it. All 67 cards /22 E2E /three
platforms remain required. No new native or full-V1 acceptance is claimed here.
