# Independent backend test review

The existing context-free `/root/index_holder_test_critic` reviewed the new
locator/commitment tests separately under backend-test-critic. Production stayed
at `63fd338` through final ACCEPT and the final pre-implementation compile baseline.
The critic did not modify files or run builds.

Initial verdict: **REVISE**. The stale-fetch negative changed its anchor, so it
would not catch a Core implementation checking only anchor identity. Add a real
alternate signed manifest with the same anchor, revision and lifetime before
the first incoming history commit. Also isolate saved-publication verification
with a valid endpoint-only mutation; changing manifest_hash could be rejected
by comparison with outgoing history without opening the signed records.

Final verdict: **ACCEPT**. Both oracles were added and retained exact SQL checks.
Nonblocking suggestions were also added: real MLS transition with fresh history
revision one and an independent v1 vector's canonical head-hash assertion.
Existing tests gain history:None only, apart from that hash assertion and shared
exporter-helper factoring, which preserves signing behavior.

Accepted SHA256:

- `crates/crypto/tests/mailbox_history.rs`:
  `0e6ce0433f6b50b60fc6b66e71d602c3de01d57abb7f498435e7ab38d4b6591a`
- `crates/crypto/tests/fixtures/mailbox-history/generate.py`:
  `d0745596fcad47097106df9a0f9c9abe457542f1bec53264771a026b9e41f9bb`
- `crates/crypto/tests/fixtures/mailbox-history/vectors.json`:
  `dcc4706ff6bccd93c47ff8964a782212f00aaa2e86c3817c001a1aee47ee02cf`
- `crates/core/tests/support/custody_history_pointer.rs`:
  `e8bf9ac2b374055b4836a424700db1e337d0618f5389a9f927d84ee38337c93f`
- `crates/core/tests/support/custody_history.rs`:
  `a573608a13574071f0da9d7058767a1924a4e65f52bc9dafba7dfd1944b468ad`
- `crates/crypto/tests/mailbox_rendezvous.rs`:
  `f27e861ae137e89c81817ca9325a1e0af404a9d93a1965912db54345c1efb11c`
- `TEST_CONTRACT.md`:
  `19d32872a2fec70b519df58ac62330d09c16a42a34d3f6e23fc8a78e4cecd342`

Baseline-1 preserves the earlier tests. Baseline-2 uses the accepted inputs:
crypto has 11 errors and Core has 21, all absent API/type/field errors. Neither
baseline executed runtime assertions; inputs were unchanged during both runs.
