# Bounded desktop history checkpoint

Full V1 and full U01/U05 acceptance remain open. This checkpoint fixes the desktop
view that previously stopped at the first 1000 stored events and polled complete
message bodies every 250 ms. On 581 frozen source inputs, all 689 workspace Rust
tests passed with zero failed/ignored (four threads), together with 7 model and
12 EVM model tests, fmt/Clippy, TypeScript and frontend build.

Core provides owner-only contact pages (32 previews, 256 Unicode characters each),
latest text pages (50 messages, at most 512 KiB serialized) and a cheap native
invalidation counter. Existing SQLCipher profiles receive guarded event indexes;
message bodies, MLS state, outbox and agent inbox cursors are not rewritten.
The selected history is separate from previews. Exclusive message cursors are
conversation-bound and survive restart and new arrivals.

Four new Core tests use 1051 real alternating-direction MLS messages, queued/live
messages, three existing control events, 33 large Unicode messages and 33 contacts.
They verify exact originals, complete text unread counts, all older pages,
restart, limits, rejected owner APIs for agents, and read-only persistence.
The new native command test crosses two actual daemons and checks window/origin
denials. Independent test-critic REVISE/ACCEPT reviews preceded implementation.

All 59 frontend tests and 9 native command tests passed. All 7 hidden WKWebView
scenarios passed, including 1051 actual Tauri->Noise/MLS->recipient messages:
50 recent rows, restart, 22 explicit older-page loads with exact IDs/text and no
duplicates, then a real UI reply. Five existing screen pairs and both history
views were personally compared against the previous native layout.

The first native run found unbounded accumulation during live updates
(`1051 != 50`). A new frontend test reproduced it before the fix. Another RED
frontend test preceded refresh of delivery receipts on loaded older pages.
These failures remain evidence, and neither native assertion was weakened.

The ordinary macOS arm64 app was rebuilt and ad-hoc codesign verified. The test
driver and test helpers are excluded. Two historical genuine RISC0 receipts passed
independent oracle and packaged verifier compatibility checks; operation
substitution and expiry were rejected. Proof/verifier binary hashes are unchanged
from the preceding checkpoint. A new paid proof and full EVM matrix were not rerun.
The app is not notarized; Linux/Windows acceptance remains open.
This paginated renderer does not yet implement the full virtualized-history,
uploads/search/notifications/lock/recovery scope. Independent replicated indexes,
authenticated gaps, automatic sender placement, R10 and repair remain V1 work.
The 64-validator/R24 engineering gate stays RED and parked. Orders/ratings/reviews
stay V2; their existing shared regressions are retained.

Raw run logs stay in managed `output/desktop-history/`, outside Git. Checked
manifests, reports, public synthetic histories and screenshots are retained here.
Contract: `spec/desktop-history-v1.md`.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
Source manifest SHA256:
`0e8bd83d24747f394ddcc0ecd5bcb4d4d66e15bec278dc48f76b5aeac5970e0b`.
Detailed binary hashes: [app-release.json](app-release.json).
