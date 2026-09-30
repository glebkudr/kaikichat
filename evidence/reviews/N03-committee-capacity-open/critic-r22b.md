FINAL ACCEPT — R22b closes the response-classification gap.

Blocking issues: None. Missing mandatory scenarios: None for this retry policy.
Real rejected and accepted bindings drive the measured 5/10/20/5-second sequence. Independent verification establishes the original signature, selected key and transport; the exact signature mutation exercises Core rejection. Acceptance installs a route, a later rejection preserves its binding and expiry, and recovery retains the same connection.
Baseline records a nonempty corrupted response, actual Core rejection, zero accepted bindings/routes/hints, then fails waiting for request 2 within eight seconds. Cleanup clean. Rust timeout test checks actual OutboundFailure::Timeout.
Manifest SHA256 matches; all 74 hashes match. Production unchanged from R22 during review. Preflight result distinguished from full gate.
Implementation may proceed, followed by both transports and full validation.
