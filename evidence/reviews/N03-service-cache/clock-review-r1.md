Independent backend-test-critic /root/common_context_test_critic, inherited context false. FINAL REVISE.

Blocking: sibling starts independently with up to 10 seconds startup; immediate running assertion can confuse first startup with false revocation. Use bounded wait while immediately rejecting generation > 1. Real delay and isolated durable effect recovery otherwise meaningful; all 128 production hashes and 2 test hashes exact.
Nonblocking: future slow-callback crossing actual authority expiry would protect post-effect fresh-time check; existing expiry and role/head revocation remain required. No edits/builds by critic.
