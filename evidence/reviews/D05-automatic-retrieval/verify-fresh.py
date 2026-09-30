import hashlib,json,sys
from pathlib import Path
r=Path.cwd();o=r/'output/custody-sync';source=r/'output/paid-ciphertext-delivery';sha=lambda b:hashlib.sha256(b).hexdigest()
sys.path.insert(0,str(r/'tests/evm'))
import postage_shared_root as zk
from postage_spend_node import decode
report=json.loads((source/'evidence.json').read_text());trace=json.loads((source/'trace.json').read_text())
assert report['passed'] and report['cleanupErrors']==[]
receipt=(source/'fresh-receipt.json').read_bytes()
assert sha(receipt)==report['retainedReceiptSha256']==report['proofs'][0]['sha256']
image=report['imageId'];assert image=='ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de'
journal=bytes(json.loads(receipt)['journal']['bytes'])
assert zk.call(zk.ORACLE,[image],receipt).decode().strip()==journal.hex()
assert journal==bytes(trace['record']['record']['journal'])
f=decode(journal);assert len(f)==14 and f[0]=='ain-private-postage-v1'
assert f[10].hex()==trace['publicEnvelope']['operation']==sha(bytes.fromhex(trace['publicEnvelope']['wire']))
assert '0x'+f[10].hex()==report['proofs'][0]['operation'] and '0x'+f[11].hex()==report['proofs'][0]['nullifier']
context=dict(zip(['issuerDomain','issuerCodeHash','registryDomain','registryEpoch','registryRoot','fundingStateRoot','network','notBefore','expiresAt','operation'],['0x'+v.hex() if isinstance(v,bytes) else v for v in f[1:11]]))
checked=trace['record']['record']['verified_at'];assert context['notBefore']<=checked<context['expiresAt']
verifier=(r/'target/release/bundle/macos/Agentic Internet.app/Contents/MacOS/agentic-postage-verifier').resolve()
request=dict(context=context,now=checked,receipt=receipt.decode())
v=json.loads(zk.call(verifier,['verify'],request));assert v['nullifier']==report['proofs'][0]['nullifier'] and v['context']==context and v['admission'] is False
wrong=dict(request,context=dict(context,operation='0x'+'99'*32))
zk.call(verifier,['verify'],wrong,rejected=True)
zk.call(verifier,['verify'],dict(request,now=context['expiresAt']),rejected=True)
result=dict(passed=True,receiptSha256=sha(receipt),imageId=image,operation=report['proofs'][0]['operation'],nullifier=report['proofs'][0]['nullifier'],journalSha256=sha(journal),envelopeSha256=trace['publicEnvelope']['operation'],envelopeBytes=len(bytes.fromhex(trace['publicEnvelope']['wire'])),oracleSha256=sha(zk.ORACLE.read_bytes()),packagedVerifierSha256=sha(verifier.read_bytes()),historicalVerificationAt=checked,operationSubstitutionRejected=True,expiryRejected=True,privateWitnessRetained=False)
(o/'fresh-receipt-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
