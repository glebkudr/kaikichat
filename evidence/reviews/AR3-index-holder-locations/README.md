# AR3 — compact authenticated data-holder locations

Implemented `PortableCustodyLocation` / `verify_custody_location` as the next
prerequisite for durable book-index discovery. The carrier replaces ciphertext
with its signed descriptor, preserving original native funding, QC, receipt and
operator/copy evidence. Primary and copy receipt verification shares the existing
historical verifier; ciphertext verification remains required after actual fetch.

The independent test critic required stronger proof/membership/consent cases,
then accepted the revised tests before production. Compilation RED was the
missing API, not a pre-existing runtime failure. Four new tests cover genuine
native paid primary/copy paths, cold readers without payer state, old admission
and consent expiry, authentic semantic substitutions, trust and retention fences.

The accepted gate passes **59 backend tests / 21 frontend history tests**,
matching Clippy and fmt, with all **575 input fingerprints unchanged**.
Accepted results are in [checks.json](checks.json); exact input hashes are in
[source-inputs.json](source-inputs.json). The reproducible runner is invoked from
the primary repository through:

```sh
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-holder-locations/verify.py
```

The initial runner used a filter matching zero public ciphertext tests. Its
actually completed 58 custody/index and 21 frontend tests remain recorded under
`candidate-1`, but that aggregate is not accepted as the complete targeted gate.
The corrected runner selects the actual public ciphertext case and rejects empty
test targets. Production and tests did not change between these two runs.
Build/run logs remain in ignored `output/ar3-index-holder-locations`; reports
retain their exact paths and hashes. No local logs are committed here.

This is an application-library gate, not a native network/index discovery gate.
Locations are not yet stored in index entries or returned over transport.
Availability, complete history, repair and successful sender retirement remain
open. Full workspace/regression belongs at the end of the complete plan.

[Contract](../../../spec/custody-holder-location-v1.md) ·
[Test rationale](TEST_CONTRACT.md) · [Critic decisions](critic.json) ·
[Next work](NEXT.md).
