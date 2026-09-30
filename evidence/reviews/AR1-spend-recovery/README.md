# AR1: missed original spend evidence

Implemented and verified after baseline a9fbad3. This does not close AR1 or V1.
See the [test contract](test-contract.md), [independent critic](critic.md),
[specification](../../../spec/postage/spend-recovery-v1.md) and [continuation](NEXT.md).

The native local-EVM gate used four ordinary daemon validator keys and two public
funded spends. One validator was offline before the first receipt was supplied
to the other three; it never received that owner input, including in setup.
After sources finalized and retired their inputs, its genuine archive delivery
required original evidence recovery. The gate demonstrates:

- A real recovery SQL insertion failure, no record and consumer cursor zero.
- Role revocation with that delivery held, followed by nine seconds without
  writes, ACK, running service or new recovery traffic while sources stayed live.
- Solo cold replay, authentic compact selected binding, and rejection of a
  changed QC signature received without candidate input from a selected test peer.
- Exact original record recovery from one ordinary source, without new quorum
  or owner resubmission, durable ACK and another isolated cold restart.
- A second spend in the same issuer log, applied by all four daemons.

The final native gate exits zero with a single copied daemon binary pinned by
hash across all restarts and no cleanup errors. It checks 24 signatures across
eight record observations. The compressed retained trace repeats those checks
offline against its retained committee after all test processes stop. Independent
selection reference also matches; checkpoint/EVM authentication was performed in
the live gate and is not freshly repeated offline.

Full regression: 832 Rust tests / 0 failed / 0 ignored / 55 nonempty suites;
59 frontend tests, TypeScript/Vite, workspace Clippy/fmt and 19 model tests pass.
Three new Rust tests are included in the workspace total. `checks.json` records
509 code/test input hashes, final commands/results and runtime-log hashes. Only
two test files changed after the workspace started; the final native execution
and Clippy cover the reviewed carrier refinements. Production stayed unchanged.

`red.json` distinguishes the initial harness assertion from genuine baseline
failure: three source records, nine verified signatures, zero lagger inputs and
30 failed archive effects. The first implementation and diagnostic runs are
retained in separate directories. They passed SQL failure/revocation but lacked
test-peer discovery; `compact-binding-run/` reached signature rejection and then
hit a redundant full-proof-RPC assertion. The final test requires the actual
authenticated peer/key/committee route, including the supported compact path.
These earlier runs do not count as full acceptance.

`live-evidence.json`, `live-trace.json.gz`, its hash manifest and
`offline-check.json` are public test evidence. Private keys, profiles, runtime
logs and build artifacts remain outside Git. This one-host test does not prove
independent operators, ciphertext custody, packaged native UI or platform parity.
The prior 144-spend lifetime gate is separate historical evidence and was not
freshly rerun. Cross-epoch handover and expired-authority network recovery remain
required; historical import itself grants no current spend authority.

Recheck retained record signatures without running a daemon or EVM:

```sh
python3 scripts/build-storage.py run python3 - <<'PY'
import gzip, json, sys
from pathlib import Path
sys.path.insert(0, 'tests/evm')
import public_spend_recovery as gate
t = json.loads(gzip.decompress(Path('evidence/reviews/AR1-spend-recovery/live-trace.json.gz').read_bytes()))
assert gate.base.ps.reference(t['selection'], 4) == t['expected']
data = dict(config=t['committee'], ident=gate.base.raw(t['scope']['committeeId']))
cases = [dict(c, journal=gate.base.raw(c['journal'])) for c in t['candidateInputs']]
records = list(t['originalRecords'].values()) + [t['recoveredRecord']] + list(t['secondRecords'].values())
assert sum(gate.authenticate_finality(r, data, cases)[0] for r in records) == 24
assert t['recoveredRecord'] in t['originalRecords'].values()
print('24 retained signature checks passed')
PY
```
