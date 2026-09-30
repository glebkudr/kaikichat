"""Reusable isolated Rust mutation runner; source edits never touch the running workspace."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile

def verify(root,prefix,crates,package,target,expected_count,mutants):
    root=Path(root);out=root/'evidence/reviews'
    paths={m['path'] for m in mutants}
    originals={p:(root/p).read_text() for p in paths}
    report={'passed':False,'baselinePassed':False,'sourceHashes':{p:hashlib.sha256(s.encode()).hexdigest() for p,s in originals.items()},'mutants':[],'cleanupErrors':[]}
    scratch=Path(tempfile.mkdtemp(prefix=prefix+'-',dir=root.parent))
    def run(label,test=''):
        args=['cargo','test','--offline','-p',package,'--test',target]+([test] if test else [])
        p=subprocess.run(args,cwd=scratch,env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')},text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=600)
        (out/f'{prefix}-mutation-{label}.log').write_text(p.stdout);return p
    try:
        for name in crates:shutil.copytree(root/f'crates/{name}',scratch/f'crates/{name}',ignore=shutil.ignore_patterns('__pycache__'))
        manifest=(root/'Cargo.toml').read_text()
        (scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]','members = '+json.dumps(['crates/'+n for n in crates]),manifest,count=1))
        shutil.copy2(root/'Cargo.lock',scratch/'Cargo.lock')
        p=run('baseline')
        assert p.returncode==0 and f'{expected_count} passed; 0 failed' in p.stdout,p.stdout[-4000:]
        report['baselinePassed']=True;print('baseline passed',flush=True)
        for m in mutants:
            changed=originals[m['path']]
            for before,after in m['changes']:
                assert changed.count(before)==1,(m['name'],before,changed.count(before))
                changed=changed.replace(before,after,1)
            (scratch/m['path']).write_text(changed)
            p=run(m['name'],m['test'])
            failed=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
            killed=p.returncode!=0 and m['test'] in failed and 'test result: FAILED' in p.stdout
            report['mutants'].append({'name':m['name'],'exitCode':p.returncode,'killed':killed,'failingTests':failed})
            (scratch/m['path']).write_text(originals[m['path']])
            print(m['name']+': '+('KILLED' if killed else 'SURVIVED OR INVALID'),flush=True)
        assert all((root/p).read_text()==s for p,s in originals.items()),'source changed during mutation gate'
        report['passed']=all(m['killed'] for m in report['mutants'])
        assert report['passed'],'negative control survived or failed to compile'
    finally:
        try:shutil.rmtree(scratch)
        except OSError as error:report['cleanupErrors'].append(str(error));report['passed']=False
        (out/f'{prefix}-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
