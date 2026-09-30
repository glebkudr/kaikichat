# Original outgoing packet archive — verified checkpoint

Baseline `c5552acb74fe8266cada97a98c3427474d3e77e7`. Contract:
`spec/outgoing-archive-v1.md`. This is a required local transport prerequisite,
not D03/D05 or full V1 acceptance.

Acknowledgment now moves the exact signed outbox packet into the same encrypted
profile transaction before consuming delivery work. A first custody preparation
after actual direct receipt/restart uses those original bytes. Failed INSERT or
DELETE rolls back the acknowledgment; retries preserve message/operation identity,
MLS and queue state. The archive adds no network replicas or automatic placement.
Version1 pending profiles upgrade without losing packets; old delivered rows whose
wire was deleted return explicit unavailability instead of creating another packet.

## Test-first evidence

Independent context-free backend-test-critic R1 ACCEPT preceded production.
Core baseline has 3 executed failures, store baseline 10 missing-method compile
errors, actual Noise daemon baseline 1 executed post-ack export failure. R2 corrected
two test derivation arguments from sender to recipient, with another FINAL ACCEPT;
all exact-byte and behavior assertions remain. The initial post-implementation
Authentication failures are preserved. See test-review.md and reviewed diffs/hashes.

## Verification

On 584 unchanged source inputs (manifest SHA256 `8db222f8b10931370ae3ce0b01547e1c1217400f3b54d1fce9d7d7e2bf075de9`):

- 696 Rust tests,0 failed/ignored, 47 nonempty suites, 4 threads, including 9 real Tauri
 command tests. Total backend runtime 1068.842s.
- 59 frontend tests; 7 model and 12 EVM model tests; fmt/all-target Clippy/TypeScript/
 frontend build passed.
- 7 actual hidden WKWebView scenarios passed. The history scenario transmitted 1051
 messages, restarted the recipient daemon, read all 22 UI pages with exact original
 IDs/text and no duplicates, and sent a real reply.
- 7 current screenshot/reference pairs were personally viewed; no visual regression.
 The retained jobs scenario is shared V2 regression coverage, not new V2 development.
- Ordinary macOS arm64 `.app` rebuilt, ad-hoc signature verified, with test driver,
 test-state helper and fan-in helper excluded. It is not notarized.
- Two genuine historical RISC0 receipts were verified by the independent oracle and
 packaged verifier; operation substitution and expiry rejected. Proof/verifier
 binary hashes are unchanged from baseline. No fresh paid proof or full EVM matrix
 rerun was needed for this archive-only change.

App: `/Volumes/ChatBuild/rust-target/release/bundle/macos/Agentic Internet.app`.
See app-release.json for exact binary hashes; regressions.json for checked results.
Raw logs remain outside Git under `output/outgoing-archive/`, authenticated by
raw-log-hashes.json. This directory holds checked reports, diffs and fixture screenshots.

## Reproduction

From `/Users/glebk/Code/chat`, use the build-storage wrapper for all commands:

```sh
python3 scripts/build-storage.py run cargo test --locked --workspace --all-targets -- --test-threads=4
python3 scripts/build-storage.py run cargo clippy --locked --workspace --all-targets -- -D warnings
python3 scripts/build-storage.py run cargo fmt --all -- --check
python3 scripts/build-storage.py run node scripts/build-desktop.mjs --debug --e2e
python3 scripts/build-storage.py run env AIN_DESKTOP_BINARY='/Users/glebk/Code/chat/target/debug/bundle/macos/Agentic Internet.app/Contents/MacOS/agentic-desktop' node apps/desktop/tests/native-e2e.mjs
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
```

Exact frontend/model commands are recorded in support-checks.json. Do not run the
parked 64-validator/R24 engineering gate as part of this reproduction.

Durable sender receipt aggregation, automatic paid placement, independently retained
metadata, authenticated finite frontiers/gaps, R10 and repair remain required.
Full virtualized history and remaining daily UI/groups/recovery/attachments/credentials/
platform acceptance are still open; orders/ratings/reviews stay V2.
