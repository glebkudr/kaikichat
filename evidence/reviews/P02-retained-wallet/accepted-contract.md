# Proposed retained paid-wallet ancestry v1 — bounded shared lineage

Live unspent paid custody/postage tickets must survive ordinary checkpoint
advancement beyond the peer archive's64-entry limit and Core restart. Preserve
all existing untrusted history limits/successor checks, the Store16-change batch
limit and the wallet's32-intent capacity. Original funding/intent rows are immutable.

For each bound operation, atomically retain one immutable l2/custody-anchors/<id>
record: version1, pinned checkpoint profile/registry domain/code/commitment and
original funding checkpoint ID, complete original-to-bound-head history (2..64
certificates, <=32KiB each and256KiB total wire), initial registry proof <=512KiB,
original verified_at and bound_checkpoint_id. Limit serialized record to2MiB.
Original paid membership proof stays in its existing funding row; no new secrets.

Keep mutable lineage in ONE l2/custody-anchor-lineage record (version1, pinned
checkpoint_profile_id, actual current checkpoint_id and at most32 anchors entries
mapping operation ID to immutable bound checkpoint ID; <=16KiB serialized). It
must exactly describe the retained anchor set for the existing custody intents.
Missing/orphan/mismatched entries, unknown versions, wrong profile/current head,
invalid IDs or bounds fail closed; never silently repair an inconsistent index.

Initial bind and an authenticated legacy bind retry atomically couple the original
funding (when new), immutable anchor (when new), shared lineage index and clock.
A valid existing retry writes neither original paid evidence nor immutable anchor.
New binding must add to the index without discarding earlier anchors. Store's16
state-change limit remains unchanged; these transactions need at most4 changes.

Both accept_checkpoint and advance_checkpoint_chain first verify every successor.
They validate the shared index and all retained anchor metadata against the exact
old Core head, then atomically advance only the index's current checkpoint ID with
the head/archive/clock. At most3 changes regardless of funded wallet size. No other
path advances lineage; failure preserves authority/archive/anchors/index (a denied
transition may separately persist a higher clock). No per-paid-row cursor rewrite.
This is a SQLCipher state-transition invariant, like Core's current head and clock,
not a peer-provided shortcut across omitted certificates. Coherent malicious local
DB rewriting or compromised pinned attestors remain outside this trust model.

At use, validate the record/index bindings under installed profiles and saved
intent/funding metadata. Reauthenticate the complete initial history and registry
proof at recorded original time, including every successor and original paid proof.
The signed last initial head must match bound_checkpoint_id. Current input must
never heal corrupt persisted evidence. Choose the latest pre-beacon checkpoint
from this complete history; its following post-beacon checkpoint fixes the common
boundary permanently. Match expected-common ID and prove the identical original
paid leaf at that common root; an older individual root is not interchangeable.

Refresh the opaque validated original/common plan against actual current Core
head, installed profiles, current registry proof and monotonic clock. Preserve
immutable epoch domain/root/count/seed/beacon/seal/admission/obligation metadata,
issuer and registry code. Require live paid expiry and current head lease. Reuse
existing crypto/assignment/statement verification. Adapter refresh requires an
already proven plan and authenticated current head; it does not discover current
authority or prove an omitted chain. Its caller must maintain lineage. Old checked
time is never current authority. Refreshed member verification uses fresh snapshot.

Legacy version1 funding lacking both its per-intent anchor and its shared-index
entry (even when the index contains other migrated owners) requires its full
ordinary archive; authenticated bind retry pins it while available. If already
evicted, fail explicitly instead of guessing past finalized ancestry. No arbitrary
import is added. Common funding proofs remain authenticated public archival RPC
inputs; this module does not implement that cache/service, jobs, UI or canonical spend.

Required tests use real paid chains31340/31341 and80 actual signed successors each.
Initial35s lease must expire before late use while current authority and the same
statement remain live. Evict original/common roots from the still64-entry archive,
reopen SQLCipher and recover all4 independent journals/nullifiers/selections for
both main paid owners. Revalidate an actual selected operator via refreshed opaque
plan with a wrong-position control. Genuine stale-current proof/old head/older
common root/other paid owner/history substitution rejects with positive controls.
Corrupt original proof, initial history/proof, stale index, mismatched index binding
or omitted index entry cannot be healed by prepare/assignment/rebind.

Faults at immutable-anchor and shared-index creation, second-owner bind/index
update and sequential two-owner legacy migration, and shared-index advancement
in BOTH head APIs, roll back coupled state. Two paid records remain in the index
and immutable through failure, successful promotion, eviction and restart. Stale
index promotion is rejected with only the separately asserted clock observation;
restore it and prove positive advancement. With two owners, omit secondary while
primary stays indexed, and separately add an orphan index entry. Both head APIs
and primary prepare/assignment/rebind reject without healing; restored full index
recovers both owners. Genuine skipped/forked heads still fail.
The full32-intent wallet uses32 distinct actually paid commitments, with30 extra
payments mined as real transactions in the same pre-beacon block as primary; no
aliased/copied paid commitment or direct contract storage mutation. All32 bind,
advance through eviction within the unchanged Store limit and recover their exact
independent selection after restart; all original funded/anchor bytes stay intact.

The bounded full85-entry and skipped-history peer APIs remain rejected with positive
controls. Run backend/frontend and genuine proof/process regressions after changes.
Earlier two-owner code review missed Store's16-change integration limit; its code
ACCEPT was revoked. This revised test/contract requires a fresh FINAL ACCEPT before
production correction. Full V1, daemon jobs, fresh receipt admission and spend remain open.
