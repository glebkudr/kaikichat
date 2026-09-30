# Continue full V1

Same-epoch selected-validator envelope replacement is one AR1 step, not epoch
handover or full authority automation. Keep 67 required cards / 22 E2E /
macOS arm64 / Windows x86_64 / Linux x86_64. Do not close AR1 or V1 on this gate.

1. Specify and implement a unique authenticated epoch-closing boundary, successor
   authority and complete issuer spent-state transfer. Registry epochs overlap;
   a later registry snapshot does not prove that an earlier spending committee
   stopped. Keep the epoch !=1 spend refusal until actual crash/partition/double
   spend gates establish continuity. Never reset the log merely because the new
   committee has another scoped ID/genesis.
2. Complete authority acquisition and pending-context renewal for ordinary
   clients/senders. This slice uses explicit owner checkpoint/roster/configuration
   updates and refreshed owner submissions to one selected validator. Automatic
   history/proof acquisition and expired/unverifiable pending reconciliation are
   still required. Preserve original spent facts and never refund exposed tickets.
   Source inspection also found a read-model gap to test: configure clears the
   verified cache, while old inputs remain persisted. `Spends::read` derives
   pending status from that cache/recovery target and can report absent for an
   unresolved stale input before refresh. Preserve truthful unavailable/pending
   semantics when retained inputs cannot yet be authenticated; this gate does
   not claim to close that lifecycle/read-model case.
3. Finish successful sender retirement and fair ready scheduling, paid network
   history/index discovery and R10, ordinary UI/CLI/MCP entry points, groups/device
   recovery, independent operators and three-platform release gates. Follow the
   effective architecture/release graph, not a reduced preview scope.

All work stays in /Users/glebk/Code/chat; all builds/checks use build-storage.py.
New backend tests precede production and require a separate no-context critic.
Keep prior 144-spend lifetime and two-spend recovery evidence distinct from this
18-spend same-epoch renewal gate. Unrelated user maintenance notes/media remain
untouched. Do not mark the active full V1 goal complete for this commit.
