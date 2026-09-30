from pathlib import Path
import hashlib
import json
import subprocess
import sys
import time

root = Path('/Users/glebk/Code/chat')
out = root / 'output/ar2-wallet-flow'
paths = set(json.loads((out / 'history-prefetch-r3-inputs.json').read_text()))
listed = subprocess.check_output(
    ['rg', '--files', 'crates', 'apps/desktop/src', 'apps/desktop/src-tauri',
     'apps/desktop/tests', 'scripts', 'spec', 'tests/evm'], cwd=root, text=True).splitlines()
paths.update(p for p in listed if Path(p).suffix in
             {'.rs', '.toml', '.json', '.py', '.ts', '.tsx', '.css', '.mjs', '.sh', '.md'})
paths.update({'output/ar2-wallet-flow/run-history-prefetch-probe-r9.py',
              'output/ar2-wallet-flow/collect-history-prefetch-probe-r9.py'})

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

before = {p: sha(root / p) for p in sorted(paths)}
(out / 'history-prefetch-probe-r9-inputs.json').write_text(json.dumps(before, indent=2) + '\n')
started = time.time()
bundle = root/'target/release/bundle/macos/Agentic Internet.app'
names = ['agentic-node','agentic-cli','agentic-mcp','agentic-postage-verifier','agentic-desktop']
binaries = {name:sha(bundle/'Contents/MacOS'/name) for name in names}
assert not (root/'output/hpp-r9/evidence.json').exists(), 'preserve prior attempts'
signature = subprocess.run(['codesign','--verify','--deep','--strict',str(bundle)],capture_output=True,text=True)
assert signature.returncode == 0, signature.stderr
result = {'passed': False, 'exitCode': None, 'fullV1': False, 'native130Passed': False,
          'bundle':str(bundle.resolve()), 'binaries':binaries, 'signatureVerified':True}
try:
    with (out / 'history-prefetch-probe-r9.log').open('w') as log:
        process = subprocess.run(
            [sys.executable, 'output/ar2-wallet-flow/run-history-prefetch-probe-r9.py'],
            cwd=root, stdout=log, stderr=subprocess.STDOUT)
    result['exitCode'] = process.returncode
finally:
    changed = [p for p, h in before.items() if not (root / p).exists() or sha(root / p) != h]
    raw = root / 'output/hpp-r9/evidence.json'
    report = json.loads(raw.read_text()) if raw.exists() else {}
    result.update(
        elapsedSeconds=round(time.time() - started, 1), inputCount=len(before),
        binariesUnchanged=binaries == {name:sha(bundle/'Contents/MacOS'/name) for name in names},
        changedInputs=changed, cleanupErrorsCount=len(report.get('cleanupErrors', [])),
        temporaryDirectoriesRemaining=len(list((root / 'output/hpp-r9').glob('ain-spend-*'))))
    snapshot_path = root / 'output/hpp-r9/recipient-stop-snapshot.json'
    diagnostic = {'captured': False}
    try:
        if snapshot_path.exists():
            snapshot = json.loads(snapshot_path.read_text())
            diagnostic.update(captured=snapshot.get('captured', False), sha256=sha(snapshot_path))
    except Exception as error:
        diagnostic['readError'] = type(error).__name__
    result['stoppedRecipientSnapshot'] = diagnostic
    result['passed'] = bool(
        result['exitCode'] == 0 and report.get('passed') and not changed
        and result['binariesUnchanged'] and report.get('verifiedSignatures') == 60
        and report.get('prefetchProbeOriginals') == 20 and report.get('native130Passed') is False
        and result['cleanupErrorsCount'] == 0 and result['temporaryDirectoriesRemaining'] == 0)
    (out / 'history-prefetch-probe-r9-run.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
sys.exit(0 if result['passed'] else 1)
