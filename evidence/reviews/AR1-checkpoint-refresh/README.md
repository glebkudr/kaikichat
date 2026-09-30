# AR1: full candidate queue across checkpoint renewal

Baseline f4693a8. This change preserves the 16-slot active queue while accepting
fresh current proof envelopes for already authenticated pending receipts. It
does not change spend/QC formats, committee selection, history namespaces or the
epoch !=1 guard. [Contract](test-contract.md), [independent critic](critic.md),
[specification](../../../spec/postage/checkpoint-refresh-v1.md), [next work](NEXT.md).

Incoming current evidence is verified first. An old envelope may be replaced
only when its receipt verifies to the same canonical journal under that current
context. Matching operation fields are insufficient; distinct valid tickets can
fund the same operation. Replacement preserves queue position, checks final size
and capacity, and commits before the new cached/network envelope becomes visible.
An exact retry needs no SQL write. Immutable original spent records are checked
before candidate mutation and never replaced by this path.

The second fix preserves cold reads of records that predate the refreshed
checkpoint's issue time. Historical record verification checks the authenticated
checkpoint upper expiry and observer time, then verifies against the immutable
committee's original registry window. Its live verifier remains unchanged. The
new Rust test rejects times outside those bounds; the native gate supplies the
actual chronological earlier spend and finishes with the reader's operator disabled.

The native gate uses actual EVM purchases/state proofs, trusted test-attestor
successor certificates and four ordinary generated validator keys. It finalizes
one spend, then fills one isolated validator with 16 pending spends. A genuine
successor checkpoint revokes old authority without changing the committee/log.
The gate requires repeated SQL failure with unchanged persisted bytes/revision,
crash recovery, 16 successful replacements in a still-full queue, another cold
restart, and valid same-operation/different-ticket refusal. No new owner receipts
are sent to the other three replicas: after actual old-lease expiry they obtain
the refreshed envelopes through ordinary peer transport. All records must remain
one canonical chain with consumer cursor 18, original QC unchanged and final solo
cold reads complete. Stale context and damaged signature never replace inputs.

Both pre-production runs reached the intended capacity failure after one genuine
spend and checkpoint renewal; `red-r1.json` records the critic-accepted version.
The first implementation run passed SQL/replacement/cold-input checks, then hit
an immediate service-ready assertion after IPC startup. Its separate evidence is
under `first-implementation/`. The accepted test refinement waits at most 15
seconds for asynchronous startup, then retains the continuous-running lease check.
It did not require another production change.

`envelope-green/` retains the first full 18-spend/216-signature result for queue
replacement. Its successor was captured before the original spend, so it did not
cover records predating checkpoint issue time. The later accepted extension adds
that requirement and a role-disabled cold reader. `history-red.json` records the
new Rust test's genuine pre-fix `Finalizer(Time)` failure. Final acceptance must
use the combined scenario, not the earlier envelope-only result.

**833 Rust tests / 0 failed / 0 ignored / 55 nonempty suites**, 59 frontend tests,
TypeScript/Vite, workspace Clippy/fmt and 19 model tests pass. `checks.json` records
546 input hashes and final command/log results. No source changed during the final
workspace run. Only the accepted legacy Rust test alignment followed the final
native gate; production/Python stayed identical. Its full native result is **18
spends / 216 signature checks**, repeated offline across 72 retained observations
with all 18 canonical parent links checked.
Runtime profiles, actual operator secrets, build products and logs stay outside Git.
This gate is explicit renewal at a selected validator. It does not prove automatic
client authority acquisition, epoch handover, expired-authority synchronization,
independent operators, packaged desktop behavior or three-platform parity. Earlier
144-spend lifetime and two-spend recovery gates remain separate historical evidence.

Recheck retained public records after the daemons and EVM have stopped:

```sh
python3 scripts/build-storage.py run python3 - <<'PY'
import gzip, json, sys
from pathlib import Path
sys.path.insert(0, 'tests/evm')
import public_spend_refresh as gate
from public_sender_daemon import authenticate_finality
t = json.loads(gzip.decompress(Path('evidence/reviews/AR1-checkpoint-refresh/live-trace.json.gz').read_bytes()))
assert gate.base.ps.reference(t['selection'], 4) == t['expected']
data = dict(config=t['committee'], ident=gate.base.raw(t['scope']['committeeId']))
cases = [dict(c, journal=gate.base.raw(c['journal'])) for c in t['candidateInputs']]
records = [r for batch in t['recordBatches'] for r in batch]
assert len(records) == 72
assert sum(authenticate_finality(r, data, cases)[0] for r in records) == 216
assert max(r['record']['verified_at'] for r in t['originalRecords'].values()) < t['successor']['head']['issuedAt']
assert all(r['record']['verified_at'] >= t['head']['expiresAt'] for r in records if r['operation'] != cases[0]['operation'])
assert t['coldRecords'][0] in t['originalRecords'].values()
assert all(r in records for r in t['coldRecords'])
print('216 retained signature checks passed')
PY
```

`offline-check.json` also records the executed full parent-chain check. These
offline checks use the retained committee and record metadata; they do not newly
authenticate checkpoint/EVM proofs or prove a cryptographic signing wall clock.
