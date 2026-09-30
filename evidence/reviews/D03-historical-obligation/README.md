# Historical custody obligation verification

Baseline: 25a310e45b6ca2ab5f8f41941daf41033a0d2147.
Contract: spec/historical-custody-obligation-v1.md.
Raw build/test/proof logs: output/historical-obligation/ (outside Git).

Tests were written before production and reviewed by a separate backend-test-critic with no inherited context. R1 required isolated QC/snapshot corruption. R2 required preserving the certificate hex prefix. R3 FINAL ACCEPT preceded all production edits. Accepted test hashes remain unchanged. The initial attempt named a nonexistent Core test target; core-green.log records that command error and is not passing evidence. historical-green.log is the successful targeted Core/spend run; node-green.log is the actual daemon access test. The full workspace run verifies the final shared-placement refactor as well.

All 715 Rust tests passed with zero failures/ignored tests, four threads and 47 nonempty suites, including nine Tauri command tests. Runtime: 1045.317 seconds. Source inputs: 594; frozen manifest SHA256 2092ff0c9a4870c1aac6e4fb721f058c2f339142a061e8629c557e74dd0571e1. All 59 frontend tests, seven models, 12 EVM models, TypeScript, Vite, fmt and all-target Clippy passed.

All seven hidden native WKWebView scenarios passed. Actual Noise/MLS delivery retained 1051 exact messages after daemon restart and across all 22 UI history pages, with no duplicates and a real reply. All seven retained screenshot pairs were personally compared against the baseline, with no visual regression beyond expected timestamps. Only these seven captures and the two fresh result files were copied from the native output directory.

The ordinary macOS arm64 application was rebuilt and ad-hoc signature verified, excluding the test driver and helpers. Two historical genuine proof fixtures and the fresh receipt were independently checked with the packaged verifier, including operation substitution and expiry rejection. Proof image and packaged proof/verifier binary hashes remain compatible with the baseline. This is not notarization or Linux/Windows acceptance.

A fresh genuine proof paid for an actual 872-byte MLS envelope; 3 actual quorum signatures were verified. Proof generation took 398894 ms. The restarted custodian's strict owner command verified the exact assignment, nullifier, actual transport identity, primary position, object expiry and all 14 finite ordinals. After sender absence, automatic private pointer lookup/read recovered one original recipient message through storage failure/retry, custodian/recipient restart, unavailable endpoint and stopped first pointer cache. Cleanup reported no errors. The live daemon query ran during admission; separate genuine fixture tests demonstrate verification after admission expiry and rejection after object expiry.

The stored public bundle contains full ZK/QC/operator evidence. Isolated corrupted QC, invalid authority certificate/issuer proof, altered resources with genuine QC and re-signed storage receipt, changed proof/operation/envelope/operator, stale clocks, incompatible installed trust and actual SQL commit failures are rejected. Core reconstructs original public history after 80 head advances and restart without installing old heads or authorizing a new spend. Independent Python vectors cover all 14 original finite positions.

This gate uses one live primary. It does not establish independent R10 storage, independent durable index/control metadata, historical new-copy permission or autonomous repair. Full V1 and D03/E05-E07 remain open. The 64-validator/R24 engineering gate remains RED and parked; orders/ratings/reviews remain V2.
