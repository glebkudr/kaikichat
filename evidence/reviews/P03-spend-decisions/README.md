# P03 authenticated spend decisions — bounded implementation

`agentic-postage-spend` checks a genuine receipt against an opaque Core context
and current issuer-bound service authority. Proposal and voting share complete
prefix validation with nullifier as the canonical consensus key. Only a real
finality ancestry proof permits an atomic SQLCipher result. Identical retries keep
the exact first certificate; another operation cannot replace the spent result.

Nine targeted tests passed: seven decision/effect scenarios, one actual four-engine
P256 partition/heal scenario and one guaranteed later-roster authority fixture control.
The four engines use real voting/WAL/Marshal and genuine resulting QC, with simulated
network/time and independent Core profiles. Unit boundary tests also assemble genuine
quorum signatures explicitly; those signatures are not claimed as a network run.

Two new genuine receipts were created from one paid canonical-issuer ticket on a
local EVM chain. Same nullifier, different operation commitments; exact independent
319-byte journals and unchanged image `ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de`.
The source chain stopped before proving. Verification uses historical authenticated
fixture time; this is not live wall-clock admission. No private funded-owner seed,
salt or witness was retained. First authoring attempt failed its fingerprint and
was rejected; that failure is retained separately from the successful second run.

The complete public corpus is in
[competition.json](../../../crates/postage-spend/tests/fixtures/competition.json),
[receipt A](../../../crates/postage-spend/tests/fixtures/receipt-a.json) and
[receipt B](../../../crates/postage-spend/tests/fixtures/receipt-b.json).
The generator is [postage_competition.py](../../../tests/evm/postage_competition.py).
It reuses actual EVM registry/funding helpers and offline prover/verifier/oracle.
Separate existing Rust CLIs validated both first/later committees and issuer bindings.
See [review](review.md), [fixture generation](fixture-generation.json),
[CLI controls](fixture-cli.json), [contract](../../../spec/postage/spend-decisions-v1.md).

Full ordinary validation passed **524 Rust tests** (zero failed/ignored), **40
frontend tests**, seven models, nine independent oracle tests, workspace fmt/Clippy,
TypeScript and Vite. See [validation](validation.json) and [summary](summary.json).
All 433 final source hashes and 15 accepted input hashes remained unchanged.
Exact frozen test/source manifests are retained here; raw compiler/test/prover logs remain ignored under
`output/spend-decisions` and `output/postage-competition`.

## Outstanding product work

No daemon spend endpoint, receipt distribution, 100-concurrent-request daemon
acceptance, actual paid ciphertext admission or epoch handover was added. The
module rejects subsequent epochs until spent-state continuity is implemented.
The local spent row alone never establishes global consensus. The full V1 goal,
including all E01–E26 scenarios, remains incomplete.

No UI/MCP endpoint changed. The desktop package, native UI, Linux network matrix
and remaining full EVM/proof suites were not rebuilt or rerun for this library-only
increment. Existing app release evidence remains at the preceding checkpoint.
