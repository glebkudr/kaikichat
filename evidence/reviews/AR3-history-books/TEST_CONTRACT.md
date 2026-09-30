# Ordinary recovery across genuinely disjoint funded books

Required outcome: two ordinary messages survive sender disappearance across
originally disjoint paid books. No owner retrieval/publication/import RPC may do
the worker's job. This is the next full scenario, not another preparatory layer.
Use existing signed history, paid-index transport, Core progress and loss helpers;
production changes are permitted only for concrete failures of this scenario.

The genuine chain fixture registers 61 real daemon operator keys and executes its
existing exit, leaving 60 active custodians. Twenty-four additional ordinary Core
book commitments are paid before the actual beacon. The registry's explicit
future-block delay leaves enough blocks for those purchases; default callers
retain their existing 17 registrations/four-purchase timing. After the genuine
seed is known, the independent existing data and index draw oracles select a pair
whose original ten index operators and original ten data holders are both disjoint.
No book is prepared/funded after the beacon to engineer that result. If the finite
prefunded population produces no pair, report a fixture failure; never fake a
roster or silently retry a different seed.

The first genuine run registered 65 keys (64 active) and failed at 4/10 data
receipts. It exposed a real resolver defect: each retry kept the same first 32
sorted connected peers. It also exceeded the ordinary connection budget with
64 providers plus two seeds. The current 60-provider fixture fits that existing
64-connection budget while retaining more than 32 candidates and both genuinely
disjoint original draws. No production connection/request/time limit is raised.
The separate maximum-capacity gate remains open.

The subsequent cold sender run reached the second book but filled 64 physical
connections with only 53 distinct peers. A separately accepted NetworkBehaviour
test now requires duplicate ordinary sockets to yield capacity at the ceiling,
preserving the last non-closing socket for each peer, selected capacity and
physical accounting until actual close. It first produced a behavioral RED.
The native scenario remains the final test of cold reconnect and both originals;
the fixture's provider count and deadlines stay unchanged.

The setup sentinel retains its existing primary book. Two selected additional
books each begin with four genuinely paid tickets and zero allocations. An ordinary
sender policy chooses the first book, sends a message with 3600-second retention,
and waits for actual finality/R10/index/manifest/pointer completion. Alice restarts,
then changes only the ordinary policy to the second book and sends a later message
with 900-second retention. Both jobs must finish on the same signed manifest;
one exact original allocation per book and immutable old prepared bytes remain.
Independent funding signature, finality, actual data receipt, original roster and
signed manifest checks bind both messages. The first longer-lived original must
remain the anchor after the book switch.

The shared ordinary recipient scenario stops Alice and creates real index/data
loss, preserving only one different paid index and one different data holder per
reference, with no ciphertext at any pointer endpoint. Bob gets only bootstrap
addresses and its pre-existing MLS identity. Missing public trust must prevent
retrieval. Actual original-message SQL failures preserve MLS/import state; enabling
one commit produces only its exact reference; cold retry with the first cache
stopped recovers the second. Both exact original messages and per-operation
progress survive a completed cold pass without duplicates or extra writes.
All ten original index and data rosters are asserted before destructive loss;
the independent pre-beacon funding/draw evidence remains in the trace.

The ordinary caller allowlist includes policy configuration and sends, plus status
and network configuration for the existing cold fixture. It excludes internal
sender work, manual mailbox publication and recipient import/retrieval APIs.
Provider registration/public trust and read-only audit helpers are setup/evidence.

This proves the complete declared two-book range within the current MLS epoch.
It does not claim real MLS control/Welcome continuity, history beyond the finite
manifest bound, successful retirement or autonomous repair. Full V1 remains
67 cards /22 E2E /three platforms. After this scenario, close user capabilities
end-to-end and update their compact evidence matrix, as the user requested.
