# V1 postage stamps: public signed stamps without ZK

**Date:** September 10, 2026. **Status:** an accepted implementation requirement;
this document does not declare implementation or acceptance complete.

Background: the user instructed to simplify stamps per ponytail, record the
requirement, and hand it to the main thread. After the "one ZK per book"
proposal, he clarified that hiding the deposit is unnecessary and chose the
simpler option: drop ZK and use public signed stamps. This clarification
replaces the intermediate decision to activate a book via ZK.

Additional clarification from the user: the desired funding-source privacy
should be provided by the crypto layer outside the messenger. In V1 this is the
responsibility boundary of the user's chosen funding method; the app does not
build its own payment anonymization layer. Actual privacy depends on the chosen
network and payment route, not on the word L1 itself.

This document refines [P02](agentic_internet_v1_1_plan/tasks/P02.md) and the
transport economy of the [V1 plan](agentic_internet_v1_execution_plan/README.md).
The overall [V1/V2 boundary](V1_SCOPE_2026_09_09.md), the mandatory status of
the other cards, and E2E remain. Where a previous description of spending
conflicts with this decision, this document applies. The requirement to hide
the source deposit is cancelled for V1 stamps; verifiable payment, one-time
spend, and E2EE remain.

## Decision

1. The user buys a packet of stamps of one resource class in advance. Batch
   purchase already exists in [PostageIssuer.sol](../contracts/src/PostageIssuer.sol);
   reuse it. The number 1 000 is a UX example, not a protocol parameter.
2. One funded batch yields a finite book bound to a spending key. A verifier in
   ordinary Rust code checks the public proof of payment against authenticated
   finalized L2 state, the commitment opening bound to the key, the paid count,
   the resource class, and the validity term. The existing funded verification
   and commitment functions are reused for this; there is no separate ZK
   activation of the book.
3. Ordinary sending uses a signature by the book key, bound to the stamp
   identifier (issuer domain, funded batch, ticket index), the hash of the
   specific operation, and the protocol version/domain. ZK is needed neither
   for purchase/book preparation nor for spending. There is no L2 transaction
   per message.
4. The existing shared spend log verifies the funding, the signature, the index,
   the limits, and one-time use, and produces verifiable proof of finalization.
   Repeating the same request returns the previous result; a different operation
   with the same stamp is rejected, including concurrent requests from different
   clients.
5. Holders verify the confirmed entitlement. Copying and repair continue the
   paid commitment within its bounds and do not charge the stamp again.

## Mandatory boundaries

- The spend identifier is stably determined by issuer domain, funded batch, and
  ticket index. A new key, salt, proof of payment, checkpoint, app network, or
  committee does not create additional stamps on the same funds. The index lies
  within the paid range; the operation is not part of the single-use identifier
  but is mandatorily part of the spend authorization signature.
- An issuer-bound shared spend policy and a persisted consistent spend state
  are used, including across restarts and epoch changes. A local `spent` per
  holder is not enough. The reference for reuse is
  [issuer-spend-policy-v1.md](../spec/postage/issuer-spend-policy-v1.md).
- The signature does not allow changing count, resource, expiry, or operation.
  Funding authenticity and finality are verified by the existing native
  verifier, and ownership by the signature of the key bound to the paid
  commitment. The public commitment opening alone does not prove key ownership.
  Trusting RPC without verification, a payment-check stub, or a client-side
  `paid=true` are unacceptable.
- A verified immutable batch description can be cached. The cache does not
  cancel freshness checks of the trusted checkpoint/lease, expiry, and the
  current spent state. Re-verifying the same funding inside a zkVM again is not
  required.
- The rules of verifiable placement and protection against cherry-picking
  convenient holders/finalizers remain. A new book key and message hash must
  not allow bypassing these rules. Placement compatibility with a public stamp
  is part of design and acceptance.
- E2EE of message contents remains. Stamps, their spending, and the source
  funding transaction are linkable; the link to the paying wallet is visible.
  A separate book key does not provide payment anonymity. These properties are
  explicitly reflected in the threat model and the user-facing description;
  deposit privacy is not promised.
- The old and new formats must not yield two independent entitlements to the
  same funds. Before the new path is enabled, a verifiable compatibility scheme
  is fixed: either isolating new issues from the old verifier, or a transition
  preserving the used limit. A mere new version/domain is not enough when both
  formats can access the same funds.

## App and agent interfaces

A shared Rust daemon serves the UI, CLI, and MCP. It checks the readiness of a
paid book, stores its key and state, and safely selects the next free stamp.
The UI shows the remaining balance for the chosen class and the
top-up/preparing/ready states. The balance accounts for stamps already taken by
sends; on depletion or not-yet-confirmed payment there is no false send-success
status. CLI/MCP get the same states and errors, without a separate
implementation of the rules.

## Tests first and acceptance

Before production code, RED tests are needed: independent public-stamp vectors,
funding checks, and signable-operation checks, then a backend-test-critic per
project rules. This is a change to live modules, not a reason to bypass
tests-first.

Mandatory scenarios that are actually used:

1. Finalized payment → native verification of funding and key binding →
   several different messages with signed stamps. A third-party node verifies
   the payment proof and signature on its own, without ZK or a company service.
2. An unfunded/non-final batch, someone else's key, altered limits, a wrong
   domain, an expired term, and an index outside count grant no spending right.
3. Re-uploading the book/payment proof, including a new key/salt and a restart,
   does not increase the limit. Repeated and concurrent spends do not spend a
   stamp twice; an identical request returns the original result.
4. A daemon/log restart, an epoch change, and a repeated CLI/MCP request
   preserve the balance and the binding of the spend to the original operation.
5. The same funds must not be spendable independently through the old and new
   protocols; the test confirms the chosen compatibility rule and the
   preservation of old state.
6. Delivery and the existing offline/retrieval/repair scenarios work with the
   new payment confirmation, including with the sender offline.
7. UI and CLI/MCP show preparation, balance, and depletion. Purchase,
   preparation, and sending work without running/having a ZK prover; E2E
   verifies the real public-stamp path, not the former ZK path with a verifier
   stub.

Book verification/preparation and sending are measured separately: signature,
verification, network finalization, and delivery. The previous ~570 seconds
belonged to a specific proof-and-checks run and do not set a new norm. The goal
is to remove ZK from the V1 stamp lifecycle; numeric latency promises are made
only after measurements. Backend/frontend regressions and the existing V1 E2E
remain mandatory after implementation.

## Implementation order

First fix the public stamp/spend format, old-path compatibility, and the tests.
Then reuse purchase, signatures, the canonical spent log, finalization, and
storage, adapting the existing interfaces. The current proof is bound to a
specific message: caching it or carrying it over to another message does not
implement this decision. The new path directly verifies funding and signature,
forms no ZK proof, and does not introduce ZK activation as an intermediate
requirement.

Do not introduce a central issuer, a new currency, a separate payment service,
or homegrown crypto primitives for this simplification. Current results and
evidence remain as results of the previous implementation; they do not close
the new criteria.
