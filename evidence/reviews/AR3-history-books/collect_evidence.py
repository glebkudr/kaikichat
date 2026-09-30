"""Bind the complete ordinary two-book result to exact source and public evidence."""
from pathlib import Path
import copy
import hashlib
import json
import sys

root=Path.cwd();folder=Path(__file__).resolve().parent
books_label,compat_label,checks_label=sys.argv[1:]
def read(label):
    report=json.loads((folder/f'{label}.json').read_text())
    assert report['passed'] and not report['source_changes'],label
    inputs=json.loads((folder/f'{label}-inputs.json').read_text())
    for path,digest in inputs.items():
        assert hashlib.sha256((root/path).read_bytes()).hexdigest()==digest,path
    return report,inputs
books,inputs=read(books_label);compat,compat_inputs=read(compat_label);checks,checks_inputs=read(checks_label)
assert inputs==compat_inputs==checks_inputs
assert books['native']['binarySha256']==compat['native']['binarySha256']==hashlib.sha256((root/'target/debug/agentic-node').read_bytes()).hexdigest()
for report in [books,compat]:
    for kind in ['trace','log']:
        assert hashlib.sha256((root/report[f'{kind}_local']).read_bytes()).hexdigest()==report[f'{kind}_sha256']
for check in checks['checks']:
    assert check['exit_code']==0
    assert hashlib.sha256((root/check['log_local']).read_bytes()).hexdigest()==check['log_sha256']
trace=json.loads((root/books['trace_local']).read_text())
required=['twoFundedBooks','preBeaconBooks','disjointOriginalIndexRosters','disjointOriginalDataRosters',
    'shorterNewLeasePreservesOlderOriginal','coldSenderBetweenBooks','completeDeclaredTwoBookRange',
    'senderAbsentDuringFetch','realDataLoss','realIndexLoss','missingPublicTrustRejected',
    'recipientCommitFailure','secondOriginalCommitFailure','perReferenceAtomicProgress','coldRecipientRetry']
assert all(books['native'][k] is True for k in required)
assert books['native']['verifiedSignatures']==6 and books['native']['ownerRetrievalCalls']==books['native']['ownerSenderWorkCalls']==0
sys.path.insert(0,str(root/'tests/evm'))
import public_index_recipient as recipient
references=recipient.check_history(trace['publication'],trace['completedSender'])
assert len(references)==2
completed=trace['completedSender']
for kind in ['receipts','indexReceipts']:
    keys=[{r['transportKey'] for r in v[kind]} for v in completed]
    assert len(keys[0])==len(keys[1])==10 and keys[0].isdisjoint(keys[1])
# Identical registry proof/checkpoint fields are shared without changing a byte.
# Per-peer original signatures, member paths and Noise transport keys stay exact.
presentations=[row['presentation'] for v in completed for kind in ['receipts','indexReceipts'] for row in v[kind]]
common={k:v for k,v in presentations[0].items() if all(p.get(k)==v for p in presentations)}
views=[]
for value in completed:
    fields=['messageId','prepared','spend','plan','state','error','history','pointer','receipts','indexReceipts']
    retained={k:copy.deepcopy(value[k]) for k in fields}
    for kind in ['receipts','indexReceipts']:
        for row,original in zip(retained[kind],value[kind]):
            row['presentation']={k:v for k,v in row['presentation'].items() if k not in common}
            assert dict(row,presentation=dict(common,**row['presentation']))==original
    views.append(retained)
fields=['bookFunding','policyChanges','originalBookRosters','publication','originalHolders','survivingHolders',
    'pointerPeers','dataLoss','indexLoss','survivingIndexes','withoutPublicTrust','failedSync','rollbackBefore',
    'rollbackAfter','secondOriginalFailure','partialProgress','recipientHead','recipient','firstSync','restartedSync',
    'recipientObserverMethods','recipientObserverCalls','senderObserverMethods','committedReferenceProgress','coldVerifiedHistoryPass']
public=dict(provenance={k:books[k] for k in ['trace_local','trace_sha256','log_local','log_sha256']},
    commonPresentation=common,completedSender=views,**{k:trace[k] for k in fields})
output=folder/'books-evidence.json';output.write_text(json.dumps(public,separators=(',',':'))+'\n')
scope=json.loads((root/'Docs/agentic_internet_v1_execution_plan/release-scope.json').read_text())
assert len(scope['required_v1_task_ids'])==67 and len(scope['required_v1_case_ids'])==22 and len(scope['required_platforms'])==3
acceptance=dict(passed=True,kind='complete_declared_two_book_range_not_full_V1',books=books_label,compatibility=compat_label,
    checks=checks_label,collectorSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),inputs=len(inputs),binarySha256=books['native']['binarySha256'],
    public_evidence=dict(path=str(output.relative_to(root)),sha256=hashlib.sha256(output.read_bytes()).hexdigest(),bytes=output.stat().st_size),
    backend_tests=sum(c['tests_passed'] for c in checks['checks'] if c['name']!='frontend'),
    frontend_tests=sum(c['tests_passed'] for c in checks['checks'] if c['name']=='frontend'),
    required_cards=67,required_e2e=22,required_platforms=scope['required_platforms'],fullV1=False)
(folder/'acceptance.json').write_text(json.dumps(acceptance,indent=2)+'\n')
print(json.dumps(acceptance))
