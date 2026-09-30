"""Tests for the plan's structure only; these do not test the messenger."""
from __future__ import annotations
import copy
import json
from pathlib import Path
import unittest
from validate_plan import validate

ROOT = Path(__file__).resolve().parent

class PlanValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.backlog = json.loads((ROOT / 'backlog.json').read_text(encoding='utf-8'))
        self.requirements = json.loads((ROOT / 'requirements.json').read_text(encoding='utf-8'))

    def test_complete_plan_is_valid(self) -> None:
        result = validate(self.backlog, self.requirements)
        self.assertTrue(result['valid_plan_structure'])
        self.assertEqual(result['task_count'], 84)
        self.assertEqual(result['requirement_count'], 52)
        self.assertFalse(result['product_validated'])

    def test_missing_requirement_coverage_is_rejected(self) -> None:
        for task in self.backlog['tasks']:
            task['requirements'] = [r for r in task['requirements'] if r != 'R10']
        self.assertInvalid('uncovered requirement R10')

    def test_dependency_cycle_is_rejected(self) -> None:
        self.backlog['tasks'][0]['depends_on'] = ['X06']
        self.assertInvalid('dependency cycle')

    def test_unknown_dependency_is_rejected(self) -> None:
        self.backlog['tasks'][0]['depends_on'] = ['MISSING']
        self.assertInvalid('unknown dependency')

    def test_unknown_requirement_is_rejected(self) -> None:
        self.backlog['tasks'][0]['requirements'].append('R99')
        self.assertInvalid('unknown requirement')

    def test_missing_adversarial_test_is_rejected(self) -> None:
        self.backlog['tasks'][0]['negative_test'] = ''
        self.assertInvalid('negative_test')

    def test_missing_fault_test_is_rejected(self) -> None:
        self.backlog['tasks'][0]['fault_test'] = ''
        self.assertInvalid('fault_test')

    def test_duplicate_task_id_is_rejected(self) -> None:
        self.backlog['tasks'].append(copy.deepcopy(self.backlog['tasks'][0]))
        self.assertInvalid('duplicate task')

    def test_wrong_reverse_mapping_is_rejected(self) -> None:
        self.requirements['requirements'][0]['planned_tasks'] = []
        self.assertInvalid('reverse mapping')

    def test_status_cannot_imply_product_completion(self) -> None:
        self.backlog['tasks'][0]['status'] = 'implemented'
        self.assertInvalid('unsupported status')

    def test_no_fake_product_assurance(self) -> None:
        report = validate(self.backlog, self.requirements)
        self.assertEqual(report['validation_scope'], 'plan structure and coverage only')
        self.assertEqual(report['product_tests_executed'], 0)

    def assertInvalid(self, fragment: str) -> None:
        result = validate(self.backlog, self.requirements)
        self.assertFalse(result['valid_plan_structure'])
        self.assertTrue(any(fragment in e for e in result['errors']), result['errors'])

if __name__ == '__main__':
    unittest.main(verbosity=2)
