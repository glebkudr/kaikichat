# Ordinary postage client — verified application checkpoint

Ordinary profiles without operator keys can durably submit genuine postage
receipts through the public protocol and independently authenticate the resulting
QC. Request/result SQL failures, selected-peer forged results, malformed ingress,
partition/healing, exact retries, verified conflict and cold reads after actual
expiry passed. Owner-only configuration/submission/read APIs and scope limits are
specified in `spec/postage/client-submission-v1.md`. Results grant no custody admission.

Validation passed 536 Rust tests (zero failed/ignored), 40 frontend tests,
workspace Clippy and formatting. The client live gate passed two fresh genuine
proofs, 1,167 owner calls and 18 independently verified signatures. Existing
selected candidate-network and ordinary spend gates passed another four fresh
proofs, 1,877/929 owner calls and 12/12 checked signatures. Every gate reported no
cleanup errors. See live.json, trace.json, validation.json, finish.json and the
network/spend regression evidence and public traces.

An existing finalizer TCP regression exposed discovery starvation after network
replacement. A reproduced diagnostic trace and three independently reviewed
scheduling tests preceded the correction: each peer now retains its own target
progress. The unchanged full TCP/QUIC gate then passed thirteen effects per profile,
1,136 owner calls and 66/66 independently checked signatures. Timeouts, rate limits,
Core authority and connection checks were retained. See finalizer-regression.json,
its two transport traces, discovery-regression.json and review.md.

The sole later source change was a reviewed one-line cfg(test) Clippy annotation;
all assertions, stimuli and 448 other source files remained identical. The original
Clippy failure and exact transition are preserved. Corrected Clippy, focused tests
and fmt passed. All 449 final frozen sources and 13 accepted inputs stayed fixed
through the fresh client/network/spend gates and release build.

The ordinary Tauri app was rebuilt and passed deep/strict ad-hoc signature
verification, automation-driver exclusion, and bundled verification of both genuine
historical receipts with the unchanged fixed image. It is not notarized. Native UI,
Linux network matrix and the remaining full V1 E01–E26 gates were not repeated.
The exact package/binary digests and compatibility controls are in release.json.

The first historical-client failure remains in failed-first-live/. The successful
client run preceding the discovery correction remains in passed-before-discovery-fix/.
The failed required finalizer run is retained in failed-finalizer-regression/;
QUIC did not execute in that failed attempt. Passing diagnostic reruns were never
substituted for the required gate. failed-discovery-clippy/ retains the lint failure.

This checkpoint does not complete P03 or V1. The 100-request acceptance, actual paid
ciphertext custody/repair, authenticated spent-state handover, UI/MCP payment flows
and remaining product scenarios are still required. Retained candidate and
consensus-prefix bounds remain in place; receipt/finality responses grant no
resource admission or refund/cancel authority.
