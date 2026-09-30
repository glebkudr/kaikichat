# AR1 runtime lifetime integration: tests before production

Baseline: 9dece5a. User goal remains all V1 (67 required tasks, 22 E2E, three
platforms). Previous turn was progress: committed authenticated history index.

This change must connect that index to actual P-256 consensus/service and built-in
issuer spend policy, and retire finalized candidate inputs from the node's active
sixteen-entry queue. It must demonstrate more than 128 actual funded distinct
spends under one issuer/log/committee, without synthetic QCs or a reset journal.
Epoch handover remains required later; do not remove epoch !=1 protection here.

## Runtime design to validate

- HistoryIndex emits an opaque checked operation result, bound to the requested
  operation, exact committee and complete finalized (height,digest) anchor.
  An anchor-only query cannot authorize any operation. Empty local history is
  only the genesis anchor; it does not let the runtime omit the preceding chain.
- Validate a bounded newest-first suffix down to that anchor, exact parent/height
  continuity, duplicate operations within the suffix and candidate absence in
  both prefix and suffix. The authenticated Simplex parent establishes prior
  ancestor application validity; current Core fences/receipt policy remain required.
- An optional trusted local asynchronous history provider connects the service
  adapter and Guarded engine to the node actor through its existing bounded
  mailbox. No new remote application registration, signer or profile database.
  Legacy applications with no indexed provider retain complete-prefix semantics.
- The node captures/compares the anchor while making its application decision;
  stale or unavailable index state fails closed. Only indexed operation paths
  drop the absolute 128-height cap. Keep a finite suffix/backlog bound and proof
  path byte/entry bounds. Archive proof export scans a bounded distance from an
  arbitrary positive target rather than stopping at absolute height 128.
- Finalized spend record, derived history index and candidate cleanup are durable
  idempotent stages before acknowledging Delivery. Original records/QCs remain
  unchanged. On replay, an existing authenticated spend record allows restoring
  derived history and completing cleanup without retaining terminal candidates.
  Failed cleanup may retain active work but must never lose the spend or ACK early.
  Store record first; no refund/reuse of an exposed or finalized ticket.

## Tests

`crates/finalizer/tests/indexed_prefix.rs`: real P-256 QC/SQLCipher prefix; 130
entries; exact next entry, old-key rejection, operation-bound capability,
anchor-only denial, malformed/missing/foreign/reordered/duplicate suffixes and
bounded backlog including the empty-index case. Intended new API:
HistoryIndex::check_operation(store, Option<operation>) -> CheckedPrefix;
CheckedPrefix::anchor() and next(suffix, operation, payload).

`tests/evm/public_spend_lifetime.py`: actual EVM purchase of 160 public tickets;
four ordinary daemons with self-generated P-256 keys selected by the real registry;
144 distinct native public spends through existing owner IPC and actual TCP/Noise
consensus. Independent canonical-CBOR/SHA/P-256 QC verification reuses existing
test oracles. Submit in batches below the active capacity, require terminal
candidate retirement, test no quorum after 128, SQL failure before spend effect,
consumer ACK ordering, solo archive replay of the single known failed delivery
129, then a separate SQL trigger rejecting candidate-array shrink at delivery
130. The second failure must leave the original spend readable, active work
retained and consumer cursor at 129. With all peers down, cold replay must finish
cleanup and advance to 130, preserving the exact original QC. Fresh failure
counter increments and cursors are retained in trace evidence. Cold exact-QC preservation,
old-ticket conflict, competing valid operations for an unused ticket after height
136 (both admitted on a sole live validator before peers restart, then real
remote candidate distribution), and actual expiry of the original accepted authority. Keep
funding source offline during voting. After actual authority expiry, restart
the final sole node again: old QC remains readable while the valid unused ticket
remains unauthorized/absent. Reuse Core PostageAuthoritySnapshot and
authenticate_postage_history for that historical-only read; no restored signer or
backdated current admission. No prover path or seeded archive.

Shared test helpers only: funded_custody scenario gains an explicit primary book
ticket count (default remains four); postage_spend_node harness reads explicit
exercise count/lease/operation options and puts temporary profiles under managed
output instead of hard-coded internal /tmp. Existing defaults remain intact. The shared public QC oracle matches a path entry
by both nullifier and exact journal, so competing operations do not get confused.

Independent backend-test-critic must ACCEPT before production code. Afterward
run the actual fresh gate plus focused unit tests and complete backend/frontend
regressions. Preserve failed evidence. Do not claim epoch transfer, durable
ciphertext, independent testnet or full V1 acceptance from this gate.

## Additional reviewed cold-reader regression and offline evidence

After the first successful lifetime run, code review found that a role-disabled
cold reader with an unresolved retained candidate could return absent when fresh
preparation failed. The focused --cold-pending mode reuses the genuine funded
setup, submits one candidate without quorum, disables the role, cold restarts,
and requires postage_authority plus the retained pending input. It claims zero
finalized spends. The same assertion precedes solo replay129 in the full gate.
After the harness's missing-preflight correction, the real RED returned absent;
critic ACCEPT preceded the narrow production fallback fix and the focused GREEN.

The full trace additionally retains the actual public committee/selection and
issuer proof inputs. verify_saved_trace reconstructs selected public keys and the
scoped committee, then independently verifies all retained QCs and the complete
canonical chain. It neither contacts nodes nor newly authenticates checkpoint
or EVM storage proofs. Run it again after the fresh full gate has stopped all
nodes and the source chain. Preserve the earlier successful run separately.
