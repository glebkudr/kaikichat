# Checkpoint network test review

Separate backend-test-critic: /root/node_test_critic, originally launched without context fork.
The parent waited for each FINAL decision and did no implementation work during review.

1. FINAL REVISE: a single held source did not prove a shared outbound slot; exact throttle and
   swarm cancellation needed stronger checks. The stale reply was a duplicate, so Core duplicate
   rejection could hide incorrect rebasing to the current head.
2. Changes: two deterministic scheduler tests for a common slot, source refresh, cancellation
   before5second timeout, exact1second global start budget across source/swarm changes, fair
   failure retry, idle30second polling and1/2/4/8/16/30/30second backoff. Actual process test now
   authenticates two simultaneous held sources and counts their combined captured wire requests.
   After an owner moves head1→head2, the held reply returns head3: valid for currenthead2, but
   invalid for original requesthead1. Head2 must remain exact. Old fixture endpoints stop after
   swarm replacement so cached bootstrap cannot legit reconnect and mask the new requested anchor.
   Decodable bad responses require rejectedResponses; an over-frame response requires failedRequests.
3. FINAL REVISE: inFlight did not imply a request was already captured at the raw peer; source
   selection could race. Added bounded waiting for exactly one actual wire capture before choosing
   the receiving peer. Final follow-up: FINAL ACCEPT, no remaining required scenarios/blockers.
4. After implementation targeted GREEN, added native acceptance through a second actual packaged
   hidden WKWebView and separate daemon. It explicitly selects trust and enters the first client's
   listener in native network settings, never fills/submits a certificate, observes the exact
   independently expected head/lease/trust, then restores after source loss and both local process
   restarts. Separate review: FINAL ACCEPT. A local unwrap_used allowance for the new unit-test
   module was also accepted as a mechanical lint fix with unchanged assertions.

Original RED: Core E0599 missing checkpoint_sync_anchor; scheduler E0432 missing network module;
all six actual-process scenarios compile and fail for absent protocol/request/convergence.
An initial fixture cleanup double panic was fixed before the retained process RED; final fixture
propagates wire failures without aborting process cleanup. No orphan daemons remained.

No production implementation preceded the tests' FINAL ACCEPT. Native additions extend actual
acceptance of existing UI polling/controls and the new daemon transport. Fixtures use synthetic
signed headers for transport/lease boundaries and do not claim live chain finality or spend balance.
