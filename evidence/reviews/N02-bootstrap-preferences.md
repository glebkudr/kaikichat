# N02: native bootstrap settings

Status: complete native/release and Linux regression gates PASS on 2026-09-05. This delivers native hint entry; complete N02 and V1 acceptance remain open.

## Observable contract

The owner enters up to four distinct bootstrap peer routes in «Peers for joining the network». Saving changes the live daemon and persists the list in the encrypted profile. Saved preferences replace CLI hints at restart. The existing signed cache remains an independent source: clearing the explicit list does not block previously verified peers, which the form explains.

`NetworkPreferences.bootstrapPeers` is always present in new output. An absent field in legacy version1 state/input means an empty list. Reading or retrying unchanged legacy preferences does not rewrite persisted bytes or increment the revision. Each route retains the existing 256-byte bound; the daemon validates supported IP transports, complete PeerIDs, distinct remote peers and one-hop relay routes. No dependency was installed.

The existing owner-only CAS path validates/constructs replacement discovery and transport before persistence. It cannot dial a new hint on invalid input, stale revision or SQL failure. A successful change retires the prior swarm while preserving identity, listener ports, IPC, history and queued ciphertext. Exact same-value retries return the existing revision and preserve live connections. Scoped agents and foreign Tauri origins/windows cannot change settings.

The panel distinguishes live signed peers from raw connection count, displays an actionable empty state and relay-only blocked hints, preserves edited values during polling, and retries the original revision after a lost reply. Verified keys do not imply reputation or registry/checkpoint acceptance.

## Tests first and independent review

- Core RED: `N02-bootstrap-preferences-core-red.log`, missing new DTO field. Two new tests plus three extended existing cases cover independent legacy serialized state, bounded input, exact retries, revocation/owner scope, SQL INSERT/UPDATE rollback and actual queued MLS delivery. Separate context-free core critic returned FINAL ACCEPT before implementation. Focused GREEN: five tests in `N02-bootstrap-preferences-core-green.log`.
- Node RED: `N02-bootstrap-preferences-node-red.log`, actual owner configure requests rejected by the old daemon. `crates/node/tests/support/bootstrap_settings.rs` adds three actual-process scenarios using the existing independently implemented raw libp2p peer, real SQLCipher failures, MLS messages and signed receipts.
- The node critic initially required stronger evidence: a reachable CLI sentinel must prove replacement rather than merged hints, and forbidden dialing must be observed across real activity after failed persistence. Both were added before FINAL ACCEPT and production changes. Tests keep SQL triggers active during successful peer delivery, assert zero requests at the new endpoint, and prove saved-hint initiation after restart with the disposable cache cleared. A live configure case verifies relay-only via actual circuit delivery and zero requests to a forbidden direct endpoint. Focused GREEN: three tests in `N02-bootstrap-preferences-node-green.log`.
- Frontend RED: `/tmp/ain-bootstrap-preferences-ui-red.log`, three failures before implementation; focused GREEN: seven API/panel tests in `N02-bootstrap-preferences-ui-green.log`. New scenarios cover polling, exact lost-reply retry, five-hint rejection, clearing the list, blocked policy diagnostics and raw connection versus verified signature distinction.
- Native RED: `N02-bootstrap-preferences-native-red.log`, the old actual packaged WKWebView times out waiting for the new bootstrap form. The independent node critic accepted the new native scenario and Tauri command assertions. The new scenario uses a separately spawned bundled provider and a fresh GUI-created profile, saves through controls, verifies reciprocal signed handshakes in both real daemons and no automatic conversation creation. Existing actual relay/AutoNAT chat and restart/receipt assertions retain their full behavior, now also checking restored bootstrap input and disabled unchanged Save.

## Relay regression oracle correction

The first full gate exposed an obsolete single-circuit assumption in `crates/node/tests/support/relay.rs`. After cache bootstrap recovery, two reciprocal application circuits can exist through the same provider; the inbound circuit reports the provider in `localAddress`, while the outbound circuit uses `remoteAddress`. The failure is retained in `/tmp/ain-bootstrap-preferences-native.log`.

The corrected helper checks **every** live application connection: all must be relayed and all must identify one common provider. The test kills precisely that provider and retains survivor, original queued operation, autonomous delivery, exact histories and signed receipt assertions. Multiple providers remain a failure at this boundary, so a pre-existing alternative circuit cannot masquerade as failover. A separate FINAL ACCEPT approved the test-only correction; focused GREEN is retained in `N02-bootstrap-preferences-relay-green.log`. No production connection limit was weakened.

## Visual and network evidence

Own visual comparison used `output/playwright/bootstrap-network-wide.png` and the compact Save/details views alongside the preceding network panel references. Actual `output/native-e2e/bootstrap-alice-connected.png` and `network-alice-restored.png` were then viewed alongside the current component reference. Long settings scroll; the native post-save screenshot is scrolled to the disabled Save control, while the restored screenshot shows the header, preserved bootstrap route and verified/blocked-policy status. No overlap or inaccessible controls was found.

Linux gate: `/tmp/ain-bootstrap-preferences-network-final.log`, all six actual network scenarios PASS on final sources, sourceHash `ff447a4f0728bda536fadc2026d587d589503ab8d2f8ca6649c11ba102f5e96c`, run `ain-nat-04996e24`, cleanupErrors empty. The earlier successful Linux run preceded the reviewed test-helper correction and is not the final source evidence.

Native/release gate: `/tmp/ain-bootstrap-preferences-native-final.log`, 201 Rust tests (core57, node unit11/process42, other suites unchanged), 29 frontend tests, strict fmt/Clippy, TypeScript/Vite, all four actual hidden WKWebView flows and the rebuilt macOS arm64 release with deep/strict ad-hoc signature verification. Normal release excludes WebDriver. Apple notarization and other-platform acceptance remain open. Final native log is retained as `N02-bootstrap-preferences-native-final.log`.

## Remaining work

Local mDNS, partial Kademlia/private rendezvous, independent deployed seeds, authenticated registry/checkpoints and full discovery acceptance remain unfinished. The current application still needs the full R=10/offline storage, group, jobs, trust, transport economy and remaining lifecycle/platform work specified by the V1 plan. These tests do not close E01–E26 or the overall goal.
