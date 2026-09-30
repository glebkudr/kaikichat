import hashlib,json,os,subprocess,sys,tempfile,time
from pathlib import Path
root=Path('/Users/glebk/Code/chat'); sys.path.insert(0,str(root/'tests/evm'))
from checkpoint_node import Node
out=root/'output/ar2-wallet-flow/skill-host-smoke'; out.mkdir(exist_ok=True)
bundle=root/'target/debug/bundle/macos/Agentic Internet.app/Contents/MacOS'
expected='Checking the conversation via the shipped CLI skill.'
reply='Test correspondent reply.\n'+'receipt-line-0123456789abcdef\n'*160
report=dict(passed=False,expectedText=expected,replyBytes=len(reply.encode()),wholeE11=False,externalNetwork=False)
def until(fn,seconds=30):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        value=fn()
        if value:return value
        time.sleep(.1)
    raise TimeoutError('fixture setup')
with tempfile.TemporaryDirectory(prefix='skill-host-',dir='/Volumes/ChatBuild/output') as temp:
    d=Path(temp); alice=Node(d/'alice',bundle/'agentic-node'); bob=Node(d/'bob',bundle/'agentic-node')
    try:
        alice.start(); bob.start()
        alice.call('create_identity',dict(name='Skill host owner')); bob.call('create_identity',dict(name='Test correspondent'))
        invitation=bob.call('create_invitation',{})
        contact=alice.call('add_contact',dict(name='Test correspondent',invitation=invitation))
        peer=until(lambda: bob.call('snapshot',{})['conversations'])
        group=contact['id']; other=peer[0]['id']
        setup=alice.call('provision_runtime',dict(operationId='skill-host-access',name='Independent skill host',agentId=[211]*32,serviceId=[212]*32,conversationIds=[group],actions=['read_inbox','send_message'],expiresAt=int(time.time())+1200,maxDataBytes=8192))
        target=bob.call('snapshot',{})['identity']['networkId']
        context=dict(cliConfig=setup['cliConfig'],recipientNetworkId=target,skillPath=str(bundle.parent/'Resources/skills/agentic-messaging/SKILL.md'))
        path=out/'connection-context.json'; path.write_text(json.dumps(context,indent=2)+'\n'); path.chmod(0o600)
        print(json.dumps(dict(ready=True,context=str(path))),flush=True)
        report['skillSha256']=hashlib.sha256(Path(context['skillPath']).read_bytes()).hexdigest()
        report['binaries']={n:hashlib.sha256((bundle/n).read_bytes()).hexdigest() for n in ['agentic-node','agentic-cli']}
        report['recipientNetworkId']=target
        sent=None; end=time.monotonic()+1200
        while time.monotonic()<end and not (out/'finish').exists():
            messages=bob.call('snapshot',{})['conversations'][0]['messages']
            received=[m for m in messages if not m['own']]
            if received:
                assert len(received)==1 and received[0]['text']==expected
                if sent is None:
                    sent=bob.call('send_message',dict(conversationId=other,text=reply,operationId='skill-peer-reply'))
                    report.update(receivedMessageId=received[0]['id'],replyMessageId=sent['id'])
                    (out/'observer.json').write_text(json.dumps(report,indent=2)+'\n')
            time.sleep(.25)
        assert (out/'finish').exists() and sent is not None
        values=alice.call('snapshot',{})['conversations'][0]['messages']; assert len(values)==2
        assert values[0]['text']==expected and values[1]['text']==reply
        assert values[0]['delivery']['phase']=='delivered'
        config=setup['cliConfig']; result=subprocess.run([config['command'],*config['args'],'inbox','poll','--from',target,'--operation-id','observer-after-agent','--max-bytes','8192'],capture_output=True,timeout=15)
        assert result.returncode==0 and result.stderr==b''
        page=json.loads(result.stdout)['result']; assert page['items']==[] and page['leaseId'] is None
        report.update(passed=True,actualAgentHost=True,originalMessageCount=2,recipientAck=True,replyAcknowledged=True,originalsPreserved=True)
    finally:
        bob.stop(); alice.stop()
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps({k:report.get(k) for k in ['passed','actualAgentHost','replyBytes']}),flush=True)
