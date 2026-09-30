# Retain finite verified claims without duplicating payment

The existing index must survive the sender and connect each descriptor to actual
selected data-holder receipts. This application step stores those claims locally;
it does not substitute for the required network/discovery gate.

The five scenarios use actual Core publication/opt-in, native paid candidates,
real finality certificates, original/copy CustodyStore admission and SQLCipher:

- A disjoint index operator retains original and selected replacement evidence
  in reverse arrival order. Exact sorted results survive cold restart and consent
  expiry without a payer wallet. Original index receipt/QC remain byte-exact,
  common evidence occurs once in SQL, and no ciphertext is present.
- Failed first/subsequent UPDATE returns no success and leaves SQL unchanged;
  cold read-clock failure also returns no bytes. Recovery/retry preserves the
  first claim and adds exactly the intended replacement.
- A location needs an existing matching index and configured trust. Other paid
  operations, wrong peer, corrupted-QC retry and a genuine conflicting claim for
  an occupied position fail without mutation. Configured Core resumes correctly.
- Serialized shared-plus-compact byte quota accepts the exact boundary, rejects
  one byte less without mutation, and releases the whole entry on original expiry.
  A backward read or expired admission cannot revive it.
- A damaged original signed claim fails closed on cold open or read and cannot
  cause a successful result or rewrite the retained corrupted bytes.

The independent critic first required computing a test input before mutable Core
borrowing and adding real unconfigured-Core write/read/retry failures. Both were
fixed, with unchanged-SQL and valid continuation assertions. The critic accepted
the revised suite before production. The existing copied-primary fixture was
extracted without replacing real publication, opt-in or storage with a mock.

The API-absent baseline is compilation RED, not a runtime behavioral failure.
Full V1, remote authorization/paging, availability and completeness remain outside
this local storage gate.
