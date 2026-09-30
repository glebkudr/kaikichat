# F05 — conditional quorum and fixed-unit risk tool

Status: this bounded tool is implemented and verified. F05/N05/V1 remain open.

Tests preceded production. `F05-committee-risk-red.log` records missing CLI failures before `tools/risk-simulator/risk.py` existed. The separate existing `/root/node_test_critic`, originally created without forked context, reviewed the explicitly supplied test file, model contract and upstream F05/N05 cards and returned FINAL ACCEPT. No work was performed while waiting. Its sole optional suggestion was more invalid-type/boundary cases specific to the quorum model; the implementation shares the strict integer validator already covered by committee inputs and the existing quorum range cases, so no review override was needed.

The tool uses Python standard-library exact Fractions and integer combinations. Small-population tests independently enumerate actual subsets and quorum pairs, rather than repeating the production formula or witness construction. The report separates structural honest-intersection failure from actual consensus attack success, and separates safety from progress under partition. Key splitting preserves fixed-unit probabilities; the rejected sqrt-per-key alternative produces a reproducible counterexample. A finite caller-assumed grinding bound uses a union bound; unknown remains null. No independence of candidate outcomes or operator infrastructure is inferred.

Verification:

- `F05-committee-risk-green.log`:all7 tests pass, including exhaustive small quorum sets, complete sampled distributions, deterministic complete-population outcomes,2^32 population/128committee bounded arithmetic, key splitting, partition, grinding and strict actual CLI rejection.
- `F05-committee-risk-mutations.json`:green baseline and four isolated compiling mutations killed: unsafe quorum equality, off-by-one intersection threshold, treating unknown grinding as one attempt and replacing fixed-unit counts with per-key square-root counts. Authoritative source was never mutated; scratch cleanup errors empty.
- `F05-committee-risk-backend.log`:333 Rust tests pass after the module was implemented.
- `F05-committee-risk-frontend.log`:40 frontend tests pass. `scripts/check.sh` now also runs the Python model tests; shell syntax checked.
- `output/risk-models/committee-v1.json`:six real CLI reports with exact input/output and source hashes. These are conditional examples, not selected public-network parameters.

The existing macOS bundle, EVM code and network implementation did not change in this module. Their preceding successful native/Anvil/Linux acceptance is retained at7d93bd5; those expensive gates were not needlessly rerun for a standalone standard-library model tool. No packages were installed. This module does not implement committed-before-beacon assignments, actual placement/repair, consensus locking/view changes, global spend state or economics, and cannot close full F05 acceptance.
