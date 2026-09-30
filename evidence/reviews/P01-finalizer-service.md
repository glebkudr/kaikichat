# P01 selected authority and embedded service review

Baseline: `3337966e4a27de436afc99dfc99078a74118e2f1`.
Full V1 and full P01 are not complete. This change supplies scoped selected runtime
credentials and an embedded dynamic application service. Actual selected voting through
the daemon's libp2p connections and production spend/group admission remain required.

## Test-first review

The independent no-fork `core_test_critic` reviewed tests before each production module.
Initial scope/Core RED runs failed on the missing APIs. The critic required: foreign
profiles with identical L2 rows/revisions; valid alternative raw JSON encodings at the
same revision; and preservation of private expiry across `into_runtime()`. These were
added, together with revision-only cases. Invalid opaque operation IDs use a control
character, preserving the established ID contract. FINAL ACCEPT preceded implementation.
Core test SHA256 at acceptance:
`cd7a81924118964e37a67bdfb425cde4f9f8fb3575b2b552213c2939727ee0ea`.
The scope suite checks independent two-chain CBOR vectors and real P-256 QCs.

The service runtime suite first failed with the absent `agentic_finalizer::service`
module. Review required complete newest-first ancestor assertions in both callbacks,
observed unauthorized proposals and honest rejection, and route expiry while the
service's own authority remains live. The revised suite also refreshes routes during
long scenarios, checks a resumed callback's complete history and separates short
service expiry from storage/route checks. A borrow conflict was fixed before ACCEPT.
Service test SHA256 at pre-production acceptance:
`d5ce09a09d0aa5904d711070f9a1791ce2b6983deca9966115ccb20c345ea06c`.

The first implemented run correctly rejected the fixture's default `0755` temporary
directories. After separate FINAL ACCEPT, successful fixtures explicitly create `0700`
directories; the intentional `0755` negative remains. A separately accepted type alias
resolves clippy's type-complexity diagnostic without changing behavior or assertions.
Final service test SHA256:
`94d2f1e9a497b49a455ea9d419e777e56c75cf1448eb60b7904c20a2079a4181`.
No concern was overridden. Tests were not weakened to obtain green results.

## Implementation

- Separate canonical application/log identities preserve the selected roster, quorum
  and shorter private proof lease. Transport authentication retains the base committee.
- Core constructs the actual selected signer only after full roster validation, owner
  enablement and durable clock commit. An opaque fence detects local identity, head,
  role, exact state bytes and revision changes.
- A supervised native runtime uses the existing Commonware engine, archives and WAL.
  Dynamic callbacks receive the complete history; a bad callback at one member cannot
  obtain honest votes for unauthorized work.
- Five bounded channels carry only selected peer keys; ingress requires a verified
  finite transport capability. Commonware supplies per-key outgoing rate limiting.
- Durable deliveries carry optional direct QCs and an explicit application acknowledgement.
  Stop/drop/expiry terminates the service; unacknowledged work survives restart.
- The configured validator executables reuse the private-directory lock implementation.

## Boundaries retained

The service tests pump messages between actual engines locally. They do not substitute
for authenticated libp2p daemon voting acceptance. Core fences must be checked by that
integration before admissions and outgoing traffic. No private key-bearing object is
exposed through owner IPC, MCP or JSON. No generic remote application-registration or
arbitrary-work admission is added. The fixture quorum is not a production sizing claim.

Full checkpoint rollback protection, epoch handover, spent-state continuity, private
postage proofs, group sequencing and actual R=10 custody remain explicit project gates.
Validation results and terminal exit statuses are recorded separately after each gate
finishes; this narrative alone is not acceptance evidence.

## Fixture reader repair

The initial aggregate exited1 during `finalizer_network.py`: it parsed a JSON line
while the fixture process was still writing the line. All456 Rust tests,29 Solidity
tests and7 model tests had passed; frontend had not yet run. The original aggregate
log and failed network report are retained, including empty cleanup errors.

Before changing the helper or consumers, three tests reproduced partial ASCII, partial
UTF-8, a valid JSON value without its newline, exact completed-record counts and
completed malformed-record rejection. The no-fork critic returned FINAL ACCEPT.
Accepted test SHA256:
`e7431c3f4e0f81899d0884eb581cd524dc85ff27ae838a02619f6e1b0513034e`.
The shared reader parses completed newline-delimited byte records, leaves the pending
suffix untouched and still raises on completed malformed output. The three affected
EVM runners include the helper in source fingerprints and preserve their assertions.
The ordinary aggregate now also runs these three reader regression tests.

Validation is completed in phases after this repair. The initial aggregate command
is recorded as failed; a successful affected recheck does not rewrite its exit status.

## Completed regression evidence

The final phased result is456 Rust +29 Solidity +7 model +3 reader +40 frontend tests
(535 functions). Formatting and workspace all-target Clippy passed in the initial
aggregate. The repaired finalizer-network, operator-network and custody-resolution
runners all exited0. All18 fresh EVM reports were checked again against their current
source hashes; cleanup errors are empty. Finalizer proof exchange used362 owner calls;
custody resolution verified295 positions through2611 owner calls.

The frontend tests, TypeScript check and production frontend build exited0. The native
command rebuilt the debug package, passed all5 actual WKWebView flows, rebuilt the
release package and passed strict deep signature/default-graph checks. Four fresh
screenshots were inspected beside their exact3337966 references, with no new layout
regression. Release signing is ad-hoc; notarization is not claimed. This increment did
not rerun Linux network acceptance; the prior7 outcomes remain historical evidence.

Terminal exits, individual source/binary hashes and exact retained reports are in
`P01-finalizer-service-validation.json`, `-source.json`, `-evm-reports.json`,
`-native.json` and `-visual.json`. The initial failed aggregate log is preserved.
No full P01 card or E01–E26/V1 product goal is declared complete by these results.
