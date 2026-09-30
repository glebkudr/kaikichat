# Public sender execution boundary

The common sender now has a Core entry point that reauthenticates a queued
ordinary message before exposing its public stamp. It rechecks current owner
pause/ceilings and the runtime's actual signed grant, including epochs, scope,
expiry and revocation. It reuses the existing atomic native message preparation;
execution cannot re-encrypt, allocate twice, debit sponsorship or mint a nonce.

Current checkpoint proof is stored separately from immutable retained ancestry.
Both explicit and retained-history refresh survive restart. Stale configuration
fails until owner refresh; the existing stamp and original envelope stay fixed.
Admission-era policy records remain usable at their original current head.

R2 critic ACCEPT preceded production. **11 new execution tests and 465
Core/daemon tests pass**, zero failed/ignored. Core's 271 PASS comes from the
initial run; the final daemon run follows the R3 timer-fixture correction. All
Core inputs remained unchanged; the final run freezes 615
source files. Workspace fmt/Clippy, 59 frontend, 7 model and 12 EVM-model
tests, TypeScript and Vite pass. See [verified checks](module-checks.json) and
[critic decisions](critic-review.md). No new live EVM or packaged-app run is claimed.

This is a Core execution boundary. The queue remains queued: preparation alone
does not establish QC, storage or delivery. The automatic daemon worker must
still drive remote finalization, verified selected custody and private pointers.
Full R10/repair, durable indexes, epoch handover, UI/CLI/MCP lifecycle and
remaining V1/platform gates remain open. Continue with the ordinary-message
gate in [the sender plan](../P02-public-sender-admission/NEXT.md), using
prepare_public_sender_job instead of bypassing the sponsorship boundary with
the low-level owner-only message-preparation API.
