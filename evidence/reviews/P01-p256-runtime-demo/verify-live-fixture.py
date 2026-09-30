"""Retain independently checked live TCP fixture evidence; never production credentials."""
from pathlib import Path
import ast,hashlib,json,socket,subprocess,tempfile,time
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import encode_dss_signature
from cryptography.hazmat.primitives import hashes,serialization
from cryptography.exceptions import InvalidSignature
root=Path(__file__).resolve().parents[3]; out=Path(__file__).resolve().parent;out.mkdir(exist_ok=True)
ref=root/'crates/crypto/tests/fixtures/mailbox/generate.py'
fn=next(n for n in ast.parse(ref.read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='cbor');scope={};exec(compile(ast.Module(body=[fn],type_ignores=[]),str(ref),'exec'),scope);cbor=scope['cbor']
sha=lambda b:hashlib.sha256(b).digest()
seeds={ec.derive_private_key(i,ec.SECP256R1()).public_key().public_bytes(serialization.Encoding.X962,serialization.PublicFormat.CompressedPoint):i.to_bytes(32,'big') for i in range(2001,2005)};keys=sorted(seeds)
count=12; work=[];policy=b'AgenticInternet/configured-batch/v1\0'+count.to_bytes(4,'big')
for n in range(1,count+1):
 op=sha(n.to_bytes(8,'big'));payload=f'ciphertext storage request {n}'.encode();policy+=op+len(payload).to_bytes(4,'big')+payload;work.append({'operation':'0x'+op.hex(),'payload':'0x'+payload.hex()})
now=int(time.time());committee={'network':[17]*32,'log':list(sha(policy)),'epoch':3,'valid_from':now-1,'valid_until':now+120,'members':[list(k) for k in keys]}
cid=sha(cbor(['ain-p256-finalizer-committee-v1',bytes(committee['network']),bytes(committee['log']),3,now-1,now+120,4,keys]));genesis=sha(b'AgenticInternet/finalizer/genesis/v1\0'+cid)
listeners=[]
for _ in range(4):
 sock=socket.socket();sock.bind(('127.0.0.1',0));listeners.append(sock)
addresses=[f'127.0.0.1:{sock.getsockname()[1]}' for sock in listeners];peers=[{'publicKey':'0x'+key.hex(),'address':addr} for key,addr in zip(keys,addresses)]
for sock in listeners:sock.close()
binary=root/'target/debug/agentic-finalizer-validator-p256'; processes=[];handles=[];histories=[];verified=0;wrong_subject=0
report={'passed':False,'configuredFixtureOnly':True,'transport':'Commonware authenticated lookup TCP, 4 separate native processes','count':count,'committeeId':cid.hex(),'binarySha256':sha(binary.read_bytes()).hex(),'cleanup':[]}
def varint(data,offset):
 start=offset;n=0;shift=0
 while True:
  byte=data[offset];offset+=1;n|=(byte&127)<<shift
  if byte<128:break
  shift+=7;assert shift<64
 assert offset==start+max(1,(n.bit_length()+6)//7)
 return n,offset
with tempfile.TemporaryDirectory(prefix='ain-p01-p256-runtime-demo-') as temp:
 try:
  for i,key in enumerate(keys):
   request={'committee':committee,'signer':'0x'+seeds[key].hex(),'listen':addresses[i],'peers':peers,'storageDirectory':str(Path(temp)/f'validator-{i}'),'work':work}
   (out/f'input-{i}.json').write_text(json.dumps(request,indent=2)+'\n')
   stdout=(out/f'validator-{i}.jsonl').open('w');stderr=(out/f'validator-{i}.stderr').open('w');handles.extend([stdout,stderr])
   child=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=stdout,stderr=stderr);processes.append(child);child.stdin.write(json.dumps(request).encode());child.stdin.close()
  deadline=time.monotonic()+60
  while True:
   histories=[]
   for i in range(4):
    lines=(out/f'validator-{i}.jsonl').read_text().splitlines();records=[json.loads(line) for line in lines if line.endswith('}')];histories.append([v for v in records if v['status']=='configured_validator_finalized'])
   if all(len(h)==count for h in histories):break
   assert time.monotonic()<deadline,[(p.poll(),len(h)) for p,h in zip(processes,histories)]
   assert all(p.poll() is None for p in processes)
   time.sleep(.05)
  for history in histories:
   parent=genesis
   for n,record in enumerate(history,1):
    payload=bytes.fromhex(work[n-1]['payload'][2:]);op=bytes.fromhex(work[n-1]['operation'][2:]);raw=cbor(['ain-finalized-entry-v1',cid,n,parent,op,payload]);digest=sha(raw)
    assert record['sequence']==n and record['entry']=='0x'+raw.hex() and record['digest']=='0x'+digest.hex();parent=digest
    if record['certificate'] is None:continue
    cert=bytes.fromhex(record['certificate'][2:]);epoch,offset=varint(cert,0);view,offset=varint(cert,offset);pv,offset=varint(cert,offset)
    assert epoch==3 and 0<=pv<view;assert cert[offset:offset+32]==digest;offset+=32;proposal=cert[:offset]
    bit_count=int.from_bytes(cert[offset:offset+8],'big');offset+=8;assert bit_count==4
    bitmap=cert[offset];offset+=1;assert bitmap&0xf0==0
    selected=[i for i in range(4) if bitmap&(1<<i)];signatures,offset=varint(cert,offset);assert signatures==len(selected)>=3
    namespace=b'AgenticInternet/finalizer/p256-v1\0'+cid+b'_FINALIZE';signed=bytes([len(namespace)])+namespace+proposal
    for i in selected:
     signature=cert[offset:offset+64];offset+=64;assert len(signature)==64
     r=int.from_bytes(signature[:32],'big');s=int.from_bytes(signature[32:],'big');order=0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551
     assert 0<r<order and 0<s<=order//2
     signature=encode_dss_signature(r,s);key=ec.EllipticCurvePublicKey.from_encoded_point(ec.SECP256R1(),keys[i]);key.verify(signature,signed,ec.ECDSA(hashes.SHA256()));verified+=1
     try:key.verify(signature,signed.replace(b'_FINALIZE',b'_NOTARIZE'),ec.ECDSA(hashes.SHA256()))
     except InvalidSignature:wrong_subject+=1
     else:raise AssertionError('wrong subject accepted')
    assert offset==len(cert)
   assert history[-1]['certificate'] is not None
  assert all([r['entry'] for r in h]==[r['entry'] for r in histories[0]] for h in histories)
  report.update(passed=True,independentVerifier='Python cryptography46.0.5 OpenSSL P-256 + independent CBOR and Simplex fixture parser',verifiedSignatures=verified,wrongSubjectRefusals=wrong_subject,recordsPerProcess=[len(h) for h in histories],tipDigest=histories[0][-1]['digest'])
 finally:
  for child in processes:
   try:child.kill();child.wait(timeout=5)
   except Exception as error:report['cleanup'].append(str(error))
  for handle in handles:handle.close()
  report['stderrEmpty']=all((out/f'validator-{i}.stderr').read_bytes()==b'' for i in range(len(processes)))
  if not report['stderrEmpty'] or report['cleanup']:report['passed']=False
  (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2));assert report['passed']
