# Full pending queue across checkpoint renewal

Baseline f4693a8. Same epoch only; 67 V1 cards, 22 E2E and three platforms remain.
An authenticated successor checkpoint may refresh the public context of an
already accepted receipt. This must replace its old active envelope durably,
without consuming another of the 16 candidate slots, changing its canonical
journal/nullifier or forgetting an earlier finalized spend. A fresh unrelated
candidate still encounters the capacity limit. Old checkpoint and altered receipt
evidence must not replace anything. Never evict unverified old inputs by trusting
their operation field; authenticate equivalence under the incoming current context.

The new native test uses actual funded public tickets, signed contiguous
checkpoint history, eth_getProof witnesses and four ordinary generated validator
keys. It first finalizes one spend, isolates one validator, fills its entire queue,
then renews its head/roster/configuration. The same canonical receipt in a differently
formatted carrier with fresh context must attempt a real SQL update; an injected
failure leaves revision and bytes unchanged after crash. Removing the fault allows
all 16 replacements with unchanged capacity; another cold restart must restore
them. Three replicas return only after the old lease actually expires. They receive
refreshed envelopes through ordinary peer transport and finalize the 16 retained
spends, then a fresh one. Independent QC verification, heights 1..18, exact original
records and consumer cursor 18 prove continuation rather than log reset. A conflicting
operation using the original spent ticket is refused. Final solo cold reads retain
all records and zero active inputs.

The only shared harness changes are an optional pre-stop genuine successor capture
hook and test-only SQL write fault/snapshot modes. No production code changed yet.
This gate does not implement or prove epoch closing/handover. It exercises selected
validator submission/rebroadcast; ordinary-client automatic renewal remains separate.

R1 critic revision: a distinct valid funded ticket with the same operation as a
retained candidate must still receive postage_limit without changing SQL bytes,
both before replacement and after the successful replacement/cold restart. This
negative case is separate from the operation-keyed independent finality oracle.
Retry twice while the SQL fault remains enabled to catch an early memory update.
Include stale-envelope rejection in the unchanged snapshot assertion and check
every canonical parent link back to the same original committee genesis.

Execution refinement: the first implementation run passed two real SQL failures,
all 16 replacements and cold pending reads, then asserted service-running on the
first loop iteration immediately after IPC startup. Service startup is asynchronous.
Wait at most 15 seconds for the renewed service, retaining the latest observed service
snapshot for failure diagnosis; the subsequent original-lease loop still requires
uninterrupted running authority. No production change accompanies this test fix.

Historical-window extension: the first green envelope run captured its successor
before the original spend's verification time. To cover real old-record reads,
capture an actual future-timestamp successor block, wait for its real issued time
before accepting it, and require all original records to predate that issued time.
The final cold reader has its operator role disabled. A new Rust test uses genuine
historical authority and QCs to distinguish opening the retrospective lower bound
from extending the checkpoint's upper expiry. Its earlier verification timestamp
is a fixture control, not evidence of a real signing clock; the native scenario
must supply the actual chronological earlier spend. No history-window production
fix was made before these tests.

Legacy-format regression alignment: the existing client_records test explicitly
rejected every record predating checkpoint issue time. The new historical-window
contract supersedes that lower bound, so retain its earlier-QC control as a positive
and move its negative to before the immutable registry window. Its future-observer,
checkpoint-expiry, receipt-expiry, damaged/insufficient-QC and metadata negatives
remain intact. The previous assertion failed on the new production behavior as
expected (output/ar1-checkpoint-history-legacy-contract.log). This test-only alignment
follows the combined native gate, which passed the actual earlier-record chronology
and role-disabled cold read; no production changes accompany it.
