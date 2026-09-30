# P02 statement checkpoint

Baseline: 119bea36785f18e4d073eba6939d756bf28992ce. This checkpoint implements the
deterministic guest relation only. It does not supply a ZK verifier, authenticate
the public context, authorize transport, or enforce one-use spending. The full P02
card and V1 goal remain open.

Tests and Python vectors were written before the API. The original missing-package
RED was followed by an empty registered crate; its compiler RED specifically
identified the three missing statement imports. These are API REDs, not evidence
of a behavioral defect in an earlier implementation.

The independent `/root/scope_replay_test_critic`, originally spawned without
inherited context, reviewed the new tests and contract. Its first FINAL REVISE
found that oversized whitespace-only JSON would fail without any size limit.
Tests now pad otherwise valid profiles/proofs to the exact byte limit, require
successful identical output, and reject the same JSON at limit + 1. Changes to
both authorization bounds are checked independently. FINAL ACCEPT arrived before
any production implementation. The critic independently reproduced all sixteen
HMAC/journal vectors and checked the actual Anvil corpus fingerprint and owner key.

All eight statement tests pass. Four temporary source mutations compiled and
failed at the intended behavioral assertions: accept index == paid count, accept
a 512 KiB + 1 valid proof, omit authorization expiry enforcement, and let the
operation alter the nullifier. The exact source was restored before the full
workspace run. Mutation patches, hashes and assertion excerpts are in this folder;
raw logs are in `output/postage-proof/`. The main source was the only mutated file;
no other build was running during these sequential experiments.

Validation: 466 Rust tests, workspace/all-target Clippy with warnings denied,
formatting, 40 frontend tests, TypeScript and Vite all passed. The existing real
funded corpus is reused without modification. Full live EVM, hidden native and
Linux network acceptance were last verified at119bea3; this unconnected relation
does not claim to rerun those gates. No product UI changed.

Next acceptance is a real pinned guest and local recursive STARK proof, verified
in a separate process without its private witness. It must reject fake receipts,
wrong images, altered journals/seals and mismatched host context. Fresh shared-root
funding with distinct owners, finite-count vectors beyond single-byte CBOR indices,
proving interruption and measured local costs are still required. P03 must enforce
the same issuer/nullifier key across operations, epochs, routes and accepted networks.

HMAC 0.13.0 was already in the workspace lockfile; its current stable version and
Sha256 usage were checked against upstream docs. No existing dependency version
was changed by this crate. RISC Zero is a researched candidate, not yet part of
the runtime or evidence of cryptographic privacy for this checkpoint.
