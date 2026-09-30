import json,hashlib,subprocess,time,os,shutil
from pathlib import Path
root=Path.cwd();out=root/'output/v1-readiness-20260909';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((out/'source-inputs.json').read_text());report=dict(passed=False,sourceManifestSha256=sha(out/'source-inputs.json'),gates=[])
commands=[('native-build',['node','scripts/build-desktop.mjs','--debug','--e2e']),('native-ui',['node','apps/desktop/tests/native-e2e.mjs']),('app-build',['node','scripts/build-desktop.mjs']),('app-check',['python3',str(out/'verify-app.py')])]
try:
 assert json.loads((out/'regressions.json').read_text())['passed']
 for name,command in commands:
  assert all(sha(root/p)==h for p,h in frozen.items())
  print('START',name,flush=True);start=time.monotonic();log=out/(name+'.log');env=dict(os.environ)
  if name=='native-ui':env['AIN_DESKTOP_BINARY']=str(root/'target/debug/bundle/macos/Agentic Internet.app/Contents/MacOS/agentic-desktop')
  with log.open('w') as f:result=subprocess.run(command,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
  if name=='native-ui':
   target=out/'native-e2e';target.mkdir(exist_ok=True)
   for source in (root/'output/native-e2e').iterdir():
    if source.is_file() and source.suffix in ['.json','.png'] and source.stat().st_mtime>=time.time()-(time.monotonic()-start)-2:shutil.copyfile(source,target/source.name)
  report['gates'].append(dict(name=name,exitCode=result.returncode,elapsedSeconds=time.monotonic()-start,logSha256=sha(log)))
  (out/'native-run.json').write_text(json.dumps(report,indent=2)+'\n')
  assert result.returncode==0,name
  assert all(sha(root/p)==h for p,h in frozen.items())
  print('PASS',name,flush=True)
 report.update(passed=True,sourceInputsUnchanged=True)
finally:(out/'native-run.json').write_text(json.dumps(report,indent=2)+'\n')
