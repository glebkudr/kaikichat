# Independent test acceptance

The existing reviewer `/root/index_holder_test_critic` was separately created
without inherited context. It reviewed the new ordinary two-book scenario and
shared helper diffs before production edits. It did not edit files or run checks.

Initial REVISE found the legacy data-draw oracle and mismatched 1200-second
checkpoint profile. Both were corrected: public ticket ID plus the existing
private-custody oracle, actual data/index receipt equality, and the existing
600-second authority lease. Funding/draw evidence is retained before pair choice.
The corrected full test and both shared freshness runners received ACCEPT.

The first native baseline on f201180 failed at 4/10 real data receipts. The
reviewer confirmed the repeated first-32 resolver window and position-zero
scheduling. The revised fixture uses 60 active providers plus two seeds within
the existing ordinary connection budget; disjoint 10+10 data/index draws and
more than 32 candidates remain. It received ACCEPT before the resolver fix.

The reviewer required assignment/purpose-specific continuation so two concurrent
jobs cannot each remain in a fixed half through a shared global cursor. It also
required retaining actual authority/Noise/connection checks, deadlines and physical
pending+draining accounting. The native E2E meaningfully guards the observed bug;
no test duplicating cursor arithmetic or fabricated additional RED was requested.
Concurrent ordinary sends remain a separate compatibility regression.

Accepted test/helper SHA256:

| File | SHA256 |
|---|---|
| tests/evm/public_history_books.py | 3bebaa8408bb2b5e6f9276738700c2af9a08c8bfafc4e08e7d17caed0a1ea12e |
| tests/evm/funded_custody.py | ae35f79a6374940e0a83735d19ecbf2c17e18fe783276249e35e1614c044c850 |
| tests/evm/postage_spend_node.py | 293aae4d4953fa040b9545388838c431be2e483b07dc6492a5bc88654dc1a544 |
| tests/evm/paid_ciphertext_network.py | c8d171f8577534cc792708882cb612ff2ea2ff124b91a00ae056cac5f6530390 |
| tests/evm/public_sender_daemon.py | b6e0c6d9091bf34f1b8ecf672d61ff8de00be912aac931188d4b72a2deb14ac3 |
| tests/evm/public_index_recipient.py | 86a186c5847d6ebf30727ac63c05fb6e0e67a43696338bb302983f44e33a7981 |
| evidence/reviews/AR3-history-books/TEST_CONTRACT.md | 6f59a3536ebf175b1178548dee3bacc14a7b9d96ff8857b89e6bcab4fdc2c171 |
| evidence/reviews/AR3-history-automatic/run_runtime.py | 9ff276195f56343ea474605764a0f06a0446600a3f88b4dfeeafecec66122a50 |
| evidence/reviews/AR3-history-automatic/run_checks.py | 65eef6cdef35135c25ffdaf54c51c33aec4bebae022cdb4365f16d9d37785f01 |

## Read the current policy revision after an ordinary send

Candidate-books-1 reached firstStored: R10, ten index promises, 100 locations and
ten history ACKs. Its next policy CAS wrongly assumed revision two, although the
ordinary message reservation had legitimately advanced that policy revision.
The fixture now reads `public_sender_policy`, configures against that actual
revision, and still asserts the exact config and revision+1. The read-only route
was added to the ordinary caller allowlist; before/after policies are retained.
No production code changed for this fixture correction. The reviewer returned
ACCEPT, with no additional required scenario. Current test SHA256:
`7a572438f1ef28a01f47aa3fc932d7c5aa83e4653f4766bbb419ffddea531cba`.
The earlier accepted hash above describes the preceding genuine failed run.

## Duplicate sockets after cold reconnect

Candidate-books-2 completed the first book and switched policies correctly, but
the second message stopped at 8/10 receipts. The final actor snapshot shows 64
ordinary connections to only 53 distinct peers. With no excess above 64, the old
connection limiter never retired duplicate ordinary sockets to make room for the
missing independent providers.

A new test in `reserved_connection_tests.rs` calls the real NetworkBehaviour
admission and connection lifecycle methods. Sixty ordinary peers plus four sibling
sockets fill the existing limit. Spare capacity permits overlap; pressure must
retire only duplicates, preserve each last live socket and two selected sockets,
and release capacity only after an actual ConnectionClosed event. Existing unique,
demotion, absolute and per-peer tests remain intact. The reviewer returned ACCEPT
before `reserved_connections.rs` changed. Accepted test SHA256:
`0b4ba730406a82773f0323244f3d8514247076c532927b3063fdcc659de4e478`.

## Finite pre-beacon population failure

Candidate-books-3 stopped in authorization, before ordinary sends: all 24 actual
purchases/draws were retained, with 39 disjoint index pairs and 28 disjoint data
pairs, but zero pairs satisfying both. This is fixture failure, not a recovery
result. The reviewer confirmed the unchanged `7a572438…` test and returned ACCEPT
for one explicitly announced fresh run. Preserve this failure, prepare/pay all
books before the new beacon, and retain population, assertions and deadlines.
There is no automatic retry. If this fixture failure repeats, revise the fixed
pre-funded population through separate review. A succeeding runtime demonstrates
that real scenario, not reliability of this probabilistic fixture.
