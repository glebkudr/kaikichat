# Continue full V1 after the finalized-prefix index prerequisite

Do not close AR-R01, AR1 or full V1 on the 130 signed fixture entries. This
primitive is not connected to voting or issuer policy yet. Follow the unchanged
67-card/22-E2E scope and AR1–AR5 order. Prior sender continuation remains applicable
at ../AR1-expired-sender/NEXT.md.

1. Integrate the existing HistoryIndex with the actual durable Marshal archive
   and node spend state. Before voting, replay/reconcile every finalized entry
   into a complete index; new/empty/partial indexes are not authority to say
   unspent. Existing service/delivery.rs replays 1..processed after each restart,
   including effects already acknowledged. Avoid an additional sender/spend DB.
2. Validate a bounded unfinalized suffix against a captured finalized anchor and
   indexed prefix membership. Preserve exact Entry/journal/QC and issuer-global
   nullifier semantics. Only then remove absolute-height guards from engine/guard,
   service/application, quiet and SpendSession. Account for index advance while a
   decision is queued; later state must not be mistaken for the captured prefix.
3. engine/proof.rs still bounds both target height and search at 128. Change to
   a bounded distance from arbitrary positive target height alongside a real
   engine archive/export test crossing 128. ancestry.rs wire now supports this;
   it still limits paths to 128 entries and rejects bad quorum/links/expiry.
4. New backend tests first and independent critic ACCEPT before implementation.
   Reuse real deterministic Commonware engine harness and real native funding
   fixture generation. Existing four-ticket fixtures cannot prove 129 distinct
   spends. Test old-ticket conflict, partition/heal, restart/replay and lease
   outage at the boundary. A storage fixture signed with test keys is not this gate.
5. Specify and implement the unique old-epoch closing boundary and new-epoch
   inherited spent state, old authority revocation and checkpoint refresh. Do not
   remove epoch !=1/HandoverRequired or accept empty new-epoch state as a shortcut.
   Current HistoryIndex refuses a different committee for an existing log; streamed
   transfer does not itself authorize handover.

Current snapshot total transfer/revalidation is O(N), storage is O(N); working
memory, pages and per-entry writes are bounded. The authentication root is the
existing finalized hash chain. Whole-profile rollback requires archive/remote
finality reconciliation. Preserve this limitation in claims and product gates.

Full backend/frontend checks and managed build-storage remain required. No fresh
live-EVM, packaged-native or external three-platform gate is implied here.
