# 15. Protocol compatibility and independent release preparation

Before the RC there is a full wire corpus, an approved testnet manifest, and a real executable release runner.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R07, AR-R16, AR-R21, AR-R23, AR-R24.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-V01** (wire-compat corpus): the conformance/golden-vector approach; MLS wire formats (RFC 9420 §7 TLS presentation language); EIP-712 domain separation as the benchmark for pinning domains.

<a id="v1-v01"></a>
## V1-V01. Assemble the full V1 wire/crypto compatibility corpus

**Type:** backend. **Source cards:** F02, F06. **Position in dependency order:** 96.
**After:** [V1-G10](08-tasks.md#v1-g10), [V1-O10](11-tasks.md#v1-o10), [V1-M05](09-tasks.md#v1-m05), [V1-B05](06-tasks.md#v1-b05), [V1-A05](03-tasks.md#v1-a05).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `fixtures`, `spec`, `tests`, `scripts`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Consolidate identity/grant/message/control/history/attachment/credential/public-postage/current-and-legacy vectors within the V1 scope only.
2. Verify golden bytes/hashes/signatures with a second codec/oracle or upstream vectors; encode→decode of one's own code is not enough.
3. Fix the unknown critical/noncritical extension policy and distinct domains/epochs.

**Verifiable scenarios:**

- Two compatible client versions read the same V1 objects on three OSes.
- Unknown critical, domain substitution, and an altered signature are rejected; an unsupported adapter is not called paid/verified.

**Checks:** CONFORMANCE DOC FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** Every V1 wire family has independent positive/negative vectors and a version policy.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-v02"></a>
## V1-V02. Pin down the independent testnet and external prerequisites

**Type:** verification. **Source cards:** F05, N02, L06, X06. **Position in dependency order:** 97.
**After:** [V1-N03](12-tasks.md#v1-n03), [V1-ECO08](10-tasks.md#v1-eco08), [V1-O10](11-tasks.md#v1-o10).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `fixtures`, `spec`, `tests`, `scripts`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Record chain/genesis/bytecode, the checkpoint profile, admissible operators/attestors, seeds/relays, and failure domains.
2. Separate the counts of keys, processes, machines, organizations, and trust administrators.
3. Check replacements for company sources and the absence of a mandatory revenue/gateway/CDN for an already working network.

**Verifiable scenarios:**

- An independent operator reproduces join/serve/retrieve with the published configuration.
- Several processes of the project owner do not count as an independent quorum; an unreachable live resource is marked blocked.

**Checks:** CONFORMANCE DOC FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The testnet manifest and accesses are ready for the full RC; admitting mainnet money is not implied.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** Independent operators, testnet funding, provider registrations, native runners, and signing setup.

<a id="v1-v03"></a>
## V1-V03. Assemble the full runner over 11 suites and 22 cases

**Type:** tooling. **Source cards:** F01, F03, F06, X06. **Position in dependency order:** 98.
**After:** [V1-C01](00-tasks.md#v1-c01), [V1-C02](00-tasks.md#v1-c02), [V1-C03](00-tasks.md#v1-c03), [V1-V01](15-tasks.md#v1-v01), [V1-P06](14-tasks.md#v1-p06), [V1-V02](15-tasks.md#v1-v02).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `fixtures`, `spec`, `tests`, `scripts`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Use the existing check scripts and scenario adapters; add the missing manifest/aggregation without a parallel test system.
2. Verify exact revision/lock/config/binary/oracle hashes, nonzero counts, and the absence of missing/skipped-required.
3. Run the full suite only after the functional tasks are complete; do not treat cargo xtask commands from the old plan as existing.

**Verifiable scenarios:**

- A fake/stale/missing PASS report does not close the RC.
- Each required case is actually selected, a child failure is not lost, and cleanup is preserved on stop.

**Checks:** CONFORMANCE DOC FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is one exact final-run command/manifest, separate native/production/live results, and a list of platform gates.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-v04"></a>
## V1-V04. Freeze the release candidate and verify launch readiness

**Type:** verification. **Source cards:** F06, X06. **Position in dependency order:** 99.
**After:** [V1-V03](15-tasks.md#v1-v03).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `fixtures`, `spec`, `tests`, `scripts`, `Docs/agentic_internet_v1_execution_plan`.

**Implementation plan:**

1. Pin the commit, clean source set, locks, schema/wire versions, bytecode/genesis/config, and the hashes of the three install artifacts.
2. Check for accepted backend test reviews for new features and all external prerequisites.
3. Produce an immutable run manifest for the next 22 cases; forbid source/config edits during the run.

**Verifiable scenarios:**

- A mismatch of any required artifact or an unready OS stops the corresponding gate with an explicit status.
- The historical Full130/871-workspace pass does not carry over automatically to the new RC.

**Checks:** CONFORMANCE DOC FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The RC is prepared for overall acceptance; product_validated stays false until all gates complete.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
