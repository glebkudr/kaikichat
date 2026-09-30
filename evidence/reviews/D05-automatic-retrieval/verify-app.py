import copy,hashlib,json,subprocess,sys
from pathlib import Path
root=Path.cwd();out=root/'output/custody-sync';app=(root/'target/release/bundle/macos/Agentic Internet.app').resolve()
report=dict(passed=False,app=str(app),notarized=False,nativeUiRetested=True,checks=[])
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,name):
 r=subprocess.run(cmd,cwd=root,capture_output=True,text=True,timeout=60)
 (out/(name+'.log')).write_text(r.stdout+r.stderr)
 assert r.returncode==0,(name,r.stderr[-2000:])
 report['checks'].append(dict(name=name,exitCode=0,logSha256=sha(out/(name+'.log'))))
 return r.stdout
try:
 assert json.loads((out/'native-e2e/result.json').read_text())['passed']
 run(['codesign','--verify','--deep','--strict',str(app)],'app-codesign')
 run(['codesign','-dv','--verbose=4',str(app)],'app-signature')
 tree=run(['cargo','tree','--locked','-p','agentic-desktop','-e','normal','--prefix','none','-f','{p}'],'app-release-tree')
 assert not any(line.startswith('tauri-plugin-wdio-webdriver ') for line in tree.splitlines())
 report['driverExcluded']=True
 assert not any('test_state_failure' in str(p) for p in app.rglob('*'))
 report['testStateHelperExcluded']=True
 assert not any('connection_fan_in' in str(p) for p in app.rglob('*'))
 report['fanInHelperExcluded']=True
 binaries={name:app/'Contents/MacOS'/name for name in ['agentic-desktop','agentic-node','agentic-mcp','agentic-postage','agentic-postage-verifier']}
 report['binaries']={name:sha(path) for name,path in binaries.items()}
 sys.path.insert(0,str(root/'tests/evm'));import postage_shared_root as zk
 fixture=root/'crates/postage-spend/tests/fixtures';v=json.loads((fixture/'competition.json').read_text())
 image=zk.call(zk.ORACLE,['--postage-image-id']).decode().strip()
 assert image==v['imageId']=='ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de'
 report['imageId']=image;report['receiptCompatibility']=[]
 for case in v['cases']:
  receipt=(fixture/case['receipt']).read_bytes(); assert sha(fixture/case['receipt'])==case['sha256']
  assert bytes(json.loads(receipt)['journal']['bytes']).hex()==case['journal'][2:]
  assert zk.call(zk.ORACLE,[image],receipt).decode().strip()==case['journal'][2:]
  request=dict(context=case['context'],now=v['checkedAt'],receipt=receipt.decode())
  verified=json.loads(zk.call(binaries['agentic-postage-verifier'],['verify'],request))
  assert verified['nullifier']==case['nullifier'] and verified['context']==case['context'] and verified['admission'] is False
  wrong=copy.deepcopy(request);wrong['context']['operation']='0x'+'99'*32
  zk.call(binaries['agentic-postage-verifier'],['verify'],wrong,rejected=True)
  zk.call(binaries['agentic-postage-verifier'],['verify'],dict(request,now=case['context']['expiresAt']),rejected=True)
  report['receiptCompatibility'].append(dict(receipt=case['receipt'],sha256=case['sha256'],historicalVerification=True,operationSubstitutionRejected=True,expiryRejected=True))
 frozen=json.loads((out/'source-inputs.json').read_text())
 assert all(sha(root/p)==h for p,h in frozen.items())
 report['sourceInputsUnchanged']=True;report['passed']=True
finally:(out/'app-release.json').write_text(json.dumps(report,indent=2)+'\n')
print('Release signature, driver exclusion and both genuine receipt compatibility checks passed')
