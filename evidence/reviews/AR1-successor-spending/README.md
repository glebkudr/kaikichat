# Successor application spending

Full validation: **867 Rust tests**, zero failed/ignored in 59 nonempty suites;
**59 frontend tests**, TypeScript/Vite, 19 model tests and workspace Clippy/fmt
pass. All **576 source inputs stayed unchanged** during the 947.62-second full
workspace run. See `checks.json` for commands, provenance and limitations.

The explicit successor session requires current selected Core authority and
complete owner-bound history. Store-aware proposal checks issuer-global spent
membership; malformed rows cannot become unused tickets. Ordinary and closing
verifiers compare full entries. Finalization refuses cross-epoch or re-sequenced
duplicates while exact retries preserve first QC/time. Historical lookup accepts
only the original QC within the chosen predecessor chain.

Five new tests passed, including actual receipt/proposal/finalization in epochs
2 and 3 after 130 genuine paid epoch-1 spends, original records after cold restart,
both predecessors' old tickets, nonempty suffixes, owner/revision checks,
independent selected-role and expiry failures, same-committee lease renewal,
corrupt retained records and a genuine unchosen historical committee carrier.
The seven prior import tests also passed after shared helper extraction and
shared public-link authentication. The client-lineage phase has separate
[tests-first review evidence](../AR1-client-lineage/).

The source uses genuine EVM-backed fixtures and P-256 signatures. This evidence
does not claim a newly run daemon multi-epoch consensus, ordinary-peer handover
or release acceptance. The daemon still uses legacy first-epoch entry points.

`test-contract.md` and `test-review.md` record the independent critic's REVISE,
repairs and ACCEPT before production. `checks.json` records actual checks and
`validated-inputs.json` binds the full workspace run to exact source contents.
Raw logs remain in output/ar1-successor-spending and output/ar1-client-lineage.
`NEXT.md` identifies concrete required node/client/network integration.

AR1 and the full 67-card / 22-E2E / three-platform V1 remain open.
