# Native checkpoint trust selection

Date: 2026-09-05. This increment connects the existing durable checkpoint verifier to
the native owner UI. Full V1/L06/E01–E26 acceptance remains open.

## Test-first evidence

Two new Core tests, one real-daemon Tauri command test, nine frontend behavior cases,
one invoke-mapping test and the actual hidden WKWebView scenario preceded production.
The separate originally context-free node test critic returned FINAL ACCEPT, and the
parent waited for that verdict before further work. See `L06-checkpoint-ui-critic.md`.

The Core RED failed for absent preview/trust APIs; Tauri RED failed for the absent
checkpoint command permission. Frontend RED failed for the absent panel/API. The first
frontend invocation ran without the desktop config; the retained final RED reran with
the correct jsdom/Vite configuration. A first Tauri GREEN attempt found the old standalone
daemon; after rebuilding that actual binary, the command integration passed.

## Observable behavior

- Core validates raw profile/issuer files through the existing strict types and returns
  a normalized metadata summary without persisting anything. It requires an identity,
  rejects wrong networks/issuers and duplicate JSON keys, and cannot replace installed
  trust. Status returns the same validated selected keys and limits after restart.
- Epoch/minimum-block identifiers are decimal strings, chainId is full-width hex and
  key order is canonical. No credentials, raw stored files or certificates are returned
  in the public summary. Small readonly getters reuse the existing validated types.
- The owner daemon has a strict preview request. Tauri registers only preview, status,
  install and accept, with explicit main-window capabilities. Other windows and remote
  origins are denied; the funding verifier remains unavailable to the WebView.
- JSON file import preserves exact text and enforces the 16 KiB file limit. Preview shows
  the full signer list and parameters; explicit consent is required for installation.
  Any file edit clears preview and consent. Selected trust is immutable in this profile.
- Signed certificate import retains the exact request/revision after a lost response.
  Polling cannot silently rebase retries or overwrite a newer successful installation.
  Explicit refresh retains the draft while allowing a new revision. Expiry, clock rollback
  and unavailable daemon have separate states; unavailable status removes an active claim.
- Successful selection/acceptance scrolls the native panel to its resulting status,
  including in a compact window.

## Native and visual scope

The new native flow uses controls in a packaged hidden WKWebView, the Tauri bridge,
bundled daemon and encrypted durable profile. It checks that preview has not installed
trust, then selects it, accepts an independently signed checkpoint, stops UI and daemon,
and verifies exact trust/head/lease/identity after reopening. The fixture explicitly uses
a synthetic current timestamp over the committed Anvil header. It proves the native
selection/acceptance path, not live chain finality. The existing real Anvil/daemon funding
gate remains mandatory and independent.

Headless Playwright screenshots at 1280×840 and 900×650 were inspected visually alongside
the existing network panel reference. The layout, key wrapping and compact resulting
status were checked. `output/playwright/checkpoint-*` are presentation fixtures; actual
native screenshots are under `output/native-e2e/checkpoint-*`. Browser/Vite processes from
this review were explicitly closed; no browser window took focus.

Certificates are still imported manually. There is no built-in provider manifest, live
observer, payment flow, spend authority or available balance in this increment.


## Final aggregate evidence

- `L06-checkpoint-ui-full-native.log`: terminal exit0; fmt/Clippy, 272 Rust tests,
  14 Solidity tests including 256 fuzz sequences, 40 frontend tests, TypeScript/Vite,
  all earlier EVM verification, five actual native flows, release bundling, deep/strict
  ad-hoc signature and no automation driver in the default release dependency graph.
- `output/native-e2e/result.json`: passed:true, five outcomes. Native active/restored
  screenshots were inspected alongside the component reference; actual keys are shown
  in the same layout and the status/lease survive restart.
- `output/evm-e2e/checkpoint-node.json`: passed:true, 53 actualOwnerCalls, four successful
  funding verifications, two profiles, real expiry denial, cleanupErrors empty;
  sourceHash `faa3210932ac8621e12c67871b59a681c75560a564e01dbdf22ab8bba65664fb`.
- `output/evm-e2e/trusted-checkpoints.json`: passed:true, 12 actual Rust CLI verifications,
  two profiles, cleanupErrors empty; sourceHash
  `82a5e4b073da9ca94d8d3e091b305c1cfcd9076c80bd84fce08c0d3ac945bc59`.
- `L06-checkpoint-ui-linux.log`: terminal exit0. Seven real Linux network outcomes pass,
  run `ain-nat-739ead40`, sourceHash
  `2c92d44cf30f69fac055970fd4354263527a6003c71f17071e27ecea301a8638`.
  Cleanup errors are empty; exact run-label container and network queries return empty.

No dependencies were installed or upgraded. Apple notarization, other platform acceptance,
automatic checkpoint observation, paid admission/custody and full V1 remain unfinished.
