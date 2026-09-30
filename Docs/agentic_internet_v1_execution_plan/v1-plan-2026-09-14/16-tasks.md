# 16. Full acceptance of a single revision

All 67 cards, 22 E2E, 11 suites, and three OSes are accepted on a single RC; missing results block the release.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R01, AR-R02, AR-R03, AR-R04, AR-R05, AR-R06, AR-R07, AR-R08, AR-R09, AR-R10, AR-R11, AR-R12, AR-R13, AR-R14, AR-R15, AR-R16, AR-R17, AR-R18, AR-R19, AR-R20, AR-R21, AR-R22, AR-R23, AR-R24.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-RC-SOAK** (24h soak): deterministic soak — FDB simulation (SIGMOD 2021); TigerBeetle VOPR; Jepsen fault histories.
- **QA gates** (E02–E26): reference points by topic — history/pagination → research §4; repair → §3; MLS/BFT → §1–2; spend/epochs → §2; network/NAT → §6; economics → §8; ZK/nullifiers → §9.

<a id="v1-rc01"></a>
## V1-RC01. Run all required suites on the RC

**Type:** verification. **Source cards:** F06, X06. **Position in dependency order:** 100.
**After:** [V1-V04](15-tasks.md#v1-v04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Run the full V03 runner: backend unit/integration, frontend, wire/crypto, simulation, contracts/proofs, CLI/MCP, and platform checks.
2. Cross-check nonzero counts, accepted reviews, and exact hashes before/after; account for heavy legacy targets separately but do not skip the required ones.
3. Record failed/skipped/missing as blocking states; after a fix, repeat the affected gates and the overall RC on the new revision.

**Verifiable scenarios:**

- All 11 required suite IDs have an actual result on the accepted set of inputs.
- A flaky/quarantined required test or an unexecuted live/native case is not hidden by an overall PASS.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The overall run is green; the 22 E2E below and the native matrix are additionally required if the same runner did not execute them.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-rc-soak"></a>
## V1-RC-SOAK. Run the 24-hour process soak and benchmark

**Type:** verification. **Source cards:** F03, F05, F06, X01, X06. **Position in dependency order:** 101.
**After:** [V1-V04](15-tasks.md#v1-v04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Run real client/node processes for 24 hours per the C04 manifest, separately from virtual-clock tests.
2. Include 10,000 duplicate/reorder messages and the accepted group/attachment/NAT loads; measure accepted/completed/errors, p50/p95/p99, CPU/RAM/disk/queues.
3. Check bounded growth, cold restart, absence of leaks, and the exact contents of received messages; save the initial thresholds before the run.

**Verifiable scenarios:**

- All messages admitted in the manifest are recovered under the stated conditions without duplicates.
- A quick rejected/timeout is not counted as successful delivery; a simulated day does not replace 24 hours of wall time.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is long-running process evidence and a benchmark against the accepted profile on an unchanged RC; resource overruns/losses block the final acceptance.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e01"></a>
## V1-QA-E01. Clean install and own identity

**Type:** release-case. **Source cards:** I01, I04, U01, U06, U07. **Position in dependency order:** 102.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-P06](14-tasks.md#v1-p06), [V1-W04](04-tasks.md#v1-w04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. On clean macOS/Windows/Linux, create a profile without OAuth, exchange the first message, close and reopen the app.
2. Check NetworkID, exact message contents, the keystore, and the real UI→Rust→daemon path.

**Verifiable scenarios:**

- Neither a Google login nor public keys in the renderer are needed for one's own address.
- Wrong key/permission failures do not create a new empty profile.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Identity and history persist on each OS; the shipping artifact is verified separately from debug.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e02"></a>
## V1-QA-E02. Company-off fresh join

**Type:** release-case. **Source cards:** N02, N05, L06, X01. **Position in dependency order:** 103.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-N03](12-tasks.md#v1-n03), [V1-V02](15-tasks.md#v1-v02).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Turn off company bootstrap/DNS/API; delete cached peers on the new client.
2. Join via an independent invitation and verify authenticated registry/trust.

**Verifiable scenarios:**

- Connectivity and network verification work via independent available sources.
- A false root/record is not accepted even from a reachable bootstrap.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is a clean-join transcript and a manifest of real independent operators.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e03"></a>
## V1-QA-E03. NAT and relay loss

**Type:** release-case. **Source cards:** N01, N04, M06, X01. **Position in dependency order:** 104.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-N04](12-tasks.md#v1-n04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. In a real isolated NAT topology, run desktop and packaged CLI exchange without a direct path.
2. Remove the active relay; verify replacement, restart, and an admissible direct upgrade.

**Verifiable scenarios:**

- The original payload/ACK reaches the recipient after the switchover.
- The relay does not disclose content and does not substitute the authenticated peer.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** NAT profiles and network faults are actually observed; a localhost smoke is not used as a substitute.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e04"></a>
## V1-QA-E04. Hostile peer and backpressure

**Type:** release-case. **Source cards:** F05, N01, D01, X01. **Position in dependency order:** 105.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-N05](12-tasks.md#v1-n05).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Send forged records/proofs, oversized frames, and slow-reader traffic to a real peer.
2. Run an honest exchange at the same time and measure memory/CPU/queues/latency.

**Verifiable scenarios:**

- The honest peer progresses within the established limits.
- A bad proof does not persist paid state; exceeding quotas does not bypass admission.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Limits and fairness are confirmed on the RC without increasing them in the test.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e05"></a>
## V1-QA-E05. Offline/crash/dedup and full history

**Type:** release-case. **Source cards:** I04, I05, D02, D05, X01. **Position in dependency order:** 106.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-H11](01-tasks.md#v1-h11), [V1-W04](04-tasks.md#v1-w04), [V1-G05](08-tasks.md#v1-g05).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Run unchanged Full130 with the sender absent, real copy loss, both SQL faults, and a cold full graph.
2. Separately go through first-offline Welcome, reorder/duplicate, and several MLS epochs via ordinary entry points.

**Verifiable scenarios:**

- Exactly one exact MessageID per original, a full plaintext oracle, and partial129→full130 before the earliest expiry.
- No full import is faked by merging two caches, increasing TTL, or a manual Welcome.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Long established history and first offline contact are covered; legacy/gap/rejoin restrictions are explicitly verified.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e06"></a>
## V1-QA-E06. Operator-only repair 10→7→10

**Type:** release-case. **Source cards:** N05, N06, D03, D04, X01. **Position in dependency order:** 107.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-R05](05-tasks.md#v1-r05), [V1-N02](12-tasks.md#v1-n02).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Reach ten valid data/index copies; stop both clients and three designated nodes.
2. Admit spares, wait for autonomous repair, read the new bytes with an independent audit client, and bring Bob back.

**Verifiable scenarios:**

- Hashes, new receipts/locations, and the final budget agree.
- Without spare/capacity the status stays degraded; the sender does not sign repair in the background.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Repair is performed by the network with proven discoverability; separate identities are not passed off as physical independence.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e07"></a>
## V1-QA-E07. Index loss, chunks, TTL, and disk-full

**Type:** release-case. **Source cards:** D01, D04, D06, X01. **Position in dependency order:** 108.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-B04](06-tasks.md#v1-b04), [V1-N02](12-tasks.md#v1-n02).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Lose an available index holder and part of the attachment chunks; recover via another path and resume.
2. Reach expiry and disk-full in separate branches; check retention and GC.

**Verifiable scenarios:**

- Before the deadline, the exact hash of the whole file is recovered.
- Expired/quota/disk-full do not become delivered and do not delete still-live paid objects.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Success and failures are verified against actual bytes, usage rows, and restart.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e08"></a>
## V1-QA-E08. Group and old/new secret boundaries

**Type:** release-case. **Source cards:** G01, G03, G05, I05, U03, X02. **Position in dependency order:** 109.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-G10](08-tasks.md#v1-g10).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Create a group, connect devices, exchange messages, remove Bob, and add a new member.
2. Check both privacy profiles and decryption of old/new epochs on separate real devices.

**Verifiable scenarios:**

- The remaining members read the new message.
- The removed device cannot read the new epoch; the new member does not get old secrets without a separate history-share.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The UI roster matches the final MLS state; the declared metadata boundaries are respected.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e09"></a>
## V1-QA-E09. Concurrent encrypted group control

**Type:** release-case. **Source cards:** G02, G03, P01, X02. **Position in dependency order:** 110.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-G04](08-tasks.md#v1-g04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Create conflicting Add/Remove/Update on the same parent and split the 4/3 committee into 2/2.
2. Restore the network; include withheld and semantically invalid MLS payload cases.

**Verifiable scenarios:**

- A single final prefix/roster after recovery; a deterministic reject/no-op for inadmissible control.
- The quorum does not drop, keys are not rolled back, a missing payload is not finalized.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Consensus and MLS semantics are verified independently and converge in a real client.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e10"></a>
## V1-QA-E10. Offline device, control loss, and safe rejoin

**Type:** release-case. **Source cards:** I03, I05, I06, G04, X02. **Position in dependency order:** 111.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-G06](08-tasks.md#v1-g06), [V1-I05](07-tasks.md#v1-i05).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Leave a device offline longer than the key window, lose part of the control log, and restore an old backup on another device.
2. Check retained catch-up, the explicit gap, and an allowed rejoin with a new MLS identity.

**Verifiable scenarios:**

- The permitted history and current membership are preserved.
- A revoked device does not regain rights; old private keys are not published for the sake of recovery.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The loss of all retained records is honestly visible; recovery does not clone active ratchets.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e11"></a>
## V1-QA-E11. Shipped CLI skill, MCP, and a real host

**Type:** release-case. **Source cards:** M01, M02, M05, M06, X05. **Position in dependency order:** 112.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-M05](09-tasks.md#v1-m05), [V1-N04](12-tasks.md#v1-n04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. On the installed artifact, the host performs identity/send/delivery/poll/ack for a new contact, then a restart.
2. An independent MCP client checks the wire lifecycle/tools/resources/scopes; repeat the exchange behind NAT/relay.

**Verifiable scenarios:**

- The CLI works without MCP and without sources; the MCP stdout contains only protocol.
- Bad scope, stale grant, and an incompatible protocol get an explicit refusal.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There are real host and NAT transcripts, stable JSON/exit codes, and a preserved inbox.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e14"></a>
## V1-QA-E14. Injection, XSS, and revocation between prepare/send

**Type:** release-case. **Source cards:** I02, M01, U04, X05. **Position in dependency order:** 113.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-U05](13-tasks.md#v1-u05), [V1-M04](09-tasks.md#v1-m04).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Deliver untrusted markdown/attachment/message asking to disclose a secret/send a message.
2. Revoke a grant after preparing an action; change the parameters of an already approved action.

**Verifiable scenarios:**

- Regular permitted correspondence continues.
- The renderer executes no XSS; the broker does not disclose the key and does not accept an old approval or a revoked actor.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Forbidden side effects are verified against the actual outbox/recipient, not a single error text.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e17"></a>
## V1-QA-E17. Live Google OAuth lifecycle

**Type:** release-case. **Source cards:** O01, O05, U02, X04. **Position in dependency order:** 114.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-O10](11-tasks.md#v1-o10).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Go through real native PKCE login, cancel/retry, and JWKS/key rotation in an admissible mode.
2. Unlink the provider, restart the app, and continue the conversation.

**Verifiable scenarios:**

- The credential is bound to the original owner; the address and history are preserved.
- Wrong state/audience/nonce and replay do not create a binding/grant.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A real registration is used; fixtures complement rather than replace the live flow.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e18"></a>
## V1-QA-E18. Telegram and site/org handoff

**Type:** release-case. **Source cards:** O02, O03, O07, O08, X04. **Position in dependency order:** 115.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-O10](11-tasks.md#v1-o10).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Go through live Telegram and the reference site OIDC; check the accepted 1-of-1 and k-of-n profiles.
2. Substitute owner/handoff/origin, then turn off the issuer/gateway and continue the existing chat.

**Verifiable scenarios:**

- Correct scoped credentials are accepted by the original owner.
- Interception gives no rights to another profile; client secrets remain only in the gateway.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** An issuer failure is limited to new credentials; messages/history keep working.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e19"></a>
## V1-QA-E19. Really collateralized free start

**Type:** release-case. **Source cards:** L03, O04, O06, U02, X03, X04. **Position in dependency order:** 116.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-ECO08](10-tasks.md#v1-eco08), [V1-O10](11-tasks.md#v1-o10).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. From a profile without a crypto balance, obtain an allowed funded grant and send a message.
2. Repeat the claim across device/wallet/attestor/providers; exhaust the fund and pass the cutoff.

**Verifiable scenarios:**

- A single entitlement is not doubled; spending is bounded by the real funded balance.
- Invalid eligibility, another namespace, and an empty fund do not create stamps; the paid path after cutoff works.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Receipts, the campaign policy, and the conservation oracle are saved; a monetary cash-out is not promised.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e20"></a>
## V1-QA-E20. Concurrent spend, handover, refund, and payout

**Type:** release-case. **Source cards:** P01, P03, P04, L05, X03. **Position in dependency order:** 117.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-A06](03-tasks.md#v1-a06), [V1-ECO08](10-tasks.md#v1-eco08).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Spend one stamp concurrently through different admissible nodes, including crash/retry and handover.
2. Complete claims/refund paths for the same operation and repeat after a cold restart.

**Verifiable scenarios:**

- One final operation-bound spend and at most one admissible payout/refund.
- Changing epoch/format/node does not create a new resource or an empty spent namespace.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** An independent oracle confirms global spent and conservation on the full trace.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e21"></a>
## V1-QA-E21. Reorg and finite L2 autonomy

**Type:** release-case. **Source cards:** L06, P04, D02, X03. **Position in dependency order:** 118.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-A06](03-tasks.md#v1-a06), [V1-ECO08](10-tasks.md#v1-eco08).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Feed a false RPC/root, a real admissible reorg, and an outage before/after the hard lease.
2. Check automatic renewal, pending inputs, and historical retrieval with expired admissions.

**Verifiable scenarios:**

- New spending is admissible only within the lease; old live objects are read by historical proof.
- Non-final funding does not become a balance; after hard expiry, admission waits.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Trust/signatures, expiry, and availability are separated; renewal does not require manually setting QC.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e22"></a>
## V1-QA-E22. Royalty conservation and absence of governance

**Type:** release-case. **Source cards:** L04, L05, P05, X03. **Position in dependency order:** 119.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-ECO08](10-tasks.md#v1-eco08).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Run the fee split/operator settlement, then repeat the payout and revert the revenue recipient.
2. Check the full role/call graph and attempts of a captured revenue key to govern the network.

**Verifiable scenarios:**

- Amounts are conserved; a blocked share is available via pull payment and does not hinder the others.
- There is no mint/pause/upgrade/blacklist/committee right and no double payout.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Bytecode/config and the independent model agree; revenue-off does not switch off the transport.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e23"></a>
## V1-QA-E23. Everyday UX and local storage secrecy

**Type:** release-case. **Source cards:** I01, I06, D06, U05, X02. **Position in dependency order:** 120.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-U05](13-tasks.md#v1-u05), [V1-I05](07-tasks.md#v1-i05), [V1-B05](06-tasks.md#v1-b05).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Go through search, send/download attachment, notifications, lock, backup/import, and disk-full/restart.
2. Check the selected WAL/temp/log/preview surfaces for sensitive content and compare the restored history.

**Verifiable scenarios:**

- Message/file states are consistent; errors give a specific action.
- Plaintext does not appear in the checked unprotected artifacts; a backup does not restore revoked rights.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Native UI evidence and verification of actual data confirm all branches.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e24"></a>
## V1-QA-E24. Native installer and production smoke on three OSes

**Type:** release-case. **Source cards:** U06, U07, I01, F06, X06. **Position in dependency order:** 121.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-P06](14-tasks.md#v1-p06).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Install the exact RC artifacts on macOS ARM64, Windows x64, and Linux x64.
2. Check the real bridge, keystore, restart, packaged CLI/MCP, and separately the shipping feature set; inspect screenshots.

**Verifiable scenarios:**

- All platforms pass on their own, without substituting a cross-compilation report.
- The shipping artifact contains no test plugins/debug crypto/dev server; native automation does not touch the login Keychain.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Three standalone install/smoke reports are tied to the same source/lock/config inputs.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e25"></a>
## V1-QA-E25. Two compatible versions and extension boundaries

**Type:** release-case. **Source cards:** F02, P02, U06, X06. **Position in dependency order:** 122.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-V01](15-tasks.md#v1-v01), [V1-P05](14-tasks.md#v1-p05).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Exchange identity/mailbox/history/attachments between the previous-compatible and current releases.
2. Connect a test settlement/verifier adapter and pass unknown critical/noncritical extensions.

**Verifiable scenarios:**

- Shared V1 data is read without loss and without a silent migration reset.
- Unknown critical is rejected; an unsupported adapter does not declare unconfirmed payment verified.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The compatibility corpus and real processes agree; jobs/reviews are not included back into V1.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-qa-e26"></a>
## V1-QA-E26. The whole network without company resources

**Type:** release-case. **Source cards:** N02, D04, G04, L06, O05, M06, U06, X01, X06. **Position in dependency order:** 123.
**After:** [V1-V04](15-tasks.md#v1-v04), [V1-V02](15-tasks.md#v1-v02).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. On the independent network, turn off company endpoints, the gateway, the release CDN, and the revenue recipient.
2. Continue DM/group/CLI/MCP, retrieval, and operator repair under the declared surviving copies/retention/budget/connectivity.
3. Separately verify a clean join via independent hints.

**Verifiable scenarios:**

- Installed clients and holders continue the permitted operations.
- New external credentials are not faked; expiry/resource exhaustion is reported honestly.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Failure domains and external faults are confirmed; the existence of ten keys is by itself not proof of decentralization.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-rc02"></a>
## V1-RC02. Accept the 67 cards and close AR1–AR5 on evidence

**Type:** verification. **Source cards:** F01, F06, X06. **Position in dependency order:** 124.
**After:** [V1-RC01](16-tasks.md#v1-rc01), [V1-RC-SOAK](16-tasks.md#v1-rc-soak), [V1-QA-E01](16-tasks.md#v1-qa-e01), [V1-QA-E02](16-tasks.md#v1-qa-e02), [V1-QA-E03](16-tasks.md#v1-qa-e03), [V1-QA-E04](16-tasks.md#v1-qa-e04), [V1-QA-E05](16-tasks.md#v1-qa-e05), [V1-QA-E06](16-tasks.md#v1-qa-e06), [V1-QA-E07](16-tasks.md#v1-qa-e07), [V1-QA-E08](16-tasks.md#v1-qa-e08), [V1-QA-E09](16-tasks.md#v1-qa-e09), [V1-QA-E10](16-tasks.md#v1-qa-e10), [V1-QA-E11](16-tasks.md#v1-qa-e11), [V1-QA-E14](16-tasks.md#v1-qa-e14), [V1-QA-E17](16-tasks.md#v1-qa-e17), [V1-QA-E18](16-tasks.md#v1-qa-e18), [V1-QA-E19](16-tasks.md#v1-qa-e19), [V1-QA-E20](16-tasks.md#v1-qa-e20), [V1-QA-E21](16-tasks.md#v1-qa-e21), [V1-QA-E22](16-tasks.md#v1-qa-e22), [V1-QA-E23](16-tasks.md#v1-qa-e23), [V1-QA-E24](16-tasks.md#v1-qa-e24), [V1-QA-E25](16-tasks.md#v1-qa-e25), [V1-QA-E26](16-tasks.md#v1-qa-e26).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. For each source card, verify the entire remaining scope, its dependency GREEN, and real test/demo artifacts; a link to one plan task does not mean acceptance.
2. Cross-check all AR findings: fixes or documented preservation of protective invariants, with no hidden preview scope.
3. Match the 67 task acceptances, 22 E2E, 11 suites, and 3 OSes to a single RC; repeat the security-sensitive review by an independent reviewer.

**Verifiable scenarios:**

- Missing/skipped-required/blocked/flaky or a different revision blocks full readiness.
- A sum of old successful runs does not substitute for current evidence.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Only with the complete set can product_validated=true be set for V1 code/testnet; mainnet_transport_approved remains a separate decision.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-rc03"></a>
## V1-RC03. Prepare the delivery and the final report

**Type:** release. **Source cards:** X06, U06. **Position in dependency order:** 125.
**After:** [V1-RC02](16-tasks.md#v1-rc02).

**Change boundary / entry points:** `scripts/check.sh`, `scripts/check-native.mjs`, `scripts/check-network.mjs`, `tests`, `apps/desktop/tests`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Assemble checksums, install/update instructions, the CLI skill, the testnet manifest, compatibility/migration notes, and security limitations.
2. Prepare the concrete release artifact and changelog from the final implementation, preserving the evidence and previous failures.
3. Finish the cleanup of processes/temporary profiles and hand the results to the user; perform external publication only within a separate assignment.

**Verifiable scenarios:**

- Another user reproduces the installation and the start of a conversation from the documentation.
- Secrets, native test keys, output caches, and local configs are not part of the delivery.

**Checks:** RELEASE profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A verifiable V1 code/testnet package is ready. This plan itself does not start publication or deployment.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
