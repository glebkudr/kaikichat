FINAL REVISE — independent no-context reviewer /root/common_context_test_critic.

Blocking: add exactly one actual matching remote ACK refusal alongside the existing two-receiver positive; add a live completed lookup whose result remains queued until after signed expiry, keeping an immediate-read positive. Existing queued-before-expiry test only verifies stale ingress.

Nonblocking: use a fresh valid final control in the hostile responder test; extend live authority-loss and relay-only coverage as discovery grows. Raw TCP/QUIC peers, local-only receiver storage, publisher shutdown, budgets and live finality oracle otherwise meaningful. All5test/spec,91production and6helper hashes match. No edits or live gates.
