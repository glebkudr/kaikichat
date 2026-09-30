# Paid independent index storage — bounded module acceptance

The backend now retains compact authenticated references independently of
ciphertext. Native verified funding supplies the public book commitment for a
stable index roster. Separate draw domains reuse the full-population sampler;
message/ticket/index/address changes cannot choose the roster. Legacy private
postage retains its old data path and cannot create a public book anchor.

New admission verifies the paid operation/resource, exact P256 spend QC and actual
selected primary's current registry proof and transport binding. An index receipt
binds the descriptor and exact immutable funding/context/QC evidence. SQLCipher
stores the obligation before a receipt escapes. Separate byte/entry quotas include
full serialized evidence; retries preserve the original receipt and deadline.
Private paged reads, expiration, monotonic clock and cold verification continue
after the short admission/binding expires. There is no ciphertext in the index.

## Tests first and independent review

- R1 RED reports missing new APIs. Critic required a genuine record mismatch,
  correctly re-signed outer receipts around corrupted funding/QC, and cumulative
  two-entry budgets. All were added before production; R2 FINAL ACCEPT followed.
- The first compiled implementation passed six of seven tests. The SQL recovery
  test incorrectly expected two full proofs within a 262144-byte page. R3 accepted
  exact recovery over two correctly bounded pages, preserving all SQL assertions.
- All seven then passed. R4 accepted one Vec-to-array lint correction with no
  assertion change. Hash manifests, decisions and failed-run metadata are retained.
- Tests use actual canonical native funding fixtures, independently encoded
  descriptors/placement, genuine P256 QCs and real SQLCipher INSERT/UPDATE triggers.
  Corrupted native funding/QC is refused even when a selected transport signs a
  new outer receipt binding the changed bytes. The full receipt oracle passes
  before the intended inner authentication rejection.

## Verified result

`checks.json`: **270 backend tests pass, 0 failed/ignored**, including all seven
new index scenarios and existing daemon process/network and custody regressions.
All **59 frontend**, **7 model**, **12 EVM-model** tests, workspace fmt/Clippy,
TypeScript and Vite pass. The 626 recorded source/test inputs stayed unchanged.
The ordinary macOS arm64 Tauri app was rebuilt; ad-hoc signature verification
passes and the default dependency graph excludes the automation driver.
`release.json` records commands, durations and executable hashes.

No new full-workspace, live-EVM funding or native UI gate is claimed for this
backend storage module. Previous d1c2fe3/1b28787 evidence remains historical.
Untracked user media and storage-maintenance notes were preserved.

## Still required

This is not full D05/E05–E07 or V1 acceptance. The daemon does not yet publish or
retrieve these independent index entries over the network. Actual data-holder
receipts/locations, sender-side index receipt persistence, network adapters,
ordinary sender/recipient integration, complete-range/gap evidence, lifecycle and
autonomous R10 index/data repair remain required; see [NEXT.md](NEXT.md).
All other remaining effective V1 messenger/platform cards and E2E remain open.
