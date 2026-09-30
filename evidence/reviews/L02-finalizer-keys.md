# Public eligible finalizer keys — L02/P01

Full V1 remains OPEN. This completes the on-chain registration prerequisite for retrieving
a complete eligible roster without asking each selected operator to reveal its key. It does
not yet make the desktop/daemon a P-256 validator or establish complete Byzantine liveness.

## Implementation and decision

An opaque paid commitment can be valid registry membership while withholding its key opening
or committing a malformed/repeated signing key. Even one selected Byzantine can then prevent
construction of the complete signing set. The new FinalizerRegistry admits only public valid
P-256 keys with an owner/deployment-bound proof of possession and persistent global uniqueness.
It inherits the original fixed-unit tree, seals, beacon, delayed exits and principal accounting.
The original opaque entry point is overridden to reject; there is no admin/update/reveal path.
Public keys remain readable after exit/withdrawal. Post-seal changes cannot alter the roster.

P-256 uses existing upstream Commonware cryptography and OpenZeppelin5.7.0 verifySolidity.
No curve arithmetic is implemented locally. The unchanged minimal vendor closure/license has
exact file hashes and source URLs. A separate new deployment/profile is required: the current
Ed25519 selected API does not silently interpret this registry. Design, exact digest/commitment
and crypto compatibility decision: spec/registry/finalizer-keys-v1.md.

## Test-first independent review

Five Solidity tests and the actual two-chain runner were written before production. Genuine
RED is retained in L02-finalizer-keys-red.log: FinalizerRegistry.sol does not exist yet. The
separate no-fork /root/node_test_critic returned FINAL ACCEPT. Test hashes at initial review:
Solidity bc3d791eb614f79714748e69279622f03eb073bc6fdd011357df316daa3fc6cf;
actual EVM e3152580bc7393f28bc187e82288364bdafc556c0789dff6bd3d387f1d8c3789.

The first Solidity run found a test-harness sequencing mistake: calculating an argument after
expectRevert made a domain() getter consume the expectation before bond(). The retained trace
is L02-finalizer-keys-bypass-failure.log. Computing that same argument before expectRevert/prank
preserves all selectors/state assertions; the separate critic reviewed and accepted the exact
change. Current Solidity SHA256 e80e314d0e2bf416fc9f11cd9fc65123f27586281e6cbceaeee07d6e079108c3.

The real upstream compatibility test verifies/produces the exact independent OpenSSL
deterministic low-s signature and rejects altered owner/namespace. It is intentionally a
pre-production compatibility gate, not the registration business RED. The first aggregate
stopped on unwrap_used lint in this new test; adding the same test-only lint attribute used
by adjacent suites received separate FINAL ACCEPT. No assertions changed. Failure retained
in L02-finalizer-keys-aggregate-lint-failure.log. Current Rust test hash
14dfa0fd7c33523a8849356fc4130277b52b9b6ca5507912e7675bf4bc8b22c7.

## Actual checks

All29 Solidity tests pass, including all previous registry/issuer tests. New tests check exact
events/public state/payment, real P-256 signatures, invalid curve points and high-s signatures,
wrong-owner/front-run before honest admission, foreign-deployment/changed-chain replay,
failed-payment retry, opaque base-ABI bypass, duplicate keys across owners/after withdrawal,
and retained public keys under immutable pre-beacon snapshots. Logs: L02-finalizer-keys-green.log.

The fresh two-chain Anvil runner makes14 successful registration transactions (including
reverted-chain history) and retrieves each frozen four-member roster with zero operator
processes. Python/OpenSSL signs independently; the existing independent complete Merkle-sum
model checks every active unit/root. Mined invalid attempts assert exact gas/principal changes,
no logs and unchanged funded state. Reorg removes an orphaned key and its uniqueness reservation;
the exact registration succeeds again. Restart and withdrawal preserve every frozen public key.
Source hashes include the full vendor closure and are checked after build/scenarios. Reports
start passed:false and retain partial traces on failure. Full transcripts are under
output/evm-e2e/finalizer-registry/. Focused report: L02-finalizer-keys-evm-focused.json.

Six compiling negative controls in a separate source/build directory are all detected:
ignore signature verification, make uniqueness owner-scoped, reopen opaque bond, hide keys
after exit, change key/commitment binding, and remove chain checking. The unchanged scratch
baseline passes all5 tests; every mutation reaches a failing business assertion. Scratch
sources are restored. Evidence: L02-finalizer-keys-mutations.json and per-control logs.
Current source manifest: L02-finalizer-keys-source.json.

## Aggregate and remaining work

The aggregate scripts/check.sh exited0:396 Rust,29 Solidity,7 model and40 frontend test
functions (472 total), all15 actual EVM runners, formatting, all-target Clippy, TypeScript
and frontend production build pass. All EVM cleanup errors are empty. The final custody
runner verifies294 positions through2640 owner calls; the earlier Ed25519 selection CLI
still passes36 checks. The new registry passes14 actual registrations across two chains,
including the orphaned/replayed transactions; every required outcome flag is true.
Its current source hash is34e24b30836265412992db860cdb5e7fd136878143b95816218f9bb333674d0d;
creation bytecode SHA2567c99c3e19ec9cb860795830c39769d4e32a4eaaa63347a502022dffc2d5c3879.
Registration gas in this local profile ranges1050158..2201514; the first unit initializes
the32-level tree. This is measured local execution, not a public-chain fee estimate.
The complete reviewed source manifest remains b032037f3b079d92f5d591ddc08cae42133faa176105b40756fe6409b9f66c1e.
Evidence: L02-finalizer-keys-aggregate.json/.log, L02-finalizer-keys-evm.json,
L02-finalizer-keys-custody-resolution.json and L02-finalizer-keys-selection.json.

No Rust application production code, Cargo/npm lock, UI or packaged binary changed.
The previous9ff1b3a native macOS5-flow and Linux7-outcome evidence covers the unchanged app.
This increment introduces no UI, platform or selected-runtime claim.

Next: authenticate this registry's reviewed deployment/type and public key membership in Rust;
bind selection to the P-256 signature/domain version; integrate the existing upstream scheme
with the durable runtime while preserving the same partition/Byzantine/WAL/proof-expiry
requirements. Then persist owner finalizer policy/keys/current-head authority in Core, exchange
proofs/endpoints and host explicitly enabled voting in daemon. Chain-state availability and
its trusted-attestor profile still need their existing stated assumptions. Roster availability
at the chain is not permission to invent missing keys or reduce quorum. Handover, finite spend,
ciphertext custody/repair, groups/jobs/trust and complete E01–E26 remain necessary for V1.
