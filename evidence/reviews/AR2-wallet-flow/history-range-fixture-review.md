# Native range fixture feasibility review — 2026-09-12

The independent context-free backend test critic revised its earlier acceptance
of the range fixture to **FINAL REVISE** after checking the real admission budget.
The recovery assertions remain required; the fixed 1800-second snapshots make
the current setup impossible even if redundant traffic is eliminated.

The test waits for eight completed batches before sending originals 129 and 130.
All 128 earlier originals use one book's stable index roster. Each common
index-holder must accept at least these Alice requests:

| Request | Minimum |
| --- | ---: |
| Index promises | 128 |
| Location batches | 128 |
| Immutable leaves | 128 |
| Binary branches | 127 |
| One completed root per batch | 8 |
| Total | 519 |

`paid_custody.rs::serve_authorized_paid_custody` applies the same
`Admission<32,16>` to these requests: 16 per sending PeerId and 32 total per
receiving daemon per 60-second window. Paid requests and exact retries have no
exemption. Responses, IPC, finality and the separate discovery protocol do not
consume this budget. Data puts, failures, duplicate work and computation are
excluded from this lower bound.

519 requests require 33 windows. Even with the most favorable phase of the first
window, at least 1860 seconds elapse between the first and last admitted request.
Both snapshots were sealed before the first request and expire after 1800 seconds.
Historical page writes can outlive the live head, but the last two new originals
cannot then be admitted. Merely keeping Anvil running does not create new heads.

## Required fixture correction

Before deployment/payment, select longer initial admission snapshots for both
registries, for example 3600 seconds, allowed by the existing contract's admission
lease range. Keep checkpoint `maxLeaseSeconds=1800`; issue real successor heads
from fresh Anvil blocks and refresh corresponding proofs and operator/finalizer
publications through the existing paths. A newer head alone cannot change an
already sealed snapshot's immutable `admissionUntil`.

Message retention and purchase lifetime remain 3600 seconds; R10, network
admission, page budgets and the final earliest-original-live assertion remain.
The longer snapshot removes the proved impossibility, not the requirement to
demonstrate actual timely publication and recovery. The new concrete fixture delta
and head-continuity oracle must be independently reviewed before execution.

R4 was interrupted with SIGINT after 32 originals completed, preserving the
[sanitized evidence](history-range-native-r4.json). All 823 inputs stayed unchanged;
exit130 and empty cleanup errors are recorded. This is a deliberate stop of an
invalid fixture, not a full native pass or a new product timeout failure.

## Implementation path for the next test delta

1. Parameterize only the two registry snapshot arguments in
   `postage_spend_node.py`; preserve default1800 for other gates. The range selects
   snapshot3600 at deployment, while its checkpoint maximum remains1800.
2. Add a test-owned real-chain renewal helper, fingerprinted by the base gate.
   It mines a fresh Anvil block, signs a linked successor under the same fixture
   trust profile, obtains exact proofs for that state root, and updates existing
   finalizer and custodian publications using actual current revisions. It must
   preserve both snapshots, paid book, committee/log identity and stored tickets.
3. Invoke the helper from ordinary sender polling and before subsequent batches.
   The ordinary Alice client must receive the successor and public authority
   through its existing peer sync; the helper must not configure Alice's client
   authority or supply wallet proofs. Read-only checkpoint status can be added
   explicitly to the test observer's permitted methods for this assertion.
4. Record every generated head and observed peer acquisition. The independent
   placement oracle must select the known head named by each completed result;
   old completed results cannot be compared blindly with the latest head. Reject
   unknown heads and keep exact ticket/journal/roster/receipt verification.
5. First exercise renewal between two ordinary paid originals in a focused gate,
   including the existing exact retries, budget and cold loss/recovery assertions.
   Require a real renewal/acquisition event, then run all130 with periodic renewal.
   Full acceptance still requires every original live at final recovery/cold scan.

The concrete correction is now implemented in the test harness and independently
[accepted](wallet-renewal-critic.json). Review first rejected a stale final balance
query, missing ordinary wallet refresh and a setup sentinel in the recovery MLS
chain; all three are corrected. This acceptance covers test validity, not native
success. The [focused successor gate](WALLET_RENEWAL.md) precedes the full130 run.
