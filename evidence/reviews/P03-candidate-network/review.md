# Independent backend test review

Reviewer: `/root/common_context_test_critic`, an existing independent agent
originally launched with no inherited context. Both review requests supplied
standalone business requirements and exact file paths. The main agent waited for
each final verdict before proceeding. The backend-test-critic skill was applied.

Reviewed test artifacts:

- `tests/evm/postage_spend_network.py`
- `tests/evm/postage_spend_node.py` (optional shared-fixture hooks)
- `crates/node/examples/operator_fixture_peer.rs` (independent test carrier)

R1 returned **FINAL REVISE**: corrupt receipt evidence from a foreign Noise peer
was rejected before reaching genuine receipt validation. A valid selected sender
could therefore bypass validation without that negative scenario detecting it.
The reviewer also requested receiver-side success evidence for route retries;
the sender's counter increments on its first successful write, not full delivery.

R2 uses the selected sender's actual ephemeral test transport identity on a second
Noise connection while its original proof connection remains live. The carrier
reads only that identity through a read-only SQLCipher connection; its key/path
arrive on stdin. A tampered freshly generated genuine seal must return exactly
`invalid_candidate`, increment rejection and leave no durable candidate before or
after recipient restart. The original receipt then arrives through the ordinary
daemon path with that same selected identity. Receiver acceptance is required
after subsequent restart and role re-enable. Only `input` uses CBOR bytes in the
carrier; existing `payload` fixtures retain their previous independent arrays.

R2 returned **FINAL ACCEPT** before any production edit. The critic confirmed the
two-connection authority/limit compatibility, receiver-side retry evidence and
preservation of the original default daemon gate. AST and the carrier build
passed. Baseline RED was `UnsupportedProtocols` in the protocol preflight, before
expensive proof generation; it proves the absent protocol, not later scenarios.

Nonblocking follow-ups remain: source restart before first dissemination,
candidate-ID mismatch at the network boundary and saturation/backpressure load.
The bounded selected-replica acceptance is not full V1 acceptance. Final hashes
and actual live/regression outcomes are retained with this review.

The first real run after implementation remained **FAILED** at the re-enable
delivery assertion, after two independently verified fresh proofs. Its raw report,
log and frozen inputs are kept under `output/postage-spend-network/failed-1/`.
R3 added only a failure wrapper retaining six explicit public node-info fields
before cleanup and re-raising the original error. The critic returned **FINAL
ACCEPT**: undoing that wrapper/rename reproduces the exact R2 test hash; assertions,
timing and topology are unchanged. The subsequent production correction paces
automatic route discovery below the existing proof admission limits instead of
relaxing the acceptance timeout or the receiver limits.
