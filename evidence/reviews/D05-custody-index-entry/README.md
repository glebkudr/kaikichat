# Compact custody index entry and owner export

The shared crypto/Core/daemon can now export a bounded, independently verifiable
description of an exact retained envelope without its ciphertext. It reuses the
existing custody exporter key, SignedDocument, canonical CBOR and envelope
verification. The original hash, index, epoch, sequence, byte length and finite
deadline are bound. Verification of a matching descriptor never replaces
authentication/decryption of the actual envelope.

Core reuses the existing validated SQLCipher envelope loader. Export is read-only:
no envelope creation, allocation, MLS/sequence change, outbox requeue or delivery
ACK. The strict owner IPC `export_custody_index_entry {messageId}` returns only
`{wire}`; signed messaging grants and caller field/time overrides are refused.

## Tests and evidence

- Independent Python/CBOR/Ed25519 fixture generator and vectors live in
  `crates/crypto/tests/fixtures/custody-index/`. Five new crypto tests include
  exact deterministic wire, signer/index/retention and semantic refusals,
  valid inconsistent signed claims, a valid descriptor for a signature-damaged
  envelope with its exact recomputed hash, and a real 48,000-byte packet whose
  compact record does not contain the packet or ciphertext.
- Two new Core tests use real SQLCipher/MLS/public-book preparation. They check
  exact export after ACK/restart with unchanged state and full outbox, unchanged
  original stamp/balance, unprepared/foreign/missing/expired refusal and actual
  corrupted saved metadata without healing. SQL mutations affect exactly one row.
- The extended actual-process test uses ordinary Noise send/ACK, both daemons
  stopped, owner restart/export, independent returned-envelope verification,
  strict owner/grant boundaries, exact second-restart retry with zero outbox,
  then sender-off recipient import with exact message deduplication.
- `red.json` and raw local `red-crypto.log`/`red-core.log`: missing new APIs.
  `red-daemon.json` and local log: an actual unknown owner method fails the
  required unprepared-export response. These are RED, not acceptance results.
- `critic-review.md` and `critic-inputs-r*.json`: independent no-context review.
  R1 requested two stronger assertions; R2 accepted before crypto/Core production.
  R3 accepted two test-only unused-mut removals. R4 accepted the real owner IPC
  extension before its production route. Reviewed final hashes are unchanged.
- `failed-checks-1.json` and `failed-checks-2.json` retain the two lint failures.
  Raw logs stay local/ignored; no skipped failure is counted as success.
- `checks.json` and `final-inputs.json`: all **35 crypto** and
  **468 Core/daemon** tests pass, zero failed/ignored, along with
  the targeted two Core tests, workspace fmt/all-target Clippy, 59 frontend,
  7 model and 12 EVM-model tests, TypeScript and Vite. 647 final inputs
  and all reviewed inputs remain unchanged. The two targeted Core tests are
  included in the full Core count, not additional unique tests.
- `release.json`: ordinary macOS arm64 app rebuilt; ad-hoc signature verified
  using `codesign --verify --deep --strict`; default graph excludes the native
  automation driver; final executable hashes and bundle location recorded.

All commands run through `python3 scripts/build-storage.py run ...` in the
canonical repository. No dependencies were installed or upgraded. No new live
EVM, full-workspace test run or native UI screenshot run is claimed for this
descriptor module. The preceding automatic sender module at `d1c2fe3` retains
its genuine live gate, 791-test workspace and seven native/screenshot outcomes.
The current release bundle includes this descriptor module.

## Remaining V1 work

This compact record is discovery metadata, not a payment check, retained index
receipt, available ciphertext, repair authorization or complete-range proof.
Independent paid index persistence/placement, authenticated reads and cursor/gap
reporting, autonomous R10 repair and the remaining messenger/platform release
requirements are still open. Actual MLS epoch handover was not newly exercised;
the exporter fails closed when the current capability cannot reproduce a retained
epoch. No new wallet UI or full CLI/MCP lifecycle acceptance is claimed.
Full D05/E05–E07 and V1 remain incomplete. Orders/reviews stay in V2; the parked
64-validator/R24 live engineering gate was not invoked.
