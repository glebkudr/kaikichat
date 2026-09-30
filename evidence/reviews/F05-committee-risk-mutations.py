"""Compile and kill isolated negative controls; never mutate authoritative source."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[2]
prefix = 'F05-committee-risk'
original = (root/'tools/risk-simulator/risk.py').read_text()
report = {'passed': False, 'baselinePassed': False, 'sourceHash': hashlib.sha256(original.encode()).hexdigest(), 'mutants': [], 'cleanupErrors': []}
mutants = [
    ('quorum_equality', 'safe = 2*quorum > count+faulty', 'safe = 2*quorum >= count+faulty',
     'test_quorum_counterexamples_match_exhaustive_sets_and_never_double_vote_an_honest_member'),
    ('intersection_off_by_one', 'distribution[max(0, 2*quorum-size):]', 'distribution[max(0, 2*quorum-size)+1:]',
     'test_complete_distribution_matches_independent_subset_enumeration'),
    ('unknown_grinding_assumed_one', 'bound = None if candidates is None else rational(min(Fraction(1), candidates * intersection))',
     'bound = rational(min(Fraction(1), (candidates or 1) * intersection))',
     'test_grinding_uses_a_union_bound_without_claiming_independent_trials'),
    ('sqrt_key_counts', "adversarial = units(value['adversarialKeys'])", "adversarial = sum(math.isqrt(v) for v in value['adversarialKeys'])",
     'test_fixed_capital_splitting_changes_only_the_rejected_sqrt_model'),
]
scratch = Path(tempfile.mkdtemp(prefix='ain-risk-mutations-', dir=root.parent))
try:
    for name in ['tools/risk-simulator/risk.py', 'tests/models/test_committee_risk.py']:
        target = scratch/name; target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(root/name, target)
    def run(label, test=None):
        args = [sys.executable, '-m', 'unittest', 'discover', '-s', 'tests/models', '-p', 'test_committee_risk.py']
        if test: args += ['-k', test]
        p = subprocess.run(args, cwd=scratch, capture_output=True, text=True, timeout=30)
        (root/f'evidence/reviews/{prefix}-mutation-{label}.log').write_text(p.stdout+p.stderr)
        return p
    baseline = run('baseline')
    assert baseline.returncode == 0 and 'Ran 7 tests' in baseline.stderr and '\nOK\n' in baseline.stderr
    report['baselinePassed'] = True
    for label, before, after, test in mutants:
        assert original.count(before) == 1
        mutated = original.replace(before, after, 1)
        compile(mutated, label, 'exec')
        (scratch/'tools/risk-simulator/risk.py').write_text(mutated)
        p = run(label, test)
        killed = p.returncode != 0 and ('FAIL: '+test) in p.stderr and 'AssertionError' in p.stderr and 'FAILED (failures=' in p.stderr
        report['mutants'].append({'name': label, 'compiled': True, 'exitCode': p.returncode, 'killed': killed, 'test': test})
        (scratch/'tools/risk-simulator/risk.py').write_text(original)
    assert (root/'tools/risk-simulator/risk.py').read_text() == original
    report['passed'] = all(m['killed'] for m in report['mutants'])
    assert report['passed'], report
finally:
    try: shutil.rmtree(scratch)
    except OSError as e: report['cleanupErrors'].append(str(e)); report['passed'] = False
    (root/f'evidence/reviews/{prefix}-mutations.json').write_text(json.dumps(report, indent=2)+'\n')
print('Four isolated compiled risk mutations killed after green baseline')
