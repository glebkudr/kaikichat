# Fresh native paid GUI — acceptance contract

The gate reuses `public_wallet_flow.py` and the independently checked real
`public_index_recipient.py` network scenario. Its new adapter is
`tests/evm/public_wallet_gui.py` with `native-wallet-driver.mjs`. Production code
for payment/send has not changed for this gate. The only new host behavior is the
isolated debug E2E vault described below. [Candidate 6 passes](GUI_ACCEPTANCE.md),
establishing the joined UI/payment/send flow. No failing payment baseline is
manufactured for functionality already implemented and separately tested.

Required observations:

1. Two isolated empty identities are created through hidden packaged WKWebViews.
   The recipient invitation and sender contact are also actual UI actions.
2. The sender explicitly selects checkpoint/issuer trust and public storage and
   finalizer registry/policy files through the trust UI. Preview alone installs
   no trust. Exact signed checkpoints are accepted through the UI. Existing
   peer-acquired client authority remains mandatory; manual sender committee or
   client configuration stays prohibited.
3. The wallet UI generates its own book UUID. Its exact visible price, ERC-681
   URI and transaction equal the normal daemon purchase. An independent Anvil
   signer decodes and pays that purchase. Before funding and during RPC failure,
   the UI cannot enable the budget. GUI refresh establishes actual verified funds
   before the inherited compatibility-sentinel import.
4. The owner saves the exact retention and budget through the UI, then sends two
   real messages through the composer. Original IDs, text and ciphertext remain
   bound to the existing independent QC/data/index/history oracle. The native
   view shows 10/10 storage and completed publication while the recipient is
   offline, without inventing a delivery ACK.
5. Inherited actual sender/data/index loss, missing recipient trust, both original
   import SQL faults, partial recovery and cold dedup all still pass. No owner
   retrieval or sender-work endpoint carries this workflow.

The shared native harness controls real hidden windows. It invokes only read
commands for assertions; writes use UI controls. Setting a `File` on the actual
file input replaces the picker, while keeping File.text(), size checks, React,
Tauri and Core in the path. No backend response or wallet UUID is mocked.

The test-only adapter seeds known random fixture database/owner secrets in a
private `0600` file before a fresh profile exists. It is restricted to isolated
managed `ain-spend-*/ciphertext-{alice,bob}` paths, refuses an existing file and
never accesses Keychain. E2E builds use the feature-gated `E2eFileStore`; normal
builds retain the system Keychain. Release compilation refuses the E2E feature.
Python owns the corresponding daemons; the native harness owns windows. All stop
before temporary profile/key cleanup. Real system-Keychain testing is a separate
explicit interactive opt-in and is not claimed by this gate.

This is macOS/local test-network acceptance, not external wallet compatibility,
new-agent contact onboarding, NAT, over-128 retained messages, other MLS epochs,
three-platform release or whole V1. Setup warmup and the inherited unpublished
sentinel are explicit fixture operations, not GUI user-flow claims.
