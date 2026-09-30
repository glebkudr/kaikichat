import hashlib,json,re,subprocess,time
from pathlib import Path
r=Path.cwd();o=r/'output/custody-sync';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((o/'source-inputs.json').read_text())
assert all(sha(r/p)==h for p,h in frozen.items())
start=time.monotonic()
with (o/'backend.log').open('w') as f:
 done=subprocess.run(['cargo','test','--locked','--workspace','--all-targets','--','--test-threads=4'],stdout=f,stderr=subprocess.STDOUT)
log=(o/'backend.log').read_text();counts=[tuple(map(int,m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)]
report=dict(exitCode=done.returncode,elapsedSeconds=time.monotonic()-start,logSha256=sha(o/'backend.log'),sourceManifestSha256=sha(o/'source-inputs.json'),testThreads=4,passed=sum(c[0] for c in counts),failed=sum(c[1] for c in counts),ignored=sum(c[2] for c in counts),nonemptySuites=sum(sum(c)>0 for c in counts))
(o/'backend-run.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
assert done.returncode==0 and report['failed']==0 and report['ignored']==0 and report['passed']>=678
assert all(sha(r/p)==h for p,h in frozen.items())
