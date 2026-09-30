# L02 — published operator proofs on authenticated peer connections

Status: implementation, focused checks and all aggregate gates pass. This increment does not close V1 or L02/N05/D03.

## Test contract and independent review

Contract: `spec/registry/operator-network-v1.md`. Tests preceded production; missing Core APIs produced the recorded RED (`L02-operator-network-red.log`). The separate existing `/root/node_test_critic`, originally created without forked context, returned FINAL ACCEPT after revisions. No work was performed while waiting for each final verdict.

The critic required executable Python import defaults, expiry checks that cannot be masked by disconnect or a preceding time-persisting status read, a genuinely accepted successor while the previous lease remains valid, disabling an enabled publication after expiry, and a valid oversized JSON proof whose rejection must occur at the wire limit. All were corrected before production. A later narrow fixture correction uses actual `node_info.listeners` addresses without appending a duplicate PeerID, strips the asserted own PeerID suffix only for restart listen arguments, and also received FINAL ACCEPT without blockers.

## Implementation

- Core stores bounded owner publications in SQLCipher, atomically with the durable checkpoint clock. Exact retries preserve revisions; conflicting updates fail; disable works after expiry. Serving requires a current, enabled publication, permitted role, valid own opening and fresh authenticated proof. It reuses existing operator signing.
- Core remote verification reuses the existing checkpoint/registry/MPT/member/binding verifier and the shared full registry result formatter. Incoming presentations are untrusted public data and do not import private operator identities.
- Daemons exchange bounded CBOR over `/agentic-internet/operators/1`. The expected Ed25519 transport key comes from the actual authenticated event PeerID identity multihash/protobuf; the derived PeerID must agree. Selected checkpoint/domain, requested member index, role and actual response connection must agree.
- Owner diagnostics allow four pending and64 retained ephemeral jobs, five-second request timeout and60-second retention. Every use of a verified result rechecks current Core trust, current actor time and its live permitted connection. Expiry, successor, disconnect or network replacement removes usable authority. Network replacement cancels pending requests; restart restores no verified job.
- Owner publication/diagnostic fields are strict and bounded. Signed agents and caller time/key/root overrides are refused. Existing inbound admission and relay-only policy are reused. No new dependencies.

## Focused results

- `L02-operator-network-core.log`:19 Core registry tests pass, including five new publication/remote-verification scenarios with real SQLCipher rollback/reopen/current-head/clock assertions.
- `L02-operator-network-node.log`:five filtered actual daemon tests pass, including two new owner/transport/bounded-cache tests over TCP and QUIC; ordinary MLS delivery remains usable.
- `L02-operator-network-clippy.log`:Clippy all targets for touched crates passes. Initial compiler corrections (closure Result annotation and optional checkpoint status) and one needless borrow were resolved before aggregate verification; they are not product test successes.
- The aggregate EVM entrypoint now invokes `tests/evm/operator_network.py`, retaining all previous live identity/registry/CLI gates. Its independent wire peer does not use production presentation structs or verifiers.

## Recorded aggregate failure and correction

The first native aggregate passed333 Rust and24 Solidity tests, then the new Anvil fixture failed before remote-role checks: it incorrectly required an unchanged global EVM state root after mining empty blocks. Its log/report are retained as `L02-operator-network-native-first-failed.log` and `L02-operator-network-anvil-first-failed.json` (zero verified remote roles, cleanup errors empty). No product success was claimed from this run.

The fixture now verifies unchanged registry storage hash and exact storage proofs while retaining actual later headers, and republishes with each header's own account proof. The verified result must identify that exact header's state root. The critic independently returned FINAL ACCEPT; the successful-old-lease/live-connection head-change and expired-result assertions remain. Production source did not change for this fixture correction.

Linux finished successfully before the EVM-only fixture correction: seven outcomes, run `ain-nat-90b1a6c7`, source hash `c2abdd359f0d47ee377f99ef7784e8f651dd2fcbb97d6ee460ba587d6225321b`, cleanup errors empty. The Linux hash inputs do not include this EVM fixture; its already-passed source and image remain unchanged. Evidence: `L02-operator-network-linux.json` and network log.

## Acceptance boundary

An authenticated paid role proves a finite registry capability bound to a live transport key. It does not prove physical/operator independence, data retention, retrieval, spending admission, placement or R=10 repair. Native registry controls, production chain observation and the remaining V1 product scenarios are still required. Local macOS release is ad-hoc signed, not notarized.

## Live operator result

`L02-operator-network-anvil.json` passes on chains31337/31338 with16 verified remote roles, two copied-binding refusals, two wrong-index refusals, two wire-limit failures followed by valid recovery, two four-pending capacity/network-replacement checks, two live-successor invalidations and expiry invalidation while original connections remain alive. Public proofs are served after actual provider restart with the source chain stopped. Source hash: `53431d3545b4fb8c2de22204d6bc52cbbaee5171c97cf980a15ee01a36de801f`; cleanup errors empty.

All old live gates remain: this run's operator identity report has96 actual provider owner calls and16 independently verified exact role signatures, source hash `83b2eb20a411bce96cd270f2cd3f58b59e3e1a69610e288807ed1437cefb15c5`, cleanup errors empty. The previous registry/CLI/funding/checkpoint gates also pass. Full suite currently reports333 Rust,24 Solidity and40 frontend tests passing.

## Completed native acceptance

`L02-operator-network-native.log` exits0:333 Rust,24 Solidity (including256-run fuzz cases),40 frontend tests, TypeScript/Vite, five actual packaged hidden WKWebView flows, release bundle, deep strict ad-hoc codesign verification and no webdriver plugin in the default release dependency graph. No further production changes occurred during the successful aggregate run.

Own visual inspection displayed current `output/native-e2e/alice-chat.png` and `checkpoint-restored.png` alongside corresponding preceding de269a2 images under `/Users/glebk/Library/Caches/agentic-internet/recovery/native-reference-de269a2`. Controls, layout and wrapping agree; only expected timestamps differ. The release remains local macOS arm64 and is not notarized. No complete V1 or independent custody claim follows from these checks.
