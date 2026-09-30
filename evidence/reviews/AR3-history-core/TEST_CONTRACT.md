# Core finite history persistence — tests before implementation

Trusted Core APIs prepare and retain an exact signed same-direction/current-epoch
manifest before network publication, and atomically retain an incoming manifest
with its anti-rollback checkpoint before exposing routes. Neither API imports a
message, mutates MLS, grants paid custody, advances fetch progress or acknowledges
delivery. Candidate index keys are caller-supplied declarations; the node must
verify actual paid obligations before using this API in its sender path.

`CustodyHistoryRoute` carries message_id and index_transport_keys. Core exports
actual retained original descriptors, checks the requested conversation, orders
references by their authenticated sequence, and sorts candidate keys (duplicates
are errors). Caller selects an included longest-lived anchor. The public exact
`CustodyHistory` carries epoch, revision, issued_at, expires_at, anchor_descriptor
and wire. prepare_custody_history(conversation, anchor_message_id, routes,
expected_revision, now) uses CAS; expected+1 with identical canonical inputs
returns the retained original bytes without writing, including after restart.

custody_history_publication/head expose only the current epoch's live manifest.
accept_custody_history(conversation, anchor_descriptor, wire, now) verifies the
incoming MLS-derived index and persists the authenticated checkpoint and exact
manifest together. Old revisions, same-revision different bytes, time rollback,
foreign scope and changed/omitted still-live references fail. Higher revisions
may change routes/anchor and prune expired references. Saved signatures are
reverified at their authenticated issuance; expiry hides routes but preserves
the checkpoint. Malformed saved bytes fail closed instead of being overwritten.

One versioned CAS row per direction/conversation/epoch under custody/history/
reuses the bounded mailbox state codec. Old rows are retained across real MLS
transitions; these tests do not claim old-epoch decryption or control/Welcome
catch-up. Long-term epoch pruning and bounded history import progress remain
follow-up work, as do locator binding, network serving and multi-book acceptance.

Seven tests use genuine SQLCipher profiles, original MLS messages, real exporter
signatures and actual INSERT/UPDATE failures. They verify disjoint routes, older
longest-lived anchor, cold exact retries, unchanged message/MLS/outbox state,
invalid CAS/scope/input, live omission, expired pruning/checkpoint persistence,
cross-anchor stale/equivocating signed updates, real MLS epoch fencing and cold
signature corruption. The shared real-MLS-transition helper only gains sibling
visibility; its behavior is unchanged.

Critic revision: successful acceptance additionally leaves every non-history
state/operation and the direct/index fetch cursors unchanged. A genuine higher
revision with earlier signed issuance (all live references intact) has an
independent crypto-positive control, fails after cold checkpoint restore, and
does not prevent a valid update of the same revision. Both current-head getters
reject local time before saved issuance.
