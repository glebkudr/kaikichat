# Next product gate: visible recovery range and gaps

Status: the [Core contract](../../../spec/conversation-recovery-observation-v1.md)
is implemented and passes [targeted checks](history-recovery-core-checks.json):
71 backend/35 frontend, Clippy/fmt, including seven new independently accepted
Core audit/reader tests. Worker leaf observation, CLI/MCP and visible UI are now
implemented with [targeted checks](history-recovery-integration-checks.json):
30 backend/45 frontend, production Clippy, frontend build, formatting and headless
visual inspection. The independently accepted [paid observation scenario passes](history-recovery-native-r1.json)
on867 unchanged inputs: six signatures, actual18+18 copy loss, failed0/2,
partial1/2, recovered2/2 and cold2/2 via real CLI/owner pages. Teardown is clean;
native GUI recovery is not claimed.
The [native desktop history regression](history-recovery-native-ui-r2.json) also
passes with1051 real messages,22 pages, daemon restart and reply;868 inputs and
five binaries stayed unchanged. Hidden E2E vault, clean teardown, newest/oldest
WKWebView screenshots inspected. This is desktop history evidence, not paid
recovery state coverage inside the native GUI.
This does not replace the full130 native loss/recovery gate, first offline Welcome,
older-epoch recovery, groups or independent R10 repair.

## Current observable gap

The conversation UI now distinguishes an empty local page from unknown or
incomplete remote history. It displays the shared signed-root observation while
keeping loaded messages and the composer usable. The automatic worker now calls
the observer on authenticated leaves. Remaining product gaps are below.

`custody_history_page_receive.rs` already persists and authenticates the exact
incoming root, verifies extension and binds it to the current pointer. Its
reference count describes the sender's declared graph, not all possible messages
or available physical copies. A newly seen pointer is insufficient to establish
an accepted root or remote completeness.

`custody_history_page_scan.rs` persists the next attempted reference before I/O.
This is explicitly a scheduling cursor. Neither visiting all ordinals nor wrapping
the cursor proves that messages were imported. `custody_history_page_import.rs`
already has exact operation/sequence progress written atomically with MLS,
message and dedup changes, and a bounded authenticated leaf projection. Reuse
these facts; do not count network attempts as received messages.

`custody_sync.rs` currently has aggregate in-memory counters for received,
deferred and failed attempts. It treats `receive_gap` as a reason to try another
reference. Those counters do not identify one conversation or survive restart.
Core independently emits `HistoryRecoveryRequired` for old unguarded contacts;
the transport maps it to `history_recovery_required`, but there is no complete
user-facing recovery/rejoin flow.

## Additional integration constraints

`desktop_history.rs::conversation_history_at` is the existing bounded owner read
and `runtime.rs` already passes the daemon clock to it. Its local page cursor
must stay independent of any remote recovery range. A shared Core projection can
be reused here, but a Runtime/CLI/MCP wrapper must still enforce the contact and
read permission; the owner desktop method is not an agent permission shortcut.

`mailbox.rs::mailbox_context` selects the current incoming MLS epoch. The retained
root helper verifies the signature with that capability even for a past-expiry
root, while `custody_history_root_head` filters expired roots. A visible observation
must therefore name its epoch and signed root and distinguish a retained historical
root from a live pointer naming a newer root. This existing path cannot establish
whole-conversation recovery across unavailable older epochs or a missing Welcome.

The graph receiver currently checks committed imports one authenticated leaf at a
time. Imported operation/sequence rows are per epoch, not per root revision. A new
root can extend or retire an expired prefix; a raw count of these lifetime rows
would overstate the new root's imported range. Any aggregate needs a bounded,
root-bound derivation with explicit unknown/unobserved references and atomic
message/import updates. Foreground reads must not scan all lifetime message bodies.
These are implementation constraints, not evidence that the product read model
or a recovery/rejoin action has been delivered.

## Required complete vertical

1. Give UI, CLI and MCP one conversation-scoped read model. Identify what the
   accepted signed root declares, the observation's scope/time and whether work
   is still pending. Retain useful observations across restart without trusting
   stale Work caches or loading every lifetime message on a foreground read.
2. Distinguish unknown history, active retrieval, missing live data/pages,
   deferred MLS predecessors, expired unavailable references and legacy/recovery
   requirements. A transient connection failure cannot become permanent loss;
   a deadline or local page boundary cannot become complete history.
3. Base any imported count or complete-range claim on committed import/message
   evidence for that exact scope. Keep attempts and successful imports separate.
   A successor root, expiry, cold restart or direct-path delivery must not yield
   a false complete claim or erase a known unresolved live gap.
4. Show meaningful text and available actions in the conversation. Keep loaded
   messages usable while retrieval continues. Explain when an older message is
   awaited or contact/device recovery is required. Do not present the empty
   conversation invitation as evidence that no remote messages exist. Reuse the
   automatic worker for retry; do not expose internal publication/import RPCs.
5. Make any recovery/rejoin action concrete and preserve antirollback, existing
   keys, messages, grants and paid evidence. A status label alone does not close
   the legacy/rejoin requirement. Older-epoch and first-Welcome acceptance remains
   separately mandatory until the corresponding real flows pass.

## Implemented Core boundary and integration history

Prefer one root-bound background audit over a second permanent per-message
ledger. The existing import operation/sequence rows remain authoritative. A
bounded pass can authenticate one leaf or a signed expired subtree, inspect its
committed imports, then advance a separate observation cursor and conservative
counts atomically. Until the pass covers the exact accepted root, display those
counts as partial observations, with the remaining references unobserved or
awaiting retrieval. The retrieval cursor must not advance this audit by itself.

A successor root starts a new audit scope; a newly received pointer that is ahead
of the accepted root makes the displayed observation historical. For an already
audited reference, a new authenticated import must update its count in the same
transaction as the existing message, MLS and import records. Refusing the audit
participant must roll back all of them. Imports outside the audited prefix remain
unobserved until the next bounded pass; the audit derives their counts from the
existing import records. Repeated leaves, direct/custody dedup and a complete
retrieval-cursor cycle cannot increment progress twice. Keep the
observation's root, epoch and time visible in the API. A completed-range label
requires that every declared live reference has committed import evidence; an
expired reference is separately unavailable, never counted as received.

The four Core audit tests now pass with exact message-row rollback, real MLS epoch
transition, constant-size audit persistence for130 references in three leaves,
actual receive-gap/direct dedup and root replacement/expiry. The three reader tests
also pass: owner pages and signed `history_recovery_get` agree during recovering,
available, expired and legacy states. ReadInbox scope, replay, nonce SQL failure,
cold retry and revocation preserve message/history/registry data. The compatibility
page returns explicit null without a clock; normal daemon owner reads use time.
See `history-recovery-status-critic.json`, `history-recovery-readers-critic.json`
and `history-recovery-core-checks.json` for exact final test hashes, RED/failed
fixture history and final targeted evidence. Native R7's838 frozen inputs were
verified unchanged before these source edits began.

The separate process contract in `history-recovery-adapters-critic.json` is also
accepted and executed. It exercises CLI `history status --from <public-id>` and
MCP `history.status` under both supported protocol versions, with real stopped
profiles, an encrypted original, authenticated root and actual import across a
daemon restart. Both adapters must expose the same `recovering` then `available`
observation as the owner read, enforce the real contact grant and preserve inbox
leases. Fresh poll operations must return `inbox_busy` after each status while
the original lease lives; those exact operations succeed after explicit ACK.
Replaying the original poll alone would not prove absence of an accidental ACK.
The domain fixture is not paid network retrieval evidence.

Its companion test-only wiring patch exposes the existing stopped-profile/time
helpers, adds the process test module and updates the expected tool inventory.
The original adapter implementation draft was
`output/ar2-wallet-flow/history-recovery-adapters-candidate.patch` (SHA256
`e1ef4ad45ebb2e8884b4775fc022d99f85dbd47ccdbbeb6795c82b6c6780a122`).
It uses the existing CLI public-ID resolver and shared signed RuntimeClient;
both adapters call the broker method rather than the owner page API. The draft is now installed. The first run reached an incorrect test expectation
for principal override; the existing broker intentionally returns unauthorized.
The independently accepted correction and all14 MCP process tests now pass.
This is process integration evidence, not full product acceptance.

The former Core candidate is installed as `crates/core/src/conversation_recovery.rs`
with module exports, atomic graph import participation, bounded owner page field
and ReadInbox-protected broker method. Original staged artifacts and patch remain
in local output as development history; current source and accepted final tests
are authoritative. One row per conversation retains the previous complete
observation during a new pass. A different signed root resets scope; inconsistent
metadata for the same root fails validation. The observer returns the existing
leaf import projection so a future worker integration can reuse it for fetching.

Whole expired-subtree observation still needs a separate contract. The current
worker integration observes authenticated leaf bodies only. A
signed expiry proves that the remote lease ended; it does not prove that no
message in that range was previously imported. The current leaf projection can
distinguish those cases when authenticated references are available. Do not
turn a scheduling skip or a missing expired page body into a count of lost local
messages, discard a previously proved complete local range, or claim an audit
has covered references it could not inspect.

The Core boundary is tested; it does not establish complete product recovery.
The automatic worker now calls the observer without treating attempts as imports.
CLI and MCP expose recovery status through the shared signed
broker method before describing those entry points as implemented.

## Evidence required before acceptance

### Confirmed integration map

- `custody_history_graph.rs::step` already obtains an authenticated leaf and
  calls Core's leaf progress query. Feed the bounded observation from this
  background path, retaining the existing attempt/import distinction. A signed
  expired subtree requires a corresponding bounded observation path rather than
  inventing imported references from the scheduling skip.
- `desktop_history.rs` and `runtime.rs::conversation_history` are the bounded
  owner page path. The shared recovery projection is implemented without changing local
  pagination or treating a foreground read as network work.
- `broker.rs` owns signed runtime grant/nonce enforcement. The implemented scoped
  observation authorizes `ReadInbox` for the selected contact before exposing
  its range. `messaging_cli.rs` and `mcp.rs` now call that same broker method;
  neither can forward an agent request to the owner's desktop method.
- `ChatShell.tsx` now renders the recovery observation for empty and loaded
  pages. `useMessageHistory.ts` preserves loaded pages and refreshes outgoing
  durability and incoming recovery scope expiry at the nearest deadline.
  Pagination and optimistic appends preserve the latest authoritative scope.

Owner/broker and worker/adapter/UI paths have targeted acceptance; native paid
observation and complete product recovery remain unaccepted.

Write backend tests first and obtain independent backend-test-critic acceptance.
Use genuinely encrypted originals and the existing signed graph/paid fixtures.
Exercise missing predecessor then recovery, missing live child/data, exact
retries, direct/import dedup, newer roots, expiry and actual SQL failure/cold
restart. Check that failures preserve both message/MLS state and the displayed
observation; no attempted ordinal or refused transaction may increment imports.

Verify the shared read model through ordinary daemon/CLI/MCP and desktop entry
points, including contact permissions. Frontend checks must cover an empty local
page with pending remote history and already loaded messages during recovery.
Inspect headless screenshots at the existing compact and wide window sizes.
The native paid loss/recovery flow must show the intermediate gap and final
committed outcome across restart. Run targeted backend/frontend checks and the
applicable native gates; keep the full 67-card/22-E2E/three-platform scope open.
