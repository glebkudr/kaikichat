# 06. Attachments, TTL, and garbage collection

Attachments are transferred in chunks, resumed, and deleted under verifiable retention without leaking plaintext.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R03, AR-R04, AR-R14, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-B01** (chunked manifest contract): content-addressed manifest + Merkle tree; LBFS content-defined chunking; Venti; BitTorrent v2 (BEP-52 hash trees); defensive early-reject parsing.
- **V1-B02/V1-B03** (resumable chunks): per-chunk verify + final manifest hash (the BitTorrent model); Kafka KIP-98 idempotency; transactional outbox.
- **V1-B04** (GC vs leases/retention): distributed GC survey; Cassandra tombstones/`gc_grace`; lease-aware GC — see research §5.

<a id="v1-b01"></a>
## V1-B01. Define the encrypted chunk/manifest contract

**Type:** tests. **Source cards:** D01, D06, F02. **Position in dependency order:** 36.
**After:** [V1-W03](04-tasks.md#v1-w03), [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/node/src/paid_custody.rs`, `crates/postage-spend/src/custody`, `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/desktop-api.ts`.

**Implementation plan:**

1. Define a bounded manifest with hashes, counts, sizes, encryption context, and expiry.
2. Bind the manifest to the message, owner/grant, and the paid class; separate read and repair capabilities.
3. Add vectors for a valid attachment and reject cases before production; take the specific fields/limits from the shared wire/resource manifest.

**Verifiable scenarios:**

- The assembled chunks reproduce the original file byte-for-byte.
- Tampering with order/hash/size, a foreign context, and an oversized manifest are rejected before expensive decode.

**Checks:** BLOB CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** A single versioned format, a clear price, and bounded memory; no new unbounded blob service.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-b02"></a>
## V1-B02. Implement resumable chunk sending

**Type:** backend. **Source cards:** D06, D01, P05. **Position in dependency order:** 37.
**After:** [V1-B01](06-tasks.md#v1-b01).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/node/src/paid_custody.rs`, `crates/postage-spend/src/custody`, `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/desktop-api.ts`.

**Implementation plan:**

1. Encrypt/read the file as a stream inside the trusted backend; the renderer gets progress without keys.
2. Persist manifest/chunk upload progress in a transactional outbox and place it through the shared paid store.
3. Retry only the missing chunks, preserving operation identity and budget.

**Verifiable scenarios:**

- A restart after some receipts continues from the right chunk without a second payment.
- Cancellation/a foreign grant/full disk do not publish a completed attachment and leave no plaintext temp.

**Checks:** BLOB CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Bounded placement of a large file works on cold retry and does not load it entirely into memory.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-b03"></a>
## V1-B03. Implement download/resume with final verification

**Type:** backend. **Source cards:** D06, U05. **Position in dependency order:** 38.
**After:** [V1-B02](06-tasks.md#v1-b02).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/node/src/paid_custody.rs`, `crates/postage-spend/src/custody`, `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/desktop-api.ts`.

**Implementation plan:**

1. Download the missing permitted chunks under the bounded manifest with shared fetch/admission.
2. Persist encrypted partial progress; before delivering the file, verify all hashes and the final size.
3. Pass the renderer a safe user-selected file handle/path; do not execute content or an active preview.

**Verifiable scenarios:**

- Network/relay loss and cold restart continue the exact parts, and the final hash matches.
- A single wrong chunk does not become a ready attachment; traversal/an unsafe name does not write outside the chosen destination.

**Checks:** BLOB CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The UI gets truthful progress/retry/error; completed means verified bytes.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-b04"></a>
## V1-B04. Tie retention/GC to live leases and repair

**Type:** backend. **Source cards:** D06, D04, P05. **Position in dependency order:** 39.
**After:** [V1-B03](06-tasks.md#v1-b03), [V1-R04](05-tasks.md#v1-r04).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/node/src/paid_custody.rs`, `crates/postage-spend/src/custody`, `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/desktop-api.ts`.

**Implementation plan:**

1. Reuse the shared GC scheduler for chunk/manifest/proof rows.
2. Hold ciphertext until the paid expiry and live references; verify repair rights separately from read.
3. On quota/disk-full, choose refusing new admission over destroying valid paid data.

**Verifiable scenarios:**

- Before TTL, an attachment is recoverable after holder loss; after TTL, retrieval is no longer promised.
- A kill during GC and a manifest shared by several recipients do not delete a still-needed chunk/proof.

**Checks:** BLOB CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Retained usage and actual bytes are consistent after a cold restart; the absence of a resource is reflected in the status.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-b05"></a>
## V1-B05. Add user-facing file sending and saving

**Type:** frontend. **Source cards:** D06, U05. **Position in dependency order:** 40.
**After:** [V1-B04](06-tasks.md#v1-b04).

**Change boundary / entry points:** `crates/protocol-types/src/lib.rs`, `crates/crypto/src/lib.rs`, `crates/core/src/lib.rs`, `crates/node/src/paid_custody.rs`, `crates/postage-spend/src/custody`, `apps/desktop/src/ChatShell.tsx`, `apps/desktop/src/desktop-api.ts`.

**Implementation plan:**

1. Integrate a file picker, upload/download progress, cancel/retry, and save into the current ChatShell.
2. Price/TTL are confirmed by the same wallet/budget flow; do not duplicate backend eligibility.
3. Check keyboard/accessibility, malicious filename/preview, and visually inspect the saved headless screenshots.

**Verifiable scenarios:**

- The user sends a file, restarts the app, and resumes the download.
- A storage/permission error stays on the specific attachment and does not break text chat.

**Checks:** BLOB CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** UI and CLI use one attachment service; component and native checks confirm the final hash.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
