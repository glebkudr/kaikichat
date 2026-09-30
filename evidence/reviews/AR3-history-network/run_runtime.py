"""Fresh server transport run with source/binary/log binding; no reuse mode."""
from pathlib import Path
import hashlib
import importlib
import json
import subprocess
import sys
import time
ROOT=Path.cwd()
label=sys.argv[1]
assert label and all(c.isalnum() or c=='-' for c in label)
ev=ROOT/'evidence/reviews/AR3-history-network'
out=ROOT/'output/ar3-history-network'
out.mkdir(parents=True,exist_ok=True)
sys.path.insert(0,str(ROOT/'tests/evm'))
module=importlib.import_module('paid_history_network')
def inputs():
    paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','--','crates','apps','scripts','tests','Cargo.toml','Cargo.lock','rust-toolchain.toml'],text=True).splitlines()
    paths.append(str(Path(__file__).relative_to(ROOT)))
    return {p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sorted(set(paths)) if (ROOT/p).is_file()}
before=inputs()
assert not (ev/f'{label}.json').exists(), 'use a fresh report label'
(ev/f'{label}-inputs.json').write_text(json.dumps(before,indent=2)+'\n')
for name in ['evidence.json','trace.json']:
    p=module.OUT/name
    if p.is_file():
        (out/f'{label}-prior-{name}').write_bytes(p.read_bytes());p.unlink()
command=['python3','tests/evm/paid_history_network.py'];log=out/f'{label}.log';started=time.monotonic()
with log.open('w') as sink: run=subprocess.run(command,stdout=sink,stderr=subprocess.STDOUT)
p=module.OUT/'evidence.json';native=json.loads(p.read_text()) if p.is_file() else {}
trace=module.OUT/'trace.json';copy=out/f'{label}-trace.json'
if trace.is_file():copy.write_bytes(trace.read_bytes())
after=inputs();changes=[p for p in set(before)|set(after) if before.get(p)!=after.get(p)]
current=native.get('sourceHash')==module.source_hash() and native.get('binarySha256')==hashlib.sha256((ROOT/'target/debug/agentic-node').read_bytes()).hexdigest()
report=dict(passed=run.returncode==0 and native.get('passed') is True and current and not changes and trace.is_file(),command=command,wrapper='python3 scripts/build-storage.py run',exit_code=run.returncode,elapsed_seconds=round(time.monotonic()-started,2),native_current=current,source_changes=changes,fingerprinted_inputs=len(before),native=native,log_local=str(log.relative_to(ROOT)),log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),trace_local=str(copy.relative_to(ROOT)),trace_sha256=hashlib.sha256(copy.read_bytes()).hexdigest() if copy.is_file() else None)
(ev/f'{label}.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True)
raise SystemExit(0 if report['passed'] else 1)
