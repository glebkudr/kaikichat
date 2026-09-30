# AR1 — hostile native history ingress (native and targeted regression passed)

The large native cumulative handover and its full regression were accepted in
`a1ebaf9`. The default verifier/build separation and current full regression were
accepted in `d95671b`. This follow-up adds adversarial network coverage of the
existing handover implementation; production daemon code is unchanged so far.

The independent backend test critic accepted the actual test and raw peer-carrier
artifacts before this run. The accepted drafts were copied byte-for-byte to
`tests/evm/public_epoch_ingress.py` and
`crates/node/examples/finalizer_replay_peer.rs`. See `TEST_CONTRACT.md` and
`test-review.json` for the review and original draft hashes.

A small genuinely funded scenario creates three spends and two real native
closings. A new third-epoch member receives authentic-but-wrong-target closing,
altered closing QC and a mixed original-history page through an actual ordinary
Noise peer. Repeated same queries, completed ResponseSent events, exact SQL
snapshots and absence of a premature signer check rejection. Exact original
answers must then resume all history after crash and preserve original records,
QCs and verification times across another cold restart.

This is coverage of already implemented behavior, not a claim of a production
RED. The actual native gate passed on its first run: three spends, two closings,
69 independently checked signatures and 750 owner calls. All three hostile
phases preserve state and deny an early signer, then exact original answers
resume after crash and preserve the original records through cold reads.
`native-check.json`, `native-trace.json.gz` and `trace-integrity.json` retain the
public evidence. Targeted regression passes 195 node tests, 59 frontend tests, node Clippy,
fmt and TypeScript/Vite. The previous selected-recovery carrier mode also passes
(two spends, 24 checked signatures and 617 owner calls). All 570 fingerprints
remain unchanged; see `checks.json`. The full workspace result on production
source d95671b remains separately attributed to AR2. Automatic renewal/outage closure,
remaining sender lifecycle, AR2 user entry points, R10/groups/recovery and the
67-card / 22-E2E / three-platform V1 remain open.


To repeat targeted regression from the repository root, first create
`output/ar1-hostile-regression`, then run
`python3 scripts/build-storage.py run python3 evidence/reviews/AR1-hostile-history/verify-regression.py`.
The copied script is the actual validation driver, including the previous
carrier mode. Run the hostile gate separately with
`python3 scripts/build-storage.py run python3 tests/evm/public_epoch_ingress.py`.
