# Next product gate: retained ciphertext on actual selected custodians

Current envelope work is a prerequisite, never a substitute for E05–E07. The
next gate must demonstrate independent node persistence and actual recipient
retrieval after the sender process stops. Do not return to 64-validator scaling.

Reuse, do not reconstruct:

- AppCore.prepare_custody_envelope produces the immutable paid operation.
- postage_spend::Submit::prepare_client authenticates the actual Core context and
  genuine fixed-image RISC0 receipt; SpendCandidate.custody_placement chooses
  public ticket positions without owner grinding.
- verify_record plus Core.authenticate_postage_history verifies the real spend
  QC and issuer policy. Compare complete journal, nullifier and operation with the
  current candidate; a winning conflict for another operation cannot admit bytes.
- PrivateCustodyPlacement.verify_member and Core.verify_checkpoint_operator bind
  selected position to a Custodian role and actual transport key. Existing
  operator publication lookup supplies locally owned membership proofs.
- ProfileStore already uses SQLCipher WAL synchronous=FULL. The custody adapter
  should retain envelope, paid authority evidence, assignment and receipt together
  before returning a durable acknowledgment, with bounded quota and expiry.
- The node processing budget/request-response codecs already bound streams and
  bytes. Reuse them for store/get/audit/repair; no parallel unrestricted listener.
- Existing private mailbox publication/read head carries an encrypted index ID
  and endpoints. Index and repair metadata need independent durable placement.

Envelope signature alone grants neither space nor a copied-message count. Initial
admission must recheck current Core context, exact operation, paid size/lifetime,
target/repair allowance and actual selected local custody role. Retained duties
must not evaporate because a 60-second connection binding expires: historical
evidence validates the obligation while fresh proofs govern new work. Receipt
signature identity and its historical binding must be explicit before tests.

Storage retrieval requires an exporter-authorized read request bound to index,
target custodian transport key, finite time, nonce and bounded page. Do not expose
an exporter or add broad agent authority. Repair reads require their own finite
paid assignment; do not reuse a recipient capability or allow arbitrary copying.

First write the actual positive/negative tests, then independent critic:

1. A genuine new receipt for the actual envelope hash and real spend QC authorizes
   only selected custodians. A correct member at a different selected position and
   a genuine conflicting operation both have positive controls before refusal.
2. Invalid signature/hash/TTL/paid bounds/wrong role produces no stored bytes,
   index or receipt. SQL failure before commit produces no durable reply; exact
   retry and restart preserve the same stored object without double quota.
3. A fresh recipient with the sender process stopped reads actual persisted bytes
   from a separate custodian, opens through AppCore and sees one original message.
   Restart/read/ack races preserve deduplication; node receipt is not recipient ack.
4. TTL and real quota refusal remain visible and never become delivered. No
   source-chain outage or synthetic transport success should hide unavailable data.
5. Extend to 10 distinct test identities and autonomous 10→7→10 repair with both
   clients stopped, three assigned custodians stopped and finite authorized spares.
   Audit new copies and indexes with an independent client; no-spare case degraded.

Only after this works wire real copy counts and automatic retrieval into desktop.
Exact E06 lifecycle and remaining group/recovery/trust/economy remain open.
