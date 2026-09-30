# Ordinary automatic manifest publication and retrieval

Base `45d2681` has authenticated durable primitives; ordinary runtime still uses
legacy index intersection. This next slice integrates sender and recipient together.

The existing native ordinary sender gate now requires a signed history diagnostic
and ten exact anchor ACKs before stored status. Genuine SQL faults separately stop
outgoing manifest preparation and ACK commit after data/index/location success.
Cold restart must preserve both failures. Removing them completes publication,
and later cold retry keeps exact history/receipts and causes no repeated index writes
or additional ticket allocation. Existing promise/location SQL fault checks remain.
Read-only independent Noise audit then fetches exact current manifest/paid-anchor
bundles from all ten claimed ACK peers, compares diagnostic hash/operation/revision
and validates each paid index with the existing independent funding/QC verifier.

The ordinary recipient gate audits the automatically stored exact signed manifest
and private locator commitment, both original references and genuine paid candidate
keys, longest-lived anchor and full wire hashes. It then removes ciphertext and
index entries from actual stopped provider stores: exactly one distinct ciphertext
holder and one distinct candidate index survive for each reference. Every pointer
endpoint has no ciphertext, and no index contains both references. Surviving paid
evidence is unchanged; unrelated namespaces are preserved by the bounded fixture.

Sender stays stopped. Bob must discover the pointer/directory and retrieve each
reference through actual Noise paid-index and holder verification. Public-trust
absence rejects retrieval. Exact original-message SQL triggers fail after the
per-reference progress stage, preserving MLS/import state. Enabling first only
commits its own operation/sequence/message ID. A cold retry after another cache
stops must recover the second. The directory must never advance legacy bookmarks.
After both references commit, a cold background lookup neither reimports them nor
rewrites MLS/import state. Its `historyChecks` diagnostic increments only after
the ordinary worker validates the exact current pointer-bound directory and loads
its per-reference progress (a cold validated local directory is allowed). The test
waits for that event and no active/pending custody work before judging dedup.
Owner-call allowlists prohibit manual publication,
retrieval, importing or route injection.

This fixture still uses one funded book. Disjoint surviving indexes are real loss,
not evidence of originally disjoint funded books; crossBookCompleteness stays false.
Genuine multi-book/epoch continuity, Welcome/control, retirement and R10 remain open.
No production edits for this slice before independent critic ACCEPT.
