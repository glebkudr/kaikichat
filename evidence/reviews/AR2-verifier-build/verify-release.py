"""Additional actual default release verification; no application UI is opened."""
from pathlib import Path
import hashlib,json,os,subprocess,sys,tempfile,time
root=Path.cwd()
sys.path.insert(0,str(root/'tests/build'))
import desktop_verifier as gate
out=root/'output/ar2-verifier-regression'
report=dict(passed=False,commands=[])
start=time.monotonic()
try:
 env=dict(os.environ,RISC0_SKIP_BUILD='1',RISC0_SKIP_BUILD_KERNELS='1')
 with (out/'release-build.log').open('w') as log:
  r=subprocess.run(['node','scripts/build-desktop.mjs'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
 report['commands'].append(dict(command=['node','scripts/build-desktop.mjs'],exit_code=r.returncode))
 assert r.returncode==0
 app=root/'target/release/bundle/macos/Agentic Internet.app'
 with (out/'release-signature.log').open('w') as log:
  r=subprocess.run(['codesign','--verify','--deep','--strict',str(app)],stdout=log,stderr=subprocess.STDOUT)
 report['commands'].append(dict(command=['codesign','--verify','--deep','--strict',str(app)],exit_code=r.returncode))
 assert r.returncode==0
 binaries=app/'Contents/MacOS'
 assert not (binaries/'agentic-postage').exists()
 verifier=binaries/'agentic-postage-verifier'
 gate.proving_unavailable(verifier)
 # The debug package's full CLI + native availability gate already passed;
 # require byte-identical verifier in the default release bundle too.
 debug=root/'target/debug/bundle/macos/Agentic Internet.app/Contents/MacOS/agentic-postage-verifier'
 assert verifier.read_bytes()==debug.read_bytes()
 evidence=json.loads(gate.RECEIPTS[2].read_text())
 for name,path in zip(('realProof','commonProof'),gate.RECEIPTS[:2]):
  expected=evidence[name]; context=expected['context']
  got=gate.verify(verifier,dict(context=context,now=context['notBefore'],receipt=path.read_text()),True)
  assert got['context']==context and got['nullifier']==expected['nullifier'] and got['admission'] is False
 with tempfile.TemporaryDirectory(prefix='release-verifier-',dir=root/'.local/verification-workspaces') as path:
  node=gate.Node(Path(path)/'owner',binaries/'agentic-node')
  try:
   for _ in range(2):
    node.start(); info=node.call('node_info',{})
    assert info['postageProofs']['available'] is False and info['postageVerifications']['available'] is True
    node.stop()
  finally:node.stop()
 inputs=json.loads((out/'source-inputs.json').read_text())
 changes=[p for p,h in inputs.items() if hashlib.sha256((root/p).read_bytes()).hexdigest()!=h]
 assert not changes
 report.update(passed=True,proverAbsent=True,provingCommandsUnavailable=True,
  originalPaidReceiptsVerified=True,verifierIdenticalToAcceptedDebugBundle=True,
  availabilityAccurateAcrossRestart=True,source_changes=changes,fingerprinted_inputs=len(inputs),
  binary_sha256={n:hashlib.sha256((binaries/n).read_bytes()).hexdigest() for n in ['agentic-node','agentic-mcp','agentic-postage-verifier','agentic-desktop']})
finally:
 report['elapsed_seconds']=round(time.monotonic()-start,2)
 (out/'release-check.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report),flush=True)
