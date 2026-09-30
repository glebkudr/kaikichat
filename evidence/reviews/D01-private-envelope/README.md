# Private custody envelope — bounded D01 prerequisite

The entire existing signed MLS message/job packet is wrapped in an anonymous,
direction/epoch-specific encrypted envelope. Its immutable wire hash identifies
the future postage operation. Core persists exact preparation before returning
bytes; retries survive restart. Public validity grants no storage or payment.
Actual owner daemon APIs prepare and import the object, with the sender process
absent during import. The existing recipient reducer supplies deduplication and
the actual recipient acknowledgment. This is not yet a custodian store/get flow.

659 Rust tests (zero failed/ignored), 51 frontend tests, TypeScript, 7 model and
12 EVM-model checks, fmt/Clippy and all six existing hidden WKWebView flows passed.
Eleven new tests cover independent Python cryptographic vectors, correctly signed
hostile public envelopes, real MLS/SQLCipher, job and text handling, forbidden
Welcome/Receipt packets, wrong conversation, capacity/expiry, SQL failures and
strict owner-only IPC. Native UI is a regression gate, not a new custody screen.

R1/R2 REVISE and R3 ACCEPT precede production. R4 ACCEPT corrects four existing
fixture-generator/provenance paths after extracting the shared independent CBOR
encoder. Original mailbox vectors regenerate identically. The old-path EVM import
failure is retained in gates-r3-failed.json and the log hashes. Crypto/Core RED
was missing APIs at compilation; actual daemon RED reached unknown owner method.
All focused tests and then the full final regression passed.

545 frozen inputs cover the full final gate. gates.json records each command,
exit code, duration and log hash. app-release.json records the ordinary macOS arm64
app, five executable hashes, ad-hoc signature, exclusion of the test driver/helpers
and two historical genuine RISC0 receipt compatibility checks. The native gate uses
the debug test-feature app built from the same sources; the ordinary release
excludes automation. No notarization, Linux/Windows native or heavy genuine-spend
EVM rerun is claimed. Paired personal screenshot review is in visual-review.json.

To reproduce, retain source-inputs.json and the two runner scripts under
output/custody-envelope and execute from /Users/glebk/Code/chat:

    python3 scripts/build-storage.py run python3 output/custody-envelope/run-gates.py

The verifier also uses the repository's retained genuine receipt fixtures and
existing oracle runtime. Run logs/builds stay on the build volume; unique fixtures,
review records and evidence stay in Git. See spec/custody-envelope-v1.md for the
format and next-storage-work.md for the next actual custodian gate.

Full V1 remains incomplete. Actual paid admission, durable custodian receipts,
private retrieval/index placement and autonomous R=10 repair are still required.
The separate unstable 64-node capacity gate remains RED and parked.
