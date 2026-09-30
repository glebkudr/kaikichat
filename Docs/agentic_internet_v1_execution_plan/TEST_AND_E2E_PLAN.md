# V1: tests, E2E and the release criterion

**September 14, 2026:** the [detailed completion tasks](v1-plan-2026-09-14/README.md)
and [targeted check rules](v1-plan-2026-09-14/RUNBOOK.md) account for the current code.
The user paused implementation and agent dispatch until further instructions.
Under the user's standing rule the full suite runs only at the end of the whole plan;
intermediate changes are checked with targeted backend/frontend clusters.

**Architecture review of September 10, 2026:** [new ordering and continuation contracts](../V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md), [24 findings and five phases](architecture-followup.json). First the long sender/spend lifecycle, then user inputs, full history/R10, groups/recovery and independent release. The active 67 cards and 22 E2E are retained.

**Current decision of September 9, 2026:** agent orders, rating and reviews are **V2**. The priority is shipping the V1 messenger with CLI/MCP. The [phase boundary](../V1_SCOPE_2026_09_09.md) and [mandatory cards/scenarios](release-scope.json) take precedence over the former 1.1 package.

Original edition: September 5, 2026; the phase boundary updated on September 9. This is the acceptance contract; for currently executed checks see ../../IMPLEMENTATION_STATUS.md. V1 mandates 22 scenarios; E12/E13/E15/E16 are moved to V2.

The main plan is [README.md](README.md); the task list is the [original backlog](../agentic_internet_v1_1_plan/BACKLOG.md); the machine mapping is [execution-map.json](execution-map.json).

## 1. What "all tests pass" means

A release candidate has a fixed list of mandatory suites, cases and platforms. A `PASS` result requires a nonzero number of executed tests, no failed/missing/skipped mandatory checks, an accepted test review for each backend module, and full agreement of source revision, lockfiles, genesis/config and artifact hashes.

`not_run`, `blocked_by_environment`, `failed` and `passed` are distinct states. An optional experiment may be excluded from the release scope before a run with an explanation; the mandatory V1 groups, repair, auth and postage are not excludable this way. Moving orders/rating/reviews to V2 is directly authorized by the user and recorded in release-scope.json. A quarantined/flaky required test blocks the release. A success after a rerun does not erase the original failure: fixing the cause or a proven infrastructure outage with a saved report is required.

Releasing finished code and the testnet is separate from clearing real funds. Mainnet transport requires additional verification of contracts/proofs, fixed monetary parameters, backing of obligations and a report on the actual operators. Green local tests do not prove any of this.

### Verification levels

| Suite | What is executed | Independent basis of the expected result |
|---|---|---|
| Backend unit | Broker, identity, reducers, resource arithmetic, parsers | Permission matrix, explicit state transitions, numeric examples from the specification |
| Backend integration | Real DB, keys, MLS store, libp2p and IPC | External client and crash checkpoints; observation of durable state |
| Frontend | Components, view models, forms, keyboard flows | User scenarios and domain statuses; without a copy of backend rules |
| Wire/crypto conformance | Golden bytes, hashes/signatures, MLS vectors, versions | RFC/upstream vectors and a second codec/verifier; not the output of our own encoder |
| Network simulation | The same production reducers with controlled clock/RNG/network | A separate reference state machine and invariant checks |
| Process/network E2E | Several real node/client processes, NAT/partition/kill | Client transcripts, received-data content and protocol certificates |
| Contracts/proofs | Local EVM, real verifiers and smart contracts | Conservation model, independent verifier, attack traces |
| MCP/A2A | Real wire clients and pinned conformance fixtures | An external client/standard, not a direct call of the Rust handler |
| Tauri native | Instrumented Tauri with a real Rust bridge/daemon | Actions through the UI and independent state verification, IPC mocks disabled |
| Production smoke | An installable artifact without test plugins | Start/restart/basic chat, OS automation and saved screenshots |
| Live providers/testnet | A registered OAuth flow, real testnet receipts | Callback/token validation and chain-specific finality evidence |

Fuzzing applies to parser/frames/credential/proof inputs; mutation — to auth, domain, epoch, limits, quorum and spending. No need to write property tests for every UI label. For recovery/state machines, real failures matter more than line-coverage percentage.

## 2. Rig

**Local base profile:** Alice, Bob, Carol with separate data directories; two agent runtimes; 14 storage profiles (10 assigned + 4 spare); four finalizer profiles with quorum 3; two relays; several bootstrap hints; local EVM; a reference OIDC provider. Roles may share processes after isolation is verified, but they have different keys/stores and separate manifest entries. The four validators verify a model of at most one Byzantine node, not public-network security.

Most tests start only the needed subset. E06 brings up the full storage profile. E09/E20 control a separate committee. Ordinary tests require no paid LLM/API or internet. The LLM runtime for protocol tests is deterministic; the real agent host is checked by a separate smoke and does not replace the contract suite.

Local containers on macOS run via **OrbStack**. NAT, routing and packet-loss tests run in a Linux VM/network namespaces; connecting two processes on localhost does not count as NAT. The WebView/installer matrix needs macOS ARM64, Windows x64 and Linux x64. Other architectures are not declared supported without their own matrix.

**Decentralization:** several processes on one laptop prove only protocol behavior. External verification needs nodes of different operators, replaceable relays/bootstrap and fixed failure domains. The report separately shows the number of keys, processes, machines and operators. Neither ten receipt signatures nor a retrieval challenge proves ten independent physical copies.

**Isolation:** every run has unique genesis/network IDs, data dirs, ports, account keys and budget. Mocks use a separate dev genesis; the production verifier does not accept their credentials, proof IDs or receipts. All clocks have an explicit source; simulated time is not passed off as actually elapsed days.

## 3. E2E scenarios

| ID | Scenario and actions | Success criterion | Main modules |
|---|---|---|---|
| E01 | Clean install, local identity without OAuth, contact, message, restart | NetworkID and history preserved; Google login is not needed for one's own address; no plaintext keys in the renderer | F I U |
| E02 | Disable company bootstrap/DNS/API; a new client joins by invitation from an independent peer | Finds an available network and verifies registry state; does not require a single vendor endpoint | N L |
| E03 | Two clients behind NAT; direct path unavailable; one relay disappears | Exchange via the other relay, then a possible direct upgrade; peer auth and E2EE preserved | N D |
| E04 | A peer substitutes a record/registry proof, sends oversize and reads slowly | Substitution rejected, memory/queues bounded; the honest peer continues the exchange | F N D |
| E05 | Recipient offline, sender restarts around commit/send; the network repeats/reorders delivery | After return, one record per MessageID, correct order within the declared model, content matches | I D |
| E06 | Reach R=10, power off both clients and three storage nodes, enable spare nodes | Operators restore the target themselves; Bob receives the message after returning; receipts/indexes available | N D P |
| E07 | Lose an index node; download a chunked attachment; wait out the TTL; fill the disk | Live data found via another path; hashes converge; expired/quota/disk-full do not turn into delivered | D |
| E08 | Create a group, invite devices, remove Bob and send a new message | Remaining members can read; the removed device cannot decrypt the new epoch; the new member does not get old secrets | I G U |
| E09 | Concurrent Add/Remove/Update, partition the committee, an invalid MLS proposal | No two accepted final epochs in the model; after recovery there is a common roster; the wrong commit is not merged | P G |
| E10 | A member offline for a long time; loss of part of the control log; identity migration to a new device | Catch-up or explicit rejoin; old keys are not cloned for convenience; a revoked device does not regain rights | I D G |
| E11 | Client + skill with CLI: using the issued handle, learn one's own ID, message another ID, receive/ack a reply, recover after a restart; also an independent MCP client over stdio: discovery/legacy lifecycle, tools, resources, bad scopes | The CLI scenario passes without MCP and internal APIs, together with NAT/relay E03; CLI JSON/exit codes are stable; MCP JSON-RPC is compatible, stdout contains only protocol; access is limited by the actor grant; the version is agreed | M I N U |
| E12 · V2 | An MCP customer finds a service, accepts a bid, the performer returns an OCR/coding/inference artifact | Terms/result hashes and signatures verified, the client validates the result; the model remains declared, settlement is separate | A M D |
| E13 · V2 | Stop the agent runtime after a lease; a second runtime continues; a cancel crosses a result | The job is not lost, leases are restored per policy; projections converge; external effects require their own idempotency | A M I |
| E14 | An untrusted message/markdown/attachment asks for secrets and a send; grant revoked between action preparation and send | The broker forbids the action; XSS does not execute; the old approval does not fit the new parameters; normal operation continues | I M U |
| E15 · V2 | Order accepted, the performer disappears without result or payment; the customer publishes a negative review | An independent client verifies the receipt and finds the review; further consent of the performer is not required | A Q M U |
| E16 · V2 | Duplicate reviews, concurrent amendments, reply/withdrawal, owner change, a censoring index | One effective vote; an equal corpus gives an equal rating; different corpora are marked partial, someone else's history is not appropriated | Q |
| E17 | Real Google native OAuth; retry/cancel; provider unlink | PKCE/state/token validation passed, the credential is bound to the original key; address and chats preserved after unlink | O I U |
| E18 | Real Telegram gateway flow and reference site OIDC; handoff/owner substitution; issuer off | The secret stays only on the gateway; interception does not give the credential to another owner; new attestations unavailable, chat works | O I U |
| E19 | Funded onboarding without user gas; repeated claim via device/wallet/attestor; subsidy time and fund exhaustion | The entitlement is not doubled; spending is bounded by the real fund, after the cutoff the paid path works without an upgrade | L P O U |
| E20 | One postage stamp is spent simultaneously through different nodes/shards; crash, handover and repeat | Only one final operation-bound spend; no double refund/payout; the resource is not created by an epoch change | P L |
| E21 | A false RPC/root, reorg, then an L2 outage before and after hard lease expiry | Non-final funding is not accepted; within the lease work continues; after expiry new admissions wait, old retrieval is available | L P D |
| E22 | The royalty recipient is compromised or reverts; an attacker tries to control the network | Amounts are preserved, royalty is accounted once, the recipient has no pause/upgrade/mint/blacklist/committee authority | L P |
| E23 | Search, attachments, notifications, app lock, backup, disk-full and restart | History is consistent; sensitive content is absent from the checked WAL/temp/log/previews; understandable recovery/error states | I D U |
| E24 | Install the native artifacts on every supported OS, check the real bridge and production smoke | All platforms work; test plugins/debug crypto are absent from production; screenshots checked by eye | F U X |
| E25 | A previous compatible client; unknown extensions; connect a test settlement/verifier adapter | Identity, mailbox, history and V1 attachments are readable; jobs/reviews interoperability — V2; unknown critical rejected; unsupported is not passed off as paid/verified | F P X |
| E26 | Disable company resources, gateway, release CDN and revenue recipient in a running independent network | Chats, groups, CLI/MCP messaging, retrieval/repair are preserved under explicit resource preconditions; new external grants are not simulated | N D G L P O M U X |

Each case verifies a positive outcome and one observable forbidden behavior. A single scenario may include several named assertions; evidence stores the result of each. The table does not replace the unit/integration checks of the 67 active V1 cards in their scope. The four deferred E2E have required_for=v2 and status=not_run, not passed/skipped-required.

## 4. Key scenarios without ambiguous shortcuts

### E06: the network actually performs the repair

1. Bob offline; Alice sends a known payload and receives ten signed durable receipts from distinct test node identities.
2. Verify the presence of the ciphertext and associated indexes via retrieval. Stop Alice and confirm her process has terminated. Bob remains stopped.
3. Stop the three **assigned** storage nodes; the inventory and the effect of time are recorded by the harness, not by the `replicas=10` value from the UI.
4. The available operators detect the loss and restore the ciphertext, manifest and discoverability pointers on spare nodes. Verify the repair authority issuer, the final spend and the absence of a storm.
5. The new copies are read by an independent audit client; the byte hash is verified. Start Bob: he finds the data and decrypts the original message.
6. Separate branch: no spare capacity. A degraded state with an honest confirmation count is expected, not endless waiting with a green "delivered".

Survival needs a surviving correct copy, connectivity, an admissible confirmed assignment, a budget and available capacity. With all copies lost or an expired TTL, recovery is not promised.

### E09: consensus and MLS are verified separately

The harness creates competing commits from different admissible participants. In the 4/3 fixture a 2/2 partition stops new finality; the quorum is not reduced. After recovery, the single control-log prefix and roster of all honest clients, the behavior of the losing intent and the unavailability of the new epoch to the removed participant are verified.

Additionally an admissible sequencing actor proposes a ciphertext that fails MLS semantics after finalization. All honest clients must reject the proposal identically and continue per the normative policy without rolling back used keys. If this path is not defined and working, G02/G03 cannot be accepted even with green BFT-library tests.

### E11 together with E03: the packaged skill and CLI behind NAT

1. On clean installs use the release client, its CLI and the packaged `SKILL.md`; the user issues a scoped connection context. The agent host receives only the skill and this context. A deterministic host checks the contract; a separate smoke with a real agent checks that the skill instructions are sufficient.
2. Through the installed CLI obtain one's own NetworkID and the permitted actor context. Pass the public NetworkID of a permitted recipient without an internal ConversationID; the daemon creates the first conversation per the contact policy. The recipient sees the original text and sends a reply.
3. Read the reply via `inbox.poll`, verify sender/MessageID/content and acknowledge only what was processed via `inbox.ack`. Repeat the send with the same idempotency key: no second message is created. A local acceptance status does not count as delivery.
4. Stop the host before the ack, send it another message, restart: unacknowledged inbox items are available, acknowledged ones do not return as new work. If a hook is chosen, additionally check its disabling and recovery through the same durable queue.
5. Run the exchange in the N04/E03 NAT matrix: hole-punch success, relay-only, relay failure and switch to an independent relay. Isolated network environments are mandatory; localhost does not confirm NAT traversal. Content and peer identity are verified at the recipient.
6. Verify CLI JSON/exit codes, the ban on someone else's recipient outside the grant, session expiry and revocation. Then pass the E11 MCP conformance check and confirm the same permission constraints.

Acceptance is tied to R02/R17 and M01/M02/M06/U06/X05 together with N04. Executable black-box tests are written first; the mere presence of skill files or a successful call of an internal handler is not enough.

### E12: three real applied work types · V2

The section is preserved for V2; it is not part of the first release acceptance.

| Work | Fixture and independent verification |
|---|---|
| OCR | A fixed image and known fields/text; the verifier compares fields/numbers, not just the presence of JSON |
| Coding | A task and a small test corpus; the artifact runs only in an isolated execution adapter without daemon keys; the tests are not formed from the submitted solution |
| Inference | A request for a structured result with verifiable constraints; schema/constraints are checked by the client; the exact model remains `executor-declared` |

In each case two real client/agent processes go through discovery → RFQ → bid → two accept signatures → persisted receipt → progress → result → client validation. The artifact is delivered via E2EE/blob delivery; the result hash is bound to the accepted terms hash. For a deterministic protocol E2E the result may be prepared in advance by a worker fixture; this is marked and does not count as an LLM quality test. An additional live host smoke confirms the actual MCP integration.

### E19/E20: solvency and absence of double spending

The source of the expected numbers is a separate conservation model. The model accounts for funding, accepted obligations, operator reserve, royalty, consume/cancel, refund/replacement and terminal payouts. Not only the user balance is checked, but also the impossibility of simultaneously returning the resource to them, paying the operator and keeping an unfunded storage lease.

One ticket is sent competitively to different admissions and repeated after kill/restart/reconfiguration. An independent verifier checks the final certificates and the uniqueness of their operation commitment. A repeat of an old operation does not remove the nullifier; an epoch/domain transition does not create a new right. Checks run on real verifiers/local contracts; a mock `paid=true` takes no part in acceptance.

### E26: what exactly is switched off

The harness blocks known company domains/IP endpoints, stops company bootstrap/relay/storage/gateway processes and denies access to the release/config API. The remaining independent operators have enough capacity, active leases and quorum. Both existing clients and a new client with an invitation from an independent participant are checked.

A new Google/Telegram/site credential through the disabled issuer may be unavailable — this is an expected local limitation. An already existing identity is not lost, claims are not replaced with forged ones, MCP messaging and existing permissions keep working. Independence of public reviews is verified in V2. L2 outage is tested separately by E21: this case does not promise infinite network funding without the chain.

## 5. Preliminary limits and measurements

The following values are proposed **test profiles**, not measured properties of the application and not ready production constants. C0/spikes must confirm attainability, after which the profile is fixed before performance implementation. Changing a threshold after a failure is recorded as a requirements change preserving the original result.

| Profile | Proposed value |
|---|---|
| Message | Up to 16 KiB of text; control header up to 16 KiB; application frame up to 64 KiB |
| Attachment | Up to 10 MiB in the base profile; a separate bulk stream with chunks up to 256 KiB, a verified manifest and hash; a chunk is not packed into the 64 KiB application frame |
| Storage | R_target=10, R_accept_min=7 with an explicit degraded state; example retention 7 days and offline 24 hours |
| Group | Up to 100 members and 3 devices per member in the acceptance profile |
| Repeats | 10,000 messages with duplicate/reorder: no lost accepted messages under the preconditions and no double application |
| Recovery | For an envelope up to 256 KiB, three failures, free capacity and a healthy network — the target is restored within 120 s wall time |
| Latency | Candidate p95 ≤2 s direct, ≤5 s relay on the manifest-specified rig at RTT 100 ms, 10 Mbit/s and the agreed load |
| Soak | 24 h process soak before release; the virtual offline/TTL test runs separately |

The benchmark manifest fixes hardware, OS, node count, payload sizes, load, RTT/loss, bytes transferred, RAM/queue caps, resource consumption and quantiles. Alongside time, correctness, timeout/error rate and completion rate are measured. A fast failure does not count as successful delivery.

The number of storage confirmations, the BFT quorum and the actual independence of operators are three different metrics. A 7/10 result is not displayed as 10/10. `recipient_durable` and `application_acknowledged` are also distinct; an agent's ack does not prove that the work was done.

## 6. Runner and CI to implement in F

Below is the proposed CLI of the future `xtask`; these commands **do not exist yet**:

```bash
cargo xtask test-module --id D04
cargo xtask verify --suite backend
cargo xtask verify --suite frontend
cargo xtask e2e --case E06 --profile v1-local --seed 3106
cargo xtask verify --profile v1 --release-candidate release-candidate.json
```

`test-module` runs the selected contract and its integration subset. The backend suite includes Rust unit/integration and the release-relevant contract/proof suites; the frontend suite — typecheck and component tests. The full runner gathers conformance, simulation, process E2E, desktop/platform, live-provider/testnet evidence and the final report.

| Point in time | Mandatory checks |
|---|---|
| Before implementing a module | RED + backend-test-critic ACCEPT; does not apply to purely documentary changes |
| After implementing an individual task | Targeted backend/frontend clusters, affected E2E, formatter/Clippy of affected crates; actual producer adapters before card acceptance |
| At an intermediate integration milestone | Only the integration/E2E and native UI related to it on the target OS, reviewed screenshots; the full suite is not run |
| Long-running checks | Targeted fault/fuzz/mutation when justified; the full set, benchmark and 24 h process soak — on the final RC. Nightly/automation are not created by this plan |
| Release candidate | All mandatory V1 suites and 22 E2E from release-scope.json, the full OS matrix, production artifacts, live smoke, soak, review and a single source revision |

Everyday frontend changes can be tested headless. Native GUI tests run in an isolated CI session; Linux — via a virtual display, macOS/Windows — on dedicated runners. We do not launch a visible Chrome/Playwright and do not activate applications on the user's desktop. For visual acceptance, screenshots are saved that the reviewer actually opens and inspects; a DOM snapshot alone is not enough.

Do not conflate Tauri instrumented E2E and production smoke. The former uses a test-only WebDriver transport with a real broker. The latter checks a separate shipping artifact without these plugins; its digest, feature set and match to the same source revision are fixed separately.

## 7. Evidence and release-report

For each card: source/task ID, test file/symbol, reason for the expected RED, red/green log, critic verdict, source revision, fixture/oracle revision and integration demo. For E2E: scenario ID, profile/config hash, binary hashes, commands, seed, topology, transcript, fault injection timeline, observations/assertions and screenshots for UI.

Proposed report format:

```json
{
  "schema_version": 1,
  "source_revision": "<git-commit>",
  "scope": "v1-code-and-testnet",
  "product_validated": false,
  "release_scope_id": "v1-scope-2026-09-09",
  "required_case_ids": ["E01", "E02", "E03", "E04", "E05", "E06", "E07", "E08", "E09", "E10", "E11", "E14", "E17", "E18", "E19", "E20", "E21", "E22", "E23", "E24", "E25", "E26"],
  "deferred_v2_case_ids": ["E12", "E13", "E15", "E16"],
  "suites": {
    "backend": {"status": "not_run", "executed": 0, "failed": 0, "skipped_required": 0},
    "frontend": {"status": "not_run", "executed": 0, "failed": 0, "skipped_required": 0},
    "e2e": {"status": "not_run", "executed": 0, "failed": 0, "skipped_required": 0}
  },
  "platforms": {},
  "evidence": [],
  "mainnet_transport_approved": false
}
```

This is a schema illustration, not a real result; the list of mandatory IDs is taken from release-scope.json and cross-checked against execution-map.json. The final runner sets `product_validated=true` only after verifying **all** mandatory suites, task acceptances, platforms and evidence; the three green fields of the example are not enough.

If there is no registered application for Google/Telegram, or no platform available for a native test, local results are kept and the corresponding gate remains unverified. Real client IDs/callback registration, gateway hosting, testnet funding, CI runners and signing/notarization are needed by C5/C6; they do not prevent writing contracts, RED tests and most of the core in C0–C4.

In V1 the goal cannot be closed by plan-generator tests, the number of files created, HTTP mock code, an `implemented` entry in JSON or a successful launch of one client. A reproducible end-to-end result on the real implementation is required.
