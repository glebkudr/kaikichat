# Test critic R3 — FINAL ACCEPT

R3 corrects a real setup defect: ProfileStore holds an exclusive OS lock for its lifetime. R2 missed that contract. The replacement test verifies competitor refusal, unchanged state, exact signature retry and the next index after owner handoff. The other seven tests are unchanged.

Concurrent UI/CLI/MCP requests remain an explicit required integration gate of the shared daemon. This Core test does not replace that gate. All R3 SHA256 inputs matched. Actual R2 RED contained 42 E0599 missing wallet API errors. GREEN for all eight R3 tests and backend/frontend regressions remain required. The critic did not edit files or run builds.
