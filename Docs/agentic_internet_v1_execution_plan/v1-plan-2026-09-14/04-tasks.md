# 04. First contact with an offline recipient

The first Welcome and subsequent application messages are delivered without the sender's presence.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R05, AR-R22.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing with no algorithmic novelty.

- **V1-W01–V1-W03** (invite/prekey/Welcome, out-of-order bodies before group state): RFC 9420 §10 (Welcome), §12 (KeyPackage/last_resort); RFC 9750 (architecture, buffering); X3DH — one-time prekey semantics; Alpenhorn (OSDI 2016) — invite without metadata leakage; Tor prop224 — bounded lookup for mailbox pointers.
- **V1-W04**: the same references + deterministic-fault scenarios (research §7).

<a id="v1-w01"></a>
## V1-W01. Specify the one-time private invite and control admission

**Type:** tests. **Source cards:** I04, D01, N03. **Position in dependency order:** 27.
**After:** [V1-C04](00-tasks.md#v1-c04).

**Change boundary / entry points:** `crates/core/src/lib.rs`, `crates/core/src/mailbox.rs`, `crates/crypto/src/lib.rs`, `crates/crypto/src/mailbox.rs`, `crates/node/src/mailbox_network.rs`, `crates/node/src/mailbox_queries.rs`, `crates/node/src/custody_sync.rs`.

**Implementation plan:**

1. Bind invite/prekey/Welcome to owner, contact, protocol version, expiry, and one-time consumption.
2. Use the shared encrypted-object pipeline with a separate control kind until the conversation is established.
3. Bound the public contact endpoint with a quota and exclude an open contact list.

**Verifiable scenarios:**

- A correct invite admits one intended recipient and the creation of a single chat.
- A foreign owner, reuse of a consumed prekey, an expired invite, and unsolicited flood do not create MLS state.

**Checks:** CONTACT HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The signed wire contract and tests are accepted before production; holders do not receive MLS secrets.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-w02"></a>
## V1-W02. Persist and publish a paid Welcome

**Type:** backend. **Source cards:** I04, D01, D02. **Position in dependency order:** 28.
**After:** [V1-W01](04-tasks.md#v1-w01).

**Change boundary / entry points:** `crates/core/src/lib.rs`, `crates/core/src/mailbox.rs`, `crates/crypto/src/lib.rs`, `crates/crypto/src/mailbox.rs`, `crates/node/src/mailbox_network.rs`, `crates/node/src/mailbox_queries.rs`, `crates/node/src/custody_sync.rs`.

**Implementation plan:**

1. Include the Welcome in the existing durable outbox/public sender with a separate valid control resource.
2. Atomically persist the contact intent, control envelope, reservation, and subsequent publication.
3. Publish discoverable private rendezvous and receipts until the sender's retirement.

**Verifiable scenarios:**

- The sender finishes the first contact while Bob is offline and leaves; the Welcome remains available.
- A kill between MLS preparation, outbox, and receipt does not lose the Welcome and does not spend a second postage stamp.

**Checks:** CONTACT HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** Control does not require an already established chat and is not provided by a free unlimited channel.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-w03"></a>
## V1-W03. Import the Welcome and continue the application history

**Type:** backend. **Source cards:** I04, I05, D05, N03. **Position in dependency order:** 29.
**After:** [V1-W02](04-tasks.md#v1-w02), [V1-H11](01-tasks.md#v1-h11).

**Change boundary / entry points:** `crates/core/src/lib.rs`, `crates/core/src/mailbox.rs`, `crates/crypto/src/lib.rs`, `crates/crypto/src/mailbox.rs`, `crates/node/src/mailbox_network.rs`, `crates/node/src/mailbox_queries.rs`, `crates/node/src/custody_sync.rs`.

**Implementation plan:**

1. Fetch only the addressed private control through the ordinary discovery/custody path.
2. Verify invite/owner/expiry, create MLS/contact and the consumption marker atomically.
3. After commit, move to the ordinary paid application history; a duplicate Welcome is idempotent.

**Verifiable scenarios:**

- The Welcome arrives after some application bodies: the saved bodies are imported after the MLS is installed.
- An SQL fault on the Welcome and a cold retry do not create a second chat; replay/foreign control does not change the profile.

**Checks:** CONTACT HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** The recipient does not require a live sender and does not import application messages into an unconfirmed contact.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.

<a id="v1-w04"></a>
## V1-W04. Accept the first offline DM via UI and CLI

**Type:** verification. **Source cards:** I04, I05, D02, D05, U01, M06. **Position in dependency order:** 30.
**After:** [V1-W03](04-tasks.md#v1-w03).

**Change boundary / entry points:** `crates/core/src/lib.rs`, `crates/core/src/mailbox.rs`, `crates/crypto/src/lib.rs`, `crates/crypto/src/mailbox.rs`, `crates/node/src/mailbox_network.rs`, `crates/node/src/mailbox_queries.rs`, `crates/node/src/custody_sync.rs`.

**Implementation plan:**

1. Create two clean profiles through the ordinary entry points; Bob offline until the first invite/Welcome.
2. Send a known first text, remove Alice and some holders, enable Bob, then repeat the cold cycle.
3. Verify plaintext/MessageID, costs, contact state, and error state on both adapters.

**Verifiable scenarios:**

- The first message is available exactly once without sources, raw owner RPC, or a manually imported Welcome.
- An incorrect invite and a shortage of paid control budget give an honest refusal.

**Checks:** CONTACT HISTORY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. Verification tasks use real producers/adapters, not manual setup of internal state.

**Done when:** There is a real-process vertical of first contact; the established-chat smoke is saved separately.

**Evidence:** task ID, test symbols/number of executed checks, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, each scenario's outcome, saved failures. Currently `not_run`.
