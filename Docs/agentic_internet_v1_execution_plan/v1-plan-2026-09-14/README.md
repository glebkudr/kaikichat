# Detailed V1 completion plan — September 14, 2026

> **26.09.2026: superseded path removed** — custody/history, finalizers and
> checkpoint, private ZK postage stamps, the L2 adapter, agent orders, their
> scenarios and scripts (including A04/H11). Status: [V1_MAILBOX_SWARM_IMPLEMENTATION.md](../../V1_MAILBOX_SWARM_IMPLEMENTATION.md).
> `plan.json` is marked `superseded`.

> **25.09.2026: the 125-task plan replaced** by the contents of
> [V1_AGENT_FIRST_SCOPE_2026_09_25.md](../../V1_AGENT_FIRST_SCOPE_2026_09_25.md)
> (phases A and B, acceptance V1-AF01…AF08 and V1-GF01…GF02). The chapters below
> are history and a source of wording for the migrated tasks.

> **24.09.2026: storage and payment architecture replaced** by the decision in
> [V1_STORAGE_REDESIGN_2026_09_24.md](../../V1_STORAGE_REDESIGN_2026_09_24.md).
> Tasks relying on postage stamp placement, index rosters, the history graph and
> spending through finalizers are not executed in their previous form: chapters 01
> (H01–H11), 03 (A01–A04, A06), 05 (R01–R05), and the affected parts of 04, 08
> (G03), 10, 12 and 16. Their replanning and removal proceed together with the
> move to the new path.

**Mode: plan only, awaiting further instructions.** Implementation under this plan has not started; no agents have been launched. The user's latest instruction cancels the previous automatic launch of the sequential Luna implementation. This document is not permission to start work, publication or deployment.

An intermediate result is already saved in commit `993decee7b05b783fa53b423c2e584ed53e4ebad` — **start luna-mix pipeline**. The plan describes the remaining work from that state. The V1 boundary is unchanged: **67 source cards, 22 mandatory E2E, 11 suites, macOS ARM64 / Windows x64 / Linux x64**. All AR1–AR5 remain open.

## What has been prepared

**125 separate tasks**: 99 preparation/implementation/targeted verification tasks, 22 acceptance E2E and 4 general RC tasks, including a 24-hour process soak. Each has a concrete outcome, a file/module boundary, dependencies, steps, real positive/negative/fault scenarios and a completion criterion. This is a decomposition of the source backlog, not a new expanded scope. New IDs start with `V1-`; the source 67 cards keep their IDs F01/I01/…; in step descriptions a short H01 means V1-H01, and the "Source cards" field always refers to the old backlog.

- [Map of all 67 cards, 22 E2E and 24 review findings](COVERAGE.md).
- [Execution, verification and evidence rules](RUNBOOK.md).
- [Prerequisites, decisions and risks](PREREQUISITES.md).
- [Machine-readable plan, dependencies and order](plan.json).
- [Plan structure validation](validation.json). It does not verify the product.
- [Map of algorithmic risks and bibliography](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Each chapter has an "Algorithmic reference points" section with a minimal per-task reading list; hard spots without a ready canonical solution are marked there as our own constructions.

## What we continue from

Transactional Core/SQLCipher, real MLS, shared broker/grants for UI/CLI/MCP, public funded stamps, BFT spent history/handover, paid data/index/locations, signed history graph, the native macOS path and independent component checks already exist. They must not be planned as entirely missing or considered fully accepted user capabilities.

At the baseline:

- Native20 R10 passed the previous native publication/loss/SQL/full/cold gates. Full130 R14 publishes 130 originals but imported only 1–30 before earliest expiry; the full and cold result is not accepted. R13 is a separate unexamined publication failure.
- Shared history collection/publisher, placement priority, arrival fences and handling of the finished reply before releasing Work are implemented with component evidence. Their effect on full native recovery still needs to be measured.
- The Core durable range API is implemented: bodies and the holder-local cursor are saved atomically for an exact head; under partial capacity the cursor does not advance. The ordinary Node still uses the old cache path.
- Six new Node range runtime tests are included in the commit: compilation passed, behavioral RED/GREEN is not there yet, the latest critic is REVISE. The missing distinguishing next_sequence/complete cases make up H01.
- The generic failure→legacy fallback has been removed. A single pending-body store, full proof continuation and epoch-aware gap/rejoin are still ahead.
- Native automation has already been moved to the temporary E2eFileStore. Continuation keeps the ban on login Keychain prompts; the regular production keystore is not replaced with a test one.

Current source evidence: [IMPLEMENTATION_STATUS](../../../IMPLEMENTATION_STATUS.md), [capability matrix](../CAPABILITY_EVIDENCE.md), [R14 contract](../../V1_HISTORY_LIFECYCLE_R14.md). Old large regression counts refer to their own revisions and are not the current overall acceptance.

## Work order

First C01–C05 fix the executable contract, parameters and a fast diagnostic loop for runs. The first production block is H01–H11: close the test REVISE, connect paid range to Core, persist graph progress, merge pending bodies, then Diagnostic32 and the unchanged Full130. After that come profile/storage, authority lifecycle and first offline Welcome; then repair/attachments, devices/backup, groups, shared agent adapters, economy/providers, network/desktop/platform and a single release acceptance.

The table is the reading order and main outcomes. The exact order with cross-block dependencies is recorded in `plan.json → execution_order`; it is topologically sorted. For example, an individual network/credential task may be ready before the whole neighboring block closes. The source DAG of 67 cards remains an additional condition of the final GREEN.

| Block | Tasks | Outcome |
|---|---:|---|
| [00. Execution contract and reproducible checks](00-tasks.md) | 5 | Real commands, resource boundaries and the evidence format are defined before any code change. |
| [01. R14: resumable reading and a single pending-body store](01-tasks.md) | 11 | The ordinary paid receiver survives Work changes and cold restart, then passes the unchanged Diagnostic32 and Full130. |
| [02. Storage and the cost of a long lifetime](02-tasks.md) | 4 | History growth does not rewrite the whole profile and does not break compatibility after an upgrade. |
| [03. Sustained payment and authority lifecycle](03-tasks.md) | 6 | The chat continues paid operations after epoch changes, authority updates and a long L2 outage. |
| [04. First contact with an offline recipient](04-tasks.md) | 4 | The first Welcome and subsequent application messages are delivered without the sender being present. |
| [05. Autonomous recovery of ten replicas](05-tasks.md) | 5 | Operators restore data and indexes 10→7→10 with clients shut down and a finite budget. |
| [06. Attachments, TTL and garbage collection](06-tasks.md) | 5 | Attachments transfer in chunks, resume and are deleted per verifiable retention without plaintext leakage. |
| [07. Devices, delegation and safe backup](07-tasks.md) | 6 | Profile restoration and device change do not restore revoked rights and do not clone active ratchets. |
| [08. Groups and epoch-aware recovery](08-tasks.md) | 10 | Real groups have roles, consistent encrypted control, safe rejoin and two paid privacy profiles. |
| [09. Shared client API and agent operation](09-tasks.md) | 5 | UI, shipped CLI and MCP show the same data and check the current limited rights. |
| [10. Testnet transport economy](10-tasks.md) | 8 | Postage stamps, subsidy, royalty and payouts are backed by resource and give no control rights to fee recipients. |
| [11. Voluntary external account verification](11-tasks.md) | 10 | Google, Telegram and site/org credentials yield only explicitly accepted claims; funded onboarding is limited by the campaign. |
| [12. Network and operator mode](12-tasks.md) | 5 | A clean client joins an independent network, communicates behind NAT and respects resource boundaries under attacks. |
| [13. Everyday desktop](13-tasks.md) | 5 | The user can use chat, search, notifications and recovery through the regular interface. |
| [14. Local IPC and delivery on three OSes](14-tasks.md) | 6 | A single Rust runtime, CLI/MCP and Tauri work on macOS ARM64, Windows x64 and Linux x64 with private local IPC. |
| [15. Protocol compatibility and independent release preparation](15-tasks.md) | 4 | Before RC there is a full wire corpus, an approved testnet manifest and a genuinely runnable release runner. |
| [16. Full acceptance of a single revision](16-tasks.md) | 26 | All 67 cards, 22 E2E, 11 suites and three OSes are accepted on a single RC; missing results block the release. |

## What counts as an atomic task

One bounded observable transition: for example, moving a verified range position into Core, a pending-body migration, issuing a replacement receipt or verifying an owner-bound handoff. Such a task may touch several neighboring files on one path, but it does not close a whole "groups" or "economy" module on a single smoke.

A backend task goes through sequential internal stages: detailed contract/tests → reproducible RED → an independent final ACCEPT of the tests → production → targeted backend/frontend GREEN → evidence. If test changes require substantial additional behavior, the task is split before production starts and its dependencies/coverage are updated. A future agent is given one current stage with a narrow file boundary; tests+production must not be issued bypassing the critic.

For distant tasks the exact new symbol/filename is determined after reading the current code when they start. The cards list verified existing entry points; the old proposed `crates/group-domain`/`crates/backup-restore` from the source package are not declared existing and do not require creating separate crates.

## V1 completion conditions

RC02 accepts all 67 cards in the active scope and with their source dependencies; all 22 E2E and 11 suites are executed with non-zero results and no required skips; native installers and production smoke pass on three OSes. Sources, locks, config/genesis, contracts, oracle and artifacts match a single RC. Failed/rejected attempts are kept; the scope is not cut to get a green report.

The result is **V1 code/testnet**. Mainnet transport with real funds requires a separate financial approval. There is no implicit transfer of Google/Telegram/site, subsidy/royalty/claims, groups, backup or platforms into the preview. Jobs, rating and reviews have already been moved by the user to V2 and stay there.

Calendar estimates are not invented: the bottlenecks are the cost of Full130, the repair/settlement resource model, real group control integration, provider registrations, independent operators and native runners. Measurable checkpoints and blocking conditions are listed in the cards and PREREQUISITES.
