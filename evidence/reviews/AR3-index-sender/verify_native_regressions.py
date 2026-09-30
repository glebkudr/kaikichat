"""Run affected existing native gates sequentially, after verify.py succeeds.

Use scripts/build-storage.py run from the repository root. No full workspace run.
--collect re-reads existing successful commands only when their original logs
are unchanged; every native source/binary and frozen input is checked again.
"""
from pathlib import Path
import hashlib
import importlib
import json
import subprocess
import sys
import time

ROOT = Path.cwd()
EVIDENCE = ROOT/'evidence/reviews/AR3-index-sender'
OUTPUT = ROOT/'output/ar3-index-sender'
inputs = json.loads((EVIDENCE/'source-inputs.json').read_text())
sys.path.insert(0, str(ROOT/'tests/evm'))


def changed_inputs():
    return [p for p,h in inputs.items() if not (ROOT/p).is_file()
        or hashlib.sha256((ROOT/p).read_bytes()).hexdigest() != h]


assert not changed_inputs(), 'Run current affected-cluster verification first'
for module_name in ('paid_index_network', 'public_epoch_ingress', 'public_epoch_handover'):
    command = ['python3', 'tests/evm/'+module_name+'.py']
    log = OUTPUT/('regression-'+module_name+'.log')
    saved_report = EVIDENCE/('regression-'+module_name+'.json')
    if '--collect' in sys.argv:
        previous = json.loads(saved_report.read_text())
        assert previous['command'] == command and previous['exit_code'] == 0
        assert previous['log_sha256'] == hashlib.sha256(log.read_bytes()).hexdigest()
        exit_code, elapsed = previous['exit_code'], previous['elapsed_seconds']
    else:
        started = time.monotonic()
        with log.open('w') as sink:
            result = subprocess.run(command, cwd=ROOT, stdout=sink, stderr=subprocess.STDOUT)
        exit_code, elapsed = result.returncode, round(time.monotonic()-started,2)
    module = importlib.import_module(module_name)
    native_output = module.OUT/('full' if module_name == 'public_epoch_handover' else '')
    report_path = native_output/'evidence.json'
    native = json.loads(report_path.read_text()) if report_path.is_file() else {}
    trace = native_output/'trace.json'
    trace_hash = hashlib.sha256(trace.read_bytes()).hexdigest() if trace.is_file() else None
    if '--collect' in sys.argv:
        # A later run must not attach new output to an earlier successful log.
        # Initial recovery from a collector path error is a separate documented
        # operation; repeated collection requires an existing complete binding.
        assert previous.get('native') and previous['native'] == native
        assert trace_hash and previous.get('trace_sha256') == trace_hash
    changed = changed_inputs()
    current = (native.get('sourceHash') == module.source_hash()
        and native.get('binarySha256') == hashlib.sha256((ROOT/'target/debug/agentic-node').read_bytes()).hexdigest())
    report = dict(passed=exit_code == 0 and native.get('passed') is True and current and not changed and bool(trace_hash),
        command=command, wrapper='python3 scripts/build-storage.py run', exit_code=exit_code,
        elapsed_seconds=elapsed, collected_existing_run='--collect' in sys.argv, native_current=current,
        source_changes=changed, fingerprinted_inputs=len(inputs), native=native,
        log_local=str(log.relative_to(ROOT)), log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
    if trace_hash: report['trace_sha256'] = trace_hash
    saved_report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='native'}), flush=True)
    if not report['passed']: raise SystemExit(1)
