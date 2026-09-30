# Public signed postage: actual paid MLS and offline retrieval

The composed gate passed on the production code at `977e51d`; no additional
production change was needed. Tests and test-harness extensions received separate
backend-test-critic R1 FINAL ACCEPT. Reviewed hashes remained unchanged.

The same ordinary sender daemon creates the actual MLS envelope and generates
the public book key in its encrypted profile. A fresh canonical issuer purchase
funds its actual commitment before the beacon. Core binds the MPT proof and
durably reserves ticket 0 for the envelope's exact SHA256. Independent Python
CBOR/Ed25519 verification checks the actual random-key signature; the oracle's
dummy-seed signature is never used. The existing tagged public spend carrier
travels through the real finalizer and custody paths.

Observed acceptance:

- Actual finalizer rejects changed operation and signature before spend.
- Four selected real Commonware/P256 replicas produce a QC with three independently
  verified signatures. An ordinary client obtains this result over the network.
- A real 872-byte encrypted MLS envelope is stored by its selected primary.
  Outgoing evidence/target write failures and retry preserve the original result.
- With both messaging clients stopped, authenticated copies reach another primary
  and a replacement. Copies, source/target restart, identity checks, permission
  revocation/retry and storage failures all retain the same paid obligation.
- Actual holder requests carry no ciphertext, share existing processing limits,
  and merge signed receipt manifests durably across restart. The independent
  attestation/metadata oracle and source/target SQL-failure checks pass.
- With sender and original storage sources absent, the recipient discovers the
  private pointer and retrieves/imports exactly the original message automatically.
  Its own restart also passes. There are zero owner lookup/read/import calls.
- Only after that retrieval, reopening the sender shows exactly one allocated
  ticket and three available; exact reservation still returns its original stamp.

Every actual daemon start/restart receives executable prover/verifier tripwires,
including restarts that change network arguments. The tripwire self-test records
one invocation; the entire public workflow adds zero. Source and daemon binary
hashes are unchanged at completion. Legacy CLI/prover/oracle checks remain in the
shared harness when no explicit public author is supplied.

Evidence: `paid-network-evidence.json`, `paid-network-trace.json`,
`fresh-public-carrier.json`, `green-r1.log`, `support-checks.json`. The 93 ms recorded
for this test's native authorization segment includes reservation, verification
and retry/balance assertions; it is not network-delivery latency or a benchmark.
The trace contains public funding/context/signatures and test ciphertext, never
the book seed. All test profiles/owned daemons were cleaned up without errors.

Seven model, twelve EVM-model, 59 frontend tests, TypeScript and Vite pass after
the test-harness change. Production source remains the same as the preceding
194-node-test / workspace fmt+Clippy checkpoint. No fresh full-workspace Rust run
or packaged desktop/native UI run is claimed by this gate.

```sh
python3 scripts/build-storage.py run python3 tests/evm/public_paid_ciphertext.py
```

Run without concurrent Cargo builds; the executable hash is part of acceptance.
This gate drives sender preparation/spend/storage through owner IPC. Automatic
sender orchestration, UI/CLI/MCP wallet lifecycle/budgets, R10, autonomous repair,
independent durable indexes, committee handover and live copying after admission
expiry remain open. Full V1 and the desktop application are not declared ready.
