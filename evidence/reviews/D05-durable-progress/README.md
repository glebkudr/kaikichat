# Durable recipient traversal — verified application checkpoint

Full V1 remains incomplete. This checkpoint fixes traversal across recipient restart
and bounded scheduler visits. It does not close all of D05 or E05–E07.

On 576 frozen source inputs, **684 Rust tests passed**, zero failed/ignored, four
threads, 47 nonempty suites. Also passed: 51 frontend tests, 7 model tests, 12 EVM
model tests, fmt, all-target Clippy, TypeScript and frontend build. Eight actual Tauri
command tests and all six hidden WKWebView scenarios passed. Five paired screenshot
views were personally inspected; see [visual review](visual-review.json).

The ordinary macOS arm64 app was rebuilt and ad-hoc codesign verified, with test
driver and helpers excluded. It is not notarized. Linux/Windows acceptance remains
open. App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
[Binary hashes](app-release.json), [gates](gates.json), [Rust report](regressions.json),
[source manifest](source-inputs.json).

## Actual traversal and failure evidence

The independent Noise fixture delivered **128 original MLS messages**, 5748073
ciphertext bytes, 26 pages and 27 read requests including a held request. The sender
was stopped. After Bob imported 10 originals and was killed with the third request
held, its first signed restart cursor was 10. The 17th resumed request required a
new lookup/visit; the existing 16-page/120-second work limits were preserved. Exact
IDs, text, authors and duplicate-free rows were checked. The recipient harness only
observed state. [Result](backlog-green-evidence.json), [trace](backlog-green-trace.json).
This fixture makes no paid-storage claim.

The original behavioral RED captured `[0,5,10,0]`, expecting 10 after restart.
The first setup failure used text that left insufficient space for signed MLS
overhead; it is retained as failed setup, not behavioral proof. The corrected corpus
still exceeds the actual page byte budget. Independent Core and Runtime reviews
preceded production; review revisions and rejected gaps are in [history](review-history.md).

Six new Core tests cover real SQLCipher progress INSERT/UPDATE and final-message
INSERT faults, atomic cursor/message persistence, reopen/retry, source/conversation
isolation, out-of-order recovery, stale responses, empty-page reconciliation,
four-source rotation and actual count/byte limits. The common complete-page
validator serves ordinary and automatic reads. Existing MLS import and durable
dedup perform all message application.

The 33-message Noise regression passed again. A fresh genuinely paid 872-byte MLS
ciphertext passed ordinary custodian storage/restart, unavailable-endpoint fallback,
actual SQLCipher failure/retry and recipient restart with one DHT cache absent.
The sender remained stopped and no owner retrieval calls drove Bob.
[Paid result](network-evidence.json), [trace](network-trace.json).

The full fresh RISC0 receipt is retained, SHA256
`3a74bc5f2dbc38fd727fbb1fd9c7d44585079d035d7061d4168e42f6e0941650`.
The independent oracle and newly packaged verifier reverified it after cleanup;
operation substitution and expiry were rejected. No private witness was retained.
[Verification](fresh-receipt-verification.json). Two earlier genuine receipts also
passed packaged compatibility checks. A complete historical EVM matrix was not rerun.

## Remaining product work

A local encrypted traversal bookmark is not proof of complete history. Independently
replicated index/control logs, authenticated missing ranges, automatic sender
placement, R10 and autonomous repair remain required. The 64-validator engineering
gate remains RED and parked. Orders, ratings and reviews remain V2; their existing
regressions are preserved.

Core desktop snapshot currently reads the first 1000 records per conversation;
newer records past that bound are absent, and sufficiently large combined histories
can exceed the 16-MiB IPC response budget. Bounded recent history and explicit UI
pagination remain required before release. This 128-message gate does not claim
that larger-history UI behavior.

Raw run logs remain in managed `output/custody-progress/`, following AGENTS.md;
[hashes](raw-log-hashes.json) identify them. Checked JSON traces, public proof,
source manifests, screenshots and review diffs are retained here.
