# Ordinary graph receiver integrated

The automatic custody worker now probes the exact signed pointer through typed
paid history reads. It verifies the full paid anchor, historical authority,
recipient, index/epoch, kind, commitment, lease and complete portable byte budget.
Cached v1 directories retain their old cursor; one failed typed root probe can
fall back to the real v1 directory request on that endpoint. Child requests use
their own signed candidate roster, preserving access after root-holder changes.

Root admission gathers the bounded old-live-peak or unchanged-v1-leaf proof
before Core's checkpoint transaction. A pending replaced pointer, wrong parent,
missing child, or SQL failure cannot advance that checkpoint. The receiver claims
a durable reference ordinal before path I/O, holds one path/current leaf, reuses
the common authenticated prefix, and consults Core leaf progress before fetching
the original descriptor and ciphertext. Sixteen attempts bound each work item.
Expired authenticated subtrees can be skipped without child bytes. Missing live
bodies stay pending; real MLS ReceiveGap releases the old attempt for later retry.

[Independent tests-first review](graph-receiver-critic.md) accepted eight Runtime
cases and three recipient paid-response cases. Runtime uses 130 genuine MLS
originals, exact IDs/authors/text, bounded cold passes, real gap errors, root
INSERT/UPDATE rollback and v1→v2/v2→v2 consistency. Paid validators receive actual
TCP/Noise fixture responses, including damaged QC evidence and missing local
historical authority. These distinct fixtures are not a funded graph-native gate.

[Checks](graph-receiver-checks.json): **30 backend /31 frontend**, all-target Node
Clippy and formatting pass on **816 unchanged inputs**. The earlier 28-test
exploratory pass also covered both operator-retention tests; it is not added to
these counts. The existing v1 attempt-budget regression caught a real integration
error, corrected without weakening its test. Initial failures remain recorded.

[Ordinary paid native regression](graph-receiver-native-c1.json) passes actual
funding, two originals, exact page ACKs, SQL faults, cold retirement and loss/
recovery with sender absent, six verified QC signatures and clean teardown.
It uses the current node/CLI sources and a new isolated `output/gr-c1` directory.
Only sanitized evidence is exported; raw traces and secrets remain local.
This is flat-v1 compatibility, not a rebuilt GUI or >128 live paid graph recovery.
Automated desktop tests continue using the isolated vault; production retains
Keychain. No Keychain authorization is needed by this native CLI gate.

Next: integrate ordinary sender child-before-parent ACK and root-before-pointer
publication using the existing paid page ledger and completed-root checkpoint.
Then verify >128 simultaneously live paid originals with sender absent. Keep
finite per-anchor quota; capacity failure cannot be treated as publication.
Product gaps/rejoin, first offline Welcome, control beyond three epochs, groups,
independent 10→7→10 repair, E11 host/NAT, providers and all 67 cards /22 E2E /
three platforms remain required. Full V1 is not accepted.
