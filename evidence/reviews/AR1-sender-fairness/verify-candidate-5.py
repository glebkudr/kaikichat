from pathlib import Path
import concurrent.futures, hashlib, json, re, subprocess, time
root=Path.cwd();out=root/'output/ar1-sender-fairness'
out.mkdir(parents=True,exist_ok=True)
ev=root/'evidence/reviews/AR1-sender-fairness'
inputs=json.loads((ev/'candidate-5-inputs.json').read_text())
def changed():
    return [p for p,h in inputs.items() if not (root/p).is_file() or hashlib.sha256((root/p).read_bytes()).hexdigest()!=h]
assert not changed(),'source changed before regression'
def run(name,args,cwd=root):
    started=time.monotonic()
    with (out/(name+'.log')).open('w') as log:
        result=subprocess.run(args,cwd=cwd,stdout=log,stderr=subprocess.STDOUT)
    raw=(out/(name+'.log')).read_text()
    item=dict(command=args,cwd=str(cwd),wrapper='python3 scripts/build-storage.py run',exit_code=result.returncode,elapsed_seconds=round(time.monotonic()-started,2))
    if name=='node-targeted':
        suites=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw)]
        item.update(passed=sum(s[0] for s in suites),failed=sum(s[1] for s in suites),ignored=sum(s[2] for s in suites),nonempty_suites=sum(s[0]>0 for s in suites))
    if name in ('models','evm_models'):
        matches=re.findall(r'Ran (\d+) tests?',raw)
        item['tests']=int(matches[-1]) if matches else None
    item.update(source_changes=changed(),fingerprinted_inputs=len(inputs))
    (out/(name+'.json')).write_text(json.dumps(item,indent=2)+'\n')
    print(name,json.dumps(item),flush=True)
    assert result.returncode==0 and not item['source_changes'],name+' failed'
# Timed native gates ran alone on the same frozen candidate. Refuse stale evidence.
# Workspace test feature unification can rebuild the output executable. Compare
# the exact standalone native binary recorded by all four completed native gates.
daemon_hash=json.loads((ev/'candidate-5-sender-integrity.json').read_text())['daemon_sha256']
for name in ['sender','fairness','custody','hostile-history']:
    report=json.loads((ev/('candidate-5-'+name+'-check.json')).read_text())
    integrity=json.loads((ev/('candidate-5-'+name+'-integrity.json')).read_text())
    assert report['passed'] and integrity['source_changes']==[], name+' native gate missing'
    assert integrity['fingerprinted_inputs']==len(inputs)
    assert integrity['daemon_sha256']==daemon_hash, name+' daemon mismatch'
assert json.loads((ev/'candidate-5-inputs.json').read_text())==inputs
# Core wallet/sender and frontend targets already completed before the user
# limited intermediate regression to affected clusters. Their reports are retained.
run('node-targeted',['cargo','test','--locked','-p','agentic-node','--lib'])
run('node-clippy',['cargo','clippy','--locked','-p','agentic-node','--all-targets','--','-D','warnings'])
