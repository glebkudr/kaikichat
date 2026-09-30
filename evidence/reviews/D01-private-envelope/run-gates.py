import hashlib,json,os,subprocess,time,shutil
from pathlib import Path
r=Path.cwd();o=r/'output/custody-envelope';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((o/'source-inputs.json').read_text())
report={'passed':False,'sourceManifestSha256':sha(o/'source-inputs.json'),'gates':[]}
commands=[
 ('models',['python3','-m','unittest','discover','-s','tests/models','-p','test_*.py']),
 ('evm-models',['python3','-m','unittest','discover','-s','tests/evm','-p','test_*.py']),
 ('fmt',['cargo','fmt','--all','--','--check']),
 ('clippy',['cargo','clippy','--locked','--workspace','--all-targets','--','-D','warnings']),
 ('backend',['cargo','test','--locked','--workspace','--all-targets']),
 ('frontend',['npm','--prefix','apps/desktop','test']),
 ('typecheck',['npm','--prefix','apps/desktop','run','typecheck']),
 ('native-build',['node','scripts/build-desktop.mjs','--debug','--e2e']),
 ('native-ui',['node','apps/desktop/tests/native-e2e.mjs']),
 ('app-build',['node','scripts/build-desktop.mjs']),
 ('app-check',['python3',str(o/'verify-app.py')])]
try:
 for name,cmd in commands:
  assert all(sha(r/p)==h for p,h in frozen.items())
  print('START',name,flush=True);start=time.monotonic();log=o/(name+'.log');env=dict(os.environ)
  if name=='native-ui':env['AIN_DESKTOP_BINARY']=str(r/'target/debug/bundle/macos/Agentic Internet.app/Contents/MacOS/agentic-desktop')
  with log.open('w') as f:done=subprocess.run(cmd,cwd=r,env=env,stdout=f,stderr=subprocess.STDOUT)
  if name=='native-ui':
   target=o/'native-e2e';target.mkdir(exist_ok=True)
   for source in (r/'output/native-e2e').iterdir():
    if source.is_file() and source.suffix in ['.json','.png'] and source.stat().st_mtime>=time.time()-(time.monotonic()-start)-2:shutil.copyfile(source,target/source.name)
  report['gates'].append(dict(name=name,exitCode=done.returncode,elapsedSeconds=time.monotonic()-start,logSha256=sha(log)))
  (o/'gates.json').write_text(json.dumps(report,indent=2)+'\n')
  assert done.returncode==0,name
  assert all(sha(r/p)==h for p,h in frozen.items())
  print('PASS',name,flush=True)
 report.update(passed=True,sourceInputsUnchanged=True)
finally:(o/'gates.json').write_text(json.dumps(report,indent=2)+'\n')
