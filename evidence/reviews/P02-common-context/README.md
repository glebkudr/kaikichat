# Common public postage context

Core now derives the shared funding root, interval and domains without reading the
sender's wallet. It authenticates the current registry snapshot and strict signed
history ending at the actual selected head, then selects the latest checkpoint
strictly before the public beacon block. All owners use the interval from that
common certificate's issue time through the snapshot's admission end.

The opaque checked context has a shorter current-authority deadline. A different
head revokes it even before that deadline. Authentication and rechecks durably
observe time, including on denials; failed SQL commits release no success. Owned
ticket preparation rechecks the context and reuses the existing encrypted-wallet
path with its exact interval, preserving paid rows and nullifiers. No new schema,
dependency, guest relation, receipt format or verifier image was introduced.

The six Core scenarios started with missing-API compile RED. They received separate
no-context critic ACCEPT before production implementation, and all six pass. They
cover both real-EVM fixture chains, receivers without custody intents, individual
versus common roots, missing or forged ancestry, stale/foreign registry proofs,
actual head and interval boundaries, SQL faults, rollback/restart, all four paid
tickets and supplied public history after eighty successors evict the common root
from the ordinary 64-entry archive. The full ordinary gate passed 491 Rust tests,
seven models, nine independent-oracle tests, formatting and workspace Clippy. All
40 frontend tests, TypeScript and Vite passed.

The added real-process scenario also passes. The actual CLI produced a 584984-byte
receipt in the scenario's 371391-ms elapsed interval. The fixed verifier and a
separate upstream `Receipt::verify` oracle accepted it; the entire journal matched
the independent Python CBOR vector. The receiver had no sender custody state,
rejected a changed interval, survived SQLCipher restart, revoked the old unexpired
context after a genuine changed-root renewal and verified the identical receipt
using freshly authenticated registry evidence. Raw mathematical validity under
the old supplied context alone did not satisfy the Core authority recheck.

This uses genuine cryptographic processes and historical Core fixture clocks. It
is not a current-time daemon/network admission scenario. No receipt produced here
authorizes spending; verifier results explicitly retain `admission:false`.

The complete real-postage regression passed: nine real proof/CLI tests and all
three Core process scenarios, with their formatting and Clippy gates. That is
503 Rust tests including the 491 ordinary tests above. The existing supervised
retry produced a 584750-byte receipt after cancellation and actual parent SIGKILL;
the independent oracle and unchanged paid-wallet/restart checks passed. The full
script exited zero. Final results and unmodified raw log hashes are in validation.json.

Input hashes preserve the pre-implementation test contract, accepted corrections,
the ordinary-gate source set and final deeper-process source set. The later changes
were only additional integration tests/vectors and sharing one existing CLI helper
between two test modules. Review findings and corrections are in review.md. The
first post-implementation compile exposed a test borrow-order error; the first
deeper check stopped at Clippy's duplicate module import. Neither is behavioral
RED/GREEN. Retained log copies omit trailing whitespace and empty end lines; raw logs remain under
`output/postage-common-context/` and their unmodified hashes belong to validation.

Limitations remain explicit: callers must supply the full bounded public history
when the common root is absent locally. Persistent public epoch-anchor availability,
daemon use of this common policy, fresh remote receipt verification and issuer-global
P03 consensus are still required. The existing explicit-interval local relation API
does not establish eligibility under this policy. A real paid leaf with expiry
shorter than the common interval has no newly isolated fixture test; it remains
subject to the reused paid-leaf and guest expiry checks. The full V1 goal remains
open. This increment did not rerun native, Linux network, real-chain EVM or the full
aggregate gate, and does not update their previously retained application evidence.

Contract: spec/postage-circuit/common-context-v1.md.
