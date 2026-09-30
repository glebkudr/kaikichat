# P03 concurrency checkpoint evidence

The independently accepted 100-request gate first failed on actual selected-peer
discovery under 13 ordinary clients. failed-initial/ retains the failure, public
trace, source/test identities and runner result. Raw logs remain in managed output;
run reports retain their hashes. discovery-review-r 1/r 2 and the exact rejected test
retain the fairness review correction before production. initial-reviewed-proposal
is historical pre-adoption metadata; its “not run” status describes that point only.

The final run passed 539 Rust tests,40 frontend tests,7 models,9 oracles, fmt/Clippy,
the full TCP/QUIC finalizer regression and the real 100-request gate. It independently
verified 50 winning retries and 50 conflicts for one operation, four durable cursor
heights 1, plus a separate formerly capacity-refused request and cold offline reads.
The 315 signature checks include repeated checks of one winning certificate.
Source manifests 451/16 remained unchanged through the ordinary app rebuild and
release checks. See validation-summary.json, run.json, evidence.json, trace.json
and release.json for exact inputs, binaries, limits, timings and practical scope.

This is one certified consensus effect, not ciphertext custody or full P03/V1.
The prior client SQL/fraud/result and candidate-network gates remain separate
mandatory tests; they were not all rerun for this scheduling-only correction.
Native UI and Linux matrix were not rerun. The app is ad-hoc signed, not notarized.
