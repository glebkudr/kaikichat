# AR3: automatic paid-index recipient retrieval

The ordinary recipient follows a private pointer to a paid index, verifies the
actual replying Noise key and original funding/QC, checks holder/copy claims,
then retrieves the exact descriptor-bound ciphertext. Message, MLS/dedup state
and the **index peer's** bookmark commit atomically. Existing authenticated
connections and current signed locator routes are tried before bounded
DHT/bootstrap/Core NodeRecord lookup. Legacy data locators retain direct reads.
Latest sender pointers now use confirmed index endpoints with all location ACKs.

Four real native gates pass on the same application source/binary:

| Final report | Verified behavior | Seconds |
| --- | --- | ---: |
| [Recipient](runtime-candidate-2.json) | Alice absent; two originals on different surviving holders; real deletion elsewhere; all pointer endpoints have zero ciphertext; no-trust rejection; rollback of both exact-original commits; cold retry/dedup/cache loss | 134.68 |
| [Owner/agent](runtime-sender-2.json) | Ordinary owner and signed-agent sends; genuine QC/R10; wrong-Noise copied offer refused; sender SQL failure/cold recovery; automatic offline recipient | 162.76 |
| [Direct compatibility](runtime-direct-compat-3.json) | Existing direct-data locator and historical copy/inspection/retrieval; recipient has no paid-index trust profiles; storage failure, sender absence and cold retry | 108.78 |
| [Index publisher](runtime-publisher-2.json) | 20 paid index promises /200 location ACKs; both sender SQL faults; cold sender/index recovery; no repeated index writes | 260.08 |

[Final checks](runtime-checks.json): **49 targeted backend tests, 21 frontend tests,
production Core/postage-spend/node Clippy and fmt**; 590 identical application/test
input fingerprints across these four runs, unchanged after final checks. All
native runs have zero cleanup errors. The binary SHA-256 is
`0ffdb0eaa3bd554f3eb2063c5cbfd0ec2e2bc2d943593b5ec45bcd81a322fce2`.

[Retained recipient evidence](runtime-recipient-evidence.json) includes both
original paid sender records/receipts, the actual publication, loss map,
original and surviving holders, missing-trust observations, SQL rollback state,
partial progress and exact recovered messages. It selects named fields without
altering their values from the full trace; its provenance records the full
trace's local path and SHA-256. Repeated status/network diagnostics remain under
`output/ar3-index-recipient`. Each report binds its fresh command, log, trace,
source fingerprint and binary; previous native output is archived before a run.

The independent context-free critic returned REVISE, then ACCEPT **before
production** after adding exact-original SQL faults and missing-public-trust
rejection. Later extraction of the public-trust setup and shortening a macOS
fixture path also received ACCEPT. No retrieval RPC or provider route is injected
into Bob by the harness. Public profiles are distinct from a recipient wallet.

Historical runs remain visible:

- `runtime-baseline-1`: publication and real data loss succeeded; old Bob made
  twelve direct completions at emptied endpoints, with no index traversal. The
  no-trust phase's expected index refusal never occurred; this is observed
  runtime RED for absent integration, not a setup or compilation failure.
- `runtime-candidate-1`: initial index-only loss scenario passed before the final
  fixture extraction and connected-route fix. It is not the current acceptance.
- `runtime-direct-compat-1`: failed before retrieval because a test AF_UNIX path
  exceeded macOS's limit. Only the fixture output path was shortened.
- `runtime-sender-1`: genuine index/location responses were received, but lookup
  spent its budget on DHT routes while usable connected/signed-locator holders
  were not preferred. The production routing order was corrected; the unchanged
  scenario then passed as `runtime-sender-2`.
- `runtime-direct-compat-2` and `runtime-publisher-1`: earlier successful
  regressions; superseded by the final same-binary runs above.

Reproduce from the repository root, using a fresh label for each native run:

```sh
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-recipient/run_runtime.py fresh-recipient-1
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-recipient/run_runtime.py fresh-sender-1 public_sender_daemon
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-recipient/run_runtime.py fresh-direct-1 public_paid_ciphertext
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-recipient/run_runtime.py fresh-publisher-1 public_index_sender
python3 scripts/build-storage.py run cargo test --locked -p agentic-core --test conversations custody_
python3 scripts/build-storage.py run cargo test --locked -p agentic-postage-spend --test paid_index
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib paid_custody
python3 scripts/build-storage.py run node apps/desktop/node_modules/vitest/vitest.mjs run --root apps/desktop tests/chat-shell.test.tsx
python3 scripts/build-storage.py run cargo clippy --locked -p agentic-core -p agentic-postage-spend -p agentic-node --lib --bins -- -D warnings
python3 scripts/build-storage.py run cargo fmt --all --check
```

The earlier Core prerequisite is retained in `core-checks.json` and
`TEST_CONTRACT.md`: seven new genuine MLS/SQLCipher tests, independent review,
59 missing-method compilation errors before implementation, then 22 custody and
21 frontend tests passing. Runtime work follows `RUNTIME_TEST_CONTRACT.md` and
[the implementation contract](../../../spec/custody-index-recipient-v1.md).

This is **one-book retrieval**, not finite history completeness. The latest
pointer still intersects live jobs' index rosters. Book/epoch continuity,
durable first Welcome, safe successful retirement and autonomous 10→7→10 repair
remain required. [Continuation](NEXT.md). The full V1 goal, 67 mandatory cards,
22 E2E and all three required platforms remain open; no full workspace suite was
run for this slice.
