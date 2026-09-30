# Durable sender directory prerequisites

Base: `a252460`. Tests precede production. This slice does not change ordinary
sender/recipient scheduling, publish v2 pointers, or claim disjoint-book delivery.

- Core inventories retained originals in the current outgoing conversation/MLS
  epoch, sorted by sequence and filtered by live lease. Delivery ACK/outbox removal
  cannot erase an original's route candidate. Reads never mutate state or MLS.
- The retained outgoing history revision remains available after anchor expiry;
  it is the CAS input for the next publication, and resets only with the MLS epoch.
- One optional manifest per existing paid outgoing index object, one set of ACK
  positions. Original payment/QC/envelope/index promises remain shared unchanged.
- Preparation authenticates the paid anchor and signed manifest. Exact retry
  preserves bytes and ACKs; an authenticated successor preserves live references
  and clears ACKs. Reject rollback, equivocation, and a foreign anchor.
- A local authenticated-transport ACK must name the exact current manifest hash,
  operation, retained index position and actual paid transport key. It is not a
  portable storage proof or recipient delivery acknowledgment.
- Existing historical payment validation, current installed trust, lease and
  monotone read clock apply to stage/read/ACK. Durable SQL failures release no
  uncommitted progress. Directory bytes consume the existing outgoing quota once.
- Corrupt cold signatures or duplicate/unretained ACK positions fail closed.

Independent test review must ACCEPT before production changes. Run targeted
Core/store baseline RED, then these groups plus frontend chat-shell regressions
and changed-library clippy/fmt on one frozen source revision.
