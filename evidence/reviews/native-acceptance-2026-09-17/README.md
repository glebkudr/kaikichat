# Native acceptance runs — 2026-09-17 (head b5c79de)

The first full series of native runs after the removal of the AF_UNIX blocker
(Linux runner: OrbStack, Ubuntu 24.04). A03 was run on portable-linux
(native arm64 container; x86_64 under Rosetta is unusable: crypto operations do
not fit the 5-second IPC deadline), H10/A04/H11 — on a macOS host through
prepared-runtime, H11 — with the release desktop bundle.

| Scenario | Platform | Result | Key fact |
|---|---|---|---|
| A03 lifecycle | Linux arm64 (portable-linux) | **PASS** | passed:true; sender-absent recovery, atomic retirement |
| H10 Diagnostic32 | macOS native | **PASS** | 32 originals, 96 signatures, 320/320 replicas, 3200 location ACKs, trace captured, execution-guard clean |
| A04 capacity | macOS + Linux | **FAIL (reproducible)** | `custody_storage` does not answer within the 5s IPC deadline (ipc.rs:21) at 33+ jobs; batch publication does not complete |
| H11 Full130 | macOS native | **FAIL (reproducible)** | "ordinary sender did not publish the complete paid batch" within 600s; jobs move through blocked(custody_capacity/postage_limit)/publishing, 0 stored — matches the open R13/R14 failures |

Diagnosis for the next step (architect): publication/placement liveness for
multi-group batches. A03 (2 originals) and H10 (32, 2 batches) pass — the
regression shows up with a larger number of groups/limits. Full logs and traces
are in `output/` of the corresponding rigs (not included in Git).
