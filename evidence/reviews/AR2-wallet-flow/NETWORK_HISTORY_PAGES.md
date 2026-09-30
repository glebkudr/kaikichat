# Paid history page network ingress

[Targeted checks](network-history-pages-checks.json) pass: six new page scenarios
and nine transport regressions, plus 31 frontend tests, node all-target Clippy
and formatting. The final checks bind 133 focused source/fixture hashes. Counts
overlap earlier reports and do not add independent product or native coverage.

The [network contract](../../../spec/paid-history-pages-network-v2.md) now exposes
typed leaf/branch/root put and exact-commitment read on the existing paid-custody
protocol. It reuses paid storage verification, accounting and WireIndex evidence.
Only a committed body returns its exact commitment. Reads return the signed page
and genuine paid anchor; missing pages remain unavailable. New page operations
return explicit capacity errors for quota, page size and full portable read budget.
Legacy messages and their error mapping are preserved.

Ordinary dispatch and tests share the same internal connection/admission boundary.
Actual peer/connection identity, relay-only policy, sixteen requests per peer and
32 total per minute remain enforced. The existing 1 MiB frame limit, four streams
and ten-second timeout remain. No public clock override or alternate server path
was introduced.

Six scenarios exercise real Runtime TCP/Noise swarms with actual paid public
fixture obligations, independently encoded request frames and exact decoded
responses. They cover old roots, missing children, legacy compatibility, reopened
storage and historical reads after the live checkpoint admission fence; wrong
anchors/purpose/signatures/commitments/capabilities; SQL failures before ACK and
read release; missing historical trust despite a real profile; quota/size/budget
boundaries and expiry; actual connection/relay/rate fences. Oversized and malformed
CBOR checks exercise the decoder directly; valid requests cross actual Noise.

Independent test review first corrected an assumed eight-request cap and a
trust-negative profile without identity. The first runtime then exposed a fixture
setup error: private postage could not admit a public index. The accepted fix
reuses the existing canonical public fixture, its actual sender key and real
checkpoint admission deadline. A final accepted test-only change shares the
existing independent serializer instead of loading it twice. Production remained
unchanged through both runtime/lint corrections. Failed logs remain recorded.

These are fixture-domain/time network checks. They do not establish an ordinary
wall-clock native paid flow, durable sender ACKs or whole-graph availability.
Ordinary sender/recipient workers still use v1. Next implement bounded durable
page publication and traversal, exact current-root/pointer fences and atomic
per-reference imports. Preserve every live paid page when capacity is exhausted.
Then run ordinary >128 paid native loss/recovery with cold sender/recipient,
disjoint rosters and missing/corrupt pages. All 67 cards /22 E2E /three platforms
remain required; V1 is incomplete.
