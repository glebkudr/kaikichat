# Core sender persistence for signed history pages

The sender now saves genuine conversation history pages through the existing
MLS exporter, prepared-envelope lookup and SQLCipher state transactions. One
normal append creates a one-reference v1 leaf, the required signed branches and
the new root. Its bounded immutable batch is addressed by the root revision;
the exact child commitment supplies both batch revision and wire hash. No
lifetime inventory is scanned or rewritten. Each batch is limited to 65 nodes
and the existing 512 KiB mailbox-state bound. The operation membership and current
head commit with these pages, below Store's 16-change transaction ceiling.

The cryptographic producer now exposes its already-selected branch/root anchor
operations alongside unchanged wire outputs. Core obtains the exact descriptors
from retained pages and verifies the resulting documents. It does not parse the
wire independently or duplicate the signer's anchor-selection algorithm.

Current-revision requests for an already declared original/routes return the
current root without writes. Lost-response retries preserve original issuance and
bytes, including after a short last leaf expires under a longer-lived root. Route
updates append another declaration. Before trusting a membership lookup, Core
opens its exact signed path and checks the leaf's ordinal, descriptor and routes.
Publication preparation changes neither MLS nor delivery or paid allocations.

The first transition wraps a still-live v1 manifest unchanged. An existing member
needs only the wrapping root; a new original gets one further append. Legacy
leaf batch, wrapping root, new append, membership and current head commit together.
The old source row remains byte-identical. A wholly expired old manifest contributes
only its revision fence. Once pages are active, outgoing v1 preparation/publication
fails rather than advertising the frozen old manifest or creating a parallel
lineage. Epochs that have not activated pages keep their existing v1 behavior.

Cold reads authenticate one bounded batch and bind the exact page commitment.
Old live roots remain available for snapshot traversal. Missing/corrupt live
children are errors; expired children return no live bytes. Consistency retrieval
opens the path around the earlier leaf-count boundary and verifies the resulting
proof with the crypto live-prefix checker. It can skip expired subtrees while
preserving the original offsets of later live leaves. These reads do not write
state or advance recipient imports.

## Tests

The context-free backend-test-critic required membership corruption, wrap-only
migration and real epoch/time fences, then required the corruption fixture to
prove that it changes real existing membership fields. All additions were
accepted before production. The final test hash is
`cc1e72f4a402d04dfadd5cb69c7d962ea7e8e98468a31ccb812ecf9e08d2b578`.
The initial RED also exposed a test helper attempting to clone a non-Clone verified
reference. Keeping the real verified leaf behind Rc fixed ownership without
reconstructing its scope. Corrected RED runs contain missing APIs only and do not
establish pre-implementation runtime assertion failures.

The nine new scenarios exercise:

- 130 real MLS originals, later-ready sequence 1 after sequence 2, and one further
  append; old batch bytes/revisions remain exact and each append changes at most
  five bounded state rows. A cold old snapshot still has 130 references, while
  current history includes all 131 originals. Their actual envelopes reach the
  recipient. Several cold prefix proofs are checked cryptographically.
- Exact retries, normalized keys, conflicting/stale CAS, route updates retaining
  earlier declarations and a short-leaf retry after expiry under a live root.
- Three append SQL faults and five migration SQL faults. Full state rows and
  operation counts remain unchanged; append tests additionally check MLS and
  outgoing work. Cold retries succeed after restoring SQL writes.
- A two-reference legacy manifest: wrapping alone yields one leaf/two references;
  wrapping plus a new original yields two leaves/three references. Source bytes,
  old membership and cold retries remain exact; old outgoing v1 paths are fenced.
- Cold substitution of a real sibling leaf and a wrong ordinal in actual persisted
  membership, without changing signed pages or SQL revision. Both fail before a
  false no-op; exact restoration recovers the original no-write result.
- Missing/corrupt live batches, foreign direction/conversation, explicit expiry,
  backward-clock refusal across all readers, and a live suffix following an
  absent expired prefix. A real MLS transition starts epoch-local revision and
  sequence at one, rejects old requests/pages/proofs and preserves prior batches.

The [final checks](core-history-pages-checks.json) pass: 81 distinct backend tests
(43 Core history/progress, ten Core envelope and 28 crypto), 31 frontend tests,
Core/crypto all-target Clippy, formatting and whitespace. All 66 focused source
hashes were verified after final checks. This is not a full
application build manifest or renewed native gate. Counts overlap prior reports
and cannot be added as independent product coverage.

The first nine-test GREEN had one unused field;
the first Clippy run requested a return-type alias and an equivalent collapsed
condition. These production-only cleanup changes did not alter accepted tests.

## Remaining whole-capability work

This is sender-local persistence, not a remote storage promise or native paid
network acceptance. The ordinary workers still use v1 until explicitly connected.
The historical MLS transition fixture applies genuine crypto outputs to profiles;
it does not establish the still-required group control-plane flow.

1. Retain/read signed pages at actual paid index holders. Account for complete
   retained page-version bytes, including superseded live roots; one paid anchor
   cannot authorize unlimited history versions. Preserve the paid expiry and
   occupied quota, atomic faults/retries and acknowledged live graph dependencies.
2. Add bounded sender publication and recipient traversal through the existing
   authenticated protocol. Persist exact paid acknowledgments/current-pointer
   fences and per-reference imports with MLS/message/dedup in one transaction.
   Handle the v1-to-root proof transition explicitly.
3. Run ordinary over-128 simultaneous paid native loss/recovery with disjoint
   rosters, shorter later leases, cold restarts and SQL failures. Complete MLS
   epochs/Welcome, repair, E11 and all 67 cards /22 E2E /three platforms.
