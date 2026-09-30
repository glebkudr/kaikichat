# Independent backend test critic

Reviewer: separate `public_postage_test_critic`, originally created with no
inherited context. Review was read-only; no production changed before acceptance.

R1 **FINAL REVISE**: missing independent signature/epoch/signed-scope controls,
runtime revocation after exposing a stamp, positive ceiling below reservations,
and authorization/MLS/outbox conservation on first successful runtime execution.
Full RED: 11 missing-method E0599, no other diagnostic.

R2 **FINAL ACCEPT** before implementation. All hashes matched. Actual signature
corruption has coherent grant/job/policy IDs and independent InvalidSignature;
all current epochs are tested with positive restoration. Actual narrowed owner
grants pass independent allowed-scope checks but reject the queued send scope.
Removed/revoked/expired sponsorship rejects before and after allocation. A
positive ceiling of one below two reservations blocks both jobs until restored.
First runtime execution conserves authorization/MLS state, outbox and snapshot.
Full RED: 14 missing-method E0599, no other diagnostic.

Nonblocking suggestions retained: independently authored legacy policy fixture
and switching configuration to another actually funded book. No remaining
blocking issues or required missing scenarios for this Core boundary.

R2 execution tests passed on their first implemented run and in the complete Core
regression. No R2 execution tests or assertions changed after their acceptance.
The initial combined regression passed 271 Core tests but failed one existing
node timer fixture (95 node unit tests passed, one failed). Its strict client
Timeout assertion did not isolate the remote side's equal timer. Five isolated
diagnostic runs passed; a controlled real server-first deadline reproduced the
actual client I/O error. This supports a timer race, not a sponsorship failure.

R3 accepts the test-only correction: the server's response timer is placed beyond
the entire observation window while the actual production client still times out
after five seconds. Exact Timeout, held real response, slot/backoff/duplicate
checks, TCP and QUIC positive/negative cases remain. No production timeout or
handling behavior changed. Final complete node and frontend checks are rerun;
the 271 Core PASS remains valid because only the node test fixture changed.

The critic's
decision does not accept the daemon QC/storage loop or the full V1 application.
Raw RED/GREEN logs are local and ignored; frozen manifests and verified checks
are retained here.
