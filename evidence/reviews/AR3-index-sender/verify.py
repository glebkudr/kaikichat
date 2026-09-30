"""Affected clusters; --native also reruns the real paid network scenario.

Invoke through scripts/build-storage.py run. Without --native, accepts only a
completed native report whose complete source hash and daemon binary still match.
--resume reuses successful identical commands only when all application/test
inputs and their original log hashes remain unchanged; empty tests cannot pass.
"""
from pathlib import Path
import concurrent.futures
import hashlib
import json
import re
import subprocess
import sys
import time

ROOT = Path.cwd()
EVIDENCE = ROOT/'evidence/reviews/AR3-index-sender'
OUTPUT = ROOT/'output/ar3-index-sender'
paths = subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','--',
    'crates','apps','scripts','tests','Cargo.toml','Cargo.lock','rust-toolchain.toml'],text=True).splitlines()
paths.extend(str(p.relative_to(ROOT)) for p in EVIDENCE.glob('*.py'))
inputs = {p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sorted(set(paths)) if (ROOT/p).is_file()}
prior={}
if '--resume' in sys.argv:
    previous=json.loads((EVIDENCE/'source-inputs.json').read_text())
    runner=str(Path(__file__).relative_to(ROOT))
    assert {p:h for p,h in previous.items() if p!=runner}=={p:h for p,h in inputs.items() if p!=runner}
    prior={c['name']:c for c in json.loads((EVIDENCE/'checks.json').read_text())['checks']}
(EVIDENCE/'source-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')


def run(name,command,cwd=ROOT):
    old=prior.get(name)
    if (old and old['command']==command and old['cwd']==str(cwd) and old['exit_code']==0
        and old.get('passed',1)>0 and hashlib.sha256((ROOT/old['log_local']).read_bytes()).hexdigest()==old['log_sha256']):
        result=dict(old,reused_unchanged_inputs=True)
        print(json.dumps(result),flush=True)
        return result
    started=time.monotonic();log=OUTPUT/(name+'.log')
    with log.open('w') as sink:
        result=subprocess.run(command,cwd=cwd,stdout=sink,stderr=subprocess.STDOUT)
    raw=log.read_text()
    counts=re.findall(r'test result: ok\. (\d+) passed;',raw)
    if name=='frontend-history': counts=re.findall(r'Tests\s+(\d+) passed',raw)
    result=dict(name=name,command=command,cwd=str(cwd),exit_code=result.returncode,
        elapsed_seconds=round(time.monotonic()-started,2),log_local=str(log.relative_to(ROOT)),
        log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
    if counts:result['passed']=sum(map(int,counts))
    print(json.dumps(result),flush=True)
    return result


if '--native' in sys.argv:
    native=run('native',['python3','tests/evm/public_index_sender.py'])
    assert native['exit_code']==0,native
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    frontend=pool.submit(run,'frontend-history',['node','node_modules/vitest/vitest.mjs','run','tests/chat-shell.test.tsx'],ROOT/'apps/desktop')
    checks=[run('paid-index',['cargo','test','--locked','-p','agentic-postage-spend','--test','paid_index']),
        run('outgoing-data',['cargo','test','--locked','-p','agentic-postage-spend','--test','paid_custody','outgoing_receipts']),
        run('public-custody',['cargo','test','--locked','-p','agentic-postage-spend','--test','public_spend','public_paid_ciphertext']),
        run('node-custody',['cargo','test','--locked','-p','agentic-node','--lib','paid_custody']),
        run('client-maintenance',['cargo','test','--locked','-p','agentic-node','--lib','maintenance_backoff_revokes_expired_client_permit']),
        run('core-sender',['cargo','test','--locked','-p','agentic-core','--test','public_postage_wallet','public_sender']),
        run('client-lineage',['cargo','test','--locked','-p','agentic-postage-spend','--test','client_lineage']),
        run('core-postage-client',['cargo','test','--locked','-p','agentic-core','--test','finalizer','postage_client']),
        run('clippy',['cargo','clippy','--locked','-p','agentic-core','-p','agentic-postage-spend','-p','agentic-node','--lib','--bins','--','-D','warnings']),
        run('fmt',['cargo','fmt','--all','--','--check']),frontend.result()]
sys.path.insert(0,str(ROOT/'tests/evm'))
import public_index_sender
native_path=ROOT/'output/index-sender/evidence.json'
native=json.loads(native_path.read_text())
native_current=(native.get('passed') is True and native['sourceHash']==public_index_sender.source_hash()
    and native['binarySha256']==hashlib.sha256((ROOT/'target/debug/agentic-node').read_bytes()).hexdigest())
changed=[p for p,h in inputs.items() if not (ROOT/p).is_file() or hashlib.sha256((ROOT/p).read_bytes()).hexdigest()!=h]
passed=all(c['exit_code']==0 for c in checks) and not changed and native_current
passed=passed and all(c.get('passed',0)>0 for c in checks if c['name'] in ['paid-index','outgoing-data','public-custody','node-custody','client-maintenance','core-sender','client-lineage','core-postage-client','frontend-history'])
report=dict(passed=passed,scope='Affected backend/frontend clusters and ordinary native sender index/location stages; no full workspace or automatic index discovery',
    wrapper='python3 scripts/build-storage.py run',base_commit=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
    fingerprinted_inputs=len(inputs),source_changes=changed,native_current=native_current,checks=checks)
(EVIDENCE/'checks.json').write_text(json.dumps(report,indent=2)+'\n')
(EVIDENCE/'native.json').write_text(json.dumps(native,indent=2)+'\n')
print(json.dumps(report),flush=True)
raise SystemExit(0 if passed else 1)
