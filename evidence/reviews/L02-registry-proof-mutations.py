"""Compiling isolated negative controls for registry proof, lifetime and context boundaries."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'evidence/reviews'
scratch=Path(tempfile.mkdtemp(prefix='l02-proof-mutations-',dir=ROOT.parent))
paths=['crates/l2-types/src/lib.rs','crates/l2-types/src/registry.rs','crates/l2-adapter/src/lib.rs']
originals={p:(ROOT/p).read_text() for p in paths}
member='operator_opening_owner_order_and_full_subtree_counts_are_not_peer_assertions'
life='snapshot_and_cached_membership_expire_at_the_earlier_checkpoint_or_admission_bound'
context='valid_quorum_and_valid_mpt_cannot_cross_the_selected_issuer_chain_or_genesis'
mutants=[
 ('ordinal_all_siblings',paths[1],[('                ordinal += sibling.count;\n',''),('            if (proof.index >> level) & 1 == 0 {','            ordinal += sibling.count;\n            if (proof.index >> level) & 1 == 0 {')],'agentic-l2','registry_proof','real_anvil_snapshot_and_member_authenticate_the_whole_gapped_active_set'),
 ('member_root',paths[1],[('current.hash != self.root || current.count != self.count','current.count != self.count')],'agentic-l2','registry_proof',member),
 ('cached_expiry',paths[1],[('checked_at < self.checked_at || checked_at >= self.valid_until','checked_at < self.checked_at')],'agentic-l2','registry_proof',life),
 ('checkpoint_lease',paths[1],[('admission_until.min(anchor.lease_expires_at)','admission_until')],'agentic-l2','registry_proof',life),
 ('chain_binding',paths[2],[('registry.chain_id() != issuer.chain_id() || registry.genesis() != issuer.genesis()','registry.genesis() != issuer.genesis()')],'agentic-l2-adapter','registry',context),
 ('genesis_binding',paths[2],[('registry.chain_id() != issuer.chain_id() || registry.genesis() != issuer.genesis()','registry.chain_id() != issuer.chain_id()')],'agentic-l2-adapter','registry',context),
 ('code_hash',paths[0],[('if proof.code_hash != code_hash {','if false && proof.code_hash != code_hash {')],'agentic-l2','registry_proof','selected_chain_deployment_and_all_configuration_fields_bind_the_snapshot'),
 ('storage_mpt',paths[0],[('.map_err(|_| FundingError::Storage)?;','.map_err(|_| FundingError::Storage);')],'agentic-l2','registry_proof','account_fields_storage_values_and_every_mpt_path_are_verified'),
 ('total_count',paths[1],[(' || current.count != self.count','')],'agentic-l2','registry_proof',''),
]
report={'passed':False,'baselinePassed':False,'sourceHashes':{p:hashlib.sha256(s.encode()).hexdigest() for p,s in originals.items()},'mutants':[],'cleanupErrors':[]}
def run(label,package,target,test=''):
 args=['cargo','test','--offline','-p',package,'--test',target]+([test] if test else [])
 p=subprocess.run(args,cwd=scratch,env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')},text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=600)
 (OUT/f'L02-registry-proof-mutation-{label}.log').write_text(p.stdout);return p
try:
 names=['l2-types','l2-adapter','protocol-types']
 for name in names:shutil.copytree(ROOT/f'crates/{name}',scratch/f'crates/{name}',ignore=shutil.ignore_patterns('__pycache__'))
 manifest=(ROOT/'Cargo.toml').read_text();(scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]','members = '+json.dumps(['crates/'+n for n in names]),manifest,count=1))
 shutil.copy2(ROOT/'Cargo.lock',scratch/'Cargo.lock')
 for label,package,target,count in [('proof','agentic-l2','registry_proof',8),('adapter','agentic-l2-adapter','registry',4)]:
  p=run('baseline-'+label,package,target);assert p.returncode==0 and f'{count} passed; 0 failed' in p.stdout,p.stdout[-4000:]
 report['baselinePassed']=True;print('baseline:8 proof and4 adapter/CLI tests passed',flush=True)
 for name,path,changes,package,target,test in mutants:
  changed=originals[path]
  for before,after in changes:assert changed.count(before)==1,(name,before,changed.count(before));changed=changed.replace(before,after,1)
  (scratch/path).write_text(changed);p=run(name,package,target,test)
  failed=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
  killed=p.returncode!=0 and bool(failed) and (not test or test in failed) and 'test result: FAILED' in p.stdout
  report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failed})
  (scratch/path).write_text(originals[path]);print(name+': '+('KILLED' if killed else 'SURVIVED OR INVALID'),flush=True)
 assert all((ROOT/p).read_text()==s for p,s in originals.items()),'source changed during mutation gate'
 report['passed']=all(m['killed'] for m in report['mutants'])
 assert report['passed'],'one or more negative controls survived or were invalid'
finally:
 try:shutil.rmtree(scratch)
 except OSError as error:report['cleanupErrors'].append(str(error))
 (OUT/'L02-registry-proof-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
