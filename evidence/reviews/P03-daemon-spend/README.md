# Ordinary daemon postage spending

The ordinary daemon now hosts the canonical issuer-bound postage application.
Owner IPC can configure its current selected service, submit genuine public
receipt candidates and read the first durable finalized result. It uses the same
Core actor, selected P256 transport, Commonware archive and ProfileStore effects.

The fresh live gate passed with four distinct selected daemon keys and processes:
700 owner calls, two genuine fixed-image proofs for different operations on one
paid ticket, a real 2+2 TCP partition, one agreed spend after healing, and twelve
independently verified QC signatures. The source Anvil chain was stopped during
voting. A real SQLCipher insertion failure held the Marshal acknowledgement cursor
at zero; disabling the key prevented a commit after removing the fault. After a
crash and stopping all peers, re-enabling the key restored the archived result
without re-submission. Exact retries preserve the first proof; a competing operation
fails. Cleanup reported no errors. See `live.json` and public `trace.json`.

Review: one startup defect in the test helper was fixed after FINAL REVISE, then
FINAL ACCEPT before production. Five reviewed test inputs and 437 frozen source
inputs remained unchanged through the live gate and ordinary validation. No tests
or assertions were weakened after acceptance. See `review.md` and the manifests.

Ordinary validation passed 525 Rust tests (zero failed/ignored), forty frontend
tests, seven model tests, nine independent oracle tests, workspace fmt/Clippy,
TypeScript and Vite. `validation.json` records the full backend run; the other
checks were run separately before it, while the same sources remained fixed.

The existing selected-service regression passed TCP/Noise and QUIC: thirteen
effects per profile on each transport, 872 owner calls and 75/87 independently
verified signatures. Scope isolation, revocation, route renewal, failed-effect
cursor ordering and cold recovery remain intact. Cleanup reported no errors.
The ordinary Tauri release was rebuilt and passed deep/strict ad-hoc codesign
verification and automation-driver exclusion. Its bundled verifier accepts both
previous genuine competing-operation receipts at their historical time and rejects
operation substitution/expiry; the fixed image is unchanged. See `release.json`.
The package is not notarized. Native UI, Linux network matrix and the remaining
full V1 E01–E26 scenarios were not repeated or declared complete.

This is not complete P03/E20 or V1. Candidates are currently supplied to each
replica by the owner; remote distribution, bounded overload/100 concurrent calls,
cold historical reads without live selected authority, authenticated epoch handover,
actual paid ciphertext custody/repair and user-facing UI/MCP spending remain open.
The retained-input cap is sixteen and the existing finalizer prefix cap is 128.
No refundable reservation or actual resource allocation is created by these APIs.
Contract: `spec/postage/daemon-spend-v1.md`.

Raw ignored logs: `output/postage-spend-node/`. No master keys, funded owner seed,
salt or private witness is retained in this evidence directory.
