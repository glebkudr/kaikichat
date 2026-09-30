#!/usr/bin/env python3
"""Describe a new H10 run against the surviving R1 summary, without changing either.

Usage (choose a fresh wrapper output directory and an actual completed H10 run):
  python3 scripts/build-storage.py --profile portable-linux \
    --output output/h10-surviving-comparison-001 run python3 \
    evidence/reviews/portable-linux-2026-09-17/tools/compare-surviving-h10.py --current output/ACTUAL-H10-RUN \
    --output output/h10-surviving-comparison-001/comparison.json

This extraction is not a native run or a replacement trace/signature oracle.
Identical metric definitions permit descriptive deltas, but different recovery
endpoints prevent a workload-matched read-reduction/performance conclusion.
"""

import argparse
import collections
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[4]
HISTORY = ROOT / 'evidence/reviews/AR2-wallet-flow'
R1 = HISTORY / 'diagnostic32-native-r1.json'
TASK = 'V1-H10'


def get(value, *keys):
    for key in keys:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def digest(data):
    return hashlib.sha256(data).hexdigest()


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def task_ids(value, prefix='$'):
    """Preserve identifiers actually present; TASK above is the requested task."""
    result = []
    if isinstance(value, dict):
        for key, child in value.items():
            pointer = prefix + '.' + key
            if key in {'task', 'taskId', 'task_id', 'taskIds', 'task_ids'}:
                result.append(dict(pointer=pointer, value=child))
            result.extend(task_ids(child, pointer))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            result.extend(task_ids(child, f'{prefix}[{index}]'))
    return result


class Inputs:
    def __init__(self):
        self.records = []

    def read(self, path, role, required=False, parse=True):
        path = path.absolute()
        if not path.exists():
            if required:
                raise ValueError(f'missing required {role}: {path}')
            self.records.append(dict(path=str(path), role=role, present=False))
            return None
        if not path.is_file():
            raise ValueError(f'not a regular input file: {path}')
        data = path.read_bytes()
        value = json.loads(data) if parse else data
        if parse and not isinstance(value, dict):
            raise ValueError(f'expected a JSON object in {path}')
        self.records.append(dict(path=str(path), role=role, present=True,
                                 bytes=len(data), sha256=digest(data),
                                 taskIdentifiers=task_ids(value) if parse else []))
        return value

    def verify_unchanged(self):
        for record in self.records:
            path = Path(record['path'])
            if record['present']:
                data = path.read_bytes()
                if len(data) != record['bytes'] or digest(data) != record['sha256']:
                    raise ValueError(f'input changed during extraction: {path}')
            elif path.exists():
                raise ValueError(f'input appeared during extraction; retry after run stops: {path}')


def raw_observations(contents):
    """Count the same events/staging sums as the surviving analyze-diagnostic32.py.

    No semantic uniqueness or admission value is inferred from these counts.
    The checked readMetrics report remains the source for those measurements.
    """
    if contents is None:
        return None
    events, waits, processes = collections.Counter(), collections.Counter(), set()
    bodies = added = 0
    for line in contents.decode().splitlines():
        if not line.startswith('{'):
            continue
        event = json.loads(line)
        if event.get('target') != 'ain_custody_read':
            continue
        fields = {}
        for span in event.get('spans', []):
            fields.update(span)
        fields.update(event.get('span') or {})
        fields.update(event['fields'])
        name = fields['message']
        events[name] += 1
        processes.add((fields['process_id'], fields['boot']))
        if name == 'custody_read_wait':
            waits[fields['reason']] += 1
        if name == 'custody_page_staged':
            bodies += fields['bodies']
            added += fields['added']
    return dict(events=dict(events), processes=len(processes), waitEvents=dict(waits),
                stagedBodyObservations=bodies, cacheAddedObservations=added)


def delta(before, after):
    if number(before) and number(after):
        return after - before
    if isinstance(before, dict) and isinstance(after, dict):
        if all(number(v) for v in [*before.values(), *after.values()]):
            return {key: after.get(key, 0) - before.get(key, 0)
                    for key in sorted(before.keys() | after.keys())}
    return None


def row(before, after, definition, baseline_source, current_source, scope='scenario-input'):
    return dict(baseline=before, current=after, delta=delta(before, after),
                definition=definition, baselineSource=baseline_source,
                currentSource=current_source, scope=scope,
                definitionComparable=True,
                status='descriptive-only' if before is not None and after is not None
                       else 'current-measurement-unavailable',
                performanceClaim=False)


def sum_field(records, field):
    if not isinstance(records, list) or not records:
        return None
    values = [get(record, field) for record in records]
    return sum(values) if all(number(value) for value in values) else None


def compare(baseline, evidence, trace, prior, raw, capture, prior_source='baseline-comparison.json'):
    """Pure extraction; absent historical fields are never reconstructed."""
    evidence, trace, prior = evidence or {}, trace or {}, prior or {}
    observed = get(evidence, 'traceObservations') or {}
    metrics = get(observed, 'readMetrics') or {}
    comparisons = {}

    def add(name, before, after, definition, historical, current, scope='scenario-input'):
        comparisons[name] = row(before, after, definition, historical, current, scope)

    originals = trace.get('completedSender')
    for name, field, report_field in (
        ('dataReplicas', 'replicas', 'originalDataReplicas'),
        ('indexReplicas', 'indexReplicas', 'indexPromises'),
        ('locationAcknowledgments', 'indexLocations', 'locationAcknowledgments'),
    ):
        measured = sum_field(originals, field)
        if measured is not None and report_field in evidence and measured != evidence[report_field]:
            raise ValueError(f'trace and evidence disagree on {name}')
        add(name, baseline.get(name), measured, f'Sum of completedSender[].{field}.',
            f'R1.{name}', f'trace.json:completedSender[].{field}')
    add('originals', baseline.get('originals'), len(originals) if isinstance(originals, list) else None,
        'Number of completed sender original records.', 'R1.originals', 'trace.json:completedSender')
    batches = trace.get('completedBatches')
    add('batches', baseline.get('batches'), len(batches) if isinstance(batches, list) else None,
        'Number of completed publication batches.', 'R1.batches', 'trace.json:completedBatches')
    add('verifiedSignatures', baseline.get('verifiedSignatures'), evidence.get('verifiedSignatures'),
        'Native report verifiedSignatures count.', 'R1.verifiedSignatures', 'evidence.json:verifiedSignatures')
    for name in ('dataLoss', 'indexLoss'):
        records = trace.get(name)
        measured = sum_field(list(records.values()), 'removed') if isinstance(records, dict) else None
        add(name, get(baseline, 'losses', name), measured, f'Sum of {name} holder removed counts.',
            f'R1.losses.{name}', f'trace.json:{name}.*.removed')

    publication = get(trace, 'publication') or {}
    pages = publication.get('graph')
    kinds = dict(collections.Counter(page['kind'] for page in pages)) if isinstance(pages, list) else None
    add('preparedPageKinds', get(baseline, 'publication', 'preparedPageKinds'), kinds,
        'Counts of prepared graph records by page kind; superseded prepared roots may be unpublished.',
        'R1.publication.preparedPageKinds', 'trace.json:publication.graph[].kind', 'publication')
    leaves = get(prior, 'leafOccupancy', 'current')
    if leaves is not None and kinds is not None and sum(leaves.values()) != kinds.get('leaf', 0):
        raise ValueError('current leaf occupancy differs from trace prepared leaf count')
    add('preparedLeafOccupancy', get(baseline, 'publication', 'leafSizes'), leaves,
        'Histogram of reference count in each prepared leaf signed body, across publication.graph; '
        'current values extracted from the run comparator, not reverified by this helper.',
        'R1.publication.leafSizes', prior_source+('.' if ':' in prior_source else ':')+'leafOccupancy.current', 'publication')
    issued = [get(record, 'prepared', 'envelope', 'issuedAt') for record in originals or []]
    completed = [get(record, 'observedAt') for record in originals or []]
    elapsed = max(completed)-min(issued) if issued and all(number(v) for v in issued+completed) else None
    add('publicationElapsedSeconds', get(baseline, 'publication', 'elapsedSeconds'), elapsed,
        'max(completedSender[].observedAt) - min(completedSender[].prepared.envelope.issuedAt); '
        'publication interval only, excludes later retrieval/SQL/cold recovery and teardown.',
        'R1.publication.elapsedSeconds', 'trace.json:completedSender', 'publication')

    for name, description in (
        ('queued', 'Number of observed queued read request identities across captured recipient processes.'),
        ('finished', 'Number of observed finished read request identities across captured recipient processes.'),
        ('selected', 'Number of distinct selected (process, boot, Work, operation) identities.'),
    ):
        add(name, get(baseline, 'observations', name), observed.get(name), description,
            f'R1.observations.{name}', f'evidence.json:traceObservations.{name}',
            'whole-capture; recovery endpoints differ')
    old_events = get(baseline, 'observations', 'events') or {}
    current_events = observed.get('events')
    if current_events is None and raw is not None:
        current_events = raw['events']
    for name, before in old_events.items():
        add('event.'+name, before, current_events.get(name, 0) if current_events is not None else None,
            f'Count of {name} log events; not a semantic-unique, repeated, or admitted read count.',
            f'R1.observations.events.{name}', f'evidence.json:traceObservations.events.{name}'
            if observed.get('events') is not None else f'recipient-read-trace.log:{name}',
            'whole-capture; recovery endpoints differ')
    old_processes = baseline.get('processes', {})
    for name, source_key, description in (
        ('globalReadRateWaitEvents', 'global_read_rate', 'Count of custody_read_wait events whose reason is global_read_rate.'),
        ('stagedBodyObservations', 'stagedBodyObservations', 'Sum of staged bodies observations; responses may overlap.'),
        ('cacheAddedObservations', 'cacheAddedObservations', 'Sum of staged added observations; not unique lifetime cache coverage.'),
    ):
        if name == 'globalReadRateWaitEvents':
            before = sum(get(p, 'waitEvents', source_key) or 0 for p in old_processes.values())
            after = (get(raw, 'waitEvents', source_key) or 0) if raw is not None else None
        else:
            before = sum(p[source_key] for p in old_processes.values())
            after = get(raw, source_key)
        add(name, before, after, description, f'R1.processes.*.{source_key}',
            f'recipient-read-trace.log:{source_key}', 'whole-capture; recovery endpoints differ')

    unavailable = {}
    for name, after, definition, source in (
        ('newReads', metrics.get('newReads'), 'Distinct canonical semantic keys among queued attempts in the checked capture.', 'readMetrics.newReads'),
        ('repeatedReads', metrics.get('repeatedReads'), 'Queued attempts minus distinct canonical semantic keys; includes failed/unfinished attempts.', 'readMetrics.repeatedReads'),
        ('admittedRequests', metrics.get('admitted'), 'Observed admission identities, including attempts not queued or interrupted before queueing.', 'readMetrics.admitted'),
        ('wallTimeSeconds', observed.get('wallTimeSeconds'), 'Outer Python monotonic elapsed time through provider teardown; not process uptime or publication elapsed.', 'wallTimeSeconds'),
    ):
        unavailable[name] = dict(baseline=None, current=after, delta=None, status='blocked-by-data-loss',
            definition=definition, baselineSource=None,
            currentSource='evidence.json:traceObservations.'+source,
            reason='R1 summary contains no such measurement and its original raw artifacts were irretrievably lost.',
            performanceClaim=False)

    # A success has no comparable pre-first-SQL-failure endpoint. Keep all
    # snapshots with their real names; never replace R1's 48 with a new phase sum.
    snapshots = dict(
        baselineFinalRecipientSnapshot=dict(source='R1.finalRecipientSnapshot',
            scope='Second recipient process snapshot before the first exact-original SQL gate was reached.',
            values=baseline.get('finalRecipientSnapshot')),
        currentFailureActor1Snapshot=dict(source='trace.json:failureActor1.custodySync',
            values=get(trace, 'failureActor1', 'custodySync')),
        currentTransportObservations=dict(source='trace.json:transportObservations',
            values=trace.get('transportObservations')),
        delta=None, status='not-scope-comparable',
        reason='Per-process counters can advance after observation. Failure snapshots, successful phase snapshots, '
               'and sums across phases have different endpoints; none are substituted for one another.')
    return dict(comparisons=comparisons, unavailableBaselineComparisons=unavailable,
        counterSnapshots=snapshots,
        currentMeasuredTrace=dict(readMetrics=observed.get('readMetrics'),
            captureScope=observed.get('captureScope'), rawEventRecount=raw,
            readAttemptCount=len(observed['readAttempts']) if isinstance(observed.get('readAttempts'), list) else None,
            wallTimeSource=get(capture, 'wallTimeSource'),
            validation='Extracted from the current oracle report; raw event counts cross-checked when available. '
                       'This helper does not run the graph, signature, or semantic-query oracle.'))


def check_consistency(baseline, baseline_hash, evidence, trace, prior, raw, capture, raw_bytes):
    evidence = evidence or {}
    embedded = evidence.get('h10BaselineComparison')
    if embedded is not None and prior is not None and embedded != prior:
        raise ValueError('evidence and standalone baseline-comparison.json disagree')
    if prior is not None:
        if prior.get('summarySha256') != baseline_hash:
            raise ValueError('run comparator is bound to a different historical summary')
        for name in ('newReads', 'repeatedReads', 'admittedRequests', 'wallTimeSeconds'):
            if get(prior, name, 'baseline') is not None or get(prior, name, 'delta') is not None:
                raise ValueError(f'run comparator invents unavailable R1 measurement: {name}')
        if get(prior, 'queued', 'baseline') != get(baseline, 'observations', 'queued'):
            raise ValueError('run comparator historical queue count differs')
        if get(prior, 'leafOccupancy', 'baseline') != get(baseline, 'publication', 'leafSizes'):
            raise ValueError('run comparator historical leaf occupancy differs')
    observed = evidence.get('traceObservations') or {}
    metrics = observed.get('readMetrics') or {}
    if prior is not None and observed:
        for name, actual in (
            ('queued', metrics.get('queued')),
            ('newReads', metrics.get('newReads')),
            ('repeatedReads', metrics.get('repeatedReads')),
            ('admittedRequests', metrics.get('admitted')),
            ('wallTimeSeconds', observed.get('wallTimeSeconds')),
        ):
            if get(prior, name, 'current') != actual:
                raise ValueError(f'run comparator and current observations disagree on {name}')
    if raw is not None and observed.get('events') is not None and raw['events'] != observed['events']:
        raise ValueError('raw capture event counts differ from oracle report')
    for owner, field in ((evidence, 'traceSha256'), (capture or {}, 'traceSha256')):
        if raw_bytes is not None and field in owner and owner[field] != digest(raw_bytes):
            raise ValueError(f'raw capture hash differs from {field}')
    if capture is not None and observed.get('wallTimeSeconds') is not None:
        if capture.get('wallTimeSeconds') != observed['wallTimeSeconds']:
            raise ValueError('outer wall time differs between capture and evidence')
    if metrics:
        if metrics.get('queued') != observed.get('queued'):
            raise ValueError('queued count differs within traceObservations')
        if metrics.get('semanticCoverage') == 'complete':
            if not all(type(metrics.get(name)) is int and metrics[name] >= 0
                       for name in ('queued', 'newReads', 'repeatedReads')):
                raise ValueError('complete semantic metrics lack nonnegative counts')
            if metrics['newReads'] + metrics['repeatedReads'] != metrics['queued']:
                raise ValueError('semantic new/repeated counts do not cover queued attempts')
        elif metrics.get('newReads') is not None or metrics.get('repeatedReads') is not None:
            raise ValueError('incomplete semantic capture claims exact new/repeated counts')
        if metrics.get('admitted') is not None and metrics['admitted'] < metrics['queued']:
            raise ValueError('admitted count is smaller than queued count')
    wall = observed.get('wallTimeSeconds')
    if wall is not None and (not number(wall) or wall < 0):
        raise ValueError('invalid current outer wall time')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--current', required=True, type=Path, help='Actual new H10 output directory; read only.')
    parser.add_argument('--output', required=True, type=Path, help='Fresh JSON path outside the selected run; parent must exist.')
    args = parser.parse_args(argv)
    current, output = args.current.resolve(strict=True), args.output.absolute()
    parent = output.parent.resolve(strict=True)
    output = parent / output.name
    if not current.is_dir():
        raise ValueError('--current must be a directory')
    if output.is_relative_to(current) or output.is_relative_to(HISTORY.resolve()):
        raise ValueError('--output must be separate from the current run and historical evidence')
    if output.exists() or output.is_symlink():
        raise ValueError('--output already exists; choose a fresh path')
    if output.suffix != '.json':
        raise ValueError('--output must name a .json file')
    inputs = Inputs()
    baseline = inputs.read(R1, 'historical-r1-summary', required=True)
    baseline_hash = inputs.records[-1]['sha256']
    for name in ('diagnostic32-native-r2.json', 'diagnostic32-checks.json'):
        inputs.read(HISTORY / name, 'historical-context-only')
    inputs.read(HISTORY / 'analyze-diagnostic32.py', 'historical-definition-source', parse=False)
    inputs.read(Path(__file__), 'comparison-helper', required=True, parse=False)
    for name in ('public_history_read_trace.py', 'history_read_trace_oracle.py'):
        inputs.read(ROOT / 'tests/evm' / name, 'current-schema-source', required=True, parse=False)
    evidence = inputs.read(current / 'evidence.json', 'current-evidence')
    trace = inputs.read(current / 'trace.json', 'current-scenario-trace')
    prior = inputs.read(current / 'baseline-comparison.json', 'current-native-comparator')
    if evidence is None and trace is None and prior is None:
        raise ValueError('--current contains none of evidence.json, trace.json, baseline-comparison.json')
    prior_source = 'baseline-comparison.json'
    if prior is None and get(evidence, 'h10BaselineComparison') is not None:
        prior = evidence['h10BaselineComparison']
        prior_source = 'evidence.json:h10BaselineComparison'
    guard = inputs.read(current / 'execution-guard.json', 'current-execution-guard')
    capture = inputs.read(current / 'recipient-capture.json', 'current-capture-manifest')
    if capture is not None and get(evidence, 'captureManifestSha256') is not None:
        if evidence['captureManifestSha256'] != inputs.records[-1]['sha256']:
            raise ValueError('capture manifest bytes differ from the evidence captureManifestSha256')
    raw_bytes = inputs.read(current / 'recipient-read-trace.log', 'current-raw-recipient-log', parse=False)
    raw = raw_observations(raw_bytes)
    check_consistency(baseline, baseline_hash, evidence, trace, prior, raw, capture, raw_bytes)
    payload = compare(baseline, evidence, trace, prior, raw, capture, prior_source)
    payload.update(schema='ain-h10-surviving-summary-comparison-v1', task=TASK,
        status='blocked-by-data-loss', fullBaselineComparisonAccepted=False,
        readReductionClaim=False, fullH10AcceptanceClaim=False,
        generatedAt=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        baseline=dict(summaryPath=str(R1), summarySha256=baseline_hash,
            runPath='output/hrt32-r1', nativePassed=baseline.get('nativePassed'),
            completePathObserved=baseline.get('completePathObserved'), nativeFailure=baseline.get('nativeFailure'),
            historicalSourceHashClaims=baseline.get('sources')),
        current=dict(outputPath=str(current), reportedNativePassed=get(evidence, 'passed'),
            reportedDiagnostic32NativePassed=get(evidence, 'diagnostic32NativePassed'),
            reportedH10AcceptanceStatus=get(evidence, 'h10AcceptanceStatus'),
            completePathObserved=get(evidence, 'traceObservations', 'completePathObserved'),
            executionGuard=guard, nativeBaselineComparison=prior,
            nativeBaselineComparisonSource=prior_source if prior is not None else None),
        dataLoss=dict(confirmedByUser=True, recoverable=False,
            fact='User confirmed the original output/hrt32-r1 artifacts and logs were irretrievably lost '
                 'after the WD4000 reformat. Only retained review summaries/checks and the analysis script survive.',
            missingHistoricalMeasurements=['newReads', 'repeatedReads', 'admittedRequests', 'wallTimeSeconds'],
            expectedHashesAreRecoveredBytes=False),
        inputs=inputs.records,
        limitations=[
            'R1 failed before the first exact-original SQL gate. The current run may cover additional phases; '
            'same-definition counts are descriptive and are not a matched-workload performance benchmark.',
            '542 seconds in R1 is the publication interval only. Its full run wall time is unknown.',
            'R1 queued/finished 57/57, applied 29, selected 19, and read-rate waits 60 do not measure semantic uniqueness or admission.',
            'R1 readRequests 48, bulkReadRequests 16, and historyPathReadRequests 15 are one recipient process snapshot.',
            'Staged bodies 12 and cache additions 10 overlap across responses; their difference is not repeated reads.',
            'Historical expected raw-file hashes preserve provenance claims; they do not restore unavailable raw bytes.',
            'Current measurements and native correctness can be reported separately, while full trace-verifiable '
            'R1 comparison remains blocked by data loss. No Full130 or full V1 acceptance is claimed.',
        ])
    inputs.verify_unchanged()
    contents = (json.dumps(payload, indent=2, ensure_ascii=False, allow_nan=False) + '\n').encode()
    # Exclusive create: concurrent callers cannot overwrite a report or input.
    with output.open('xb') as stream:
        stream.write(contents)
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(dict(output=str(output), bytes=len(contents), sha256=digest(contents),
                          task=TASK, status=payload['status'])))
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f'comparison failed: {error}', file=sys.stderr)
        sys.exit(2)
