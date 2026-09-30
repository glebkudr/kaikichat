# Continue through real successor spending and peer bootstrap

The goal remains the full 67 mandatory cards / 22 E2E / three-platform V1. This
source completes the application evidence-import layer, not AR1 or release V1.
See checks.json for actual test results. Do not present fixture signatures as
live multi-epoch consensus. Current SpendSession::new and prepare_client still
return HandoverRequired outside registry epoch 1.

1. Bind current successor selected authority AND an owner-bound SpentContinuity
   before creating a new SpendSession. A closing QC or new registry alone cannot
   authorize empty genesis. Recheck current Core fences and the continuity store
   capability on relevant decisions. Old spent keys stay issuer-global. Existing
   indexed proposal has no store argument and only checks current-epoch history:
   add an explicit complete-continuity proposal/verification path that rejects any
   old finalized nullifier before voting. Do not leave the old API as a bypass.
2. Add original-committee historical lookups. SpendSession::lookup currently uses
   one committee and cannot authenticate earlier-epoch records. Retain each old
   closing/source snapshot; bind a parsed committee hint only after authenticating
   the actual stored QC and exact history. Preserve original QC/time, including
   alternate valid retries. Completion/open already checks all older records with
   bounded memory; avoid rescanning all history on every warm request.
3. Separate role-free public chosen-epoch lineage from full validator spent state.
   An ordinary client must authenticate the cumulative closing choices and current
   public authority without downloading all SpendRecords. Link each choice back
   to epoch 1, allow genuine skipped epochs and retain current fence checks. Keep
   this proof separate from SpentContinuity's complete local evidence capability.
4. Integrate actual node lifecycle. Configure currently constructs SpendSession
   before persisting Saved, and State::prepare binds history before signer launch.
   Persist pending successor bootstrap and retain unresolved paid inputs; stop/join
   old signers before replacement. Activate only after complete application state
   and current selected authority. A partially downloaded store cannot sign.
   Cache checked source/index/continuity once per lifecycle, not O(N) each tick.
5. Add bounded authenticated ordinary-peer handover. Existing selected recovery
   admits old selected keys, so genuinely new selected nodes need a narrow read
   path for complete old history and original records. Reuse existing connection,
   presentation, discovery and rate/resource checks. Owner-provided public closing
   or routing evidence may seed discovery, but actual records must transfer over
   the network. Do not substitute owner RPC page copying for this gate. Fetch
   cumulative older epochs first; incomplete old state must stay unavailable.
6. Extend the native closing gate using its real 18 ordinary nodes, authenticated
   old/new/third rosters and dynamic post-freeze entrant. Require a fresh spend in
   the successor, refusal of an old paid ticket, exact original records, overlapping
   old validators and new members, partial-transfer crash/SQL failure, pending paid
   work preservation, partition and another transition. Pin each binary/source and
   independently verify real QCs. The new 160-ticket fixture/helper can support
   application tests; it is not an actual node gate.
7. Closing after old authority outage remains separate. Never revive ordinary
   old spending through historical verification. Also cover full ordinary queue
   closing admission, malformed mixed candidates and a hostile closing carrier
   through actual selected transport, as noted by the prior node critic.
8. Continue automatic client/sender authority/context renewal and reconciliation,
   sender retirement/fair scheduling, paid network history/R10, UI/CLI/MCP,
   groups/recovery, independent operators and three-platform release acceptance.

Implementation details to preserve:

- SpentHistorySource serves at most four entries with exact original SpendRecords;
  its open validates history and each page validates records. SpentContinuity.open
  performs the full cumulative scan. It does not trust a completion marker alone.
- Both fresh and existing-complete-history import paths commit progress with
  verified records, reject stale/foreign handles, and preserve original proofs.
- Generic HistoryImport::push_with_states forbids application writes to the
  finalizer/history namespace. Store's sixteen-state limit stays unchanged.
- Committee::for_log includes the base committee identity in config.log, so old
  epochs coexist in one store although the logical issuer log stays the same.
- Scope remains available on a closed node with an empty candidate list; removing
  the scope also disables its historical record server (a previously fixed bug).
- Source and Git remain /Users/glebk/Code/chat; use build-storage.py for all checks.
  Preserve unrelated Docs/maintenance/build-storage-cleanup.md and media/.
