# AR3 exact history locator acceptance

Accepted codec/Core prerequisite: an encrypted locator commits to the exact
manifest and anchor wires. Core publishes only committed current outgoing history
and accepts a fetched directory only against its current live incoming pointer.
This is not native multi-book manifest traversal or complete conversation history.

The [contract](../../../spec/mailbox-history-locator-v1.md) preserves legacy CBOR
and JSON compatibility, outer mailbox signature/AEAD, fixed padding/wire size,
daily windows and resource bounds. History revision/issuance cannot roll back or
equivocate within an epoch, and a newer pointer cannot silently downgrade to v1.
Current MLS scope and exact hash checks precede incoming history persistence.

Three crypto and four Core tests received independent **REVISE → ACCEPT** before
production. [Baseline-2](baseline-2.json) against `63fd338` has 11 crypto and 21
Core missing-API/type/field compiler errors; no runtime assertion executed.
Earlier baseline evidence remains historical. The [critic record](critic.md)
preserves the accepted hashes and fixes for same-anchor substitution and isolated
endpoint-only corruption. [Independent Python vectors](../../../crates/crypto/tests/fixtures/mailbox-history/generate.py)
use genuine CBOR, AES-GCM and Ed25519; every hostile inner locator has a valid
outer signature/AEAD. No vector establishes paid storage or actual network custody.

The [affected gate](checks.json) passes **107 backend /21 frontend**, production
Clippy and fmt, with **602 unchanged source inputs**: 27 crypto, 33 Core custody,
six Core mailbox, 31 paid index, one custody-node and nine mailbox-node tests.
Additionally, all **six existing real-node mailbox process regressions** pass in
64.99 seconds including build time ([report](process-checks.json)). They check
legacy cold sender-absent lookup/cache loss, hostile DHT results, request/queue
limits and honest failure behavior. They do not claim new manifest transport.
The process run uses the same source fingerprints and records its actual node
binary hash. Raw logs remain in `output/ar3-history-pointer/`; report hashes and
[source fingerprints](source-inputs.json) identify exact inputs and results.

Core tests include real SQLCipher INSERT/UPDATE failures, exact cold retries,
same-anchor/same-revision alternate signed manifest rejection before first commit,
stale in-flight fetch after anchor change, current scope, real MLS epoch transition,
signed downgrade/equivocation/earlier issuance and unchanged fetch/message state.
Saved publication plaintext is re-opened from signed records; saved incoming
plaintext must match its checkpoint hash. Separate endpoint-only and commitment
mutations fail cold read/retry without repairing the damaged state.

[Next work](NEXT.md): reference-bound atomic message import, native manifest
put/read and ordinary sender/recipient use, genuinely funded disjoint books,
control/Welcome continuity, successful retirement, attachment coverage and
autonomous R10 repair. Preserve 67 cards /22 E2E /three platforms. V1 stays open.
