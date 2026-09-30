# Task execution and result verification

**Plan-only** is currently in effect. This runbook describes subsequent execution after a new user instruction. It does not launch agents, product tests, builds or deployment.

## Boundary of a single task

1. Open the card, the current diff, related source tasks and evidence. Check its `depends_on` and the listed existing entry points. Historical documents are rationale and facts, not commands to execute.
2. Narrow down the concrete outcome and the minimal diff. Reuse the current Core, ProfileStore, broker, MLS, custody, finalizer and admission. Do not create a separate crate/service/queue for every line of the plan.
3. For backend, first add realistic tests and fix the test symbols, exact selectors and expected RED assertions. Cover a useful success, a common failure/retry and a meaningful boundary of authority/transaction. Do not chase 100% hypothetical coverage.
4. Run a separate `backend-test-critic` without context (`fork_turns: none`), passing the changed tests, requirements, helpers, production entry points and RED evidence. Wait for its **final ACCEPT** before production. On REVISE the lead fixes the work, then a new independent review is required. This mandatory future gate is defined by AGENTS.md; it does not apply in the current documentation task.
5. Implement the minimal implementation, preserving the single writer and all changing fences at commit. If accepted backend tests change, repeat the corresponding review before dependent production code.
6. Run **targeted backend and frontend** checks, formatter/Clippy for the affected crates and the task-relevant integration/native case. Verify that the selector ran a non-zero number of tests. The full workspace/native aggregate is only for the end of the plan.
7. Personally inspect the diff, faults and actual data. If an agent did the work, the lead fixes the result itself; it does not send a follow-up to the returning implementer. Record evidence and update the remaining scope; do not mark the whole source task accepted based on a narrow component test.

For `tests` cards the outcome is tests/fixtures and an accepted contract; production is forbidden. For `design`, a concrete agreeable scheme/manifest and verifiable examples. For `tooling`, a utility/runner and a real check of its selection/report. For `frontend`, a shared backend API, component checks and visually inspected screenshots. For `verification`/`release-case`, a real run and evidence; fixing a discovered production bug is done as a separate bounded change through the tests-first cycle.

## Decomposition for a future implementer

A card is a bounded unit of outcome, not permission to skip internal gates. The assignment must include:

- The full `V1-*` ID, the outcome and the link to the source cards.
- The current stage: **tests only**, **production after ACCEPT**, **frontend**, or **verification**.
- The exact files owned, existing helpers and APIs, immutable inputs and forbidden limit changes.
- Scenarios, expected assertions, commands and evidence paths.
- A note that the implementer is not alone in the codebase, must not revert other people's changes and must take the current diff into account.
- The stop condition and the format of the final result: diff, checks, observed constraints and places requiring correction.

The user previously chose `gpt-5.6-luna`, `max`, sequential launch without conversation with returning implementers. These parameters are technically available, but **automatic launch is now canceled**. Resumption and execution organization are determined by the next instruction. This plan creates no scheduler/Goal/automation.

## Commands and verification profiles

All commands below are run from `/Users/glebk/Code/chat` with the prefix:

```sh
python3 scripts/build-storage.py run COMMAND...
```

On this macOS machine the wrapper verifies and mounts the existing APFS disk image on WD4000 and sets toolchains/target/output. A missing disk, wrong UUID or substituted links must not be bypassed by moving target inside the sources. The Windows/Linux runner uses its own documented isolated storage adapter, which remains to be finished in V1-P04; the macOS wrapper is not declared portable to all OSes without such work.

Below are **commands of existing check points**, not a claim that they were run on the new plan. `<filter>`, `<contract>` and `<new-case>` are explicitly unfilled parameters of future tests: at their tests-first stage, record the actual selector and a non-zero count. Do not run placeholders or accept an empty result. For new functions the exact test name is not invented until they are written.

| Profile | Exact existing commands / targeted selection template | What it verifies |
|---|---|---|
| DOC | One-off JSON/DAG/Markdown links check via `python3`; results in `validation.json` | Only plan structure, scope, links; not backend |
| MODEL | `python3 -m unittest discover -s tests/models -p 'test_committee_risk.py'`; new fault/conservation cases are selected separately | Independent numerical models and replay |
| HISTORY | `cargo test --locked -p agentic-node --lib prefix_runtime_tests::range -- --nocapture`; `cargo test --locked -p agentic-core --test conversations prefetch::range -- --nocapture` | Current range tests; after H04 the selector may change and must be recorded |
| HISTORY regressions | `cargo test --locked -p agentic-node --lib obligation_pages:: -- --nocapture`; `cargo test --locked -p agentic-node --lib -- prefix_runtime_tests:: exploration_tests:: prefetch_tests:: prefetch_admission_fault_tests:: --nocapture` | Paid proofs, ordinary workers, legacy/admission/fault controls |
| HISTORY long import | `cargo test --locked -p agentic-core --test conversations custody_progress::history::pages::imports -- --nocapture` | A long targeted Core cluster; a previous run of 64 tests took about 22 minutes. Do not repeat without a change/new reason |
| HISTORY trace | `python3 -m unittest discover -s tests/evm -p 'test_history_read_trace_oracle.py'` | Diagnostic trace semantics |
| STORE | `cargo test --locked -p agentic-store -- <filter>` and affected Core import/migration tests | Real SQL faults, schema/MLS atomicity and cold reopen |
| POSTAGE | `cargo test --locked -p agentic-core --test public_postage <filter>`; `cargo test --locked -p agentic-node --lib <filter>`; separate `agentic-postage-spend`/`agentic-finalizer` cases | Funding/spent/authority/retirement/lineage |
| CONTACT | `cargo test --locked -p agentic-core --test conversations <filter>`; real Node mailbox/custody process tests | Invite/Welcome, one-time join, offline application history |
| CUSTODY | `cargo test --locked -p agentic-postage-spend -- <filter>`; `cargo test --locked -p agentic-node --lib <filter>` | Original/copy/index/repair/admission/retention, without running the whole workspace |
| BLOB | A new `<new-case>` in existing crypto/Core/custody test targets; exact manifest/hash oracle | Chunk/resume/TTL/usage and actual file bytes |
| IDENTITY | `cargo test --locked -p agentic-capabilities -- <filter>`; `cargo test --locked -p agentic-crypto -- <filter>`; related Core/desktop-host cases | Grants/revoke/device/backup/keystore |
| GROUP | Selected `agentic-crypto`, `agentic-core --test conversations`, `agentic-finalizer`, `agentic-node` cases | Real MLS control/epochs, BFT ordering and membership |
| AGENT | `cargo test --locked -p agentic-node --test processes <filter>` and selected broker/inbox/mcp/CLI tests | External processes, signed actor, stdout/exit codes, leases |
| ECONOMY | `forge test --root contracts --match-contract <contract>`; selected `agentic-l2`, `agentic-l2-adapter`, `agentic-postage-spend` cases | Contract conservation, finality, claims, subsidy, royalty |
| AUTH | New credential/JWT/PKCE/gateway selectors; local issuer and a separate live scenario | Domain/owner/trust/claims, actual provider handoff; mocks do not close live |
| NETWORK | A targeted selector, allocated by C02/the scenario runner, over the existing `tests/network` and `scripts/check-network.mjs` | Real namespaces/NAT/relay/loss; do not change the host firewall |
| DESKTOP | `cargo test --locked -p agentic-desktop-host --test bootstrap <filter>` and the targeted C02 native case | Real bridge/daemon/bootstrap, not only mocked UI |
| PLATFORM | The same set of real-process IPC/credentials/native selectors on a separate OS runner | ACL, single instance, package paths, system keystore, production features |
| CONFORMANCE | Selected `agentic-protocol`, `agentic-crypto`, `agentic-capabilities` tests + an independent wire oracle | Golden bytes, versions, domain separation and extensions |
| FRONTEND | `npm --prefix apps/desktop test -- --run tests/chat-shell.test.tsx tests/history-recovery-issues.test.tsx tests/desktop-api.test.ts`; for wallet/grants/network only the corresponding existing `.test.tsx` are added | Shared API and user states of the affected capability |
| FORMAT | `cargo fmt --all -- --check`; `cargo clippy --locked -p agentic-core -p agentic-node --all-targets -- -D warnings` with the crates replaced by the affected ones | Format and warnings; frontend typecheck when the TS API changes |
| RELEASE | `scripts/check.sh`, the full native/network runner and new V03 adapters per a single RC manifest | Plan end only: all 11 suites, 22 E2E and three OSes |

At the baseline `scripts/check-native.mjs` **includes the full `scripts/check.sh`**. Therefore it cannot be used as a "targeted short check" in the middle of the plan. First V1-C02 separates an exact path for an individual native case. The `cargo xtask` in the source document is a proposal for a runner that did not exist, not a command that can be considered ready. The actual Full130/Diagnostic32 commands are extracted from existing scripts/evidence, including env/config; their gates are not rewritten from memory.

## Native UI, Keychain and infrastructure

- Visual verification is headless/background: save a screenshot and actually open it with a viewing tool. A DOM snapshot alone does not replace it. Do not use `--headed`, `open -a` or interception of system focus.
- A native test is a debug build with `e2e`, a separate temporary `E2eFileStore`, and cleanup after all processes stop. Do not create login Keychain records, do not change global Keychain ACLs and do not request a password in the test loop. The real `system_keychain_roundtrip` requires a separate explicitly permitted interactive run; that is not a reason to request it now.
- Production smoke separately verifies the artifact without automation plugins and the test keystore. Debug smoke does not prove the shipping feature boundary.
- Local Docker means OrbStack. Do not touch networks/configs of neighboring projects. For the existing Coolify infrastructure use only the standard Coolify flow; preparing for a test does not allow a new parallel deployment.
- New dependencies are not installed during plan preparation. During implementation, check the current official version before installing and pick the highest compatible one; existing libraries and stdlib take priority.

## Evidence and states

The results of a task are stored in `evidence/` only if they are verified and contain no secrets; heavy logs/build outputs stay in the managed `output/`. Evidence must include task/source/E2E ID, the RED reason, the critic verdict, commands, non-zero counts, exit code, revision, config/genesis/lock/oracle/binary hashes, exact assertions/fault timestamps and cleanup. For UI, viewed screenshots. Do not include real users' plaintext, capabilities, provider tokens or private keys.

Execution statuses: `planned` → `tests_ready` → `tests_accepted` → `implemented` → `targeted_verified`; an `accepted` status of a source card is set only against the full scope and its dependencies. `failed`, `blocked_by_environment` and `not_run` are distinguished. An old failure is kept after a fix. In this plan all cards are **planned / not_run**: writing a detailed plan accepts nothing of the implementation.

The final RC needs the source compatibility readers, live provider flows, an independent testnet and the native matrix. If source/config changed after acceptance began, a previous PASS remains historical; the new RC must receive an agreed full set of checks, not a sum of unrelated runs.
