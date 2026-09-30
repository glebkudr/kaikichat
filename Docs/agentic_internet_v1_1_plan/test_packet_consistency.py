"""Consistency checks for the files in this planning packet, not the product."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import re
import tempfile
import unittest
from build_packet import PRIMARY, render_task, render_backlog, render_requirements, full_text, manifest, is_packet_source
from validate_plan import validate
ROOT = Path(__file__).resolve().parent

class PacketConsistencyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.b = json.loads((ROOT/'backlog.json').read_text())
        cls.r = json.loads((ROOT/'requirements.json').read_text())

    def test_metadata_files_agree(self):
        m = json.loads((ROOT/'metadata.json').read_text())
        self.assertEqual(m, self.b['metadata'])
        self.assertEqual(m, self.r['metadata'])

    def test_task_markdown_matches_json(self):
        for t in self.b['tasks']:
            self.assertEqual((ROOT/'tasks'/f"{t['id']}.md").read_text(), render_task(t), t['id'])

    def test_task_files_are_exact(self):
        self.assertEqual({p.stem for p in (ROOT/'tasks').glob('*.md') if is_packet_source(p)}, {t['id'] for t in self.b['tasks']})

    def test_manifest_ignores_appledouble_without_hiding_real_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'F01.md').write_text('task')
            (root/'UNEXPECTED.md').write_text('an extra real document')
            (root/'._F01.md').write_bytes(bytes.fromhex('00051607') + b'metadata')
            self.assertEqual(
                {entry['path'] for entry in manifest(root)['files']},
                {'F01.md', 'UNEXPECTED.md'},
            )

    def test_indexes_match_json(self):
        self.assertEqual((ROOT/'BACKLOG.md').read_text(),render_backlog(self.b))
        self.assertEqual((ROOT/'REQUIREMENTS.md').read_text(),render_requirements(self.r))

    def test_full_document_is_exact_assembly(self):
        self.assertEqual((ROOT/'V1_FULL_PLAN_RU.md').read_text(),full_text(ROOT,self.b))

    def test_metadata_dag_matches_computed(self):
        levels=validate(self.b,self.r)['parallel_frontiers']
        self.assertEqual(self.b['metadata']['parallel_frontiers'],levels)
        self.assertEqual(self.b['metadata']['topological_order'],[t for x in levels for t in x])

    def test_required_amendment_docs_exist(self):
        for name in PRIMARY+['README.md','SOURCE_AMENDMENT_2026_09_05.md']:
            self.assertGreater((ROOT/name).stat().st_size,100,name)

    def test_no_task_lacks_positive_negative_fault_acceptance(self):
        for t in self.b['tasks']:
            for key in ['tests_first','negative_test','fault_test','acceptance_demo']:
                self.assertTrue(t[key],(t['id'],key))

    def test_all_task_requirement_references_resolve(self):
        tids={t['id'] for t in self.b['tasks']};rids={r['id'] for r in self.r['requirements']}
        for t in self.b['tasks']:
            self.assertLessEqual(set(t['depends_on']),tids)
            self.assertLessEqual(set(t['requirements']),rids)

    def test_new_sources_have_provenance(self):
        source=(ROOT/'SOURCE_AMENDMENT_2026_09_05.md').read_text()
        for q in self.r['requirements']:
            for ref in q['sources']:
                if ref.startswith('AMENDMENT-'): self.assertIn(ref,source)

    def test_execution_scope_is_not_product_validated(self):
        report=json.loads((ROOT/'plan_validation.json').read_text())
        self.assertIs(report['product_validated'],False)
        self.assertEqual(report['product_tests_executed'],0)

if __name__=='__main__':unittest.main(verbosity=2)
