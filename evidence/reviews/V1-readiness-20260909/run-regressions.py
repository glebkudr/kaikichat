import hashlib,json,subprocess,time
from pathlib import Path
r=Path.cwd();o=r/'output/v1-readiness-20260909';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((o/'source-inputs.json').read_text());report={'passed':False,'sourceManifestSha256':sha(o/'source-inputs.json'),'gates':[]}
commands=[('model-oracles',['python3','-m','unittest','discover','-s','tests/models','-p','test_*.py']),('evm-oracles',['python3','-m','unittest','discover','-s','tests/evm','-p','test_*.py']),('fmt',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','--workspace','--all-targets','--','-D','warnings']),('backend',['cargo','test','--workspace','--all-targets']),('frontend',['npm','--prefix','apps/desktop','test'])]
try:
 for name,cmd in commands:
  assert all(sha(r/p)==h for p,h in frozen.items())
  print('START',name,flush=True);start=time.monotonic();log=o/(name+'.log')
  with log.open('w') as f:done=subprocess.run(cmd,cwd=r,stdout=f,stderr=subprocess.STDOUT)
  report['gates'].append(dict(name=name,exitCode=done.returncode,elapsedSeconds=time.monotonic()-start,logSha256=sha(log)))
  (o/'regressions.json').write_text(json.dumps(report,indent=2)+'\n')
  assert done.returncode==0,name
  assert all(sha(r/p)==h for p,h in frozen.items())
  print('PASS',name,flush=True)
 report.update(passed=True,sourceInputsUnchanged=True)
finally:(o/'regressions.json').write_text(json.dumps(report,indent=2)+'\n')
