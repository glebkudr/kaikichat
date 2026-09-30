# N05 — automatic resolution of paid custody positions

Status: implementation, focused checks and full backend/frontend/EVM/native/Linux gates pass. Full V1 remains open.

## Test-first review

The new contract is `spec/registry/custody-resolution-v1.md`. Tests were written before production. The separate no-context-fork node test critic first returned REVISE: oversized responses could fail for an unrelated reason, held requests relied only on daemon diagnostics, and binding expiry was not isolated from assignment expiry. The accepted tests retain valid JSON when padding proofs, distinguish transport/codec failures from rejected presentations, capture actual held requests, and check the exact binding horizon under a longer valid assignment. Core and real owner/agent RED logs show absent APIs and unknown_method before implementation.

FINAL ACCEPT preceded production. Further narrow ACCEPTs approved only setup corrections: creating64 SQLCipher rows in batches within the existing16-state transaction limit; bounding live expiry observations by RPC start/end times and comparing unique expected PeerIds; using a short temporary AF_UNIX path, registering startup cleanup first, and appending the new runner after all prior EVM gates. No production continued while a verdict was pending.

## Implementation

Core reuses its stored intent/funding and authenticated checkpoint history to prove the selected position, then checks the custodian binding against the actual transport key. Shared private verification helpers preserve the single durable observed-clock transaction. The result's validity is the minimum of assignment and operator validity. SQL failures release no verified result.

The provider locates existing enabled publications through bounded, sorted, case-sensitive namespace discovery. All namespace suffixes must be canonical u32 values. Membership proves the full-registry ordinal independently of the gapped index; existing publication loading, registry verification and binding signing are reused. No key export or state migration is introduced.

The new `/agentic-internet/custodians/1` protocol carries public proof requests and responses with finite byte limits. The owner starts a job using its operation ID, ticket index and registry proof. The daemon obtains its actual head and snapshots at most32 allowed connected peers; positions remain the deterministic paid selection. Four requests may be in flight globally, two jobs active, and16 results retained. Each candidate is attempted once per position. A job ends after60seconds and completed results expire after60seconds.

Every result read revalidates saved presentations through Core and the actual connection. Expired or disconnected offers become unavailable; changed heads, expired assignments and network replacement invalidate the whole result. Relay-only filtering applies to candidates, responses and reads. Restart restores no verified result cache. Diagnostic counters distinguish nonempty rejected presentations from outbound stream/codec failures.

## Scope still open

A resolved position proves a currently connected selected custodian, not stored ciphertext, a consumed ticket, an independent operator or R=10 durability. Discovery currently uses connected peers; reaching disconnected selected operators requires a routing directory. Actual custody transfer, receipts, retained funding history, repair, payment UI and the remaining mandatory E01–E26 product scenarios are unfinished. No real-money deployment occurred.

## Focused evidence

- `N05-custody-resolution-core.log`: all7 new Core cases pass on independent paid fixtures, including exact ordinal, wrong-position positive controls, legacy publication discovery, malformed namespace, role/size bounds, binding expiry, head/clock/SQL failures and reopen.
- `N05-custody-resolution-registry.log`: all32 registry/Core cases pass after sharing the verification helpers.
- `N05-custody-resolution-store.log`: exact prefix/literal wildcard behavior,64-row capacity,65th-row refusal and encrypted reopen pass.
- `N05-custody-resolution-node.log`: actual owner/agent authorization, forbidden override fields, valid padded proof boundary, queue preservation and original delivery after restart pass.
- `N05-custody-resolution-clippy.log`: targeted all-targets Clippy with warnings denied passes.

## Actual paid resolution evidence

`N05-custody-resolution-live.log` exits0. The preserved focused JSON report records288
independently verified position outcomes through2652 real owner calls across chains31337
and31338. Each chain runs16 separate active operator daemons with fresh private registration
keys actually bonded into the registry, plus the owner daemon. The owner generates its own
intent before beacon and binds actual payment. TCP and QUIC both carry resolved positions.
The independent complete-list oracle supplies exact selected ordinals and registry indices.

The live runner confirms four actually received held requests, no fifth stream before a slot
is released, rejection of a third active job, and the full60-second deadline while the
assignment remains live. Binding expiry is observed separately under the same live head;
fresh resolution recovers. A disconnected selected operator affects only its position;
copying its current public binding onto another Noise identity is rejected. Valid-JSON proof
padding beyond the wire bound causes a transport failure distinct from Core rejection.
Relay-only replacement invalidates cached direct offers and sends no direct candidate query.
Reconnect, accepted successor, republishing and daemon restart are covered. Both source
chains stop before actual current-head expiry invalidates every cached position.

Focused source hash: `b72da42a22ef2364a32a28020c1aa826cf6d08db29509bf702793268716432cc`.
Cleanup errors are empty. The aggregate reruns this same scenario after all previous EVM gates.

## Real cancellation regression

A further separately reviewed test (`tests/evm/custody_resolution_slots.py`) holds four actual
requests, accepts a real successor checkpoint and starts a fresh paid job. Before the fix,
`N05-custody-resolution-slots-red.log` fails on a fifth captured request: deleting logical
pending entries did not close the handler's existing streams. Its failed JSON report is retained.
After this observed RED, a bounded draining set keeps retired request slots occupied until
Response/OutboundFailure; dropping the old swarm on network replacement clears them.
The authority of the old job is removed immediately. Owner maintenance and Core use the
same captured clock value, avoiding a second observation advancing time inside one call.
The new scenario is included in the source hash and has a mandatory final report flag.
The test and its integration received FINAL ACCEPT before this production fix.

## Linux verification

The final Linux image passes all7 actual network outcomes, with no cleanup errors.
The retained initial failure occurred before the cancellation fix: after successful relay
failover delivery, an immediate first-connection route assertion failed. The unchanged retry
passed all7 outcomes, and the final implementation passed them again. The failure's log/JSON
and the pre-cancellation successful JSON are retained; no test was relaxed and no relay code
was changed. This evidence does not establish a root cause for the transient route observation.

Final Linux run: `ain-nat-12283c76`, source hash `c966a888df23fcb07346fb271fef69736082facbfe0918642026a910e93c1b5b`.

After the cancellation fix, the full focused two-chain runner exits0: 292 verified position outcomes, 2693 owner calls, mandatory headCancellationRetainsSlots=True, cleanup errors empty. Source hash `48b96a8a59c9e430d1c7f6331cf3786b2e8462c0a24f44d8d1fe51a5e2423ea8`; retained report `N05-custody-resolution-final-focused.json`.

## Final aggregate and native visual check

The final `N05-custody-resolution-native.log` exits0. It runs360 Rust tests,24 Solidity
tests,7 Python risk-model tests and40 frontend tests (431 total), warnings-denied Clippy,
TypeScript and Vite, all prior EVM gates and the new paid resolution scenario. The final
aggregate resolution report records293 verified position outcomes through2693 owner calls;
the mandatory cancellation flag is true and cleanup errors are empty. Its source hash equals
the final focused runner's hash above. The different position tally reflects which bindings
have expired at the independently bounded expiry observation, not different selected positions.

All5 actual hidden WKWebView flows pass. The rebuilt macOS arm64 release contains the
normal desktop/daemon/MCP binaries, passes strict deep codesign verification, and excludes
the webdriver plugin from its default dependency graph. It is ad-hoc signed, not notarized.
The final `alice-chat.png` and `checkpoint-restored.png` were visually inspected alongside
the same images at9a10ce0. Layout, messages, delivery state, trust keys and controls match;
only expected generated timestamps differ. Screenshots and native JSON are retained.

The seven final Linux outcomes pass on the source hash recorded above. Full V1 and all
remaining product cards remain open; these results do not assert stored replicas or repair.
