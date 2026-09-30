# F02 signed-document test review

Independent agent: `wire_test_critic`; no conversation fork; skill `backend-test-critic`.

Round 1: REVISE. Required additional tests for canonical outer/inner CBOR containers, weak-key strict Ed25519 rejection, and extension/aggregate resource limits.

Round 2: ACCEPT. Reviewed updated tests plus all 19 independent fixture records (18 legitimate signatures including structurally invalid signed encodings, and one deliberate weak-key forgery). No blockers remained for this bounded slice. Production code was still a module comment at approval.

Reviewer limitation: huge-declared-length rejection test does not prove allocation order. Implementation uses borrowed `minicbor::Decoder::bytes` slices, checks full input size first and checks field/map limits before copies; it does not allocate from an untrusted declared length.

Scope: shared signed wrapper only. Domain schemas, E2EE, persistence, authorization, replay state and platform acceptance remain separate tasks.
