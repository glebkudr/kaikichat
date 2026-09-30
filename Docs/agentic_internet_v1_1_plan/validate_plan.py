#!/usr/bin/env python3
"""Validate task DAG/traceability. Not a messenger test or security audit.

Usage: python validate_plan.py [--root PATH] [--output PATH]
Uses Python 3.10+ standard library only.
"""
from __future__ import annotations
import argparse
from collections import Counter
import json
from pathlib import Path
import sys
from typing import Any


def validate(backlog: dict[str, Any], requirements: dict[str, Any]) -> dict[str, Any]:
    """Return structural validation with explicit product non-assurance."""
    errors: list[str] = []
    tasks = backlog.get('tasks', [])
    reqs = requirements.get('requirements', [])
    if not isinstance(tasks, list) or not isinstance(reqs, list):
        return _report(errors=['tasks and requirements must be lists'])
    if any(not isinstance(x, dict) for x in tasks + reqs):
        return _report(errors=['every task and requirement must be an object'])
    tids = [t.get('id') for t in tasks]
    rids = [r.get('id') for r in reqs]
    if any(not isinstance(x, str) or not x for x in tids + rids):
        return _report(errors=['IDs must be non-empty strings'])
    for ident, count in Counter(tids).items():
        if count > 1:
            errors.append(f'duplicate task {ident}')
    for ident, count in Counter(rids).items():
        if count > 1:
            errors.append(f'duplicate requirement {ident}')
    task_map = {t['id']: t for t in tasks}
    req_map = {r['id']: r for r in reqs}
    coverage: dict[str, list[str]] = {rid: [] for rid in req_map}
    required_text = ('title', 'owns', 'result', 'negative_test', 'fault_test',
                     'acceptance_demo', 'non_goals', 'contract_review')
    for task in tasks:
        tid = task['id']
        for field in required_text:
            value = task.get(field)
            if not isinstance(value, str) or not value.strip():
                errors.append(f'{tid}: missing {field}')
        checks = task.get('tests_first')
        if not isinstance(checks, list) or len(checks) < 2 or any(
            not isinstance(x, str) or not x.strip() for x in checks
        ):
            errors.append(f'{tid}: at least two non-empty tests_first are required')
        if task.get('status') != 'planned':
            errors.append(f'{tid}: unsupported status for this planning artifact')
        refs = task.get('requirements', [])
        deps = task.get('depends_on', [])
        if not isinstance(refs, list) or not all(isinstance(x, str) for x in refs):
            errors.append(f'{tid}: requirements must be string IDs')
            refs = []
        if not isinstance(deps, list) or not all(isinstance(x, str) for x in deps):
            errors.append(f'{tid}: depends_on must be string IDs')
            deps = []
        for rid in refs:
            if rid not in coverage:
                errors.append(f'{tid}: unknown requirement {rid}')
            else:
                coverage[rid].append(tid)
        for dep in deps:
            if dep not in task_map:
                errors.append(f'{tid}: unknown dependency {dep}')
            elif dep == tid:
                errors.append(f'{tid}: self dependency cycle')
    for rid, items in coverage.items():
        if not items:
            errors.append(f'uncovered requirement {rid}')
        reverse = req_map[rid].get('planned_tasks', [])
        if not isinstance(reverse, list) or set(reverse) != set(items):
            errors.append(f'{rid}: inconsistent reverse mapping')
    for rid in ('R10', 'R25', 'R45', 'R46', 'R47', 'R48', 'R49', 'R50', 'R51', 'R52'):
        if rid in req_map and req_map[rid].get('scope') != 'V1':
            errors.append(f'{rid}: mandatory V1 requirement moved out of scope')

    # Explicit amendment guardrails: these validate declarations, not semantics.
    required_policy = {
        'desktop': 'tauri-2',
        'auth_providers': ['google', 'telegram', 'organization-oidc'],
        'single_issuer_profile_allowed': True,
        'reviews_phase': 'V1',
        'review_eligibility': 'bilateral-accepted-order',
        'review_requires_provider_approval': False,
        'review_requires_payment': False,
        'review_requires_completion': False,
        'v1_execution_provenance': 'executor-declaration',
        'economic_agent_engine_phase': 'V2+',
        'transport_economics_phase': 'V1',
        'credentials_are_identity_roots': False,
        'review_is_quality_proof': False,
    }
    meta = backlog.get('metadata', {})
    policy = meta.get('revision_contract', {})
    for key, expected in required_policy.items():
        value = policy.get(key)
        if value != expected or type(value) is not type(expected):
            errors.append(f'revision contract {key}: {value!r} != {expected!r}')
    for rid in ('R45', 'R46', 'R47', 'R48', 'R49', 'R50', 'R51', 'R52'):
        if rid not in req_map:
            errors.append(f'mandatory revision requirement {rid} missing')
    for task in tasks:
        expected_scope = 'V1-contract-only' if task['id'] == 'A04' else 'V1-implementation'
        if task.get('delivery_scope') != expected_scope:
            errors.append(f"{task['id']}: invalid delivery_scope; expected {expected_scope}")
    if req_map.get('R23', {}).get('scope') != 'contract-now/future-implementation':
        errors.append('R23 economic engine phase is not contract-now/future-implementation')
    if requirements.get('metadata') != backlog.get('metadata'):
        errors.append('backlog/requirements metadata mismatch')

    # Kahn's algorithm. Unknown references are errors, not ignored prerequisites.
    pending = set(task_map)
    completed: set[str] = set()
    levels: list[list[str]] = []
    while pending:
        ready: list[str] = []
        for tid in sorted(pending):
            deps = task_map[tid].get('depends_on', [])
            if isinstance(deps, list) and all(isinstance(d, str) for d in deps):
                if set(deps) <= completed:
                    ready.append(tid)
        if not ready:
            errors.append('dependency cycle or unresolved dependency: ' + ', '.join(sorted(pending)))
            break
        levels.append(ready)
        completed.update(ready)
        pending.difference_update(ready)
    for document, count_key, actual in (
        (backlog, 'task_count', len(tasks)),
        (requirements, 'requirement_count', len(reqs)),
    ):
        declared = document.get('metadata', {}).get(count_key)
        if declared != actual:
            errors.append(f'metadata {count_key} mismatch: {declared} != {actual}')
    return _report(errors=errors, task_count=len(tasks), requirement_count=len(reqs),
                   coverage=coverage, parallel_frontiers=levels)


def _report(*, errors: list[str], **details: Any) -> dict[str, Any]:
    return {
        'validation_scope': 'plan structure and coverage only',
        'valid_plan_structure': not errors,
        'product_validated': False,
        'product_tests_executed': 0,
        'errors': errors,
        **details,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    try:
        backlog = json.loads((args.root / 'backlog.json').read_text(encoding='utf-8'))
        requirements = json.loads((args.root / 'requirements.json').read_text(encoding='utf-8'))
        report = validate(backlog, requirements)
        text = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
        if args.output:
            args.output.write_text(text, encoding='utf-8')
        print(text, end='')
        return 0 if report['valid_plan_structure'] else 1
    except (OSError, ValueError, TypeError) as error:
        print(f'Plan validation failed: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
