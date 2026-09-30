# Paid history directory storage — tests before production

Preserved pre-implementation contract; [README](README.md) records subsequent
independent acceptance, current-input RED and the implemented checks.

Extend only an exact already-paid anchor index with the signed finite history
manifest accepted in `8b49cdd`. Reuse original native funding/QC/operator binding,
current historical Core trust, existing index namespace/CAS/clock and quota.
The anchor lease is unchanged; no second spend, new ciphertext or rewritten
index receipt. The anchor index need not hold every referenced message/index.

Proposed trusted APIs: `retain_index_history(core, operation, wire, now)` and
`index_history(core, operation, now)`. Proposed server-side read primitive:
`read_index_history(core, targetBoundRead, operation, now)`; retain existing
location-read scope behavior: foreign direction/epoch or exhausted sequence
returns no bytes; wrong target/expired capability is an error. Whole manifest
bytes must fit the read byte budget; never return a partial signed document.

One optional `history` byte vector lives with each retained anchor. Quota adds
its exact compact JSON representation length, with the existing 16 MiB outer
state bound independent. Reads and writes commit clock/pruning before release.
Exact same-time retry preserves SQL bytes/revision. Higher revisions may update
candidate routes or add references, but cannot remove or change any still-live
descriptor reference. Expired references can be pruned. Cold old-revision or
same-revision changed-body updates fail without altering the retained directory.

Five tests reuse real `Paid` funded candidates/P256 QCs, SQLCipher and independent
descriptor fixtures. They cover cold read after admission expiry with installed
public trust/no wallet; absent referenced local index; original evidence/quota;
actual first-manifest/update/read-clock SQL faults and cold exact retry; stale,
equivocating and live-omitting updates; expired-ref pruning and terminal expiry;
exact byte quota and subsequent index admission accounting; read byte limits,
missing anchor/trust, wrong scope/peer, expired capability and corrupted cold SQL.

Only anchor admission establishes paid storage in these tests. Manifest candidate
keys and other references are signed discovery claims; they do not establish
their own paid placement. No multi-book/native/MLS continuity, completeness,
sender retirement or R10 repair acceptance is claimed. New tests first, separate
context-free critic ACCEPT before production, then actual RED and affected
backend/frontend checks through `scripts/build-storage.py run`.
