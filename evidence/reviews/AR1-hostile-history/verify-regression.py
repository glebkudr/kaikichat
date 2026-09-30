from pathlib import Path
import concurrent.futures,hashlib,json,re,subprocess,time
root=Path.cwd();out=root/'output/ar1-hostile-regression'
paths=subprocess.run(['git','ls-files','-co','--exclude-standard','crates','apps','scripts','tests','Cargo.toml','Cargo.lock','.cargo'],check=True,text=True,capture_output=True).stdout.splitlines()
inputs={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sorted(set(paths)) if (root/p).is_file()}
(out/'source-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')
def run(name,args,cwd=root):
 start=time.monotonic()
 with (out/(name+'.log')).open('w') as log:
  result=subprocess.run(args,cwd=cwd,stdout=log,stderr=subprocess.STDOUT)
 raw=(out/(name+'.log')).read_text()
 checks=dict(command=args,cwd=str(cwd),wrapper='python3 scripts/build-storage.py run',exit_code=result.returncode,elapsed_seconds=round(time.monotonic()-start,2))
 if name=='node':
  suites=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw)]
  checks.update(passed=sum(s[0] for s in suites),failed=sum(s[1] for s in suites),ignored=sum(s[2] for s in suites))
 checks.update(fingerprinted_inputs=len(inputs),source_changes=[p for p,h in inputs.items() if hashlib.sha256((root/p).read_bytes()).hexdigest()!=h])
 (out/(name+'.json')).write_text(json.dumps(checks,indent=2)+'\n')
 print(name,json.dumps(checks),flush=True)
 assert result.returncode==0 and not checks['source_changes'],name+' failed'
def frontend():
 for name,args in [('frontend',['node','node_modules/vitest/vitest.mjs','run']),('typescript',['node','node_modules/typescript/bin/tsc','--noEmit']),('vite',['node','node_modules/vite/bin/vite.js','build'])]:
  run(name,args,root/'apps/desktop')
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
 ui=pool.submit(frontend)
 run('fmt',['cargo','fmt','--all','--','--check'])
 run('node',['cargo','test','--locked','-p','agentic-node','--all-targets'])
 run('clippy',['cargo','clippy','--locked','-p','agentic-node','--all-targets','--','-D','warnings'])
 ui.result()
# This exercises the pre-existing carrier's selected recovery mode, which must
# remain compatible when the new optional history behavior is disabled.
run('existing-recovery-carrier',['python3','tests/evm/public_spend_recovery.py'])
