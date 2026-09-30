# Signed finite history directory — crypto prerequisite accepted

The new codec authenticates an exact finite directory of descriptors and distinct
candidate index keys within one MLS direction/epoch. Its longest-lived anchor
preserves older entries when a later message expires sooner. It rejects foreign
signers/scopes, malformed/canonicality violations, rollback and same-revision
equivocation. A fetched descriptor must match the exact signed reference and have
existed at manifest issuance. Paid placement and availability remain separate.

**PASS: 73 backend /21 frontend tests, production Clippy and fmt, 593 unchanged
application/test input fingerprints.** The affected backend gate includes 24
crypto tests (seven new), 22 Core custody tests, 26 paid-index tests and the node
custody codec test. [Exact commands and logs](checks.json) ·
[Source inputs](source-inputs.json) · [Contract](../../../spec/custody-history-manifest-v1.md).

The separate context-free [critic](critic.md) returned REVISE then ACCEPT before
production. [Initial RED](baseline.json) and [accepted-input RED](baseline-2.json)
both compile the actual tests against `31ab80d`: one missing-import E0432 and
eight missing-method E0599 errors. These are missing-API baselines, not executed
behavior failures. Each retains its own exact input manifest and log hash.

The independently generated Python CBOR/Ed25519/AES-GCM vectors and generator are
retained in `crates/crypto/tests/fixtures/custody-history/`. Crypto candidate keys
are real test keys, not evidence of selected or funded index rosters. The maximum
positive case signs 128 distinct descriptor references with four candidate keys;
a distinct 129th is rejected without increasing global document limits.

No new native gate or release acceptance is claimed. The prior automatic runtime
at `31ab80d` still uses its live-job index intersection. The new format needs
durable index/Core storage, transport and automatic publication/import before
it can replace that condition. Multi-book/MLS-epoch recovery, completeness,
Welcome, retirement and R10 repair remain open. [Required continuation](NEXT.md).
