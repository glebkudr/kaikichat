# AR1: indexed runtime and terminal spend lifecycle

This is progress toward AR1, not V1 or cross-epoch acceptance. Baseline `9dece5a`.
The independent test critic accepted tests before production, with separately
reviewed cold-pending and compatibility refinements. See `critic.md`,
`test-contract.md`, `red.json` and `cold-pending.json`.

## What changed

- A checked complete SQL history prefix supplies an opaque operation-bound
  capability. Service/engine validation must reach its exact committee/height/
  digest anchor from the actual authenticated Simplex parent through the complete
  bounded suffix. An empty DB cannot omit history. Current Core receipt/fence
  policy, voting WAL, quorum and epoch protection remain intact.
- Actual indexed runtime progress may exceed absolute height 128; the suffix,
  archive-export distance and proof bytes/entry counts remain bounded. Generic
  complete-prefix applications keep their existing limit.
- A spend record commits before its derived history and candidate retirement;
  all three stages precede consumer ACK. Exact replay uses the original record,
  retains its original QC, and completes any outstanding durable stages.
  Both valid alternatives for a spent ticket retire; the active limit stays 16.
- Public historical authority evidence is retained for cold QC lookup after
  actual expiry. Historical reads cannot renew admission. Unresolved inputs
  with unavailable current authority return unavailable, never false absence.

## Evidence

The complete native gate buys 160 public tickets on an actual local Anvil EVM,
selects four ordinary daemons with self-generated validator keys through real
registry/checkpoint proofs, and requires 144 finalized spends in one issuer log.
It independently verifies the QC signatures, canonical chain and journal binding.
The funding source is offline during consensus. It tests no quorum after 128,
real SQL spend-insert failure at 129, cleanup failure after durable spend at 130,
solo crash recovery of both known deliveries, remote candidate distribution and
same-ticket competition after 136, cold exact-QC preservation and real 600-second
lease expiry. This is one host, not independent operators or a public testnet.

`initial-run/` preserves the first successful lifetime run before the later
cold-pending refinement and addition of complete public verifier inputs. Its
live oracle verified 2163 signatures, but its retained trace lacked the committee
needed for later independent QC re-verification. It is not the final revision's
acceptance. `initial-workspace.json` likewise retains the first 829-test workspace
run before the final small reader fix; final checks supersede it.

The later `unpinned-run/` adds complete public verifier inputs and passes offline
QC checks, but a concurrent workspace build changed the shared target executable.
A single executable across all restarts was therefore not established. The final
opt-in harness pins a copied daemon in managed temporary storage and checks its
hash before and after the whole scenario; it supersedes both earlier runs.

The final pinned run passes: 144 spends, 2,163 live QC signature checks, original
QC preserved after actual expiry, and no cleanup errors. After the command exited
and all test processes stopped, the compressed retained trace independently
passed all 432 record signatures and the complete 144-entry chain. Final Rust
regression is 829 tests; frontend is 59 tests. `checks.json` records source and
binary fingerprints, commands, log hashes and the accepted test-only harness
delta after the workspace started.

Final trace is compressed JSON, with decompressed and compressed hashes in
`live-trace-manifest.json`. It contains public test funding/selection/committee
inputs, actual records/QCs, the canonical chain, stage timings and SQL failure
counter/cursor observations. No daemon private keys or encrypted profiles are
included. Runtime logs are in managed `output/`, not Git.

To independently recheck the retained committee derivation, QC signatures and
chain after all daemons and the EVM have stopped:

```sh
python3 scripts/build-storage.py run python3 - <<'PYVERIFY'
import gzip, json, sys
from pathlib import Path
sys.path.insert(0, 'tests/evm')
from public_spend_lifetime import verify_saved_trace
trace = json.loads(gzip.decompress(Path('evidence/reviews/AR1-runtime-lifetime/live-trace.json.gz').read_bytes()))
print(verify_saved_trace(trace))
PYVERIFY
```

This offline helper does not independently re-authenticate the checkpoint/EVM
state proofs; the public inputs are retained and were authenticated in the live
funded gate. Final results and commands are in `checks.json`; do not add counts
from distinct revisions or targeted subsets to the workspace total.

## Remaining work

See `NEXT.md`. Epoch seal/handover and authority refresh, authenticated recovery
of missed receipt evidence, expiry/terminal reconciliation of unresolved work,
sender ready scheduling and successful sender retirement remain required. This
gate does not demonstrate ciphertext custody, R10 repair, message history,
packaged native UI, platform parity, or the full 67-card/22-E2E V1 release.
