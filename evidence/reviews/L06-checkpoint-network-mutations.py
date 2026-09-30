"""Compiling isolated negative controls for the reviewed checkpoint network oracles."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'evidence/reviews'
NAMES=['core','store','crypto','protocol-types','capabilities','l2-types','l2-adapter','node']
scratch=Path(tempfile.mkdtemp(prefix='l06-network-mutations-',dir=ROOT.parent))
paths=['crates/node/src/checkpoint_network.rs','crates/node/src/checkpoint_schedule.rs']
originals={p:(ROOT/p).read_text() for p in paths}
schedule='runtime::checkpoint_schedule_tests::checkpoint_schedule_has_one_shared_slot_and_start_budget_survives_source_and_swarm_replacement'
race='checkpoints::network::checkpoint_network_held_response_cannot_overwrite_owner_head_and_replacement_releases_slot'
mutants=[
 ('shared_slot',paths[1],[('self.active.is_some() || now < self.next_start','now < self.next_start')],'unit',schedule),
 ('global_throttle',paths[1],[('self.active.is_some() || now < self.next_start','self.active.is_some()')],'unit',schedule),
 ('response_rebase',paths[0],[('            || anchor.checkpoint_id != pending.head\n',''),('.advance_checkpoint_chain(&wires, pending.head, time)','.advance_checkpoint_chain(&wires, anchor.checkpoint_id, time)')],'process',race),
 ('swarm_slot',paths[0],[('        self.pending = None;\n','')],'process',race),
 ('serving_admission',paths[0],[('let response = if !allowed {','let response = if false && !allowed {')],'process','checkpoints::network::checkpoint_network_public_serving_is_bounded_read_only_and_never_exports_private_state'),
 ('response_frame',paths[0],[('.set_response_size_maximum(132096)','.set_response_size_maximum(300000)')],'process','checkpoints::network::checkpoint_network_rejects_independent_bad_pages_without_resetting_anchor_and_recovers'),
]
report={'passed':False,'baselinePassed':False,'sourceHashes':{p:hashlib.sha256(s.encode()).hexdigest() for p,s in originals.items()},'mutants':[],'cleanupErrors':[]}
def run(label,target,test):
 args=['cargo','test','--offline','-p','agentic-node']+(['--lib'] if target=='unit' else ['--test','processes'])+[test]
 p=subprocess.run(args,cwd=scratch,env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')},text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=600)
 (OUT/f'L06-checkpoint-network-mutation-{label}.log').write_text(p.stdout)
 return p
try:
 for name in NAMES:shutil.copytree(ROOT/f'crates/{name}',scratch/f'crates/{name}',ignore=shutil.ignore_patterns('__pycache__'))
 manifest=(ROOT/'Cargo.toml').read_text()
 (scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]','members = '+json.dumps(['crates/'+n for n in NAMES]),manifest,count=1))
 shutil.copy2(ROOT/'Cargo.lock',scratch/'Cargo.lock')
 for target,test,count in [('unit','checkpoint_schedule',2),('process','checkpoint_network',6)]:
  p=run('baseline-'+target,target,test);assert p.returncode==0 and f'{count} passed; 0 failed' in p.stdout,p.stdout[-4000:]
 report['baselinePassed']=True;print('baseline:2 scheduler and6 actual-process scenarios passed',flush=True)
 for name,path,changes,target,test in mutants:
  changed=originals[path]
  for before,after in changes:assert changed.count(before)==1,(name,before,changed.count(before));changed=changed.replace(before,after,1)
  (scratch/path).write_text(changed);p=run(name,target,test)
  failed=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
  killed=p.returncode!=0 and test in failed and 'test result: FAILED' in p.stdout
  report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failed})
  (scratch/path).write_text(originals[path]);print(name+': '+('KILLED' if killed else 'INVALID OR SURVIVED'),flush=True);assert killed,p.stdout[-4000:]
 assert all((ROOT/p).read_text()==s for p,s in originals.items()),'production source changed during mutation gate'
 report['passed']=True
finally:
 try:shutil.rmtree(scratch)
 except OSError as error:report['cleanupErrors'].append(str(error))
 (OUT/'L06-checkpoint-network-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
