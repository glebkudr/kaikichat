import json, subprocess, time
from pathlib import Path
report=Path('output/network-e2e/result.json')
run=json.loads(report.read_text())['runId']
read_code="""import importlib.util,json,pathlib,subprocess
s=importlib.util.spec_from_file_location('f','/usr/local/lib/ain-network-fixture.py');f=importlib.util.module_from_spec(s);s.loader.exec_module(f)
v={'node':f.rpc({'method':'node_info'})}
v['rules']=subprocess.run(['iptables-save','-c'],text=True,capture_output=True).stdout
v['log']=pathlib.Path('/tmp/ain-network-profile/daemon.log').read_text()[-8000:]
print(json.dumps(v))"""
with Path('evidence/reviews/L02-autonat-observation.jsonl').open('w') as out:
    for _ in range(330):
        status=json.loads(report.read_text())
        if status.get('cleanupErrors') is not None: break
        for role in ['autonat-public','autonat-service']:
            name=run+'-'+role
            p=subprocess.run(['docker','exec',name,'python3','-c',read_code],text=True,capture_output=True,timeout=12)
            if p.returncode==0:
                out.write(json.dumps({'time':time.time(),'name':name,'value':json.loads(p.stdout)})+'\n');out.flush()
        time.sleep(1)
print('Observation ended:', run)
