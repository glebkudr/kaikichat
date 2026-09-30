# Core graph reference attempts and authenticated expiry skips

Seven independently reviewed tests pass before ordinary Node graph integration.
The cursor records a bounded selection attempt before network I/O, survives cold
restart and counts references rather than leaves: 128 originals inside an exact
wrapped v1 leaf plus two v2 leaves produce 130 selectable ordinals. A full
16-reference pass yields without writing. The cursor remains a scheduling hint;
only checked leaf progress and atomic import establish delivered originals.

One small row per conversation/epoch binds the incoming index. Preserve the
successor without modulo so accepted growth at the old end is tried next. Apply
modulo only when selecting against the current accepted root. Reference ordinals
are not stable identities when signed expired prefixes change. SQL INSERT/UPDATE
failure returns no claim; cold retry keeps the unconsumed position. Real MLS epoch
change creates a separate cursor while retaining old bytes and refusing old work.

Expiry skipping reuses the authenticated root/path verifier, counting references.
At most 63 signed branches lead to an expired subtree; an expired signed peak
needs no child bytes. Missing bodies, live subtrees and mixed-TTL live leaves
cannot be skipped. Exact root/current pointer guards run before idempotent retry.
A later different claim refuses a stale advance; failed SQL leaves the old cursor
for retry. Neither a skip nor exhaustion asserts complete history or availability.

The tests cover a complete genuine wrong-parent path, actual pointer advancement,
root expiry, cold exact retries and authenticated 17-reference expired ranges.
The 130-reference attempt scan leaves MLS/messages/imports/ACKs unchanged; the
last original still produces actual ReceiveGap, while an authenticated first
original imports through the existing reducer. Existing complete 130-original
import and immutable append regressions also pass with the shared path verifier.

[Checks](graph-scan-checks.json): 105 targeted backend (65 custody +40 sender),
31 frontend, Core/node all-target Clippy and fmt pass on [811 unchanged inputs](graph-scan-inputs-c1.json).
Counts overlap older reports and exclude the repeated initial seven-test pass.
[Critic](graph-scan-critic.md): two REVISE rounds corrected fixture setup and a
masked negative oracle before final ACCEPT and production changes.
[Contract](../../../spec/custody-history-page-scan-v2.md).

Ordinary Node still uses the flat v1 directory. This component did not rebuild or
rerun native; the prior [normal C8 gate](PUBLISHED_GRAPH.md) retains its original
source and two paid originals. Next connect typed paid root/path reads, admission
and the durable cursor to ordinary receiver work, then child-before-parent ACK
publication. The >128 simultaneously live paid native recovery gate, explicit
product gap/legacy/rejoin, first offline Welcome, control/epochs, independent
repair, E11 and the whole 67-card/22-E2E/three-platform V1 remain required.
Automated E2E continue using the isolated file vault without login-Keychain access.
