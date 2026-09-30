# Automatic private recipient retrieval

The ordinary daemon now discovers a private mailbox pointer and retrieves its
original paid MLS message with the sender stopped, without owner lookup, connect,
read or import calls for the returning recipient. The live gate passed unavailable
first-endpoint fallback, an actual SQLCipher read failure and retry, custodian
restart, recipient restart with one of two independent DHT caches stopped, and
exactly one original message. This is one paid primary. Sender-side placement and
pointer publication are still explicitly orchestrated by the harness.

On 572 unchanged source inputs, all 678 Rust tests passed (zero failed/ignored,
four test threads), alongside 51 frontend tests, 7 model tests, 12 EVM model tests,
TypeScript, frontend build and fmt/Clippy. Eight Tauri command tests and all six
hidden WKWebView scenarios passed. Five result/reference screenshot pairs were
personally inspected. The ordinary macOS arm64 app was rebuilt, ad-hoc codesign
verified, and checked for exclusion of the test driver/helpers. It is not notarized;
Linux/Windows native acceptance is open. `app-release.json` records all five binary
hashes. Native tests use the debug test-feature bundle from the same sources.

`network-evidence.json` and `network-trace.json` retain the live result and public
synthetic protocol evidence. A freshly proved RISC0 receipt binds the actual
872-byte ciphertext. Three P256 QC signatures and the complete Ed25519 storage
receipt were independently verified. The full public proof is retained as
`fresh-receipt.json`; `fresh-receipt-verification.json` records its subsequent
independent oracle verification and verification by the packaged CLI, including
operation-substitution and expiry refusal. No private witness/profile was retained;
process/profile cleanup succeeded. Two historical genuine receipt compatibility
checks also passed. The new proof took 345852 ms in this run.

The separate Noise pagination fixture serves 33 genuine MLS envelopes, selecting
pages from the actual signed cursor. All original IDs/authors/texts arrive once,
including after recipient restart and another real read. The latest run captured
eight requests and checked advancing cursors, independent Ed25519 signatures,
signer-to-index derivation, document kind/epoch and finite capability/byte bounds.
It makes no paid-storage claim; see `pages-green-evidence.json` and its trace.

Core scans ordered conversation IDs without loading histories or changing state.
Four background work items reuse the existing four outbound custody slots. Automatic
mailbox discovery uses at most one of the two shared DHT query slots, preserving
room for peer/owner lookup; owner joins promote a background lookup. The full
backend run initially exposed slot exhaustion. A real held GET_RECORD regression
was written and independently accepted before the fix. It and all six routing
process scenarios now pass, also in the complete run. The original failures,
preimplementation RED and R1–R7 critic decisions remain retained.

Execution remains bounded: 120 seconds and 16 successful pages per work item,
six seconds per dial, 30 seconds between scans, 24 background reads per minute and
12 per peer (below existing receiver limits). Diagnostics reset at process startup.
A pointer or counter never proves complete history, durable copies or delivery ack.

**Known remaining limitation:** cursors belong to the in-memory visit. A restart
or new visit begins at zero, so a sufficiently long/large valid retained backlog can
exceed the budget and fail to catch up. Durable progress, authenticated range/gap
information and independently durable index/control logs remain required V1 work.
Automatic sender placement/publication, R10 and autonomous repair are also open.
This directory is an implementation checkpoint, not completion of backlog card
D05, E05–E07 or the full release.

The prior intermittent announcement retry test passed unchanged in the full run;
its earlier cause remains unestablished. The 64-validator/R24 engineering gate
remains RED and parked. This checkpoint does not rerun that gate or the entire
historical EVM suite. Existing private jobs remain V2 work; no new job features
were added. No external dependency was installed.

Reproduce from `/Users/glebk/Code/chat` through the storage wrapper:

```sh
python3 scripts/build-storage.py run python3 tests/evm/custody_sync_pages.py
python3 scripts/build-storage.py run python3 tests/evm/paid_ciphertext_delivery.py
```

`backend-gate.py`, `run-gates.py`, `verify-app.py` and `verify-fresh.py` preserve this
checkpoint's invocation and assertions; they write to local build storage under
`output/custody-sync`. `source-inputs.json` binds the regression/build input files.
