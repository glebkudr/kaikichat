# Independent backend-test-critic R2

Reviewer /root/common_context_test_critic, inherited context false. FINAL ACCEPT.

Both blockers resolved. Cold load modifies exactly one signed wire inside valid JSON, preserves metadata and restores original bytes. Unseeded cold recovery excludes network/peer-records through a helper holding actual ProfileStore lock, transactionally deleting only that row and comparing every other state. Signed service cache retained; contacts, LAN, relay and AutoNAT sources excluded. Prior fault-injection branch byte-exact; previous live assertions not weakened. No new blockers or mandatory missing scenarios.

Nonblocking: exact 160KiB positive boundary, Runtime persistence-error/no-dial recovery control.

All 9 test/spec/fixture, 84 production and 11 helper hashes exact. Production unchanged, registration-only additions confirmed. Reviewer did not modify files or run builds/live gates. Acceptance covers tests only; actual RED/GREEN and full validation still required.
