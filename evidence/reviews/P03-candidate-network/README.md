# Selected daemon candidate distribution

The ordinary daemon now distributes genuine public postage receipt/context inputs
to currently selected peers over a separate bounded protocol. A recipient uses
the same public-context, fixed-image receipt and durable-store checks as owner
submission. Scope and actual Noise/P256 authority precede import. The verified
journal determines candidate identity before persistence; transport replies never
create local finality. Shared revocable writers fence both bulk and consensus traffic.

The fresh four-daemon gate passed with two accepted owner seed submissions,
automatic selected-peer distribution, two genuine proofs, 2,607 owner calls and
twelve independently checked QC signatures. A damaged genuine seal from a real
selected transport identity was rejected before durable import. Foreign selected-key
claims and oversized input failed. Receipts survived recipient crashes; repeated
delivery did not consume another candidate slot. A real eight-second 2+2 partition
could not finalize. Role disable prevented sends, and re-enable restored verified
delivery after a recipient restart. Healing agreed one spend; the last recipient
recovered its exact result with all senders stopped. Cleanup had no errors.

The first live attempt failed on role re-enable delivery. Its report remains
`failed-first-live.json`. Automatic discovery could exhaust proof admission by
retrying each unknown key/peer pair independently. Pacing the complete discovery
stream and each peer below existing receiver limits fixed the accepted scenario
without raising those limits or extending the timeout. See `review.md` for the
test-first R1/R2 review and separately accepted diagnostics-only R3 change.

The final ordinary backend run passed 525 Rust tests with zero failures/ignored,
workspace formatting and Clippy. Forty frontend tests passed after the correction;
seven model tests, nine independent oracle checks, TypeScript and Vite also passed.
The selected-service regression passed TCP/Noise and QUIC: thirteen effects per
profile, 955 owner calls, and 75/69 independently checked signatures. Scope replay,
role/head/network revocation, binding renewal, failed-effect cursor ordering and
cold recovery remain intact. No cleanup errors were reported. The 439 source inputs
and three accepted test files were frozen for these backend/live regressions.

The original ordinary-daemon spend gate also passed on the final sources, with
another two fresh genuine proofs, 820 owner calls and twelve independently checked
QC signatures. Its actual SQLCipher failure held acknowledgement until durable
commit; role removal, source crash, cold recovery without peers, exact retry and
competing-operation refusal passed. See `spend-regression.json` and its complete
public trace. It supplies candidates to replicas through owner IPC; the separate
network gate above supplies the selected-peer distribution evidence.

The ordinary Tauri package was rebuilt from the same frozen sources. Deep/strict
ad-hoc codesign verification passed, the automation driver is excluded, and the
bundled verifier accepts both historical genuine competing-operation receipts with
the unchanged fixed image while rejecting operation substitution and expiry.
`release.json` records binary hashes and these checks. The app is not notarized;
native UI and the Linux/full V1 network matrix were not repeated for this backend
increment. `finish.json` records the additional spend regression and package checks.

This module does not complete P03/E20 or V1. Nonmember-client ingress, saturation
and 100-request acceptance, cold historical reads without live selected authority,
authenticated epoch handover, actual paid ciphertext custody/repair, UI/MCP payment
flows and the remaining product scenarios are still required. The selected-replica
path retains at most sixteen candidates; its consensus prefix cap remains 128.
The current receipt/QC response grants no resource admission. Source restart before
first dissemination and network candidate-ID mismatch remain explicit test gaps.

Contract: `spec/postage/candidate-network-v1.md`. Raw ignored runs are under
`output/postage-spend-network/`. The live test authenticated its QCs independently
during execution; `trace.json` retains public heads/results but not the complete
enrollment fixture or private profiles. No owner token, master key, ticket seed,
salt or private witness is retained here.
