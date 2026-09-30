# P01 public P-256 selection and QC verification

Baseline:0b27dfe. Full V1 is not achieved. This slice authenticates the separate public-key
registry in Rust and verifies selected P-256 certificates. It does not yet run P-256 voting,
use the new committee in daemon/Core, authorize spending or provide ciphertext custody.

## Test-first review

Before production, tests/evm/p256_selection.py deployed the existing actual FinalizerRegistry
and PostageIssuer on two disposable Anvil chains31337/31338. It retained real account/storage
proofs, signed trusted-attestor heads, paid admissions and full gapped Merkle-sum membership
in crates/finalizer/tests/fixtures/p256-selection/vectors.json. Seventeen registrations,
exit index2, sixteen frozen eligible units, and post-seal live changes exercise the full
population without contacting operator processes. Independent Python CBOR/full-list sampling
produces exact policy, selection and committee IDs for n4/7/10/16.

The separate no-fork backend-test-critic initially returned REVISE: single-coordinate attacks
could be masked by point validation, CLI overrides used invalid types, and the runner source
fingerprint omitted transitive helpers. Corrections add a complete valid nonselected public
key against the unchanged selected proof; typed overrides and identical duplicate fields;
and existing full fingerprint helpers plus post-build/post-scenario checks. Explicit tuple
types remove incidental compilation errors. FINAL ACCEPT followed before production.
RED contains only missing intended P-256 APIs. SHA256 at first ACCEPT:
- tests/p256_selection.rs:46842928a88950cd2992fea3e549d0b65e7a61f338427653ad2f18fd1941e2a9
- tests/support/p256_selection.rs:9a9fcd3350d27e20086cb11276f739e5e7846de4d7297c62c56c9652955462cf
- test fixture:af9e99c9011a56d3b14791085ec2137559d44b5b8d5cd3d5cbad2a0728b1a320
- Python runner:c0addd95c984d740f39949f4c05abff3cc11e17eaf6cd44b823997ce152c4df0

A later harness failure was caught by the new equality assertion: a root `proof` duplicate
was inserted into members[0].proof because their field names coincide. The corrected helper
targets the entire uniquely matching root field value. The same critic gave a separate FINAL
ACCEPT; all equality and rejection assertions were preserved. Final test SHA256:
ae0943ca40f77997c54c6888e1093bbc6979e707566f1cbfc664af1041546a71.

## Implementation

l2-types shares one full Merkle-sum path verifier between old opaque openings and new public
coordinate commitments. The new private-construction ProvenRegistryUnit does not infer
operator roles. P256SelectionPolicy requires explicit nonzero registryCodeHash in addition
to the pinned network/checkpoint/registry domain. Actual account code must match that profile.
The caller must select the reviewed immutable deployment: parsing a hash is not code review.

Both selection modes share policy validation/encoding, the bounded sparse sampler, exact
ordinal set matching and ordering. Different policy/draw/committee/signing namespaces prevent
cross-scheme reinterpretation. Old config keys remain32bytes by default; P-256 uses33byte
compressed SEC1. Canonical committee encoding, finite authority and actual QC authentication
are shared. The P-256 verifier uses unchanged installed Commonware2026.9.0 with an identity
BiMap between participant and signing keys. No new dependency or custom crypto was added.
The existing Ed25519 runtime/WAL and journal retain their schema and API.

Strict select-p256-committee CLI uses bounded raw nested JSON, preserves duplicate rejection,
and reports exact selection, key roster, source checkpoint and finite authority deadline.
Its caller-supplied checkedAt/profile/head are explicit verifier inputs; the response does not
prove that the daemon currently accepts them. Serialized config alone does not preserve the
shorter authenticated authority interval. See spec/finalizer/p256-selection-v1.md.

## Verification

Eight focused Rust tests pass, including real upstream P-256 signatures/quorum/epoch/view/
subject checks, wrong entry/key/membership, strict CLI inputs and exact resource boundaries.
A fresh two-chain live gate passes28 actual CLI invocations after stopping each chain, with
zero operator processes, unchanged committee on refresh, and old-proof rejection after real
wall-clock expiry. Success brackets refusal attempts. Cleanup is empty.

Six isolated compiling mutations are detected: disabled key commitment binding, ignored code
pin, accepted unselected ordinal, ignored authority deadline, skipped certificate verification,
and checkpoint-dependent committee namespace. The scratch copy restores each original source;
it uses its own Cargo workspace/cache, separate from product acceptance builds. Baseline8/8
passes before mutation. Logs and P01-p256-selection-mutations.json retain outcomes.

The complete `scripts/check-native.mjs` run exited0. It includes the standard aggregate:
404 Rust +29 Solidity +7 model +40 frontend test functions =480, all16 actual EVM runners,
formatting, Clippy all-targets, TypeScript and frontend build. The fresh new CLI passes28
checks and the legacy selection CLI passes36. Custody resolution checks290 positions through
2633 owner calls, with every required lifetime/cancellation/limit flag and empty cleanup.
P01-p256-selection-evm-reports.json fingerprints every successful current EVM report.

The same run builds a hidden WKWebView test bundle and passes all5 native product flows,
then builds the macOS arm64 release app with bundled daemon/MCP. Strict deep codesign and
exclusion of the automation driver from normal dependencies pass. Signing remains ad-hoc;
this is not a notarized distribution. No release installation/publishing was performed.
Current chat and restored-trust screenshots were viewed side by side with the saved baseline
in recovery/native-reference-0b27dfe. Layout/content match except expected test timestamps.
These native tests cover existing product behavior; the new finalizer CLI is a separate binary.

The independent OrbStack Linux transport gate exited0, seven outcomes and empty cleanup:
run ain-nat-ac15e373, source78de16e149976be618229a22a85f1fb8337f3fdb88f46062dc71ac7ee08e3190.
It rebuilds existing daemon transport and checks NAT/relay/LAN behavior; it does not claim
P-256 voting on Linux. Sources remained stable across both gates. Source manifest, terminal
log hashes and release binary hashes are in P01-p256-selection-validation.json.

Next: integrate typed P-256 authority into the shared Simplex voting runtime with actual
partition/fault/expiry/restart/WAL tests, preserving old Ed25519 persistent namespaces. Then
connect the selected service to daemon-owned current policy/head and explicit operator keys.
Historical funding, spend/custody/repair, handover and complete E01–E26 remain required.
