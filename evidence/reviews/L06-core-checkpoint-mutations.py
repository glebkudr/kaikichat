"""Isolated behavioral mutation experiments; never mutate the application's working source."""
from pathlib import Path
import hashlib, json, os, re, shutil, subprocess, tempfile
root = Path(__file__).resolve().parents[2]
scratch = Path(tempfile.mkdtemp(prefix='l06-core-checkpoint-mutations-', dir=root.parent))
names = ['core', 'store', 'crypto', 'protocol-types', 'capabilities', 'l2-types', 'l2-adapter']
for name in names:
    shutil.copytree(root / f'crates/{name}', scratch / f'crates/{name}')
manifest = (root / 'Cargo.toml').read_text()
(scratch / 'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]', 'members = ' + json.dumps(['crates/' + n for n in names]), manifest, count=1))
shutil.copy2(root / 'Cargo.lock', scratch / 'Cargo.lock')
source = scratch / 'crates/core/src/checkpoints.rs'
original = source.read_text()
mutants = [
    ('accept_denial_clock', '        let loaded = self.save_checkpoint(loaded, now)?;\n        result?;', '        result?;\n        let loaded = self.save_checkpoint(loaded, now)?;'),
    ('funding_denial_clock', '        self.save_checkpoint(loaded, now)?;\n        result', '        let value = result.map_err(|error: CoreError| error)?;\n        self.save_checkpoint(loaded, now)?;\n        Ok(value)'),
    ('clock_fence', 'if now < self.stored.observed_at {\n            return Err(CoreError::CheckpointClockRollback);', 'if false {\n            return Err(CoreError::CheckpointClockRollback);'),
    ('stale_cas', '(replacement && expected_revision != loaded.revision)', 'false'),
    ('successor', 'Some(previous) => head.successor_of(previous)?,', 'Some(previous) => head.id() != previous.id(),'),
]
env = {**os.environ, 'CARGO_TARGET_DIR': str(scratch / 'target')}
evidence = root / 'evidence/reviews'
report = {'sourceSha256': hashlib.sha256(original.encode()).hexdigest(), 'scratch': str(scratch), 'baselinePassed': False, 'mutants': [], 'passed': False, 'cleanupErrors': []}
def run(name):
    p = subprocess.run(['cargo', 'test', '--offline', '-p', 'agentic-core', '--test', 'conversations', 'checkpoint_state::'], cwd=scratch, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=300)
    (evidence / f'L06-core-checkpoint-mutation-{name}.log').write_text(p.stdout)
    return p
try:
    p = run('baseline')
    assert p.returncode == 0 and '6 passed; 0 failed' in p.stdout, p.stdout[-4000:]
    report['baselinePassed'] = True
    print('baseline: 6 passed', flush=True)
    for name, before, after in mutants:
        assert original.count(before) == 1, (name, original.count(before))
        source.write_text(original.replace(before, after, 1))
        p = run(name)
        failures = re.findall(r'^test ([^\n]+) \.\.\. FAILED$', p.stdout, re.M)
        killed = p.returncode != 0 and bool(failures) and 'test result: FAILED' in p.stdout
        report['mutants'].append({'name': name, 'exitCode': p.returncode, 'killed': killed, 'failingTests': failures})
        print(f'{name}: {"KILLED" if killed else "INVALID OR SURVIVED"}: {failures}', flush=True)
        assert killed, p.stdout[-4000:]
    assert (root / 'crates/core/src/checkpoints.rs').read_text() == original
    report['passed'] = True
finally:
    try:
        shutil.rmtree(scratch)
    except OSError as error:
        report['cleanupErrors'].append(str(error))
    (evidence / 'L06-core-checkpoint-mutations.json').write_text(json.dumps(report, indent=2) + '\n')
