# U05 durable network preferences

Baseline a9c9fd4. Tests first: three real AppCore/SQLCipher/OpenMLS cases preserve identity/history/pending message across preference save and reopen, reject stale changed values and scoped-agent attempts, inject actual INSERT and UPDATE failures, and deliver the original pending MLS message after recovery. Exact same-value lost-response retry returns the original revision. RED /tmp/ain-network-preferences-core-red.log reports the missing DTO and methods before production changes.

Separate context-free core_test_critic returned FINAL ACCEPT with no blockers. Optional improvements were retry after reopen and direct MLS state-byte comparison; current tests already compare exact pending ciphertext/IDs and complete an actual recipient decrypt/receipt after reopening.

Implementation reuses ProfileStore's encrypted state/CAS transaction under a bounded versioned namespace, independent of libp2p. The DTO bounds provider counts/text; node-specific multiaddr/PeerID validation and commit-before-live-apply belong to the following node adapter slice. No generic state-writing API or agent permission is introduced. Native controls and runtime application remain unimplemented at this checkpoint; full V1 remains active.

GREEN: all3 new cases passed (/tmp/ain-network-preferences-core-green.log), then complete backend/frontend gate passed169 Rust tests,20 frontend tests, strict formatting/Clippy, TypeScript and production Vite (/tmp/ain-network-preferences-host.log). Packaged/network gate last ran at a9c9fd4; its binary is still the current release artifact. The forthcoming node integration must test real live application, invalid routes, storage failure without disconnect, restart and persisted identity/history/listeners.

Subsequent node/native integration and current evidence are recorded in [U05-live-network-settings.md](U05-live-network-settings.md); the Core-only limitation above describes that earlier checkpoint.
