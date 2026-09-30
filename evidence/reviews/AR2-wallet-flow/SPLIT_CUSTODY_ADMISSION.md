# One original across authority renewal

The unchanged full130 R10 gate stopped after48 stored originals. In the next
batch, sequences49,51,53 each had10 genuine data receipts and0 index receipts.
Those three originals had actual data presentations from both sides of the
peer-acquired authority successor. Recovery was never reached.

An original's data and index admissions can legitimately occur under different
accepted authority heads. Retaining only the first data receipt's proof caused
the sender to reject later index receipts, and caused compact holder lookup to
reconstruct valid receipts with the wrong proof.

The sender now retains an optional per-receipt proof when it differs from the
object's common proof. Index publications retain their own admission proof.
Full and compact location export reconstruct the exact holder context; the
recipient uses the same reconstruction before its existing full verification.
The envelope, finality QC, signed receipts, placement and expiry remain exact.
Additional serialized proof bytes count against existing sender/index limits.

## Test-first evidence

- [Store test review](split-admission-test-review.json): three genuine failing
  tests before production, then focused GREEN. Covers old data/new index, mixed
  data receipts, cold persistence, full/compact refusal with an intact signature
  but wrong proof, atomic SQL rollback/retry and evidence byte limits.
- [Recipient test review](split-recipient-test-review.json): genuine failure then
  GREEN through the actual holder-check helper, including a same-network reader
  without installed trust, substituted proof and expiry refusal.
- [Native test review](split-admission-native-test-review.json): accepts the
  focused ordinary CLI scenario, with real storage faults and genuine renewal.
  Test review is not an execution result.

The independent implementation review accepted the tested storage and Node
reconstruction paths. [Related checks](split-admission-checks.json) passed59
custody,56 index,1 recipient and51 frontend tests, plus Clippy and formatting.
The initial duplicate fixture/import/format failures are retained separately;
fixture reuse was reviewed and the Node test repeated. Full130 and the
complete67/22/3 release scope remain open.

[Focused native R1](split-admission-native-r1.json) passes in300.1seconds on889
unchanged inputs. One real successor splits the first original's data admissions
around a cold sender restart. Both ordinary CLI originals publish completely;
six finality signatures are independently verified. After18 actual data and18
index copies are removed, the recipient passes missing-trust refusal and both
SQL import failures, then recovers both originals with the sender absent and
preserves them across cold restart. Teardown reports no errors or profiles.

## Remaining boundaries

The subsequent [inspection fix](split-inspection-review.json) now normalizes
optional proofs against the actual local/requester context and fully verifies
differing proofs. Its genuine RED2/GREEN2 and [18 backend/51 frontend checks](split-inspection-checks.json)
pass, including signed tight pagination, atomic byte-quota refusal/retry and cold
receipt/observation retention. [Expanded native R3](split-admission-native-r3.json)
passes on890 unchanged inputs in216.6seconds: real bidirectional mixed-head holder
inspection/cold manifests and the complete original two-message recovery flow.
R2 preserves a test hex-decoder failure; its correction retains exact byte equality.
The native R1 above predates the inspection follow-up.
The [same-head native regression](split-inspection-same-head-r1.json) also passes:
105.4seconds,890 unchanged inputs, real captured proof-bearing requests, shared
queue/deadline checks, independent attestation metadata oracle, storage failures,
cold manifests and automatic recipient retrieval after both sources disappear.
Both native runs clean up without errors or remaining profiles.
The separate mismatch between graph publication order and MLS generation order
also remains; preserving receipt proofs does not solve recovery ordering.

The [ordinary desktop release bundle](split-admission-release-build.json) has now
been rebuilt with this change on890 unchanged inputs. Strict ad-hoc signature
verification and the default feature graph pass; the automation driver is absent
and unsigned node/CLI/MCP binaries match native R3. The bundle is not notarized.
This does not establish native GUI paid recovery or full V1 acceptance.
