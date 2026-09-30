"""Isolated compiling mutation checks for catch-up, expiry and atomic archival guarantees."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'evidence/reviews'
NAMES = ['core', 'store', 'crypto', 'protocol-types', 'capabilities', 'l2-types', 'l2-adapter']
scratch = Path(tempfile.mkdtemp(prefix='l06-history-mutations-', dir=ROOT.parent))
paths = ['crates/core/src/checkpoint_history.rs', 'crates/core/src/checkpoints.rs', 'crates/l2-adapter/src/lib.rs']
originals = {path: (ROOT / path).read_text() for path in paths}
report = {'passed': False, 'baselinePassed': False, 'mutants': [], 'cleanupErrors': [],
          'sourceHashes': {p: hashlib.sha256(s.encode()).hexdigest() for p,s in originals.items()}}
# Each mutant must compile and fail the particular reviewed business test, not a compile error.
mutants = [
 ('future_issue_hint', paths[2], 'now.min(issue_hint)', 'issue_hint', 'l2', 'historical_authentication_preserves_expiry_and_cannot_turn_history_into_current_funding'),
 ('funding_expiry', paths[2], '        self.check_time(now)?;\n        validate_issuer(', '        validate_issuer(', 'l2', 'historical_authentication_preserves_expiry_and_cannot_turn_history_into_current_funding'),
 ('skip_successor', paths[0], '&& !head.successor_of(prior)?', '&& false && !head.successor_of(prior)?', 'core', 'checkpoint_state::checkpoint_catchup_denies_partial_bad_pages_stale_anchors_future_and_clock_rollback'),
 ('stale_anchor', paths[0], 'if expected_head != loaded.head.as_ref().map(|head| head.id().0)', 'if false && expected_head != loaded.head.as_ref().map(|head| head.id().0)', 'core', 'checkpoint_state::checkpoint_catchup_denies_partial_bad_pages_stale_anchors_future_and_clock_rollback'),
 ('denial_clock', paths[0], 'self.save_checkpoint(loaded, now)?;\n                Err(error)', 'let _ = loaded;\n                Err(error)', 'core', 'checkpoint_state::checkpoint_catchup_denies_partial_bad_pages_stale_anchors_future_and_clock_rollback'),
 ('manual_archive_atomicity', paths[1], 'states.push(archive.change()?);', 'let _ = archive.change()?;', 'core', 'checkpoint_state::checkpoint_history_and_head_commit_atomically_on_archive_insert_and_update_failure'),
]

def run(label, package, test):
    crate, target = ('agentic-core', 'conversations') if package == 'core' else ('agentic-l2-adapter', 'checkpoints')
    p = subprocess.run(['cargo','test','--offline','-p',crate,'--test',target,test], cwd=scratch,
        env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')}, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=300)
    (OUT/f'L06-checkpoint-history-mutation-{label}.log').write_text(p.stdout)
    return p

try:
    for name in NAMES:
        shutil.copytree(ROOT/f'crates/{name}',scratch/f'crates/{name}',ignore=shutil.ignore_patterns('__pycache__'))
    manifest = (ROOT/'Cargo.toml').read_text()
    (scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]', 'members = '+json.dumps(['crates/'+n for n in NAMES]), manifest, count=1))
    shutil.copy2(ROOT/'Cargo.lock',scratch/'Cargo.lock')
    for package,test,count in [('core','checkpoint_state::',14),('l2','historical_authentication',1)]:
        p=run('baseline-'+package,package,test)
        assert p.returncode == 0 and f'{count} passed; 0 failed' in p.stdout,p.stdout[-4000:]
    report['baselinePassed']=True
    print('baseline: 14 Core and 1 historical L2 test passed',flush=True)
    for name,path,before,after,package,test in mutants:
        assert originals[path].count(before)==1,(name,originals[path].count(before))
        (scratch/path).write_text(originals[path].replace(before,after,1))
        p=run(name,package,test)
        failures=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
        killed=p.returncode!=0 and test in failures and 'test result: FAILED' in p.stdout
        report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failures})
        (scratch/path).write_text(originals[path])
        print(name+': '+('KILLED' if killed else 'INVALID OR SURVIVED'),flush=True)
        assert killed,p.stdout[-4000:]
    assert all((ROOT/p).read_text()==s for p,s in originals.items()),'working source changed during mutation gate'
    report['passed']=True
finally:
    try:shutil.rmtree(scratch)
    except OSError as error:report['cleanupErrors'].append(str(error))
    (OUT/'L06-checkpoint-history-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
