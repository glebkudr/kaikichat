# Validation of the prepared plan

September 5, 2026. Validation scope: the documents, implementation map and utilities of the original planning package. **The messenger product is not implemented and not validated.**

## Actual state

| Item | Result |
|---|---|
| Working folder | Contained `Docs/`; Git repository, Cargo workspace and frontend package were absent |
| Source ideation | 27 entries; all user messages and the architectural parts of the replies were read |
| Source plan | 84 planned tasks, 52 requirements; dependency DAG retained |
| Execution plan | 13 modules, 7 integration milestones C0–C6, 26 planned E2E |
| Tests-first | In every module tests are described before implementation; a separate backend-test-critic and waiting for its result are included in the future workflow |
| Structure and mapping | All 84 task IDs are included exactly once; all 52 requirement IDs are covered by the map; E2E/module back-references are consistent |
| Product tests | 0 executed; backend/frontend/E2E remain `not_run` |
| Documentation checks | 40/40 passed |
| Monetary/provider/platform gates | Planned, not executed |

Existing archive SHAs from the old metadata were not re-verified: the folder contains unpacked materials. [execution-map.json](execution-map.json) holds SHA-256 of the actually read `messages.json`, `backlog.json`, `requirements.json` and the amendments to the sources.

## Source package portability fix

The first run of the existing suite finished with 38 passing checks out of 39. `test_task_files_are_exact` saw 84 service files `._<ID>.md` in `tasks/`. The checked files have AppleDouble magic `00051607`; these are macOS metadata, not additional tasks.

Before the fix, `test_manifest_ignores_appledouble_without_hiding_real_files` was added. It created temporary `F01.md`, `UNEXPECTED.md` and `._F01.md` and reproduced RED: the manifest included the service file. Keeping `UNEXPECTED.md` in the expected inventory preserves detection of genuinely extraneous documents.

`build_packet.py` gained a shared `is_packet_source`, reused by the manifest and the card-list check. No one's documents or OS metadata were deleted. The source package README and final log were updated; the generator recomputed the manifest. Checks of card content, requirements and the dependency graph are preserved.

The new regression test concerns the documentation build utility. This is a fix of a found defect, not a new messenger backend module.

## Commands and evidence

From the `Docs/agentic_internet_v1_1_plan` directory:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v test_validate_plan test_revision_contract test_packet_consistency
PYTHONDONTWRITEBYTECODE=1 python3 validate_plan.py
PYTHONDONTWRITEBYTECODE=1 python3 build_packet.py
```

All 40 unittest checks passed, the validator returned `valid_plan_structure=true`, the generator assembled 84 cards and the manifest. After the build, hashes of all files in the manifest and the absence of AppleDouble sidecars in it were verified separately.

- [Actual log of the 40 tests](evidence/documentation-tests.txt).
- [Source validator result](evidence/source-plan-validation.json).
- [Validation of the new plan map](planning-validation.json).

Additionally verified: local Markdown links, pairing of code fences, E01–E26 agreement between the document and JSON, presence of tests-first before implementation in all 13 sections, and preservation of the source integration frontiers. This validation establishes structural consistency, not correctness of the not-yet-implemented distributed protocol.

Technical underpinnings were cross-checked against primary sources: Tauri capabilities/WebDriver, MCP specification/Rust SDK, libp2p, OpenMLS/MLS architecture, Commonware Simplex, Google native OAuth/OIDC, Telegram Login, rusqlite, A2A and CBOR. Links sit next to the corresponding decisions in the main plan. Documentation review of an external component does not count as its integration test.

## Readiness boundary

The current request is treated as plan preparation. The working boundary keeps the transport economics of the original 1.1 package as a separate V1 milestone and leaves the complex work economics for V2+. This is an explicitly stated assumption pending scope clarification by the user.

The implementation criterion "all tests and E2E pass" is **not yet met**. Its verifiable definition is in [TEST_AND_E2E_PLAN.md](TEST_AND_E2E_PLAN.md); completion happens at C6, not at green tests of this document.
