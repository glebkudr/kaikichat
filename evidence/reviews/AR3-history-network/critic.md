# Independent backend test critic

Separate context-free reviewer `/root/index_holder_test_critic` returned **ACCEPT**
before production changes. It reviewed the three store tests, new native gate,
existing genuine fixture helpers and the test-only stopped-sender Core manifest
preparation. No blocking gaps or required additional scenarios were found within
this storage/server transport contract. The reviewer did not edit or run tests.

The reviewer accepted exact nonfirst anchor, complete payload limits, actual Core
signature, historical trust, real SQL faults, cold recovery, revision rollback and
no ciphertext at the index. Ordinary sender/recipient manifest use remains a
separate required integration, without being claimed by this gate.

Optional suggestion: record intermediate public history responses before assertions
for better failure diagnosis. Existing native trace retains inherited paid proofs
and successful history results; failure logs and independent input/binary hashes
remain available for every run.

Accepted SHA256:

- `crates/postage-spend/tests/support/index_history_bundle.rs`:
  `c60f4b1be389d5459b62d52fd16883823c7a3a2ea740df271e9c150093e46b43`
- `crates/postage-spend/tests/support/index_history.rs`:
  `ef9e0d9f09c89a0d3888605448862d16d11d4ddc92c9c4d8b537e1f3b179016d`
- `tests/evm/paid_history_network.py`:
  `f86004b5d5f0a622a638d392b600c1abfc0808b3a9b1b9a68c4223babe944c5a`
- `crates/node/examples/postage_spend_store_failure.rs`:
  `55ef7065b4f638f99e287f34817e0c1e46664d8294a369f3db66d5823211164e`
- `evidence/reviews/AR3-history-network/TEST_CONTRACT.md`:
  `c060a81afdca5414969e3b9b1eafc9c373ff85734dbe44a139da06f1e62510c3`

Baseline store-1 is compilation RED with one E0432 and 11 E0599 missing-type/API
errors. Baseline native-1 is runtime RED at the new typed history ingress before
funding/finality/history assertions. Its current source/binary hashes match, 607
inputs are unchanged and cleanup completed without errors. Neither RED is claimed
as execution of the later successful-history assertions.

## Fixture batching correction

Candidate native-1 passed original paid/index/copy recovery and reached the history
guards after committed put, SQL rollback, cold read and independent manifest
signature checks. The independent peer then panicked on its deliberate four-request
limit because the test supplied five guards together. The candidate report remains
failed; this is not a product acceptance result.

Only the native test changed: the same five requests and expected outcomes were
split into batches of three and two. Production stayed unchanged. The separate
critic returned **ACCEPT** before rerun, confirming no assertion was removed or
weakened. Final native test SHA256:
`7ed79b6023c1499c55f3bafac1dc1982b8bb8ca1085c7addcf999446b9c8db04`.
The other accepted test/helper/contract hashes above remain unchanged.
