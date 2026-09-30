# Independent backend test critic, R1

Agent: /root/public_postage_test_critic. No inherited context. FINAL REVISE.

Receiver-side mutations retained the original signature; failures could be caused
only by signature mismatch. Unpaid/post-beacon/short lifetime cases exercised only
signing. Required independently valid signatures for receiver out-of-count,
short-lived, post-beacon, foreign-key and legacy-funding attempts, with explicit
signature controls, plus corrupted MPT paths preserving all signed fields.

Non-blocking: same book under a second authenticated network. Later integration
must cover spent-log concurrency/retries, handover, wallet persistence, wire and
delivery; none is accepted by the native module.

Response: R2 adds seven Python-signed receiver claims, verifies their signatures
in the Rust test before calling Core, corrupts account/all four storage paths
without changing the signed fields, and checks foreign-seed signing refusal.
