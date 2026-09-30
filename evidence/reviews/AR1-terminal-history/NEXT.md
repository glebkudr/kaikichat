# Continue full V1 from terminal history

Full goal remains 67 cards / 22 E2E / three platforms; AR1 and epoch handover open.

1. Make terminal meaningful for the actual issuer application. Authenticate a
   successor's FinalizerRoster with Core's installed trust at the same current
   checkpoint as the still-live old service. Do not replace the published old
   roster while it must finalize closure. Reuse FinalizerRoster::check_at and
   postage_scope; reject fabricated committee configs, foreign issuer/policy,
   stale checkpoint and non-increasing epochs. Return opaque checked authority.
2. Bind old issuer/committee and exact successor identity in a canonical closing
   payload. Reserve one deterministic terminal operation for that old issuer
   committee and bind it before the signer starts. All proposal/verify/delivery
   paths must distinguish closing entries from receipts. Gate one chosen successor
   with the old terminal QC and preserve it across candidate/record/ACK failures.
3. Bootstrap the successor only from complete authenticated prior spent history
   and its closing QC. New scoped committee IDs have different genesis; never
   turn that into an empty issuer spent set. Preserve original records and
   historical authority snapshots for every prior committee without new lifetime
   caps. Multiple registry epochs overlap; a later snapshot alone cannot revoke
   the old committee. Explicitly solve closing after an old-lease outage without
   reviving ordinary spending authority. Keep all epoch !=1 guards until real
   application and node crash/partition/double-spend/lease gates pass.
4. Continue client/sender authority acquisition/context renewal and stale-input
   reconciliation (see AR1-checkpoint-refresh/NEXT.md), successful sender
   retirement/fair scheduling, paid network history/R10, UI/CLI/MCP, groups/device
   recovery, independent operators and platform release checks.

The new terminal-history gate is generic signed operations on real deterministic
engines, not funded native epoch handover. The native 144-spend, recovery and
18-spend refresh evidence belong to their earlier revisions. Current tests use
unchanged Entry/QC codecs and quorum/bounds; no deployment or native packaging
was done in this slice. All checks use build-storage.py from the source checkout.

Source notes for the next test authoring pass:
- `competition.json` already has genuine epoch1/2 selections, but their current
  checkpoints are different roots (sequence6/7). It does not supply the old
  epoch's registry proof at the successor root. Generate both old and new proofs
  while the new local EVM is still alive; do not relabel the old proof's root.
- `tests/evm/postage_competition.py::next_epoch` reconstructs actual registered
  keys and has a hardcoded checkpoint sequence7. The current public native fixture
  already uses sequence7; adapt a reused hook to its actual predecessor/sequence.
- The existing spent namespace is already issuer/nullifier global, but
  SpendRecord::authenticate currently uses the session's single committee. Old
  records therefore need indexed historical authority by their certified committee
  when a successor session becomes possible. No change to nullifier derivation.
- Actual node upgrade must stop/revoke an already running policy-free service
  before binding terminal and restarting; the generic API contract explicitly
  requires configuration before signer startup. A captured earlier capability
  cannot retroactively acquire the new rule.
