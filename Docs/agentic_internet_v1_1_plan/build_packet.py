#!/usr/bin/env python3
"""Regenerate derived Markdown and hashes from the planning packet.
No messenger code is built or tested. Python 3.10+, standard library only.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
PRIMARY = [
    'CHANGELOG_V1_1.md', 'V1_IMPLEMENTATION_PLAN_RU.md',
    'AUTH_AND_ATTESTATION_V1.md', 'PUBLIC_REVIEWS_PROTOCOL_V1.md',
    'BACKLOG.md', 'REQUIREMENTS.md', 'DECISIONS_AND_VERIFY_GATES.md',
    'AGENT_WORK_ORDER.md', 'DAG.md', 'SOURCE_MAP.md',
    'SOURCE_AMENDMENT_2026_09_05.md',
]

def render_task(t: dict[str, Any]) -> str:
    checks = '\n'.join(f"{i+1}. {x}" for i, x in enumerate(t['tests_first']))
    text = f"""# {t['id']} · {t['title']}

**Revision:** 1.1. **Status:** planned; product code/tests are not declared done by this card.  
**Delivery:** {t['delivery_scope']}.  
**Requirements:** {', '.join(t['requirements'])}.  
**Integration GREEN dependencies:** {', '.join(t['depends_on']) or 'none'}.  
**Ownership:** `{t['owns']}`.

## Observable outcome

{t['result']}

## Tests first

{checks}

**Negative test:** {t['negative_test']}

**Failure/race:** {t['fault_test']}

{t['contract_review']}

## Acceptance demo

{t['acceptance_demo']}

## Outside the packet

{t['non_goals']}

## Definition of Done

""" + '\n'.join('- ' + x for x in t['definition_of_done']) + f"""

Contract/tests: `{t['id']}.Txx`; negative: `{t['id']}.N01`; fault: `{t['id']}.F01`; black-box: `{t['id']}.E01`. See `AGENT_WORK_ORDER.md`; AUTH_AND_ATTESTATION_V1.md and PUBLIC_REVIEWS_PROTOCOL_V1.md define the matching new boundaries.
"""
    return text.strip() + '\n'

def render_backlog(b: dict[str, Any]) -> str:
    text = f"""# Backlog V1 · revision 1.1

{len(b['tasks'])} atomic packets; {b['metadata']['requirement_count']} requirements. All tasks are planned. A04 is contract-only for V1, not a mandatory V2 engine implementation. In the table, dependencies mean readiness for integration acceptance, not a ban on writing independent tests early.

| ID | Observable outcome | Depends on | Scope |
|---|---|---|---|
"""
    for t in b['tasks']:
        text += f"| [{t['id']}](tasks/{t['id']}.md) · {t['title']} | {t['result'].replace('|', '/')} | {', '.join(t['depends_on']) or '—'} | {t['delivery_scope']} |\n"
    return text

def render_requirements(r: dict[str, Any]) -> str:
    text = f"""# Requirements matrix · revision 1.1

{len(r['requirements'])} requirements. CURRENT is the original request; Uxx/Axx are 27 messages of the source archive; AMENDMENT-01…05 is the user's addition of September 5. Old IDs are kept, later decisions take priority. R23 is now contract-now/future-implementation; public reviews did not move together with economic agents.

| ID | Requirement | Sources | Scope | Packets |
|---|---|---|---|---|
"""
    for q in r['requirements']:
        text += f"| {q['id']} | {q['title']} | {', '.join(q['sources'])} | {q['scope']} | {', '.join(q['planned_tasks'])} |\n"
    text += '\n## Acceptance criteria\n\n'
    for q in r['requirements']:
        text += f"### {q['id']} · {q['title']}\n\n{q['acceptance']}\n\n"
    return text.strip() + '\n'

def full_text(root: Path, backlog: dict[str, Any]) -> str:
    head = f"""# Agentic Internet · Full V1 plan · revision 1.1

September 5, 2026. {len(backlog['tasks'])} planned packets, {backlog['metadata']['requirement_count']} requirements. Tauri, Google/Telegram/organization attestation and public reviews are in V1; complex economic agents are V2+. This is a plan, not a product implementation/audit.

"""
    files = PRIMARY + [f"tasks/{t['id']}.md" for t in backlog['tasks']]
    for name in files:
        head += '\n---\n\n# File: ' + name + '\n\n' + (root / name).read_text(encoding='utf-8').rstrip() + '\n\n'
    return head.rstrip() + '\n'

def is_packet_source(path: Path) -> bool:
    """Exclude generated inventory and OS/runtime sidecars from packet sources."""
    return (
        path.is_file()
        and not path.name.startswith('._')
        and '__pycache__' not in path.parts
        and path.suffix != '.pyc'
        and path.name != 'FILE_MANIFEST.json'
    )


def manifest(root: Path) -> dict[str, Any]:
    files = []
    for path in sorted(root.rglob('*')):
        if not is_packet_source(path):
            continue
        data = path.read_bytes()
        files.append({'path': path.relative_to(root).as_posix(), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    return {'scope': 'file integrity, not protocol correctness', 'version': '1.1', 'files': files}

def main() -> None:
    b = json.loads((ROOT / 'backlog.json').read_text())
    r = json.loads((ROOT / 'requirements.json').read_text())
    from validate_plan import validate
    report = validate(b, r)
    if not report['valid_plan_structure']:
        raise SystemExit('Invalid plan: ' + '; '.join(report['errors']))
    for t in b['tasks']:
        (ROOT / 'tasks' / (t['id'] + '.md')).write_text(render_task(t), encoding='utf-8')
    (ROOT / 'BACKLOG.md').write_text(render_backlog(b), encoding='utf-8')
    (ROOT / 'REQUIREMENTS.md').write_text(render_requirements(r), encoding='utf-8')
    (ROOT / 'V1_FULL_PLAN_RU.md').write_text(full_text(ROOT, b), encoding='utf-8')
    (ROOT / 'FILE_MANIFEST.json').write_text(json.dumps(manifest(ROOT), ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f"Generated {len(b['tasks'])} task cards, indexes, full plan and manifest. Product implementation is not validated.")

if __name__ == '__main__':
    main()
