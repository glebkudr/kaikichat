"""Read preserved native evidence; never alter the run or retry its scenario."""
import collections
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT/'tests/evm'))
import history_graph_oracle as graph
from history_read_trace_oracle import check_trace, reference_bindings

run = ROOT/'output/hrt32-r1'
native_log = ROOT/'output/ar2-wallet-flow/diagnostic32-native-r1.log'
report = json.loads((run/'evidence.json').read_text())
assert report['passed'] is False
assert 'AssertionError: indexed original fetch did not reach its atomic message/progress transaction' in native_log.read_text()
trace = json.loads((run/'trace.json').read_text())
guard = json.loads((run/'execution-guard.json').read_text())
assert guard['sourceUnchanged'] and guard['binariesUnchanged']
publication, originals = trace['publication'], trace['completedSender']
references = graph.check(publication, originals)
bindings = reference_bindings(references,
    bytes(publication['locator']['history']['manifest_hash']).hex(),
    originals[0]['prepared']['envelope']['indexId'], publication['locator']['history']['epoch'])
contents = (run/'recipient-read-trace.log').read_bytes()
observations = check_trace(contents, bindings, require_complete=bool(report['passed']))

processes = {}
focus = {str(n):dict(selected=0, queued=0, waits=collections.Counter(),
                     imports=collections.Counter()) for n in (1,31)}
for line in contents.decode().splitlines():
    if not line.startswith('{'): continue
    event = json.loads(line)
    if event.get('target') != 'ain_custody_read': continue
    fields = {}
    for span in event.get('spans', []): fields.update(span)
    fields.update(event.get('span') or {})
    fields.update(event['fields'])
    key = str(fields['process_id'])+':'+fields['boot']
    process = processes.setdefault(key, dict(reads=collections.Counter(),
        imports=collections.Counter(),deferredAttempts=0,deferredImports=0,
        waitEvents=collections.Counter(),stagedResponses=0,stagedBodyObservations=0,
        cacheAddedObservations=0))
    name = fields['message']
    sequence = bindings.get(fields.get('operation'), {}).get('sequence')
    target = focus.get(str(sequence))
    if name == 'custody_reference_selected' and target is not None: target['selected'] += 1
    if name == 'custody_read_queued':
        process['reads'][fields['read_kind']] += 1
        if target is not None: target['queued'] += 1
    if name == 'custody_import_finished':
        process['imports'][fields['outcome']] += 1
        if target is not None: target['imports'][fields['outcome']] += 1
    if name == 'custody_read_wait':
        process['waitEvents'][fields['reason']] += 1
        if target is not None: target['waits'][fields['reason']] += 1
    if name == 'custody_deferred_retry':
        process['deferredAttempts'] += fields['attempted']
        process['deferredImports'] += fields['imported']
    if name == 'custody_page_staged':
        process['stagedResponses'] += 1
        process['stagedBodyObservations'] += fields['bodies']
        process['cacheAddedObservations'] += fields['added']

leaves = collections.Counter()
for page in publication['graph']:
    if page['kind'] != 'leaf': continue
    signed = graph.base.decode(graph.base.decode(bytes(page['document']['wire']))[0])
    body = graph.base.decode(signed[7])
    leaves[len(body[5])] += 1

first = min(v['prepared']['envelope']['issuedAt'] for v in originals)
completed = max(v['observedAt'] for v in originals)
expiry = min(v['prepared']['envelope']['expiresAt'] for v in originals)
sources = [run/'evidence.json',run/'trace.json',run/'execution-guard.json',native_log,
    run/'recipient-read-trace.log',Path(__file__),
    ROOT/'tests/evm/history_read_trace_oracle.py',ROOT/'tests/evm/history_graph_oracle.py']
result = dict(kind='Post-run analysis of preserved Diagnostic32; no new native run',
    nativePassed=report['passed'], completePathObserved=observations['completePathObserved'],
    nativeFailure='First exact-original SQL fault was not reached within the original 120-second gate',
    originalReportTraceError=report.get('traceError'),
    postRunCorrelationValidated=True, ordinalConvention='zero-based graph order, independent of sender sequence',
    sourceAndBinaryGuard=guard, originals=len(originals), batches=len(trace['completedBatches']),
    verifiedSignatures=report['verifiedSignatures'],
    dataReplicas=sum(v['replicas'] for v in originals),
    indexReplicas=sum(v['indexReplicas'] for v in originals),
    locationAcknowledgments=sum(v['indexLocations'] for v in originals),
    losses={k:sum(v['removed'] for v in trace[k].values()) for k in ('dataLoss','indexLoss')},
    publication=dict(firstOriginalIssuedAt=first,lastSenderObservedAt=completed,
        elapsedSeconds=completed-first,earliestOriginalExpiry=expiry,
        originalLeaseSeconds=expiry-first,remainingSeconds=expiry-completed,
        preparedPageKinds=dict(collections.Counter(p['kind'] for p in publication['graph'])),
        leafSizes=dict(leaves),references=len(references)),
    withoutPublicTrust=trace['withoutPublicTrust'],
    finalRecipientSnapshot=trace['failureActor1']['custodySync'],
    observations=observations,processes=processes,focus=focus,
    cleanupErrors=report['cleanupErrors'],remainingProfileDirectories=[p.name for p in run.iterdir() if p.is_dir()],
    sources={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
    limitations=[
        'The original failed report is preserved; post-run ordinal repair is not native acceptance.',
        'No first SQL rollback, partial/full import or cold full-graph completion was reached.',
        'Focus events bind selected operations only; a bulk response may contain other operations.',
        'Staged body and added counts may overlap and are not unique cache coverage.',
        'Native32 admission waits do not establish the cause of the separate Full130 R14 failure.',
        'Prepared root count is not the number of fully published roots.',
        'No Full130 or full V1 acceptance is claimed.'])
print(json.dumps(result, indent=2)+'\n')
