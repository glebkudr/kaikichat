"""Read-only structural measurements; no signature revalidation or native execution."""
import argparse, collections, hashlib, json
from zipfile import ZipFile
from pathlib import Path
parser = argparse.ArgumentParser(description=__doc__)
inputs = parser.add_mutually_exclusive_group(required=True)
inputs.add_argument('--root', type=Path, help='Extracted review bundle directory')
inputs.add_argument('--archive', type=Path, help='Original ZIP review bundle')
parser.add_argument('--out', type=Path, default=Path('r14_structural_measurements.json'))
args = parser.parse_args()
def read(name):
    if args.archive:
        with ZipFile(args.archive) as archive:
            return archive.read(name)
    return (args.root / name).read_bytes()
class CBOR:
    def __init__(self, data): self.data, self.pos = bytes(data), 0
    def take(self,n):
        out=self.data[self.pos:self.pos+n]
        if len(out)!=n: raise ValueError('truncated CBOR')
        self.pos+=n; return out
    def item(self):
        b=self.take(1)[0]; m,a=b>>5,b&31
        if a<24: n=a
        elif a in (24,25,26,27): n=int.from_bytes(self.take(1<<(a-24)),'big')
        else: raise ValueError('indefinite CBOR not supported')
        if m==0:return n
        if m==1:return -1-n
        if m==2:return self.take(n)
        if m==3:return self.take(n).decode()
        if m==4:return [self.item() for _ in range(n)]
        if m==5:return {self.item():self.item() for _ in range(n)}
        if m==6:return self.item()
        if m==7 and a in (20,21,22):return {20:False,21:True,22:None}[a]
        raise ValueError((m,a))
def decode(data):
    decoder=CBOR(data); result=decoder.item()
    if decoder.pos!=len(decoder.data):raise ValueError('trailing bytes')
    return result
def compact(v): return json.dumps(v,separators=(',',':'),ensure_ascii=False).encode()
def comm(page):
    d=page['document']; signed=decode(decode(d['wire'])[0]); body=decode(signed[7])
    return dict(epoch=d['epoch'],revision=d['revision'],issued_at=d['issued_at'],expires_at=d['expires_at'],
        anchor_operation=list(body[3]),anchor_descriptor_hash=list(hashlib.sha256(bytes(d['anchor_descriptor'])).digest()),
        manifest_hash=list(hashlib.sha256(bytes(d['wire'])).digest()))
def body(page): return decode(decode(decode(page['document']['wire'])[0])[7])
trace=json.loads(read('review/runtime/hr-r14/trace.json'))
focus=json.loads(read('review/R14_FOCUS.json'))
pub=trace['publication']; pages={hashlib.sha256(bytes(p['document']['wire'])).digest():p for p in pub['graph']}
visited={}; refs=[]; leaf_counts=[]
def walk(h):
    p=pages[h]; visited[h]=p; b=body(p)
    if p['kind']=='leaf': refs.extend(b[5]); leaf_counts.append(len(b[5])); return
    for _,_,child,_ in b[5]: walk(bytes(child[6]))
walk(bytes(pub['locator']['history']['manifest_hash']))
cache=dict(version=1,root=pub['locator']['history'],entries=[dict(commitment=comm(p),page=p) for p in visited.values() if p['kind']!='root'])
cache_hex=json.loads(json.dumps(cache))
for e in cache_hex['entries']:
    for k in ('wire','anchor_descriptor'):e['page']['document'][k]=bytes(e['page']['document'][k]).hex()
a=set(focus['postStop']['cachedSequences']['deferred']);b=set(focus['postStop']['cachedSequences']['prefetch']);i=set(focus['postStop']['exactImportedSequences'])
result=dict(derivation='Offline JSON/CBOR structural analysis of archived R14; not a new run, not cryptographic revalidation.',
 trace_sha256=hashlib.sha256(read('review/runtime/hr-r14/trace.json')).hexdigest(),
 all_prepared_page_kinds=dict(collections.Counter(p['kind'] for p in pages.values())),
 current_reachable_page_kinds=dict(collections.Counter(p['kind'] for p in visited.values())),
 leaf_size_histogram=dict(collections.Counter(leaf_counts)),
 references=len(refs), graph_reference_sequences=[r[2] for r in refs],
 original31_graph_ordinal=[r[2] for r in refs].index(31)+1,
 full_current_root_page_cache_json_bytes=len(compact(cache)),page_cache_limit_bytes=1048576,
 same_cache_with_hex_encoded_two_byte_arrays_bytes=len(compact(cache_hex)),
 deferred_prefetch_overlap=sorted(a&b),imported_prefetch_overlap=sorted(i&b),
 unique_cached=len(a|b),unique_imported_or_cached=len(a|b|i),
 publication_sample_count=len(trace['samples']),
 publication_status_rpc_wait_seconds=sum(s.get('statusSeconds',0) for s in trace['samples']),
 note_status='Caller wall time waiting for status RPCs, not measured CPU time or attributable publication delay.',
 sender_final_selected_metrics={k:trace['samples'][-1]['network'][k] for k in ('custodyResolution','processingCapacity')})
assert result['graph_reference_sequences']==json.loads(read('project/evidence/reviews/AR2-wallet-flow/history-range-native-r14.json'))['graphReferenceSequences']
args.out.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ('graph_reference_sequences','sender_final_selected_metrics')},indent=2))
