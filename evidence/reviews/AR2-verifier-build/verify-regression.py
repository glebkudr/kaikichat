from pathlib import Path
import subprocess, time, re, json, hashlib
root=Path.cwd()
out=root/'output/ar2-verifier-regression'
paths=subprocess.run(['git','ls-files','-co','--exclude-standard','crates','apps','scripts','tests','Cargo.toml','Cargo.lock','.cargo'],check=True,text=True,capture_output=True).stdout.splitlines()
inputs={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sorted(set(paths)) if (root/p).is_file()}
(out/'source-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')
backend=__import__('sys').argv[1:] == ['--backend']
checks=json.loads((out/'checks.json').read_text()) if backend else {}
def run(name,args,cwd=root):
    started=time.monotonic()
    with (out/(name+'.log')).open('w') as log:
        result=subprocess.run(args,cwd=cwd,stdout=log,stderr=subprocess.STDOUT)
    raw=(out/(name+'.log')).read_text()
    item=dict(command=args,cwd=str(cwd),wrapper='python3 scripts/build-storage.py run',exit_code=result.returncode,elapsed_seconds=round(time.monotonic()-started,2))
    if name in ('workspace','legacy'):
        suites=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw)]
        item.update(passed=sum(s[0] for s in suites),failed=sum(s[1] for s in suites),ignored=sum(s[2] for s in suites),nonempty_suites=sum(s[0]>0 for s in suites))
    if name in ('models','evm_models'):
        matches=re.findall(r'Ran (\d+) tests?',raw)
        item['tests']=int(matches[-1]) if matches else None
    changes=[p for p,h in inputs.items() if not (root/p).is_file() or hashlib.sha256((root/p).read_bytes()).hexdigest()!=h]
    item.update(source_changes=changes,fingerprinted_inputs=len(inputs))
    checks[name]=item
    (out/'checks.json').write_text(json.dumps(checks,indent=2)+'\n')
    print(name,json.dumps(item),flush=True)
    assert result.returncode==0 and not changes, name+' failed'
for name,args in ([] if backend else [
    ('models',['python3','-m','unittest','discover','-s','tests/models','-p','test_*.py']),
    ('evm_models',['python3','-m','unittest','discover','-s','tests/evm','-p','test_*.py']),
    ('fmt',['cargo','fmt','--all','--','--check']),
]): run(name,args)
for name,args in ([] if backend else [
    ('frontend',['node','node_modules/vitest/vitest.mjs','run']),
    ('typescript',['node','node_modules/typescript/bin/tsc','--noEmit']),
    ('vite',['node','node_modules/vite/bin/vite.js','build']),
]): run(name,args,root/'apps/desktop')
# Cargo operations remain sequential with packaging, controlled by the caller.
if backend:
    run('legacy',['bash','scripts/check-postage.sh'])
    run('workspace',['cargo','test','--workspace','--all-targets'])
    run('clippy',['cargo','clippy','--workspace','--all-targets','--','-D','warnings'])
