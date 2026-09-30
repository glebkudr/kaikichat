# Prerequisites, decisions and risks

This is a list of conditions for future implementation and acceptance. The user has so far asked only for a plan: no answers, access or permissions are required to continue preparing the document.

## What is already fixed and is not silently revisited

| Contract | How it is preserved |
|---|---|
| 67 V1 tasks / 22 E2E / 3 OS | `release-scope.json` is not changed; the source DAG is preserved in `plan.json` |
| Public postage stamp signatures without ZK anonymity | Funding/operation/resource binding, the strict parser and the shared spent namespace remain; the legacy historical verifier stays compatible |
| One mutable Core writer | Message/MLS/outbox/import/receipt/cursor stay atomic; workers return a verifiable result rather than mutate Core independently |
| R=10 and repair without both clients | Actual copies, receipts and discoverability are verified separately; the network repairs with a live copy, connectivity, authority, resource and capacity |
| Resource limits R14 | Work 120 s / 16 visits; existing admission/proof/body budgets; no TTL or deadline increase for a PASS |
| Pending-body migration | Keep the combined previous budget of 128+128 entries, 4+4 MiB; one body per operation, metadata separately |
| Holder-local complete | Does not mean global history complete; a missing reference below the cursor is available for exact repair |
| Original expiry | Retention is not extended by transfer/repair; expired admissions and historical retrieval have different checks |
| Keychain | E2E uses a temporary debug file store; the regular build keeps the system keystore |
| V2 | Jobs/services/A2A, rating and reviews do not return to default V1; old readers are preserved |

## External conditions and decisions before dependent stages

| Condition | Where needed | What can be prepared without it | What is not considered complete without it |
|---|---|---|---|
| Available WD4000/ChatBuild with the correct UUID | All local build/test | Documents and sources | Any local check through a wrapper bypass |
| A chosen EVM L2 testnet, genesis/issuer/checkpoint profile | C04, ECO01–08, A01–06 | Local EVM, fixtures and provider-neutral adapters | Live finality/renewal/settlement |
| Monetary/resource parameters: campaign pool, caps/decay/cutoff, fee split, repair/operator reserve | C04, ECO03–07 | Conservation model and parameterized test examples | The testnet economic configuration; values cannot be invented during implementation |
| An external wallet and testnet funding | ECO01, QA-E19–22 | Existing wallet fixtures and transaction preparation | A real wallet→funded book→message |
| Google native OAuth client, callback registration, a test account | O03–04, QA-E17 | JWT/JWKS and PKCE tests | Live login/cancel/retry |
| Telegram registration, secret on the gateway, HTTPS/callback | O06, QA-E18 | Handoff/domain/replay fixtures | Live Telegram flow |
| Site/org issuer, public origin, agreed 1-of-1/k-of-n trust | O02/O05, QA-E18–19 | Local OIDC/reference issuer | External owner-bound credential acquisition |
| Several independent operators and trust sources | N03, V02, QA-E02/E26 | A local protocol/fault rig | A claim of operator independence and a company-off testnet |
| macOS ARM64, Windows x64, Linux x64 runners | P02–06, QA-E01/E24/E25 | Common IPC/platform code and cross-build | Actual ACL/keystore/native install/production smoke for each OS |
| Signing/notarization/update signing setup | P04–05, RC03 | Local test packages and feature graph verification | Ready installable signed release artifacts where a signature is required |
| A real agent host and explicitly permitted test recipients | M05, QA-E11/E14 | Black-box CLI/MCP clients, fixtures | A host smoke of a new contact; automatic sending to people is not allowed by this plan |
| Previous-compatible shipping artifact | V01, P05, QA-E25 | Wire vectors and an old profile fixture | Two-way compatibility of two real versions |

If a condition is missing, the corresponding gate gets `blocked_by_environment` or stays `not_run`; the remaining independent preparation can continue once work resumes. This is not a reason to silently drop provider/platform/economic scope.

## Measurements that still must confirm the architectural hypotheses

| Risk / uncertainty | Known now | Checkpoint and decision |
|---|---|---|
| Cause of missing original 31 in R14 | The full path trace has not been recovered; fragmentation and retries are confirmed, causation is not proven | H10/H11: trace and exact SQL/import gates. Do not add a retry "specifically for 31" |
| Effect of batching on native throughput | The shared publisher/collector is accepted at component level; R14 had 117 leaves, 106 singleton | H10: measure leaf occupancy and actual time. Guidelines for two batches of 16 are ≤4 leaves, Full130 about 17; this is an efficiency hypothesis, not a replacement for correctness |
| Cost of repeated recovery | The baseline Diagnostic32 did not finish; accepted read counts cannot be extrapolated to Full130 | Compare identical gates/trace; the goal of cutting admitted reads at least in half is verified together with the full result |
| Whole Full130 before expiry | The old run did not fit, exact imports 1–30 | H11: 390 signatures, real 1170+1170 losses, trust refusal, both SQL rollbacks, partial129/full130/cold before the original deadline. If it does not fit, that is a measured blocker and a separate decision, not a deadline increase in the test |
| Discovering the only live copy | The current fixture picks a survivor from four advertised routes | N02 and QA-E06: verifiable discovery for other allowed survivors. The current Full130 success does not prove an arbitrary nine losses out of ten |
| MLS key window | Past epochs/ratchets are bounded; infinite retention changes security | G05/G06: epoch-aware import before GC, or an explicit gap and authorized rejoin |
| Group BFT + MLS | The primitives exist individually, there is no finished vertical | G02–G04: payload preservation until finality, real concurrent commits and deterministic reject/no-op |
| Cost of a long lifetime | Some rows are already normalized; residual whole-snapshot/proof work must be measured | S02–S04: bytes/SQL/CPU/latency baseline, minimal elimination of measured repetition |
| Trust and Sybil | A 4/3 local committee and ten keys do not mean operator independence | C04/V02: manifest assumptions, actual operators/failure domains. Several RPCs do not amount to a light client |
| Local agent host | A single OS UID is not a sandbox against malicious code of the same user | M03/P02: an explicitly supported trust boundary and a scoped API; do not promise a strong OS sandbox without a separate implementation/verification |
| Economic proofs | Receipt/retrieval do not prove an independent disk or the honesty of the whole service | ECO06–08: formal claim scope, funded caps and a measured residual collusion risk |

## Performance and soak

The old TEST_AND_E2E_PLAN has a **proposed** acceptance profile: text 16 KiB, control header 16 KiB, application frame 64 KiB; a 10 MiB attachment with 256 KiB chunks; a group of 100 members with three devices each; 10 000 duplicate/reorder messages; recovery within 120 s at the given profile; p95 direct 2 s/relay 5 s at RTT 100 ms and 10 Mbit/s; a 24 h process soak. These proposals are not achieved characteristics and do not replace the more precise current R14 limits.

V1-C04 must reconcile them with the actual resource contract, explicitly record the accepted benchmark manifest and the discrepancies before the respective tests. The mandatory scenario must not be reduced and no new latency SLA declared without a decision and measurements. The final RC runs a real 24-hour process soak; virtual TTL/clock replay runs separately. Accepted/completed/error counts, p50/p95/p99, resources and restart/leak results are recorded; a fast failure is not a successful delivery.

## Stop condition of the current assignment

The plan documents are agreed on scope/links/DAG, the user has been given the entry file, implementation and agent launches have not continued. The next step is determined by a new user instruction.
