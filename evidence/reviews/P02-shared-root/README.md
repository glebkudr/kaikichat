# P02 shared-root funded ownership

Two independently random owners paid for class-1 batches from distinct Anvil
accounts before the same beacon. Their actual EIP-1186 witnesses share one block
hash/state root. Both local recursive STARKs use the identical public context;
the default verifier and independent upstream oracle accepted them after the chain
had stopped. Verification and reverification used actual current time within the
funded window. The nullifiers are distinct and stable across retries.

`shared-root.json` retains the original run report. `receipt-0.json` and
`receipt-1.json` are byte-identical copies of its public receipt artifacts. The
unordered deposit cohort has no explicit receipt-to-deposit mapping. Seeds,
private openings and witness requests were not saved. This is relation/privacy
boundary evidence under the backend assumptions, not a formal anonymity theorem,
timing protection, current checkpoint authentication or spend admission.

All focused checks exited 0: 466 ordinary Rust and nine genuine proof/CLI tests,
7 models, 9 fixture/oracle checks, 40 frontend tests, formatting, default/proving
Clippy, TypeScript/Vite, and the existing 28-check funded-custody EVM regression.
The two new proofs took 356731 and 321274 ms and contain 584886 and 585071 JSON
bytes. The separate proof regression passed nine tests in 613.63 seconds.

`evidence.json` records the checks and external raw-log hashes. All 30 inputs in
`source-hashes.json` and the three binaries in `shared-root.json` stayed unchanged.
`accepted-tests.json` identifies the exact tests/infrastructure accepted by the
separate no-context critic before applying the build/oracle changes. The critic's
initial nonblocking suggestions—bounded live output collection, initial
`passed:false`, and distinct payer assertion—were applied and received FINAL ACCEPT.
This adds acceptance coverage for an existing generic backend; no missing-
production RED is claimed.

The final independent evidence audit also returned FINAL ACCEPT: 30 source hashes,
six accepted files, three binaries, five logs and both receipt copies matched;
journals, live timestamps, regression counts and stated limits were confirmed.

Reproduce through `python3 scripts/build-storage.py run python3
tests/evm/postage_shared_root.py`. The mandatory EVM gate includes it; the shared
`scripts/build-postage-cli.sh` also serves the unchanged mandatory nine-test proof
driver. Historical receipts can be audited, while reproducing the live-clock gate
requires fresh funded state. No new desktop bundle, native/Linux rerun or full
EVM aggregate is claimed. Core integration and complete V1 remain unfinished.
