"""Isolated compiling negative controls for the reviewed custody business tests."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'evidence/reviews'
NAMES=['l2-adapter','l2-types','protocol-types']
SOURCE='crates/l2-adapter/src/custody.rs'
original=(ROOT/SOURCE).read_text()
report=dict(passed=False,baselinePassed=False,sourceHash=hashlib.sha256(original.encode()).hexdigest(),mutants=[],cleanupErrors=[])
result=OUT/'N05-funded-custody-mutations.json'
result.write_text(json.dumps(report,indent=2)+'\n')
mutants=[
 ('beacon_equality','first.block_number() >= snapshot.beacon_block()', 'first.block_number() > snapshot.beacon_block()', 'funding_in_the_beacon_block_or_after_it_is_not_precommitment_even_with_authentic_history_and_funding'),
 ('selected_position','if member.ordinal() != *selected {','if false && member.ordinal() != *selected {','every_selected_rank_requires_its_actual_gapped_merkle_member_and_opening'),
 ('successor_link','if !next.successor_of(&previous)? {','if false && !next.successor_of(&previous)? {','historical_signatures_do_not_allow_gaps_forks_wrong_current_head_or_cross_chain_roots'),
 ('sample_rank','selected.push(swaps.remove(&rank).unwrap_or(rank));','selected.push(rank);','actual_paid_precommitments_match_independent_full_list_selection_and_funded_resources_on_two_chains'),
]
scratch=Path(tempfile.mkdtemp(prefix='n05-custody-mutations-',dir=ROOT.parent))
def run(label,test=''):
    p=subprocess.run(['cargo','test','--offline','-p','agentic-l2-adapter','--test','funded_custody',test],cwd=scratch,
      env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')},text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=300)
    (OUT/f'N05-funded-custody-mutation-{label}.log').write_text(p.stdout)
    return p
try:
    for name in NAMES:shutil.copytree(ROOT/f'crates/{name}',scratch/f'crates/{name}',ignore=shutil.ignore_patterns('__pycache__'))
    manifest=(ROOT/'Cargo.toml').read_text()
    (scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]','members = '+json.dumps(['crates/'+n for n in NAMES]),manifest,count=1))
    shutil.copy2(ROOT/'Cargo.lock',scratch/'Cargo.lock')
    p=run('baseline');assert p.returncode==0 and '8 passed; 0 failed' in p.stdout,p.stdout[-4000:]
    report['baselinePassed']=True;print('baseline: 8 passed',flush=True)
    for name,before,after,test in mutants:
        assert original.count(before)==1,(name,original.count(before))
        (scratch/SOURCE).write_text(original.replace(before,after,1))
        p=run(name,test)
        failures=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
        killed=p.returncode!=0 and test in failures and 'test result: FAILED' in p.stdout
        report['mutants'].append(dict(name=name,exitCode=p.returncode,killed=killed,failingTests=failures))
        (scratch/SOURCE).write_text(original)
        print(name+': '+('KILLED' if killed else 'INVALID OR SURVIVED'),flush=True)
        assert killed,p.stdout[-4000:]
    assert (ROOT/SOURCE).read_text()==original,'authoritative source changed'
    report['passed']=True
finally:
    try:shutil.rmtree(scratch)
    except OSError as error:report['cleanupErrors'].append(str(error))
    result.write_text(json.dumps(report,indent=2)+'\n')
