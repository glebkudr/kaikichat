# Current application verification, 9 September 2026

All checks in regressions.json and native-run.json passed on the exact 525-file
source-inputs.json snapshot (SHA256 8f586caa577feeb3b6f88a91922712b8b92e8b5e639bbcb88e88ed55df4abfa3).
629 Rust tests, zero failed/ignored; 42 frontend tests; model/EVM oracles,
fmt/Clippy; a fresh debug automation bundle and five actual hidden native flows;
an ordinary release bundle, codesign and test-helper exclusion; two historical
genuine RISC0 receipts including negative operation/expiry checks.

App path and all five executable hashes are in app-release.json. Native outcomes
and screenshots are in native-e2e. Paired personal visual inspection is recorded
in visual-review.json. Run logs and builds stay outside Git in output/ and target/.
The three runner scripts retain the reproducible commands and frozen-input checks.

This is a checked development application, **not full V1 readiness**. Missing
product flows and the separate unstable 64-node capacity gate remain explicit in
Docs/V1_READINESS_2026_09_09_RU.md. No heavy genuine-spend/EVM E2E rerun or new
Linux/Windows native acceptance is claimed. The macOS arm64 app is ad-hoc signed,
not notarized. Native acceptance uses the test-feature debug bundle; the normal
release is separately built from the same sources and excludes that driver.
