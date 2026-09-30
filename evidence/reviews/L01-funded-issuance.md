# L01 funded issuance — local contract checkpoint

Date:2026-09-05. User objective remains the complete working V1; L01 and E01–E26 are not
accepted merely because this dependency passes. Scope is `spec/postage/funded-issuance.md`.
No production network, public testnet, money transfer or wallet credentials were used.

## Test-first evidence and independent review

Tests were written before `contracts/src/PostageIssuer.sol` existed. Initial Forge RED
is `L01-issuer-red.log`; the Anvil runner RED is `L01-issuer-anvil-red.log` and fails while
compiling the missing source. `/root/node_test_critic` was reused as the separate originally
context-free critic; the parent waited for FINAL verdicts before continuing production work.

Initial REVISE found four gaps: callback increments could revert with the nested call,
17-class input also had invalid fields, ABI-by-name could hide overloads, and an Anvil
artifact could be stale. Corrections preserve callback observations with a caught nested
revert, fill17 valid classes, compare all signatures/mutability without collapsing entries,
and force-build plus hash current sources before/after the run. FINAL ACCEPT preceded
production implementation. Mechanical helper naming and explicit local Ethereum profile
were separately accepted after Anvil rejected auto-selected Optimism/Cancun on chainID84532.
That failed attempt is retained in `L01-issuer-anvil-profile-failure.log`.

Mutation testing then exposed a separate verifier failure: Foundry1.8 dynamic linking
rewrote `new` to `vm.deployCode`, causing the first expected constructor revert to terminate
the entire test. A mutant permitting17 classes survived despite valid per-class entries.
Evidence: `L01-constructor-test-trace.log`, `L01-mutant-seventeenth-class-survived.log`.
A scratch run with `dynamic_test_linking=false` executed all9 CREATE cases and failed at the
ninth, as required: `L01-constructor-mutant-no-linking.log`. The critic gave FINAL ACCEPT
to this configuration correction. The final gate and all6 mutants were rerun afterward.
The earlier `L01-issuer-forge-green.log` is historical, superseded evidence.

## Observed implementation

Constructor-only resource classes bind native unit price, byte allowance, retention,
replica count and finite repair allowance. Purchase requires exact payment, a nonzero unused
commitment, known class,1..65,535 tickets and a future bounded expiry. No division/refund or
external call occurs. Successful state/log/counters commit in the same transaction; a failed
transaction consumes only gas and leaves principal and resource state unchanged.

Standard Solidity ABI domains bind chain ID, issuer address, genesis and complete resource
configuration. A batch leaf binds that issuance domain, commitment, class, count and expiry;
historical batches retain their original domain. Unknown batches are zero-valued. There is
no owner, proxy, setter, blacklist, pause, arbitrary-call, alternate mint, fallback or receive
surface. Deposits currently stay in the contract pending separately designed immutable
settlement; this contract is unsuitable for real-money use in its present stage.

## Final validation

- `L01-issuer-all-green.log`: final corrected full gate exit0,242 Rust tests,14 Solidity
  tests,3 Anvil profiles,30 frontend tests, fmt/Clippy/types/production frontend. Its EVM
  sourceHash is `529a0dd3cc0282142c9ffe5644f19317138ab3ff2b2c42bc8ccb17e06cf87585`.
- `L01-issuer-evm-green.log`:14 Solidity tests, including256 runs of a12-purchase sequence
  with independent buyers, exact balances/counters, unpaid attempts and duplicate attempts;
  constructor bounds, wide arithmetic, expiry, domains and callbacks are covered.
- Three real local Anvil processes execute the same current creation bytecode. Two differ
  only in genesis, and two only in chain ID, at the same deployer/nonce/issuer address.
  Each scenario checks funded storage/events, four mined failures, gas/principal conservation,
  process termination and disk restart, and provisional state/receipt disappearance on rollback.
  A replacement purchase on surviving history funds the resource exactly once.
- `output/evm-e2e/result.json`: passed:true, cleanupErrors:[],3 localOnly profiles. A chain ID
  of84532 here is a local domain fixture, not execution on Base Sepolia or its finality rules.
- `L01-issuer-mutations.json` and6 mutant logs: remove exact-payment guard, remove duplicate
  guard, hardcode chain ID, admit17 classes, call payer while ignoring revert, or admit expired
  tickets. Every mutant compiles and fails actual tests. Patch descriptions are in the report;
  copies were isolated temporary APFS directories and original source was unchanged.
- `L01-issuer-native-green.log`:242 Rust tests,30 frontend tests, fmt/Clippy/types/Vite,
  all4 actual hidden WKWebView flows, normal release build and deep/strict ad-hoc signature.
  Its initial Forge section predates the constructor-linking correction; use the final EVM
  log above instead. Native runtime sources did not change with that test-profile correction.
- Native restored-settings screenshot was viewed beside the preceding pointer-network
  reference. Layout, relay policy/status and restored route fields are intact. Profiles,
  Keychain fixtures and child processes were cleaned; no test Anvil process remains.

`./scripts/check-evm.sh` runs the required EVM gate, and `./scripts/check.sh` now includes it.
`AIN_FOUNDRY_BIN` selects a Foundry binary directory; `AIN_SOLC` optionally selects an installed
compiler. Exact toolchain versions, checksums and primary references are in dependency decisions.
Forge lint emits advisory findings, including locked Ether (an explicit local-stage limitation),
constructor-loop validation, checked narrowing, timestamp expiry and equality assertions in tests.
A clean compiler/test result is not a completed security audit or public-deployment approval.

## Traceability and unfinished requirements

L01.T01 conservation has real local evidence; L01.T02 uses identical creation bytecode under
isolated domain configurations. L01.N01 domain isolation and no-callback/no-unpaid-mint
behaviour are covered only at this contract boundary. No external receipt is accepted by an
application verifier yet. L01.F01 demonstrates actual removal of provisional EVM state, but
**does not** implement or prove an off-chain finality adapter. L01.E01 public testnet payment
and authenticated commitment consumption remain open.

There is no Merkle anonymity tree, ZK verifier, spent-nullifier quorum, available balance,
operator registry, custody admission, fee splitting or payout path in this increment. The
batch mapping and events are inputs to future authenticated state proofs. Timestamp validity
inherits chain timestamp semantics; it is not a trusted wall clock. Mempool timing/privacy,
paid preemption of a revealed commitment and root authentication need later protocol treatment.
Numbers in fixture classes are not economic calibration or promises of independent replicas.

Next integration: bounded Rust representations/verification of an explicit issuer profile and
funded record, authenticated finalized checkpoints, approved proof/consensus backends, finite
spend admission, durable indexes/ciphertext, independent keepers and autonomous repair. Current
desktop still provides the verified chat/MCP/network functionality recorded in implementation
status. Full V1 goal stays active.
