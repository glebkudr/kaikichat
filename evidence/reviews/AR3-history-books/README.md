# Ordinary recovery across genuinely disjoint funded books

Full declared scenario: two ordinary messages, two real paid books with disjoint
original ten-holder and ten-index rosters, a cold sender between books, a shorter
second lease, then sender absence and actual provider loss. The recipient must
recover both exact originals through their own remaining paid indexes/holders,
with atomic SQL rollback, partial progress, cold retry and deduplication.
[Exact test contract](TEST_CONTRACT.md) · [Independent review](critic.md).

The fixture has 61 genuinely bonded daemon operators and an actual exit, leaving
60 active providers plus two seeds within the existing ordinary connection limit.
Twenty-four Core commitments are paid before the genuine beacon. Existing
independent data/index draw oracles select a disjoint pair only after the seed;
there are no post-beacon purchases, fabricated assignments or owner work RPCs.

Two observed network failures required changes. Bounded custody resolution now
continues its candidate/position scan separately for each authenticated assignment
using its existing short-lived job cache. At the ordinary connection ceiling,
duplicate sockets yield capacity to distinct peers while retaining a live socket
per peer and preserving selected capacity and physical close accounting.
The 32-candidate, four-request, 64-ordinary-connection and timeout bounds stay.
[Resolution contract](../../../spec/registry/custody-resolution-v1.md).

## Retained unsuccessful runs

| Run | Result and response |
|---|---|
| [baseline-books-1](baseline-books-1.json) | Four data receipts; retries kept the first 32 candidates. Initial 64 providers plus two seeds also exceeded the ordinary limit. Reduced only the fixture to 60 active providers through separate review, retaining disjoint draws and >32 candidates. |
| [candidate-books-1](candidate-books-1.json) | First book fully stored. The fixture assumed a policy revision that ordinary reservation legitimately advanced. It now reads the actual revision before CAS; independently accepted. |
| [candidate-books-2](candidate-books-2.json) | First book fully stored; second stopped at eight receipts after cold restart. Sixty-four sockets reached only 53 distinct peers. |
| [duplicate-capacity-red](duplicate-capacity-red.json) | The separately accepted real NetworkBehaviour test reproduced starvation before the connection fix. The affected 18-test cluster subsequently passed. |
| [candidate-books-3](candidate-books-3.json) | Fixture stopped before sends: 24 pre-beacon books produced 39 disjoint index pairs and 28 disjoint data pairs, but no pair satisfying both. No recovery result. One explicitly announced unchanged retry received independent ACCEPT. |

[Resolver diagnosis](baseline-diagnosis.json),
[duplicate capacity diagnosis](duplicate-capacity-diagnosis.json) and
[finite-population diagnosis](fixture-population-diagnosis.json) preserve concrete
observations. Raw process logs/traces stay in local managed output; reports bind
those files by SHA256. No unsuccessful result is converted to GREEN.

## Acceptance boundary

[Candidate-books-4](candidate-books-4.json) passed in 418.26 seconds with 614
unchanged inputs: six independent QC signatures, 20 original data receipts,
20 index promises, 200 location acknowledgments and both exact imported messages.
Cold sender/book switch and mixed retention passed; actual index/data loss,
missing trust, both SQL faults, partial progress, cold recipient/cache loss and
dedup passed. Owner work/retrieval calls and prover invocations are zero.
[One-book compatibility](one-book-compat-1.json) and [final checks](checks-1.json)
pass on the same application inputs and binary: **94 backend /21 frontend, Clippy/fmt, 614 unchanged inputs**.
[Acceptance](acceptance.json) binds all source/log/trace hashes and independently
rechecks the signed directory. [Retained public evidence](books-evidence.json)
includes genuine funding/draws, exact prepared messages and receipts, signed
history, loss and atomic/cold progress. Shared identical presentation fields are
factored with exact reconstruction checks; no original signature is dropped.
This native result proves the declared two-original range within one MLS epoch.
It does not prove
arbitrary history beyond the finite manifest, actual MLS control/Welcome
continuity, successful retirement, autonomous R10 repair, independent testnet or
three-platform release. V1 remains 67 required cards /22 E2E /three platforms.

[Next work](NEXT.md) · [Full capability matrix](../../../Docs/agentic_internet_v1_execution_plan/CAPABILITY_EVIDENCE.md).
