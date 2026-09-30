# AR1 terminal history boundary

Source baseline: `6295391`. This component configures a trusted application
terminal operation in the existing P-256 history index. The Entry/QC wire format,
Simplex algorithm, quorum, issuer nullifiers and resource bounds are unchanged.

- [Test contract](test-contract.md), [independent critic](critic.md),
  [source fingerprints](validated-inputs.json), [checks](checks.json).
- [Protocol contract](../../../spec/finalized-terminal-history-v1.md).
- [Next implementation work](NEXT.md).

The five new SQL/history tests use genuine existing P-256 fixture signatures.
They cover a 131-entry terminal history, durable binding, ordinary cold reopen,
exact original-QC preservation, current expiry, direct versus descendant terminal
certificates, seven-entry import, failed final activation and malformed terminal
placement with genuine tip QCs. An ordinary ancestor proof through a terminal QC
remains valid. Binding failure cannot invalidate the existing usable history.

A sixth test runs four real Commonware P-256 engines on the deterministic network.
All test applications authorize nine entries; at least one tries entry9 after
terminal8. The terminal entry obtains its own real archive QC while every node
refuses child9. The first phase uses the full actual-parent suffix from genesis.
Then actual archive proofs enter the SQL index, the entire runtime is destroyed
and recovered, and generic SQL opens preserve the terminal rule. A live quorum
again attempts entry9 and refuses it; the terminal archive proofs remain exact.
No hand-authored QC is injected into this engine test.

Initial RED ended101 because the new APIs did not exist. The first targeted run
passed49 tests; it reported one unused-assignment compiler warning in the new
binding test. A later warning-only test cleanup was independently accepted, and
full workspace regression uses that final source. Source inputs were captured
before the full run and all 547 matched at completion. Run logs remain in managed
`output/`; checks.json records their hashes and terminal exit codes.

Final validation: **839 Rust tests / 55 nonempty suites, zero failed or ignored**;
**59 frontend tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt
all pass.

This is generic application closure, not funded postage or epoch handover. No
native funded-spend or packaged application gate was rerun in this component.
The actual node still refuses epoch !=1. Authentication of a successor, issuer
spent-state transfer/bootstrap, old-lease outage handling and actual node epoch
crash/partition/double-spend gates remain mandatory. AR1 and full V1 are open;
prior 144-spend, missed-record recovery and 18-spend renewal evidence remain
historical, distinct runs rather than validation of this new terminal path.
