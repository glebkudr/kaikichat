# Signed service address records and shared cache

The record/cache implementation, workspace checks and ordinary Tauri package have
passed. `release.json` and `validation-summary.json` retain the results. Full
service discovery and full V1 remain incomplete.

The record uses the existing libp2p SignedEnvelope around the existing finite
P256 binding, a positive address sequence and bounded supported addresses. Both
signatures, exact lookup scope and the terminal transport PeerID must agree.
It contains no chat/root identity. Canonical wire checks reject alternate encodings;
the original binding expiry remains the deadline. No crypto scheme or external
package was introduced; an existing locked minicbor dependency moved to production.

The existing Kademlia store retains up to 32 live service records alongside its
128 private mailbox records, with separate quotas and shared expiry, clock floor,
publisher removal and TTL clamping. Updates compare P256 issuance first, then
address sequence, allowing a genuinely newer binding to rotate transport after
an extreme old address sequence. These ephemeral public hints grant neither
selected membership nor custody admission.

Eight tests preceded production code. The independent no-context critic accepted
the tests and checked all 36 independent P256 fixture signatures and lookup hashes.
The actual compile RED confirmed missing APIs. Tests cover both signature layers,
address substitution, strict wire, finite leases, duplicate/conflicting versions,
transport rotation, separate full quotas and a genuinely signed weak transport key.
Five reviewed files remain byte-identical. The reviewed runtime.rs test registration
remains identical after removing only the subsequently added production module
declaration. The final source manifest pins that complete production file as well.

Workspace results: 559 Rust tests across 73 suites, zero failed or ignored, and 40 frontend tests;
fmt and Clippy passed. The unchanged actual daemon mailbox and routing tests include
offline sender/cache loss and moved-address delivery through intermediaries.
The specialized selected-finalizer EVM/TCP/QUIC regression was not rerun in this
record/cache slice; its preceding checkpoint remains separately documented.

Native UI and Linux matrix were not rerun. The ordinary application is ad-hoc
signed and not notarized. Release signature, test-driver exclusion and both genuine
historical receipts against the unchanged fixed image passed. All 473 final source
inputs remained fixed throughout checks and packaging.

Remaining: actual service publication/query dispatch through the existing guarded
Kademlia, on-connect key announcements, unknown-PeerID committee formation,
persistent selected-route rollback protection, reserved committee connections and
DHT roles, then paid ciphertext custody/repair and the remaining V1 product flows.
The current mailbox query result validator still accepts only mailbox records;
passing the record/cache tests does not establish service network lookup.
