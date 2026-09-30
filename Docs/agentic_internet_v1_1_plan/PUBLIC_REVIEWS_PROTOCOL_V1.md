# V1 · Basic protocol for orders, public reviews, and rating

## 1. What the protocol attests

V1 attests the signing keys, their authority, and the link of events to a specific order. The executor signs `ExecutorDeclaration`: "under such terms I got such a result". The customer signs `CustomerValidation` and/or a public `ReviewEvent`: their checks and impression. These are different statements.

Neither the executor's signature, nor successful client tests, nor five stars prove which model/hardware the executor used inside. Independent TEE/zkML/verifier/arbitration markets are V2+. Machine-verifiable result criteria are available already, but this is precisely a result check with a named author/methodology.

## 2. The right to review arises on order acceptance

```text
ServiceCard → RFQ → Bid / terms → two-sided Accept
                                      │
                           AcceptedOrderReceipt
                                      │
                         ReviewRight at the customer
                                      │
        Result / Timeout / Cancel / No response — any of these branches
                                      │
                      Review → Amendment / Withdrawal
                                      │
                          Reply + RatingSnapshot
```

A basic standard order is always reviewable after acceptance. The executor cannot disable this with a `no_negative_reviews`/`private_reputation` flag, demand payment, first deliver a result, or approve the review one more time. The client gets the right as soon as both signatures exist; the review can be about the process, non-performance, or the final result. An early rating is not called proof of completed work.

An unaccepted RFQ does not give a verified-order review. The justification of the right is the two-sided receipt, not the client's one-sided claim "I ordered". The client stores the receipt reliably until the UI declares the order accepted. This does not require escrow, proof of payment, or a complex economic agent.

## 3. Receipt and bounded disclosure

The minimal publicly verifiable `AcceptedOrderReceipt` contains:

```text
protocol_domain, protocol_version, order_id
customer_id, provider_id, service_id
customer/provider ownership_epoch, service_epoch
salted_private_terms_commitment
public_review_policy_version
acceptance_context and proofs of authority to accept
customer_signature, provider_signature
```

The specific timestamp/checkpoint and signature suites are fixed by F02. The private terms commitment uses a sufficient random salt; a hash of a short guessable assignment without a salt is not considered confidentiality. The order ID has sufficient randomness and domain separation. Repeated acceptance of the same receipt is the same deal, not a new right.

The prompt, inputs, full price/terms, correspondence, and results remain private unless the client separately decided to disclose them. Both sides sign the minimal header and consent to its publication in the review in advance. The receipt is not broadcast publicly automatically for every order: it is published by the author together with the review. This preserves private messaging, but **the public review itself discloses the link between the customer's pseudonym and the service**.

In V1, a standard order has no hidden provider veto for the sake of privacy. The user sees the disclosure before acceptance; they may decline such an order or not publish a review. Any future fully anonymous proof of an order is a separate profile, not a promise of the current scheme.

## 4. Events and rights

`ReviewRight` is logically derived from a valid receipt and does not require a separate issuer. It is bound to the customer identity/authority, not to the current OAuth account or wallet. Delegating publication to an agent runtime is a separate narrow capability; permission to read an order does not mean permission for public text.

`ReviewEvent` contains an order/receipt reference, author, service+owner epochs, version, rating 1–5, bounded UTF-8 text, declared outcome, revision, prev_event, network domain, and signature. The declared outcome is the author's statement. If needed, agreed job-state evidence is attached separately, without passing off an unconfirmed timeout as an objective verdict.

One effective review per `(order_id, customer_id)`. `amend` is the next signed version; `withdraw` is a signed removal of the current rating; `reply` is a separate message from the executor, not a second vote. The executor does not get the right to change/remove the client's rating. History is kept under the paid retention policy; withdrawal does not promise to physically erase replicas or readers' memory.

If one customer key signed incompatible edit branches, all valid branches are kept as evidence. The normative draft reducer: root revision=0; the parent belongs to the same review and revision=parent+1; among complete valid branches, the head with the maximum `(revision, canonical_event_id)` in a fixed byte order is selected. Missing parents are pending, not a new independent vote. The specific revision order/limits are fixed by vectors before code. The author can change their own opinion but cannot get extra votes.

The right check takes into account accepted identity/revocation epochs and historical proofs, not only the current display name. A change of service owner does not erase old reviews; by default, the rating of the new owner/service epoch is shown separately with the available history.

## 5. Publication without the executor's server

A public review bundle consists of the receipt, the minimal history for verification, and the signed event. It is stored openly as deliberately public data; private correspondence remains E2EE. Sizes, signatures, and proof eligibility are checked before expensive indexing.

The publisher pays for transport/storage within the selected class; a pre-limited sponsor is possible. Storage/repair/TTL use the existing D/P/N primitives, not a free eternal obligation of the network. The receipt, text, amendments, and discoverability pointers are replicated; expiry and availability are shown separately.

The index is addressed by service/owner epoch and served by several independent peer paths. It does not have to belong to the executor. The client verifies signatures and merges answers. Censorship of one index does not prevent finding the review if an available honest path exists; absolute availability when all copies/TTL are gone is not promised.

Publicity does not mean a globally complete journal. When completeness is unknown, the API returns `partial/unknown_completeness`, the list of queried sources/cursors, freshness, and the corpus hash. Two clients with different observations may see different numbers; an identical corpus always gives an identical result.

## 6. Basic rating

This is a reproducible representation of reviews, not a global consensus about true quality. The baseline policy does not require a Google/Telegram credential, does not weight votes by money, and gives no automatic weight by trust level.

For a specific `ServiceID + service_epoch + provider ownership_epoch`:

```text
eligible = valid receipts + authorship + permitted rights
current = one effective review version per order/customer
active = current without withdrawal, with a permitted score
count = number of active
sum = sum of integer scores
histogram = number of scores 1, 2, 3, 4, 5
mean = sum / count if count > 0; otherwise no_rating
```

The answer includes rating_policy_version, dataset_hash, sources/cursors, freshness/retention, completeness, count, and distinct customer count. Display rounding is fixed separately from the exact sum/count. A reply does not change the rating; an amendment replaces the old contribution; repeated delivery/copies do not increase count.

A known self-review of an OwnerID is excluded from the basic rating. Different colluding identities can sign a fictitious order. One right per order does not solve Sybil, bought reviews, wash-trading, or identity change. V1 prevents forgery of rights, double voting, and hidden sample substitution, but does not claim manipulation is impossible. Hardened trust/economic policies are future derivatives that do not change the raw signed evidence.

## 7. Moderation, security, and UX

Text is untrusted data. HTML/scripts are not executed; external media are not loaded automatically. The publication preview shows the fields being disclosed and binds the confirmation to a specific action hash. Neither a task result nor prompt injection can grant itself the `review.publish` scope.

Local mute/filter/report is allowed, but it does not erase the event in the network and does not disguise a filtered aggregate as the basic result over the same dataset. The operator's storage/content policy is disclosed separately. In V1 there is no global rating administrator who "fixes the truth".

The service screen distinguishes: declared characteristics; confirmed account control; the number of confirmed orders for which reviews were found; the reviews and their sampling; the executor's statements; client checks; the absence of independent provenance. An unavailable index is not "0 reviews".

## 8. MCP and acceptance

`reviews.eligibility`, `reviews.publish`, `reviews.amend`, `reviews.withdraw`, `reviews.reply`, `reviews.list`, `reviews.get`, `ratings.get`. The tools use the same core as Tauri. Reading and publishing are different grants; a separate trusted flow is applied for human confirmation.

Mandatory demonstration: the parties accept an order, the executor disappears without a result/payment, the client publishes a negative review from their receipt, an independent client finds it and computes the rating. Then redelivery, two concurrent edits, a provider reply, a withdrawal, a vanished index, a key change, and a partial corpus do not break rights/deduplication/honesty of statuses.

Cards Q01–Q06, M07, U08, and X07 are V1 implementation. They do not depend on the presence of the V2 economic engine. This document does not declare product code and tests ready.
