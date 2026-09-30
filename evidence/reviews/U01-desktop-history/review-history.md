# Test-first review

Baseline c0890808e1b5b9a6bdf42937a021568fda79eeec. No new production code was written before final ACCEPT. Reviewer: separately spawned `desktop_history_test_critic`, fork_turns=none, using backend-test-critic.

R1: REVISE. Agent denials used malformed/out-of-scope requests and did not establish owner-only access. Core RED: 21 E0599 errors for absent APIs.

R2: ACCEPT. Valid owner request shapes tested against the granted conversation, persisted state unchanged; exact preview checks, incoming revision/read stability and three real interleaved existing job events added. Core RED: 24 E0599 errors for absent APIs. Existing V2 job events are regression fixtures, not new V2 work.

R3: ACCEPT. Added hidden WKWebView scenario for 1051 real native messages, recipient daemon restart, all22 UI pages with exact original IDs/text and a reply. Rustfmt and borrowing the author string preserve assertions. Reviewer checked both JS files syntactically and verified hidden/unfocused launch/argument forwarding. Native execution and visual inspection remained pending at acceptance.

All raw logs remain in managed output. These reviewed tests exercise local history and owner permissions; they do not establish full V1, authenticated completeness, R10 or repair.

## Post-implementation UI findings

An additional frontend regression was written and observed RED before fixing receipt refresh on already loaded older queued messages. It reuses the bounded history API, with no new backend API or permissions.

The first actual native run completed1051 real sends and rendered the newest message, then failed `1051 !== 50`: merging overlapping live pages accumulated the whole unexpanded history. Native R1 result/log and screenshot are retained. A focused frontend test reproduced accumulation before the UI fix. Only an explicitly requested older page now expands the window. Both regressions and the complete59-test frontend pass. Native R2 and complete workspace remain pending at this note.

Final native R2 passed all7 scenarios, including1051 messages/all22 pages/restart/reply. All689 workspace Rust tests passed with zero failed/ignored;59 frontend,7 model and12 EVM model tests passed. The ordinary release app was rebuilt and codesign/driver exclusion plus two historical genuine receipt compatibility checks passed. Seven screenshot pairs were personally inspected. Full V1 is not accepted.
