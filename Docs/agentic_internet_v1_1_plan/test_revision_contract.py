"""Revision contract tests written before updating plan data/validator.
These check planning metadata and guardrails, not a messenger implementation.
"""
from __future__ import annotations
import copy
import json
from pathlib import Path
import unittest
from validate_plan import validate

ROOT = Path(__file__).resolve().parent

class RevisionContractTests(unittest.TestCase):
    def setUp(self):
        self.b = json.loads((ROOT / 'backlog.json').read_text())
        self.r = json.loads((ROOT / 'requirements.json').read_text())

    def contract(self):
        return self.b.get('metadata', {}).get('revision_contract', {})

    def invalid(self, fragment):
        report = validate(self.b, self.r)
        self.assertFalse(report['valid_plan_structure'])
        self.assertTrue(any(fragment in e for e in report['errors']), report['errors'])

    def set_policy(self, key, value):
        self.b.setdefault('metadata', {}).setdefault('revision_contract', {})[key] = value

    def test_tauri_is_required(self):
        self.assertEqual(self.contract().get('desktop'), 'tauri-2')

    def test_google_telegram_site_profiles_are_required(self):
        self.assertEqual(set(self.contract().get('auth_providers', [])), {'google', 'telegram', 'organization-oidc'})

    def test_single_issuer_is_explicitly_allowed(self):
        self.assertTrue(self.contract().get('single_issuer_profile_allowed', False))

    def test_reviews_belong_to_v1(self):
        self.assertEqual(self.contract().get('reviews_phase'), 'V1')

    def test_economic_agent_engine_belongs_to_v2(self):
        self.assertEqual(self.contract().get('economic_agent_engine_phase'), 'V2+')

    def test_review_right_does_not_depend_on_provider_completion_or_payment(self):
        self.assertEqual(self.contract().get('review_eligibility'), 'bilateral-accepted-order')
        for key in ('review_requires_provider_approval', 'review_requires_payment', 'review_requires_completion'):
            self.assertIs(self.contract().get(key), False)

    def test_executor_claim_is_not_independent_attestation(self):
        self.assertEqual(self.contract().get('v1_execution_provenance'), 'executor-declaration')

    def test_retired_ui_is_rejected(self):
        self.set_policy('desktop', 'egui')
        self.invalid('revision contract desktop')

    def test_missing_telegram_is_rejected(self):
        self.set_policy('auth_providers', ['google', 'organization-oidc'])
        self.invalid('revision contract auth_providers')

    def test_vendor_veto_is_rejected(self):
        self.set_policy('review_requires_provider_approval', True)
        self.invalid('revision contract review_requires_provider_approval')

    def test_payment_gate_is_rejected(self):
        self.set_policy('review_requires_payment', True)
        self.invalid('revision contract review_requires_payment')

    def test_completion_gate_is_rejected(self):
        self.set_policy('review_requires_completion', True)
        self.invalid('revision contract review_requires_completion')

    def test_reviews_cannot_be_moved_to_v2(self):
        self.set_policy('reviews_phase', 'V2+')
        self.invalid('revision contract reviews_phase')

    def test_economic_engine_cannot_silently_return_to_v1(self):
        self.set_policy('economic_agent_engine_phase', 'V1')
        self.invalid('revision contract economic_agent_engine_phase')

    def test_single_issuer_cannot_be_disallowed_again(self):
        self.set_policy('single_issuer_profile_allowed', False)
        self.invalid('revision contract single_issuer_profile_allowed')

    def test_new_requirement_cannot_be_removed(self):
        self.r['requirements'] = [x for x in self.r['requirements'] if x['id'] != 'R49']
        self.invalid('mandatory revision requirement R49')

    def test_amended_traceability_and_scope(self):
        tasks = {t['id']:t for t in self.b['tasks']}
        for tid in ['O07','O08','Q01','Q02','Q03','Q04','Q05','Q06','M07','U07','U08','X07']:
            self.assertIn(tid,tasks)
        self.assertEqual(tasks['A04'].get('delivery_scope'), 'V1-contract-only')
        self.assertEqual(len(tasks), 84)
        self.assertEqual(len(self.r['requirements']),52)

if __name__ == '__main__':
    unittest.main(verbosity=2)
