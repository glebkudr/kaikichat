# Independent backend test review

Baseline: b3d9b54. Reviewer: `/root/common_context_test_critic`, created without
inherited context and reused with standalone bounded instructions. FINAL ACCEPT
preceded changes to production code. The seven accepted inputs are recorded in
accepted-inputs.json; the preceding common/foreign/legacy helpers remain unchanged.

The initial REVISE found two blocking gaps. A sleeping verifier wrapper does not
observe abrupt parent death before exec, so a SIGKILL test could leave that helper
orphaned. The accepted revision instead verifies graceful shutdown of an active
held worker and abrupt daemon death after a genuine verification has completed.
It does not claim that the sleeping helper detects parent death. The second gap
was a missing positive full-history verification before checking non-retention.
The revision verifies successfully on a cold receiver, checks omitted/null history
fail both before and after restart, then explicitly pins and verifies successfully.
Completed-job head revocation was also added alongside pending-job revocation.
The critic returned FINAL ACCEPT with no remaining blockers.

The ordinary RED run passed the three existing IPC cases and failed the new case:
unknown_method instead of checkpoint_rejected for retain_postage_context. The final
EVM RED also failed the cheap capability probe with unknown_method before chain
setup or proving. Those baseline runs do not execute the later cryptographic,
peer-history, expiry or cleanup assertions. No production stub was added.

The accepted EVM test wraps and runs the complete preceding combined suite. It
uses actual EVM headers/proofs and the already independently verified genuine
receipt, with no caller clock or synthetic positive verifier. Eighty successors
travel over the real daemon peer protocol. Extra actual roots permit a short
current-head lease and renewal; their certificates are signed with the correct
successor linkage after the expiry change. Auxiliary nodes are cleanup-owned
before startup, and a wrapper-level failure always resets aggregate passed=false.

The first packaged EVM run produced and independently verified a genuine receipt
(350927 ms, 584953 bytes), then timed out during a peer catch-up wait. The
aggregate remained passed=false and cleanupErrors was empty. The original report,
receipt and logs are retained under output/postage-retained/failed-peer-sync-1/.

A separately reviewed test-only correction changes each 16-successor wait from
30 to 60 seconds and adds bounded per-batch progress diagnostics. The existing
scheduler waits 30 seconds after an empty page, then needs four requests with
5-second deadlines and 1-second progress pacing. The prior deadline could not
cover this normal sequence. The critic independently confirmed the exact delta,
unchanged acceptance assertions, all frozen hashes and existing scheduler tests,
and returned FINAL ACCEPT. Production and packaged binaries are unchanged. The
exact first-run cause was not captured; only the rerun diagnostics can support
the timing explanation. A diagnostic IPC error could obscure the original wait
error, a nonblocking limitation noted by the critic.

The complete rerun passed. The first batch took 8369 ms; the remaining batches
took 33837, 33825, 33787 and 33739 ms. All expected heads matched, with exactly
20 accepted pages, no rejected/failed responses and no rate-limited requests.
This supports the idle-poll explanation without claiming an unrecorded batch
state from the first failure. The two genuine receipts and all new/prior regression
checks passed on the unchanged ordinary release binaries.
