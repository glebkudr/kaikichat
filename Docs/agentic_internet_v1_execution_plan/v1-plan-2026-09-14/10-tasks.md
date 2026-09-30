# 10. Testnet transport economy

Postage stamps, the subsidy, royalty, and payouts are backed by resource and grant no governance rights to fee recipients.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R07, AR-R08, AR-R21, AR-R23, AR-R24.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-ECO01/V1-ECO07** (finality/reorg-safe settlement): Gasper (arXiv 2003.03052); Ebb-and-Flow (S&P 2021); "Three Attacks on PoS Ethereum" (FC 2022); EIP-1186 storage proofs; consensus-specs fork choice. Idempotent claim IDs per KIP-98.
- **V1-ECO02** (registry snapshots, seed): Algorand sortition (SOSP 2017); Dfinity threshold relay; drand; RANDAO biasability — last-revealer attacks (arXiv 2403.09541); VDFs (Wesolowski EC 2019; Pietrzak ITCS 2019); EIP-4399; **edge case:** EIP-2935 + the 256-block BLOCKHASH window for a delayed seed — see research §8.
- **V1-ECO03** (SubsidyVault decay): Roughgarden, "Transaction Fee Mechanism Design"; Swarm price oracle + public criticism of storage incentives; Filecoin economy.
- **V1-ECO04** (cross-domain claim dedup): Semaphore scope-nullifier pattern; KIP-98.
- **V1-ECO05** (royalty splitter): OpenZeppelin PaymentSplitter pull-payment — the engineering benchmark.
- **V1-ECO06** (proof of service): PoR/PDP/MR-PDP/HAIL/Mirror — boundaries in research §3; Filecoin RepGame; Sia storage proofs; Swarm redistribution game.
- **V1-ECO08** (invariant testing): Jepsen-style oracles; property-based testing (proptest).

<a id="v1-eco01"></a>
## V1-ECO01. Finish the external wallet→funded book flow

**Type:** backend. **Source cards:** L01, P02. **Position in dependency order:** 62.
**After:** [V1-A05](03-tasks.md#v1-a05), [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Use the current public purchase flow with a real external wallet and a single chosen EVM L2 testnet.
2. Verify exact chain/genesis/issuer, resource class, amount, and confirmation/finality before funding.
3. Persist pending tx/reorg/retry without re-issuing tickets.

**Verifiable scenarios:**

- A clean user buys a resource and sends a message through the UI without internal commands.
- Wrong chain/issuer, a rejected tx, and a reorg do not create an available balance.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Local EVM and the testnet use pinned bytecode/config domains; there are real receipts from an external wallet.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** A testnet, a funded test account, and a compatible external wallet; no real money required.

<a id="v1-eco02"></a>
## V1-ECO02. Finish the registry bond/unbond and snapshot lifecycle

**Type:** backend. **Source cards:** L02, N05. **Position in dependency order:** 63.
**After:** [V1-C04](00-tasks.md#v1-c04), [V1-A02](03-tasks.md#v1-a02).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Review the already implemented registry/selection and close the missing bond/unbond delays and obligation fences.
2. Tie the committed assignment to a future beacon and an authenticated stake snapshot.
3. Fix an independent reference computation of root/count/units and the admissible bias/withholding.

**Verifiable scenarios:**

- New stake does not change an already selected roster; an allowed unbond does not remove a live obligation.
- A stale/reorg snapshot, early withdrawal, and split identities do not bypass fixed units.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A real testnet registry epoch is verified by clients and operators; keys and operators are not mixed in the report.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco03"></a>
## V1-ECO03. Implement the funded SubsidyVault and schedule

**Type:** backend. **Source cards:** L03, P05. **Position in dependency order:** 64.
**After:** [V1-C04](00-tasks.md#v1-c04), [V1-ECO01](10-tasks.md#v1-eco01).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Fix campaign/global/epoch caps, cutoff/decay, and integer rounding from the accepted manifest.
2. Issue the standard stamp format only within the deposited collateral and the limited sponsor gas.
3. Keep an independent paid path after cutoff/exhaustion; do not make the optional EntryBond a mandatory dependency without a separate decision.

**Verifiable scenarios:**

- Time/epoch boundaries yield a continuous bounded entitlement without negative remainders.
- An empty fund, a repeat of the same claim, and a reentrant call do not create unbacked stamps.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The conservation model and contract tests agree; there is no promise of money/cash-out for login.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco04"></a>
## V1-ECO04. Implement atomic claim-state for campaign entitlement

**Type:** backend. **Source cards:** L03, O04, P03. **Position in dependency order:** 65.
**After:** [V1-ECO03](10-tasks.md#v1-eco03).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Tie the entitlement to the campaign and a proven provider subject namespace, not to a wallet/device.
2. Implement a single spent claim-state with an idempotent claim ID and domain separation.
3. Keep the eligibility adapter provider-neutral; concrete credential validators are wired up in O06.

**Verifiable scenarios:**

- A concurrent claim through two gateways/wallets yields one final issuance.
- Changing the owner secret, salt, or policy version does not create a repeat entitlement in the same campaign.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Replay/cross-domain restrictions are verified before connecting external providers.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco05"></a>
## V1-ECO05. Add immutable royalty accounting and pull payout

**Type:** backend. **Source cards:** L04. **Position in dependency order:** 66.
**After:** [V1-ECO03](10-tasks.md#v1-eco03), [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Define the exact fee base and rounding per the manifest; separate the operator reserve from the royalty.
2. Add a minimal splitter/vault to the existing contract package without administrative rights for the revenue recipient.
3. Account for each payout exactly once and allow the share to accumulate when the recipient reverts.

**Verifiable scenarios:**

- The sum of all shares and the remainder equals the inflow; a repeated withdrawal does not pay twice.
- A compromised recipient gets no mint/pause/upgrade/blacklist/committee authority and does not block other payments.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Role/call graph and conservation tests confirm the non-governing share; contract addresses/bytecode are included in the manifest.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco06"></a>
## V1-ECO06. Fix a sufficient proof of service for claims

**Type:** design. **Source cards:** L05, P05, D04. **Position in dependency order:** 67.
**After:** [V1-R05](05-tasks.md#v1-r05), [V1-ECO02](10-tasks.md#v1-eco02), [V1-ECO05](10-tasks.md#v1-eco05).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Separate the fact of a signed receipt, actual retrieval, and the claim of physical independence of copies.
2. Define the payable unit of original/index/copy/repair, the admissible claimant, and the finite reserve.
3. Define only formalizable reject/slash cases; a missed ping is not proof of fraud.

**Verifiable scenarios:**

- A single real delivery with repair decomposes into services and maximum payouts.
- Recipient/operator collusion does not create payouts beyond the funded pool; the residual laundering risk is measured.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** A verifiable claim contract is reconciled with real receipts and the resource model before settlement implementation.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco07"></a>
## V1-ECO07. Implement operator claims and settlement

**Type:** backend. **Source cards:** L05, P03, P05. **Position in dependency order:** 68.
**After:** [V1-ECO06](10-tasks.md#v1-eco06).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Accept bounded batch service proofs; verify operation/holder/epoch and the funded reserve.
2. Mark the claim spent atomically and pay out/accrue the admissible share with royalty taken into account.
3. Keep the retry/reorg/finality policy in the L2 adapter and report pending/finalized honestly.

**Verifiable scenarios:**

- The operator is paid for original storage and admissible repair; a repeated batch is not paid.
- Someone else's receipt, a changed service class, an expired claim, and a crash/reorg do not create a double payout/refund.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The full delivery→repair→settlement path passes on local EVM and the chosen testnet.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-eco08"></a>
## V1-ECO08. Verify economic attacks in a single scenario suite

**Type:** verification. **Source cards:** L01, L02, L03, L04, L05, L06, P06, X03. **Position in dependency order:** 69.
**After:** [V1-ECO07](10-tasks.md#v1-eco07), [V1-A06](03-tasks.md#v1-a06).

**Change boundary / entry points:** `contracts/src`, `contracts/test`, `crates/l2-adapter/src`, `crates/l2-types/src`, `crates/core/src/public_purchase.rs`, `crates/core/src/public_book.rs`, `crates/postage-spend/src`, `tools/risk-simulator`.

**Implementation plan:**

1. Assemble independent conservation/spent/claim oracles with concurrent spends, handover, subsidy drain, recipient revert, and reorg.
2. Check finite autonomy before/after a hard lease and stale retrieval during an L2 outage.
3. Save seeds, bytecode/config, all failed candidates, and the exact security assumptions.

**Verifiable scenarios:**

- No path increases the funded transport resource or the total payout.
- Several malicious RPCs do not bypass the trust model; quota exhaustion remains a bounded failure.

**Checks:** ECONOMY POSTAGE FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The targeted economic suite is green on a single revision; E19–E22 are repeated on the final RC.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
