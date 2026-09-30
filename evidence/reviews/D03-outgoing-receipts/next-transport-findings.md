# Next transport boundary after durable sender evidence

Read-only source investigation while the current 586 inputs are frozen.

D03 still requires selected target10/min7 with real retrieval and own placement
for manifest/control metadata; its demo requires the sender to disappear before
additional copies complete. Ten in-process stores and one live primary do not
close it. D04 still requires autonomous10->7->10 with both clients absent.
The 64-validator engineering gate remains parked; no new job/review work.

The current network fixture already registers17 distinct Custodian units and
transports via real daemon registration. It can be reused for an actual ten-peer
live gate. Its `receipt` oracle currently fixes primary position0; a subsequent
reviewed extension must independently select each position/ordinal/member and
check target transport/signature instead of reusing the position0 assertion.
Quorum consensus remains the existing four selected finalizers, not the parked
64-validator exercise. Receipt counts do not prove physical independence.

A restart-safe scheduler needs portable placement authority beyond transient
SpendCandidate, which holds a verified proof and a checked live context but has
no wire constructor. Retained SpendRecord stores the exact finalized journal,
QC and verified_at, not the raw RISC0 seal or the live CheckedPostageContext.
The journal authenticates public resources and registry root, but immutable
registry selection/membership/history must still be reauthenticated explicitly.
PostageAuthoritySnapshot authenticates finalizer roster/issuer binding, not a
blanket authorization to use an expired custody checkpoint for new work.

Do not manufacture a SpendCandidate from arbitrary journal bytes or bypass the
current authority fence to make restart/repair pass. A following module should
reuse verify_record and introduce an explicit historically authenticated paid
obligation/placement view, with no new spend or signer authority. Public resource
parsing should be shared with the existing journal decoder, not copied as a
second proof verifier. Test genuine expired-current-authority/remaining-TTL
obligations, wrong registry/membership, exhausted repair allowance and real
recipient ciphertext retrieval before new production authority transitions.

Independent index/control commitment still needs inner sender-root authentication:
the shared directional exporter authenticates envelope access but is also known
to the recipient. A finite signed frontier proves its named prefix only. Persist
observed floors and report unknown/stale/expired/unavailable intervals. Metadata
must outlive ciphertext where long-offline gap reporting requires it. Budget and
repair capability must be bound to paid obligations, with finite deduplicated
replacement positions and no indefinite copied work or ping-timeout slashing.

Current source freeze includes no next-module tests or production changes.

Further read-only boundary: current `derive_public_policy` authenticates a
common pre-beacon funding root and a finite registry snapshot at the current
head, then `require_postage_context` fences that opaque result. It cannot simply
be called with an older host clock after restart. `verify_record` already checks
QC ancestry and the entire public journal; extracting authenticated resources
from that path can avoid duplicating journal semantics. Any historical placement
view must be read-only and must not satisfy new-spend/live-finalizer APIs.

Current operator transport binding is capped at registry admission_until. If an
old paid obligation outlives that window, fresh transport consent for its selected
node needs a specifically bounded obligation authorization; blindly renewing the
old generic Custodian binding would extend new-work authority. Test original
expired admission versus remaining paid TTL explicitly before choosing a repair
carrier. Preserve issuer resource bounds (replicaCount<=32,repairAllowance<=64)
and actual selected registry membership, rather than host-assigned replica IDs.

Do not silently equate a certified journal with independently retained zkVM seal.
The existing negative fixture `quorum_signed_other_resources` can certify a
journal whose resources differ from a genuine candidate; current custody correctly
refuses that mismatch. Historical copy/repair tests must preserve this boundary.
If portable admission re-verifies the original relation, retain the original
public receipt/Submit evidence first; SpendCandidate currently discards raw seal
bytes and is insufficient to reconstruct them after restart. No current receipt
ledger claim includes retained raw RISC0 seal or historical new-copy authority.
