# Exact paid history transport — tests before production

Add a recipient read returning both the exact original PortableIndexObligation
and signed manifest, addressed by anchor operation rather than first sequence.
IndexHistory { anchor, manifest } uses canonical compact portable JSON payload
accounting (both fields and framing); this must fit capability max_bytes before
read-clock commit or byte release. The outer existing 1 MiB wire bound remains.
Existing legacy manifest-only methods retain their byte accounting. One shared
storage loader/verification/read-clock path must serve both APIs.

Missing manifest/anchor or another direction/epoch/exhausted range yields no
bundle. Actual current historical trust, target/time capability and exact paid
anchor validation remain mandatory; original QC/funding/receipt stay unchanged.
Three store tests cover nonfirst anchor despite limit=1/after=0, cold historical
public-trust recipient without new admission, full-payload exact size boundary,
real SQL rollback, missing/scope/target/trust/expiry and idempotent retry.

Existing bounded Noise paid-custody protocol adds index_history_put and
index_history_read, reusing server storage and admission/connection controls.
A successful put ACK identifies operation and exact manifest hash. A successful
read returns history { anchor, manifest }; missing state is Unavailable, never
empty-history success. Typed server ingress and actual retained custody checks
must run; no extra spend, raw database injection or owner-only retrieval path.

The native paid_history_network.py gate reuses paid_index_network.run: genuine
funded book, finalized QC, native MLS ciphertext, selected paid index and actual
holder/copy paths. A test-only fixture prepares signed history from the stopped
sender's actual Core profile (holds ProfileStore lock); it does not inject data
into the index or recipient. Independent raw Noise peers forward put/read with
Bob's actual target capability. Verify exact original anchor/evidence, independent
manifest signature/fields, cold index with sender absent, missing/malformed/oversize
history, wrong target/range, manifest-only-too-small budget, real put/update/read
SQL failures, same-anchor update and rollback refusal. Retain the inherited
legacy compatibility and exact one-ticket balance proof. This is server transport
acceptance, not ordinary automatic multi-book publication/traversal.

Backend-test-critic must ACCEPT before production. Preserve 67 cards /22 E2E /
three platforms, no push; full suite only at the end of the entire V1 plan.
