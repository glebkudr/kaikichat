# Authenticated old postage epoch closing

This component connects durable terminal history to Core issuer policy and the
ordinary node. It does not complete spent-state handover, successor spending,
old-authority outage recovery, AR1 or V1. Baseline source: `7534465`.

The [contract](../../../spec/postage/epoch-closing-v1.md) describes the canonical
closing operation, authenticated successor, immutable original record, owner APIs
and existing selected-replica transport. [Test contract](test-contract.md) and
[independent reviews](test-review.md) record the test-first sequence. The first
Core review required revisions; repeated Core and separate node reviews accepted
the tests before their respective production phases.

The actual native result is [native-evidence.json](native-evidence.json), with
complete public proof inputs and exact records in [native-trace.json](native-trace.json).
The test registers ordinary daemon-generated keys for all original and successor
registry members, pays for a four-ticket public book on the local EVM, and uses
a pinned daemon binary. It finalizes one spend, then closes the same old issuer
committee with one initial owner proposal distributed through normal gossip.

Three validators retain the direct closing QC. The fourth holds an unrelated
pending paid input and misses the closing input. SQLCipher failure prevents the
closing record and consumer ACK. Role revocation stops recovery; after a crash,
only one source joins the lagger. The original record is recovered without a new
three-signature quorum or owner resubmission. Another cold restart with the role
disabled preserves the exact record, old SpendRecord, terminal height 2 and the
unresolved input's original persisted hash/revision. Post-close spends and a
competing successor are rejected. Independent verification checks **24 P256
signatures** and the direct terminal entry's parent against the actual first spend.

The native RED report is [native-red.json](native-red.json). A subsequent
[failed run](native-failure-1.json) with [public trace](native-failure-1-trace.json)
exposed a real coupling: suppressing candidate broadcasts also removed the
configured scope used by the historical responder. The fix retains that scope
with no outgoing candidates, so old records remain available for authorized
recovery. Raw logs remain under managed `output/` and are identified by hashes.

[fixture-generation.json](fixture-generation.json) identifies the original genuine
EVM corpus authoring run. Its original source hash is separate from the final
implementation fingerprint; later helper changes add an opt-in ordinary future
key without changing the fixture bytes. Seven application tests cover current
and revoked fences, exact terminal binding, independent historical evidence
validation, competing successors, original-QC preservation, SQL failure and cold
bounded history import. Existing fixture controls separately authenticate all
three epochs, renewal and paid stamps before the new APIs are used.

[Final validation](checks.json): **847 Rust tests**, zero failed/ignored in 56
nonempty suites; **59 frontend tests**, TypeScript/Vite, 19 model tests and
workspace Clippy/fmt pass. All 567 captured source inputs stayed unchanged through
the workspace gate. The final source manifest is
[validated-inputs.json](validated-inputs.json).

Two unchanged native scenarios were rerun because this component changes shared
candidate/recovery plumbing: [ordinary missed-spend recovery](spend-recovery-evidence.json)
with [public evidence](spend-recovery-trace.json) passes 2 spends / 24 signature
checks, including a hostile QC carrier; [full-queue renewal](spend-refresh-evidence.json)
with [public evidence](spend-refresh-trace.json) passes 18 spends / 216 checks,
including SQL failure, exact pending replacement and the real old-lease boundary.
These are separate native runs, each with its own pinned binary and source hash. Full V1 still requires
67 cards, 22 E2E and three platforms. No new packaged native application or other
platform run is claimed. [Continue here](NEXT.md).

Run the native test from the source checkout:

```sh
python3 scripts/build-storage.py run python3 tests/evm/public_epoch_closing.py
```

The local EVM and all daemon processes stop during normal and failed cleanup.
No production deployment, ZK process or synthetic finalizer QC is used by this gate.
