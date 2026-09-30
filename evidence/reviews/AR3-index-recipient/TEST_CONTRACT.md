# AR3 — accepted Core prerequisite test contract

Sender source is accepted at `2fd8be4`. These tests are now applied before
any recipient production change. The independent context-free critic first
returned REVISE, then ACCEPT after adding genuine MLS epoch transition (including
empty tokens), separately expiring retention and nonzero-cursor replay rejection.
The applied file matched the reviewed draft byte-for-byte. Its compilation
baseline produced exactly 59 E0599 errors for the three absent APIs, with no
fixture/path errors. The critic confirmed final ACCEPT before production.

Production shares request preparation, current source/revision checks and the
existing progress transaction builder between direct and indexed reads. All
seven new tests and six original progress tests pass; the broader conversation
custody group passes 22 tests. That Core gate remains historical. Subsequent runtime integration is accepted
separately in `runtime-checks.json` and `RUNTIME_TEST_CONTRACT.md`.

The first Core step prepares a recipient capability for one index entry using
the existing per-source durable bookmark (limit 1, maxBytes 262144). Accepting
the descriptor page checks the live request, current incoming MLS index/epoch,
signed descriptor, ordering and exact next cursor without writing anything.
It returns an opaque in-memory token. Zero entries require unchanged cursor.

Completion fetches one exact descriptor-bound ciphertext, rechecks the current
incoming direction, bookmark revision, retention and token time, then reuses the
existing atomic message/MLS/dedup/bookmark commit. An empty result resets that
source's traversal to zero only at completion. Neither result proves complete
history. The source cursor belongs to the index peer, not the ciphertext holder.

The request's 30-second capability authenticates the index response when it
arrives. The accepted local token lasts at most 120 seconds (the existing runtime
work budget), so subsequent holder lookup/fetch does not require extending a
remote read capability. Acceptance after capability expiry, completion before
acceptance, after token expiry, after descriptor retention, after incoming epoch
change or after another bookmark commit must fail. Each actual holder request
gets a fresh target-bound capability and all actual Noise/connection checks.

One descriptor per traversal avoids interpreting the index response's compact
byte budget as a bound on an arbitrarily larger batch of fetched ciphertexts.
Existing data-page reads retain their 32-entry behavior and four source bounds.

Tests planned before production: genuine MLS delayed fetch beyond read-cap TTL;
no writes/ACK on planning; cold independent source recovery and dedup; valid
foreign/reverse descriptors, mismatched/missing ciphertext and cursor rejection;
live request/token/current progress fences; real INSERT and UPDATE SQL faults
with cold retry; empty-page reconciliation. Reuse conversation/custody fixtures.

Then runtime index -> verified compact holder -> authenticated endpoint -> exact
ciphertext fetch/import, plus migration of latest pointers to confirmed index
endpoints. A real ordinary sender-off, cold recipient/index gate must recover
messages from disjoint surviving holder sets with no owner retrieval work. Book
and epoch continuity/completeness, durable Welcome, successful retirement and
autonomous R10 repair remain separate required work, not implied by this slice.

## Runtime notes to validate before implementation

Reuse `Runtime::lookup_peer`: it already bounds DHT routing, obtains a signed
NodeRecord through the authenticated bootstrap exchange and populates Core's
checked route cache. Derive the peer from the verified holder transport key;
never let a relayed location choose another Noise peer. Keep existing shared
custody slots, response deadlines and recipient admission bounds.

The node verifies the selected paid index obligation against the actual replying
peer before Core accepts its descriptor. It reconstructs each compact location
with that immutable index descriptor/payment/QC, runs the historical holder/copy
verifier, then fetches sequence `descriptor.sequence` with a fresh capability
targeted at the actual holder. Core's final token binds imported ciphertext to
the accepted descriptor and the original index source's bookmark.

Existing pointers have no explicit data/index kind. A compatible migration must
retain bounded direct-data fallback for old locators. Merely replacing endpoint
selection with the latest message's index list can lose earlier books: preserve
the existing cross-message reachability condition until explicit durable book
continuity replaces it. Same-book index traversal is not cross-book acceptance.
