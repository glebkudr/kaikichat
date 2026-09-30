"""Fresh native gate with exact source/binary binding, including expected RED.

Invoke through scripts/build-storage.py run; report label is mandatory. There is
no reuse/collect mode. Failed runs retain evidence without becoming acceptance.
"""
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
module_name=sys.argv[2] if len(sys.argv)>2 else 'public_index_recipient'
assert module_name in {'public_index_recipient','public_index_sender','public_sender_daemon','paid_index_network','public_paid_ciphertext'}
evidence=ROOT/'evidence/reviews/AR3-index-recipient'
output=ROOT/'output/ar3-index-recipient'
sys.path.insert(0,str(ROOT/'tests/evm'))
module=importlib.import_module(module_name)
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','--',
    'crates','apps','scripts','tests','Cargo.toml','Cargo.lock','rust-toolchain.toml'],text=True).splitlines()
paths.append(str(Path(__file__).relative_to(ROOT)))
inputs={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sorted(set(paths)) if (ROOT/p).is_file()}
input_file=evidence/f'{label}-inputs.json'
assert not input_file.exists(), 'use a new report label; do not replace prior evidence'
input_file.write_text(json.dumps(inputs,indent=2)+'\n')
command=['python3',f'tests/evm/{module_name}.py']
log=output/f'{label}.log'
# Remove ambiguity with an older run, while retaining its diagnostic bytes.
for name in ['evidence.json','trace.json']:
    previous=module.OUT/name
    if previous.is_file():
        (output/f'{label}-prior-{name}').write_bytes(previous.read_bytes())
        previous.unlink()
started=time.monotonic()
with log.open('w') as sink:
    completed=subprocess.run(command,cwd=ROOT,stdout=sink,stderr=subprocess.STDOUT)
native_path=module.OUT/'evidence.json'
native=json.loads(native_path.read_text()) if native_path.is_file() else {}
trace=module.OUT/'trace.json'
trace_copy=output/f'{label}-trace.json'
if trace.is_file():trace_copy.write_bytes(trace.read_bytes())
changed=[p for p,h in inputs.items() if not (ROOT/p).is_file() or hashlib.sha256((ROOT/p).read_bytes()).hexdigest()!=h]
current=(native.get('sourceHash')==module.source_hash() and native.get('binarySha256')==hashlib.sha256((ROOT/'target/debug/agentic-node').read_bytes()).hexdigest())
report=dict(passed=completed.returncode==0 and native.get('passed') is True and current and not changed and trace.is_file(),
    command=command,wrapper='python3 scripts/build-storage.py run',exit_code=completed.returncode,
    elapsed_seconds=round(time.monotonic()-started,2),native_current=current,source_changes=changed,
    fingerprinted_inputs=len(inputs),native=native,log_local=str(log.relative_to(ROOT)),
    log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),trace_local=str(trace_copy.relative_to(ROOT)),
    trace_sha256=hashlib.sha256(trace_copy.read_bytes()).hexdigest() if trace_copy.is_file() else None,
    fresh_native_report=native_path.is_file(),fresh_trace=trace.is_file())
(evidence/f'{label}.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
raise SystemExit(0 if report['passed'] else 1)
