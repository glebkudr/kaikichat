# Agentic Internet · V1 implementation packet · revision 1.1

**The effective release boundary was changed by the user on September 9, 2026:** orders, rating, and reviews were moved to **V2**. [Decision and full list of amendments](../V1_SCOPE_2026_09_09.md), [current phases/dependencies/acceptance](../agentic_internet_v1_execution_plan/release-scope.json). The original revision 1.1 packet is preserved below; its older jobs/reviews commitments and the historical validator do not override this decision. For V1, 67 active cards and 22 E2E in the refined scope apply.

**September 5, 2026.** Updated plan per the user's remarks: Tauri 2, Google/Telegram/organization attestation, a basic order protocol with public reviews and rating. Complex economic agents are V2+. Transport economics, groups, and autonomous repair remain V1.

## Documents

[Changes 1.0 → 1.1](CHANGELOG_V1_1.md) show the accepted edits and what is preserved. The [main plan](V1_IMPLEMENTATION_PLAN_RU.md) describes the architecture and the release. [Authorization and attestation](AUTH_AND_ATTESTATION_V1.md) and [public reviews/rating](PUBLIC_REVIEWS_PROTOCOL_V1.md) are worked out separately.

[Backlog](BACKLOG.md) contains **84 atomic packets**; the cards are in `tasks/`. The [matrix](REQUIREMENTS.md) links **52 requirements** to the original Uxx/Axx and the new AMENDMENT-01…05. Previous IDs are not removed. A04 is now `V1-contract-only`, and R23 is `contract-now/future-implementation`: the autonomous economic engine is not a V1 dependency.

[Decisions and gates](DECISIONS_AND_VERIFY_GATES.md), the [test-first order](AGENT_WORK_ORDER.md), [DAG](DAG.md), and [sources](SOURCE_MAP.md) are aligned with the new boundary. The [user amendment](SOURCE_AMENDMENT_2026_09_05.md) records the provenance of the changes. The [full document](V1_FULL_PLAN_RU.md) combines all materials and all cards.

## Machine-readable sources

`backlog.json` — tasks, dependencies, delivery phase, authority, tests-first, negative/fault/acceptance, and Definition of Done. `requirements.json` — requirements, scope, source IDs, and back-references. `metadata.json` — version, SHA-256 of the original and of the previous plan archive, counts/DAG, and the mandatory revision contract.

The status of all product tasks is `planned`. The test ID names in the cards are a contract for the future implementation, not already executed messenger tests.

## Validation and reproducible document build

Python 3.10+, standard library; no internet or third-party packages needed.

```bash
python validate_plan.py --output plan_validation.json
python build_packet.py
python -m unittest -v test_validate_plan test_revision_contract test_packet_consistency
```

When updating JSON, the independent test contracts are edited first, then the data and the aligned architecture documents; `build_packet.py` regenerates the cards, indexes, the full document, and the manifest. `DAG.md` is a human-readable overview; the exact computed graph is verified in the JSON metadata.

`REVISION_VALIDATOR_RED.txt`: 17 new revision-contract tests failed on the 1.0 plan before the update. `PLAN_VALIDATOR_GREEN.txt`: **40 checks** of structure/new guardrails/document consistency and portability pass after fixing the handling of macOS AppleDouble files. The original revision contained 39 checks; the added regression test first reproduced the bug. `plan_validation.json` confirms 84 tasks, 52 requirements, no cycles or broken links, and presence of planned coverage.

After changing any files or updating the logs, run `python build_packet.py` to recompute `FILE_MANIFEST.json`. The manifest does not hash itself and excludes `__pycache__`, `.pyc`, and service AppleDouble files `._*`. The check of the exact card set uses the same filter; extra meaningful `.md` files are still detected.

The applied order of Tauri/Rust implementation, module checks, and E2E are described in the [V1 execution plan](../agentic_internet_v1_execution_plan/README.md). Product tasks in this packet remain `planned`.

Clarification of September 8, 2026: in [section 10 of the main plan](V1_IMPLEMENTATION_PLAN_RU.md#10-cli-skill-mcp-api-and-operation-of-disabled-agents), R02/R17 and M01/M02/M06/U06/X05 explicitly secure the delivery of **client + skill with CLI**: access handle → own ID → message to another ID → inbox via polling/optional hook; transport via libp2p behind NAT. Acceptance E11 is performed together with the NAT scenario E03.

**These checks are not messenger tests, a cryptographic audit, real OAuth verification, or confirmation of being mainnet-ready.** There is no product code in this packet. Release gates remain planned until actually executed.

## Handoff to the agent

`AGENT_WORK_ORDER.md`, the selected card, its Rxx, and upstream contracts are handed over. For auth/reviews, the corresponding specialized specifications are mandatory. The critic checks completeness against the original request and the amendment: it does not remove complex V1 features and does not bring explicitly deferred economic engines back into V1.

## What an acceptable organizational issuer means

A single site/organization issuer may only issue voluntarily accepted scoped credentials. Its unavailability blocks new login/attestation on that branch, but not independent transport, keys, already issued postage stamps, or reviews. A specific state identity system is not considered integrated until it is named and its real adapter is verified.
