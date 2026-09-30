# Independent backend test review

Baseline: aa286d9. The existing no-context reviewer
`/root/common_context_test_critic` received a standalone request for the new Core
postage service contract, four test scenarios, genuine two-chain fixture/generator,
shared helpers and missing-interface RED. No production change preceded acceptance.

The initial verdict was REVISE: the head-renewal scenario did not explicitly pair
the correct new checkpoint ID and complete new roster with an old genuine bound
issuer proof. The test now rejects exactly that combination and requires the fresh
proof to return the new state root. The fixture generator makes a real 28-wei
purchase of four tickets between these roots, while preserving both binding words.
No expected success or previous assertion was removed.

The reviewer then returned FINAL ACCEPT. It confirmed all twelve frozen inputs,
unchanged previous Rust assertions, distinct actual roots and the issuer balance
change on both chains. Its independent CLI verification passed eight binding checks
(including stale-proof denial on both chains) and eight finalizer selections. It
reported no remaining blockers or material missing scenarios. It edited no files.

The RED failed solely because PostageServiceAuthority and prepare_postage_service
were absent. This demonstrates the missing API, not execution of the assertions.
The separate fixture-only run validated six policy bindings and eight selection
results with the chains stopped; it explicitly recorded coreTestsRun:false and
was not treated as Core acceptance. Initial fixture-authoring mistakes about an
empty block's root and hashing the explicit output write were corrected before
final review. No mock proof verifier or production stub was introduced.

After ACCEPT, production reuses the existing full-roster check, selected enabled
key, finite service fence and durable checkpoint clock. The supplied binding proof
is verified at the current authenticated root against the installed policy. Both
public preparation paths share one internal constructor and commit observed time
before returning the signer. No dependency or wire/IPC endpoint was added.

Final results are in validation.json, evm.json and compatibility.json. The EVM gate
uses fresh actual proofs for all four Core scenarios after both chains stop; the
ordinary suite uses the separately accepted committed public fixture. Accepted
input hashes remain fixed. This increment supplies current Rust service authority;
it does not start consensus, create a spent ledger, grant admission or solve epoch
handover. Full V1 acceptance remains open.
