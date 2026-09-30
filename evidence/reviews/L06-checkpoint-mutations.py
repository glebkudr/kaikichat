from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile
root=Path('/Users/glebk/Library/Caches/agentic-internet/worktree')
scratch=Path(tempfile.mkdtemp(prefix='l06-checkpoint-mutations-',dir='/Users/glebk/Library/Caches/agentic-internet'))
names=['l2-adapter','l2-types','protocol-types']
for name in names: shutil.copytree(root/f'crates/{name}',scratch/f'crates/{name}')
manifest=(root/'Cargo.toml').read_text()
(scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]', 'members = '+json.dumps(['crates/'+n for n in names]),manifest,count=1))
shutil.copy2(root/'Cargo.lock',scratch/'Cargo.lock')
source=scratch/'crates/l2-adapter/src/lib.rs'; original=source.read_text()
root_age='''            || statement
                .block_timestamp
                .checked_add(u64::from(profile.config.max_root_age_seconds))
                .is_none_or(|bound| expires_at > bound)'''
mutants=[
('quorum_formula','usize::from(config.threshold) <= 2 * n / 3','usize::from(config.threshold) <= 2'),
('configured_threshold','n < profile.threshold() as u64','n < 3'),
('distinct_signers','|| !authors.insert(author)','|| { authors.insert(author); false }'),
('root_age',root_age,''),
('issuer_code_binding',' || issuer.code_hash() != self.issuer_code_hash',''),
('issuer_domain_binding','issuer.domain() != self.issuer_domain || ',' '),
('successor_profile','''        if self.profile_id() != previous.profile_id() {
            return Err(CheckpointError::Successor);
        }
''',''),
('expiry_at_use','        self.check_time(now)?;',''),
]
env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')}
evidence=root/'evidence/reviews'
report={'sourceSha256':hashlib.sha256(original.encode()).hexdigest(),'scratch':str(scratch),'baselinePassed':False,'mutants':[],'passed':False}
def run(name):
    p=subprocess.run(['cargo','test','--offline','-p','agentic-l2-adapter','--test','checkpoints'],cwd=scratch,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
    (evidence/f'L06-checkpoint-mutation-{name}.log').write_text(p.stdout)
    return p
try:
    p=run('baseline')
    assert p.returncode==0 and '9 passed; 0 failed' in p.stdout,p.stdout[-4000:]
    report['baselinePassed']=True
    print('baseline: 9 passed',flush=True)
    for name,before,after in mutants:
        assert original.count(before)==1,(name,original.count(before))
        source.write_text(original.replace(before,after,1))
        p=run(name)
        failures=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
        killed=p.returncode!=0 and bool(failures) and 'test result: FAILED' in p.stdout
        report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failures})
        print(f'{name}: {"KILLED" if killed else "INVALID OR SURVIVED"}: {failures}',flush=True)
        assert killed,p.stdout[-4000:]
    report['passed']=True
finally:
    source.write_text(original)
    (evidence/'L06-checkpoint-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
