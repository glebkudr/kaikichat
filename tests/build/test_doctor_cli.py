"""Exercise doctor host selection through the actual storage wrapper CLI."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(sys.platform == 'linux', 'This contract requires an actual Linux host')
class DoctorCliTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for relative in ('scripts/build-storage.py', 'scripts/build_evidence.py'):
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, destination)
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.environment = dict(PATH=str(self.bin), HOME=str(self.root / 'home'))

    def run_wrapper(self, *arguments):
        return subprocess.run(
            [sys.executable, str(self.root / 'scripts/build-storage.py'), *arguments],
            cwd=self.root, env=self.environment, text=True, capture_output=True, timeout=10)

    def test_default_mac_doctor_on_linux_is_structured_unsupported_before_config_or_tools(self):
        config = self.root / '.local/build-storage.json'
        # Unsupported-host diagnosis must not depend on a usable Mac config or
        # invoke a tool. Malformed config proves it was not parsed on this host.
        for state in ('missing', 'malformed-config'):
            with self.subTest(config=state):
                if state == 'malformed-config':
                    config.parent.mkdir()
                    config.write_text('not valid JSON; must not be parsed on Linux')
                output = self.root / 'evidence' / state
                result = self.run_wrapper('--output', str(output), 'doctor',
                    '--suite', 'build-tooling')
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                report = json.loads(result.stdout)
                self.assertEqual(report['kind'], 'preflight')
                self.assertEqual(report['profile'], 'mac-apfs')
                self.assertEqual(report['suite'], 'build-tooling')
                self.assertEqual(report['status'], 'unsupported')
                self.assertIs(report['passed'], False)
                self.assertEqual(report['requirements'], [], 'Unsupported host probed tools')
                self.assertIn('host', report['reason'].lower())
                self.assertEqual(json.loads((output / 'check.json').read_text()), report)
                self.assertEqual(result.stderr, '')
                for relative in ('target', 'output', 'cache', '.local/verification-workspaces'):
                    self.assertFalse((self.root / relative).exists(),
                        f'Unsupported doctor mutated managed storage: {relative}')

    def test_default_check_on_linux_still_requires_mac_configuration(self):
        result = self.run_wrapper('check')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('Configure', result.stderr)
        self.assertIn('build-storage.json', result.stderr)
        self.assertEqual(result.stdout, '')
        for relative in ('target', 'output', 'cache', '.local'):
            self.assertFalse((self.root / relative).exists())


if __name__ == '__main__':
    unittest.main()
