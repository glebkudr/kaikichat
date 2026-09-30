# Independent test review — public message owner IPC

Reviewer: `/root/public_postage_test_critic`, an independent agent originally
started without inherited context. No production adapter changes preceded
FINAL ACCEPT of the current R2 tests.

R1 reviewed the new strict owner route in the existing real-process permission
test and the extension of the actual public paid-MLS custody gate. The reviewer
checked the entire relevant source chain: after retrieval the funding history
contains the refreshed head, and the request retention derives from the original
stored envelope. All earlier actual QC, copies, SQL-failure, manifest, automatic
recipient retrieval and no-ZK tripwire assertions remain present.

The first attempted Cargo target was wrong (`registry` rather than `processes`).
Its exit 101 is preserved locally in `incorrect-target-r1.log` and is **not RED**.
The correct `red-r1.log` compiled the real process test and failed because the
new route returned `unknown_method`, where a recognized route must return
`invalid_request` for the malformed request. One test failed, zero passed.

R2 adds the reviewer's optional real funded negative controls before production:
a changed positive retention conflicts, an unknown message ID refuses, balance
stays at one allocated/three available, and the original request still returns
the exact prepared result. The existing Core error mapping is checked rather
than inventing a new adapter-specific error code. Three R2 hashes match; reversing
the added negative block restores the exact R1 hash.

**FINAL ACCEPT — current R2.** No blocking issues or required missing scenarios
for this thin adapter. The reviewer did not change files or run builds.

Implementation reuses Core's accepted atomic preparation, the existing bounded
authority authenticator and common error/serialization adapter. Targeted GREEN,
node/frontend regressions and the fresh live-EVM/network gate are recorded in
the separate command and module reports. This review does not declare the full
V1 application, autonomous sender or R10/repair ready.
