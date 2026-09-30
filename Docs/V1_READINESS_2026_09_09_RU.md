# V1 readiness and the nearest delivery — September 9, 2026

**Scope update by direct user decision of September 9:** agent jobs, rating,
and reviews are moved to V2. [The current V1 boundary](V1_SCOPE_2026_09_09.md)
cancels their former status as release blockers; the CLI skill and MCP for
messaging remain V1. Historical job verification results below are kept as
evidence for V2.

**Full V1 is not ready yet. A working desktop already exists, but personal
messaging and private jobs work, while the other mandatory scenarios are not
finished.** Debugging the maximum committee took a disproportionately long
time. It remains an open scaling check and no longer defines the nearest
stage of work. The test is not removed, its time limits are not extended, and
the result is not marked GREEN.

The new checkpoint eliminates cursor reset after a recipient restart and a
new bounded pass. An independent Noise peer delivered 128 original MLS
messages across 26 pages. After 10 stored messages Bob was stopped in the
middle of a request; after restart, reading continued from the signed cursor
10. Requesting the 17th page required a new lookup/pass. All texts, authors,
and IDs matched, with no duplicates; the sender was off, the test only
observed the recipient. This is a traversal check, not proof of paid storage
of the entire corpus or of history completeness.

On 576 unchanged input files, 684 Rust tests, 51 frontend tests, eight
Tauri command tests, and six native scenarios passed. Real SQLCipher
failures on writing the cursor and the last message, retry after restart,
different sources, and out-of-order delivery were verified. The 33-message
scenario passed again. A fresh delivery of one genuinely paid MLS message
passed with a new genuine RISC0 proof, restarts, a storage failure, and loss
of the DHT copy. The regular Tauri bundle was rebuilt, the signature and the
absence of the test driver were verified; five pairs of screenshots were
reviewed. [Evidence and boundaries](../evidence/reviews/D05-durable-progress/README.md).

Independent indexes, explicit gaps, automatic sender preparation, R=10, and
repair remain open. A separate desktop limitation was found: the snapshot
reads the first 1000 dialog records, later ones never reach the UI; a large
total volume can also exceed the IPC limit. Bounded fresh history and
pagination are needed.

The previous checkpoint added paid ciphertext storage in SQLCipher and
private read requests via the Core/owner API daemon. 671 Rust tests and 51
frontend tests, models, TypeScript, the frontend build, and fmt/Clippy
passed. Six genuine RISC0 proofs backed the paid-storage fixtures. At that
time network storage was not yet implemented, and the regular bundle was not
rebuilt at that stage. Full long-history sync, independent indexes, and
repair remain open even after the new network step. The first parallel run
revealed a single failure of an existing announcement test; without changing
the test, two separate launches and a full run with four test threads passed.
The failure cause was not identified;
[checks and result boundaries](../evidence/reviews/D02-paid-custody/README.md).

## What can be done in the existing application

The fresh build is located at
`/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
On the current 576 pinned input files, 684 Rust tests and 51 frontend tests,
TypeScript, models, fmt/Clippy, eight genuine Tauri command tests, six
native scenarios, and a regular Tauri build passed.
[Bundle check and hashes of the five executables](../evidence/reviews/D05-durable-progress/app-release.json),
[check results](../evidence/reviews/D05-durable-progress/regressions.json),
[build and native run](../evidence/reviews/D05-durable-progress/gates.json).
This is macOS arm64 with an ad-hoc signature; notarization and the full
matrix of other OSes are not complete.

| User action | What is confirmed | Evidence and boundary |
|---|---|---|
| Create a profile, exchange invitations, write and reply | Two genuine WKWebView/Tauri clients, libp2p, OpenMLS, delivery confirmation signature | [Six native scenarios](../evidence/reviews/D05-automatic-retrieval/native-e2e/result.json). Delivery to the recipient does not imply ten replicas |
| Close the UI and open it again | Keychain identity and history persist; the independent daemon receives messages while the window is closed | The same native result. This is not recovery of a lost device |
| Connect an agent via MCP | The UI grants limited permissions; the shown configuration launches the bundled MCP; real exchange, poll/ack inbox, revoking permissions denies the action to the same client | [MCP in the bundle](../evidence/reviews/M06-packaged-mcp.md), the current native result above. Private jobs and text artifacts are also available via MCP |
| Order work from a known contractor and accept the result | Two MCP clients create an RFQ, sign the order and the result; after a restart of both sides the result is automatically visible in desktop and the owner explicitly accepts it; permissions are revoked from the same client | The sixth native scenario; [guide](AGENT_JOBS_RU.md). The service code is transferred manually; public search and settlement do not exist yet |
| Configure relay, AutoNAT, DHT client/server | Settings persist, apply to the real daemon, and survive a restart; a message passes through the circuit; relay-only suppresses direct DHT | [DHT role check](../evidence/reviews/N03-dht-roles/README.md). Local NAT checks do not prove every internet topology |
| Choose a trust profile and receive a checkpoint from a peer | Real Tauri commands, verification of a signed test checkpoint, persistence after source loss and restart | The fifth native scenario. This is not wired-up production L2 finality and not a monetary balance |

Fresh native screenshots were compared against the previous verified
delivery:
[five comparisons of the current build](../evidence/reviews/D05-automatic-retrieval/visual-review.json),
[previous check of the other screens](../evidence/reviews/V1-readiness-20260909/visual-review.json).
Network and process Rust tests are part of the run above. In the previous
checkpoint a real TCP/QUIC announcement preflight separately confirmed Core
rejection/acceptance and 5/10/20/5 second retries. Additionally, a new
genuine-spend/EVM scenario of storing a real message passed; the entire
historical EVM suite was not re-run. Two historical and one full fresh RISC0
receipts were verified by the bundled verifier, including rejection of a
substituted operation and an expired term. The fresh proof was also
re-verified by an independent oracle. The full 64-node gate remains RED, as
described below. None of these numbers is proof of full V1 readiness.

## What blocks full V1

| User outcome | Status | Relation to the original acceptance |
|---|---|---|
| Agent communicates via the CLI skill/MCP with limited permissions | Private jobs are kept as V2 work; contractor search and job lifecycle no longer block V1. Messaging/CLI/revoke remain mandatory | E11, E14; E12/E13 — V2 |
| The recipient returns while the sender is off; the network itself restores ten copies after failures | A paid message was automatically retrieved after restart with the sender off. One holder and explicit sender preparation. The local cursor was verified on 128 messages and a restart; independent indexes, explicit gaps, and R=10 repair are not finished | E05–E07 |
| A group adds and removes devices, reconciles concurrent changes, and brings back an offline participant | MLS and consensus components exist, but no finished product integration | E08–E10 |
| Restore identity/devices without restoring revoked permissions | The full recovery/device model is not finished | E10, E23 |
| Identity credentials are available independently of a single mandatory provider | Issuer flows remain V1. Jobs, review publishing, rating, and their UI/MCP are moved to V2 | E17/E18/E26; E15/E16 — V2 |
| Real payment for a resource, a single spend, and carrying spent state into the next epoch | There are genuine proofs/QC and separate local EVM checks; storage integration, handover, and the full user money flow are open | E19–E22 |
| Attachments, search, notifications, backup, and delivery on supported OSes | Full product and platform acceptance is open | E23–E25 |

The scenarios are defined in the
[E01–E26 plan](agentic_internet_v1_execution_plan/TEST_AND_E2E_PLAN.md). The
matrix distinguishes working components from the end-to-end product. A
missing scenario must not be counted as passed because of a green test of
its dependency.

## Where the 64 validators and 45 seconds came from

The original request contains no requirement for 64 validators. In the
provided [P01 card](agentic_internet_v1_1_plan/tasks/P01.md) the acceptance
demo is described as "Four test validators survive one Byzantine"; the same
card says the fixture does not define the production quorum. The general
principle is `n=3f+1`, `q=2f+1`.

The upper bound of 64 was set by the later
[certificate journal engineering spec](../spec/finalizer/certificate-journal-v1.md),
introduced by commit `796a74d` on September 6. The current
[scaling contract](../spec/committee-capacity-v1.md) extended it to a real
rig of 64 processes and required that after a simultaneous restart of all
processes without seeds, each one restore routes to the other 63 within 45
seconds. This is a criterion of maximum-size implementation, not a
requirement of the original user E2E.

The 64 processes are separate local daemons with validator keys; these are
not 64 messenger users or 64 development agents. The quorum is 43. Ten
storage replicas are a different role with a different requirement. The
number 4 in the original fixture is no reason to reduce the quorum of the
already chosen production committee.

Actual progress: one full TCP launch passed, including cold recovery and the
third job. In the same launch QUIC restored all connections, but five nodes
were each missing one verified route by the deadline. The next diagnostic
TCP launch left two missing routes and recorded real
`shared processing capacity exhausted` failures for announcement requests.
Thus the case remains unstable, the cause of the specific failures is known,
and a release promising reliable operation of the maximum composition is not
yet confirmed.

Raw results and unmodified REDs are stored in
`output/committee-capacity-planning/announcement-retry-attempt1/` and
`output/committee-capacity-planning/announcement-diagnostic2/`. The new
processing reservation fix is not implemented yet; the R24 test draft is kept
outside the build until priorities are revisited.

## The bounded nearest stage

1. **Done.** Full workspace backend/frontend, fmt/Clippy, and models were
   repeated on the current pinned sources, Tauri was built, and six genuine
   native scenarios were run. Screenshots were reviewed. The stage result is
   a specific verified build, its hashes, and user-facing evidence; this is
   not the closure of all of V1.
2. **Done.** Private job via MCP: request, proposal, two-sided acceptance, a
   signed text result, and the customer's explicit decision in desktop;
   restart/retry/revoke were verified. Tests preceded the implementation; an
   independent critic reviewed every backend test change. Deterministic
   contractors exercise the protocol and do not pretend to be an LLM. Price
   is separate from transport; payment state is explicitly `not_funded`, no
   settlement is claimed. Public search, large artifacts, reassignment, and
   review publishing remain open.
3. **Prerequisite D01 done.** A private message/job envelope with a separate
   key, an immutable identifier of the future paid operation, and persistence
   before handover. Cryptographic, Core, and process tests were written
   before the implementation and accepted by the critic. This is not network
   storage yet: the next stage is genuine paid store/get with the chosen
   custodians, independent indexes, and R=10 recovery.
4. **First automatic retrieval done.** A fresh paid MLS message was found and
   retrieved by the returning daemon with the sender off, with fallback, an
   SQLCipher error and retry, and restarts without duplication. The recipient
   operates on its own; for now the sender explicitly places one copy and a
   pointer. Next come independent indexes and autonomous R=10.
5. **Local durable cursor done.** Persisting together with the last message
   is atomic; real SQLCipher failures, different sources, and retry after
   restart were verified. The 128-message corpus passed through bounded
   traversals. History completeness is not proven by this; the desktop limit
   of the first 1000 records also needs fixing.
6. Then bring offline/R=10 and groups/recovery to the corresponding
   end-to-end demos, then finish credentials, the transport economy,
   desktop, and the remaining mandatory V1 scenarios. Jobs/reviews/rating
   are V2.

The 64 debugging does not continue as a parallel endless chain of changes.
Its next bounded attempt: one isolated RED for announcements being crowded
out by consensus load, one critic-reviewed fix option within the previous
shared limits, one full TCP/QUIC run. A failure remains a separate open
defect and requires a new priority assessment. It must not be renamed a
success, given more time for GREEN, or used as the sole explanation of all
unfinished V1 features.
