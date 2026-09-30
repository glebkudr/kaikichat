# Independent backend test review — Core purchase request

Reviewer: `/root/index_holder_test_critic`, created without inherited context.
Only tests and the stated business contract were reviewed before production edits.

Initial verdict: REVISE. Required alternate tariff success, independent class/expiry
funding mismatches, and retries at later host times. All three were implemented.

Final verdict: ACCEPT. The reviewer confirmed independent resource/price/ABI
assertions, genuine funding mismatch rejection without spendable balance, exact
warm/cold retries, SQL failure atomicity and no secret/false-balance fields. No
additional mandatory Core scenarios were requested. The reviewer did not run tests.

Accepted test SHA256: `c446d6796b3521a6761240328cdd1077ef2e0e7d561cda4e84ad2dace4d3adbf`.
`core-purchase-red.json` records the expected missing-API compilation RED before
production changes. Candidate 1 found an Alloy tuple ABI type error; production
encoding now uses the same explicit SolType tuple form as existing contract code.
All four new Core tests subsequently passed. This does not accept the whole wallet
flow or full V1.

## Core RPC roots and local authority

Reviewer `/root/index_holder_test_critic`: initial REVISE required authority
retention before bind/configure and real checkpoint clock UPDATE failure. Both
were added, plus pinned domains/code hashes. Final ACCEPT covered six tests,
SHA256 `34afe2ecdd1cc843e1cf1b137c9d226a81c521c5993083d810431e1b2e4dac9d`.
No production RPC code preceded ACCEPT. Compilation RED then reported only
26 missing-method errors. All six tests passed after implementation, including
genuine funded proof, cold ordinary send and recipient plaintext, retained-state
INSERT rollback, clock UPDATE rollback and stale-head rejection.

## Async RPC transport

Independent reviewer requested chunked response limits, mutually exclusive RPC
result/error fields and redacted transport failures; all were added. Final ACCEPT
covered five tests, SHA256 `52886f00d0781fd2ad2691b5a044f3240947401e00c165e980e4541dcabd7fa4`.
The actual missing-module RED preceded implementation; all five TCP/HTTP tests
then passed, including exact 512KiB/512KiB+1 streaming boundaries.

Dependency `reqwest 0.13.5` was checked against latest official docs at
https://docs.rs/reqwest/latest/reqwest/ and fetched with the project build wrapper;
Cargo reports MSRV 1.85, compatible with workspace 1.91. Only rustls is enabled.
Proof calls follow https://eips.ethereum.org/EIPS/eip-1898 and
https://eips.ethereum.org/EIPS/eip-1186; no response itself grants authority.

## Ordinary native wallet flow

Independent reviewer `/root/index_holder_test_critic` first requested a valid
actual payment timestamp and real refused-RPC retry before compatibility import.
Both were fixed; cold inventory is also read before prepare retry. Final ACCEPT:
`public_wallet_flow.py` SHA256
`59a765ba68d90da1a0886afcf85f04af056cba230603ad52ff9e045624574a63`;
`funded_custody.py` SHA256
`029a873f1442f55ca21892fb5a6f6b68ddb23d5361782682d31a5e062199863b`.
Actual native RED reached `wallet_resources` and failed with `unknown_method`
before controller implementation. This built real contracts and native daemons.
The later one-line Runtime fixture initializer received separate ACCEPT; it
creates an idle HTTP client without changing prior assertions.

## Desktop boundary and external payment request

Independent bridge review first required the actual Core error message, owner
refresh coverage, and a valid attempted identity creation before the profile
exists. All were fixed. ACCEPT test SHA256
`983ba4ea5d233841105216e30d891eb538d27ffc0e8c66b7b2ea5e10d25fd767`.
Observed RED: missing `wallet_command` in native Tauri ACL/catalog before bridge.

The reviewer separately accepted ERC-681 purchase tests before production URI
implementation: Core test SHA256
`6b0dd197b93b4bf2daf6970ddfe18420bb0ab732b476fe5c0e1ede2f2696938c`,
real-EVM payment helper SHA256
`12f4cff18325522355af8f0210a2a815d40d102c729c4c5346b7d3ac5761f27d`.
Observed Core RED: exact quote-field assertion missing `paymentUri`. The native
external signer now independently decodes typed URI arguments and executes them
with Foundry ABI encoding; prior native candidate 1 predates this URI extension.
Standard: https://eips.ethereum.org/EIPS/eip-681. Support in a particular external
wallet application has not been claimed by these tests.

## Ordinary network profile selection

Independent reviewer `/root/index_holder_test_critic` accepted the real native
profile-selection boundary tests before the bridge whitelist was extended.
Test SHA256 `e9b52021c335291240a7e00eeb58d6bccb0087e6d350c7e8ab2542bc4bedefb3`;
module registration SHA256
`a7498b8603f0d2ace91b5047c0cee074a558cfc7eb890825bccb5396652ec6bb`.
Observed RED: `Unknown desktop wallet command` at `registry_selection`.
The negative replacement case checks inconsistent registry/policy inputs, not
independent valid-policy replacement. These tests do not demonstrate automatic
acquisition of a committee roster or postage client authority from the network.

## Public client authority from connected peers

Independent critic first required the corpus certificate's `0x` carrier prefix
and a full-length roster with a mismatched genuine membership proof. Both were
fixed; cold repeated import and live checkpoint without a policy were also added.
Final Core ACCEPT SHA256
`09304d1a0cc73a74df3fc49aa475efab7ddc6725189ab0b3a9893669eebb4e4d`;
registration `b955e6b7dda7eba0185b93034da92377a6a29b0b96910630b4e5bce8d55241d7`.
The missing-API RED preceded implementation; it was recorded on the first test
revision. Current import reuses issuer/committee validators and shared roster
staging; the roster and monotonic clock commit together before authority release.

The reviewer separately ACCEPTED the native gate removing manual Alice roster
publication/client configuration, requiring real peer acquisition, persisted
configuration after process loss, and the existing two sends/R10/index/recovery:
`public_wallet_flow.py` SHA256
`60db9fe20387d3a89108ffd8cff1faf95af392df2bec70c2a63ac4cffa2ebac0`;
`public_index_sender.py` SHA256
`53a1280f075bf6272588420efc8d3e3050c988e00277be1a8c1aabb901634b3b`.
Malformed/oversized wire response plus retry is a nonblocking additional scenario,
not proved by this native test. The protocol must reuse existing bounded transport,
Scheduler/Admission, authenticated connected sources and relay policy.

## Standalone scoped CLI

Independent critic `/root/index_holder_test_critic` initially required the real
`idempotency_conflict` code, both exact NetworkID metadata expectations, a strict
single error envelope with retryable/exit-code agreement, and a genuine active
inbox lease followed by the same successful poll after ACK. These were fixed
before production; ACCEPT hashes:

- CLI process tests: `cbc70796caff95f4160ab8b76520dca2d6aa4e3931ef5d2c6900268a541e1f61`.
- Core metadata: `bbfcf53ca0ead321c4d5a2d36ec78d51f893a2b588274e613de404f950b6ca34`.
- MCP metadata tests: `383ad8348579cb27b246871adb619a316c4eb3fb90cf887254a472a7dc6c2a7b`.

Recorded initial REDs: CLI executable did not exist; Core metadata did not contain
NetworkIDs. The implementation extracts the existing credential/signing client,
preserving MCP cancellation and framing. Three CLI and three metadata tests pass.

The following provisioning delta was separately ACCEPTED before `materialize`
changed: exact adjacent CLI command plus same credentials, real MCP-to-CLI retry,
cold restoration and UI-grant revocation. MCP tests hash
`50591130af03fd00e7a29ea9271b58a2e6a10ed1f49be0daddf4f5b32daf0f09`;
CLI helper visibility-only update hash
`a727d6c11e616135e9e3e9bce2f041245fb5648553e976f00f4cda91f328002b`.
Actual RED was missing `cliConfig` (two failures, eleven existing MCP cases green).
All thirteen MCP cases passed after implementation. The frontend command first
failed its missing-field test, then all twenty targeted frontend checks passed.

## Paid CLI and packaged native command

The paid native test review found that the isolated daemon lacked adjacent runtime
binaries. The fixture now copies and hashes both MCP and CLI before/after the run.
Final ACCEPT hashes:

- `public_wallet_cli.py`: `f01a0e154ce10abc080b7353ec78ada41d551b201a2f6fb7d342b7bc524b5972`.
- Recipient send hook: `498be874039fad6864d06bab93b51752ba420e8c1db030ea95deee1f9e16095b`.
- Native build/isolation fixture: `763aca665725daea14f4223e1491ce811136990e4334b6a9d6b363a96cc26e89`.

The genuine paid CLI scenario passes with six QC signatures and 561 unchanged
inputs. Native GUI assertions were separately ACCEPTED at
`db2eba03e15e18f0cd9534f09c867e119f4c15f67321739ea967416f8abbbf79`:
shown command execution, independently obtained identities, same send/delivery,
UI revoke and exact message lists. The full hidden packaged WKWebView gate passes.

Five equivalent cloned-singleton-slice replacements in existing recipient tests
were also ACCEPTED (`f77dbaef892a99d06900e319b168ed8a45a1ed02cd8468b3a86cbfe15712af76`).
They remove Clippy warnings without changing the oracle; all seven index tests and
Core/node all-target Clippy pass. No new V1 card is closed by these checks.

## Shared persisted status and successful sender retirement

The independent critic required realistic bounded message data, both terminal SQL
failure points, exact pointer batch binding, count bounds, idempotent failures and
late callbacks. The corrected Core tests were ACCEPTED before production:

- Progress tests: `83b8ecc9836194e5729f7272caadfe3f215b0bec2e5a1251fe385e55bad44129`.
- Registration: `be12de061b03793c259835e7f6dcd2023700fcac79fb3ba60a8c6c8e1e56f7a8`.
- Shared retirement setup: `aa0c2e7bac53c1af3f753fd600b99387b6429d0aed084ad4babe42f7e8b9db77`.
- Contract at review: `a1e2a45985a04b9139875e77a9337dec6f5cc354158f547e3b548f4d62f63fcb`.

The recorded Core RED is nine missing-type/method compiler errors, not a runtime
failure. Six new tests subsequently passed, followed by 36 sender regressions.
They explicitly test trusted-host projection and SQL atomicity; they do not
impersonate the Node cryptographic verifier. The capacity assertion reopens an
active queue slot; it does not prove over 128 live paid messages.

For the actual network gate, the critic required a compatible 600-second authority
lease and observation of the partial-progress fault before terminal faults could
mask it. Cold active work can record newly verified intermediate progress; strict
row equality is required after completion. ACCEPT hashes:

- `public_sender_lifecycle.py`: `192818c1ae12e7d1c1d1b8a9ac589749dc1bf20a1e27bb7d21dd1bf3c228bed9`.
- Read-only recipient guard: `c695c57c8d50dc862997116fceea303ec0056dae9d3de0f5b815e1f162bf7115`.
- SQL fixture initially: `899a5fdee9d0d20b0c0ee80871e5fb234744482d2b16f24c6c3b1c057be2f1f9`.
- Separately accepted `u64` to `i64` SQL-revision decoding fix:
  `8076270339053afa63e15d96c5b101eceb7a37acdb45aef1c34b2cda7371f08c`.

Native candidate 1 failed before sending because the output directory made its
AF_UNIX socket path too long on macOS. Its 697 source inputs were unchanged; this
is a fixture launch failure, not native behavioral RED. Candidate 2 uses a short
directory on the same managed ChatBuild volume. Its result is recorded separately.

Candidate 2 reached both sends, all three lifecycle SQL faults and cold status,
then independently verified six QC signatures. It failed before recipient recovery:
the inherited wallet assertion expected a ready RPC view after additional daemon
restarts. No whole native success is claimed. The critic separately ACCEPTED an
ordinary wallet refresh before the unchanged exact balance assertion, requiring
both completed projections and all lifecycle SQL rows to remain exactly unchanged
across that refresh. The passive owner guard adds only `wallet_refresh`; `finally`
now retains lifecycle trace on later failures without suppressing the exception.

- Lifecycle test: `1523f1728a1212ca7de0087457b1f95526130fcceb09f6e7871aa7941b2ad96f`.
- Recipient guard: `a94ae2531a5dbc2b302bbe10cf0ca729bb86b0559d2c7eddd280ae912116e235`.

## Packaged messaging skill

Independent artifact reviewer `/root/messaging_skill_reviewer` required saving the
poll operation ID and exact arguments before invocation, and practical recovery
from `inbox_item_too_large` within the existing grant. Both were fixed. The skill
now states real argument bounds, retains uncertain operations, separates receipt
from paid durability and requires a new poll ID when changing page arguments.
Final ACCEPT SHA256:
`8cb97a5c5219dbb787cf2c9a4e52c248bad6d4b4f3edfee4ab2cb398351d2bbf`.
This reviews the instructions, not the full E11 host/NAT scenario.

The native UI test delta was separately ACCEPTED by the backend critic: compare
the complete source with the real readonly WKWebView textarea after owner-issued
runtime provisioning, expand it and retain a screenshot. All prior CLI/MCP,
revocation and history assertions remain unchanged. Native test SHA256:
`2641ec82e297cc62cf3e18f771e77004d155591cf7269d038b641e6977b9814e`.
Actual packaging also compares the resource bytes with that same accepted source.
# Fresh native paid GUI test review

Candidate 1 failed before identity creation: the helper-owned macOS Keychain ACL
denied the real desktop. The collector records 744 unchanged inputs and identical
bundled binaries; no payment/send occurred. A second fixture error left the Node
driver waiting for stdin after cleanup. The screenshot and local logs are retained.

Correction: use macOS `security -X` for raw fixture bytes and an explicit `-T`
entry for the actual test desktop, keeping the canonical path/account guards and
no overwrite/global ACL changes. Remove the Rust helper and close driver stdin.
The critic initially required redacting `TimeoutExpired`, which otherwise embeds
secret-bearing arguments; a fixed error with `from None` resolves it.
Final correction ACCEPT: `public_wallet_gui.py`
`cd442e6cfd61d620178808da73bcbd4472253d9f7b9f72ace36a0137e5ab9976`.

Candidate 2 passed both GUI profile creations and clean shutdown, then failed
reading the real invitation because a test script embedded unescaped quotes.
The selector is now passed via `arguments[0]`; all 21 embedded browser scripts
were syntax-checked. Independent correction ACCEPT, `native-wallet-driver.mjs`:
`4c328a11270224c501744466446e866794992387da2235476e89d28ba32f4557`.
The collector now starts with a fresh screenshot directory after retaining each
previous candidate's artifacts. Candidate 2 had 743 unchanged inputs and unchanged
bundled binaries; it reached no payment/send acceptance.

Candidate 3 reached native trust/contact/purchase, actual external Anvil payment
and cold purchase, then hit the old harness's ambiguity: a valid wallet result
with `error:rpc_unavailable` was interpreted as a WebDriver failure. The UI showed
the intended refusal. The harness now wraps native success/failure separately;
actual transport rejections still throw. Wallet refusal/balance/budget assertions
remain. A second screenshot scrolls the payment fields into view.
Independent correction ACCEPT: harness
`ff1c83156de8701272f8d0768179492ff8a9a1342719973ba6b2f71fbed211a3`, driver
`258f64bd95e7e7805ad3c473a00e6bc024008e609bbb48c41568c6bcb3428e9b`.

Candidate 4 passed native payment, RPC refusal, verified refresh and budget. Its
next file upload raced asynchronous trust-form loading. The fixture now waits
for the actual enabled file input before emitting File/change, with no timeout
increase or weakened assertion. Independent ACCEPT, driver:
`85ab9cc3e692e27a3185ca51ef277f0df932ccaa5275a0ebeb43258d7b27d031`.

Independent `index_holder_test_critic` accepted the new GUI payment adapter and
shared native harness extraction before the first run. It confirmed real control
mutations, exact displayed transaction/URI binding, funding before compatibility
setup, unchanged independent QC/history/loss/SQL oracles and explicit daemon
ownership. No production change was made for this test phase.

Accepted SHA-256: `public_wallet_gui.py`
`1101a789b4e4f33b85a91009fe6261fa06b237053d1ca916091d9b3e1027a881`;
`native-wallet-driver.mjs`
`46634e9fcacaabb821c20ea833475abdeaadddf891e020ffdb0e91e4ad698378`;
`native-harness.mjs`
`c29a02fd6122ee4b39ca6156214900782ab62d343b31515b7cca62e3fbb5c170`;
`native_fixture_keychain.rs`
`ce700d8d2eea277af04cc3f191d9c82ddda0c099f44214af70446868afaab1f6`.

Run acceptance additionally requires an external collector binding frontend
inputs and all actual bundled binaries before/after, plus visual inspection.
A possible stale prior error during GUI refresh is a nonblocking fixture concern:
the inherited oracle still requires the new successful ready state.

## Unattended E2E vault and final GUI acceptance

Candidate 5 again encountered Keychain authorization on a cold GUI restart.
The user explicitly reported the password dialogs. Automation no longer uses
Keychain; previous fixture ACL/helper approaches above are historical failures.
The production system vault and its access controls were not changed.

The independent critic reviewed new real daemon/SQLCipher restart tests before
the E2E-only file vault implementation. Initial REVISE required exact raw bytes,
foreign-account reads and a truncated saved key. Final ACCEPT covers those cases,
missing/exposed files, refusal to overwrite and unchanged identity/database on
failed restarts. A private-directory setup correction reuses the existing helper;
the deliberate shared-directory refusal remains. Accepted test SHA:
`3141a36ab347631f3ce0c32afea50d55114bbb3781fca900c6b39d48f1c9fe2c`.

The critic also accepted the GUI fixture's private raw seed and removal of
Keychain cleanup. Accepted GUI adapter SHA:
`886faf5d39119e947eeb4ff558b2e9c0541be6826b6266fe806c8fd8a8bd76b6`;
harness `d1b949ca577b00a4780995587f62609cc80d342c653144be32d952f67c7d58f6`;
ordinary native suite
`241f4679a34476fa3e9a92a95be68f3a40fe2824615ee204aa2551ddeba53500`.

The rebuilt ordinary native gate and subsequent paid GUI candidate 6 both pass.
All 754 inputs and five bundle binaries are unchanged; no test process remained
after teardown. Payment/status screenshots were inspected with prior references.
[GUI acceptance](GUI_ACCEPTANCE.md), [vault evidence](E2E_SECRETS.md).
