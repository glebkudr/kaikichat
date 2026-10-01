#!/usr/bin/env python3
"""Regressions for build provenance and command evidence, using no build tools.

Run through scripts/build-storage.py. Temporary repositories and child processes
are fixtures; these checks never compile the application or change its deadlines.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
import build_evidence as evidence


class CommandEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.output = evidence.new_run_dir(self.root / 'runs')
        self.report = dict(passed=False, commands=[])

    def run_command(self, program, timeout=5):
        return evidence.run_command('fixture', program, self.report,
            output=self.output, cwd=self.root, timeout=timeout)

    def saved_command(self):
        saved = json.loads((self.output / 'check.json').read_text())
        self.assertFalse(saved['passed'])
        self.assertEqual(saved['commands'], self.report['commands'])
        self.assertEqual(len(saved['commands']), 1)
        entry = saved['commands'][0]
        self.assertEqual(entry['name'], 'fixture')
        self.assertGreaterEqual(entry['elapsedSeconds'], 0)
        self.assertEqual(entry['cwd'], str(self.root))
        return entry

    def test_each_attempt_preserves_the_previous_report_and_log(self):
        self.run_command([sys.executable, '-c', 'print("first attempt")'])
        first = self.output
        original = (first / 'check.json').read_bytes()
        second = evidence.new_run_dir(self.root / 'runs')
        self.assertNotEqual(first, second)
        self.assertTrue(second.is_dir())
        self.assertEqual((first / 'check.json').read_bytes(), original)
        self.assertIn('first attempt', (first / 'fixture.log').read_text())

    def test_nonzero_exit_retains_output_and_error_in_report(self):
        args = [sys.executable, '-c', 'print("compiler diagnostic", flush=True); raise SystemExit(7)']
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_command(args)
        entry = self.saved_command()
        self.assertEqual(entry['status'], 'failed')
        self.assertEqual(entry['args'], args)
        self.assertEqual(entry['exitCode'], 7)
        self.assertIn('compiler diagnostic', Path(entry['logPath']).read_text())

    def test_spawn_error_is_a_recorded_command_not_an_empty_report(self):
        with self.assertRaises(OSError):
            self.run_command([str(self.root / 'missing-compiler')])
        entry = self.saved_command()
        self.assertEqual(entry['status'], 'spawn_error')
        self.assertIsNone(entry['exitCode'])
        self.assertIn('FileNotFoundError', entry['error'])

    def test_child_observes_its_running_command_already_persisted(self):
        program = (
            'import json, pathlib\n'
            f'report = json.loads(pathlib.Path({str(self.output / "check.json")!r}).read_text())\n'
            'assert len(report["commands"]) == 1\n'
            'assert report["commands"][0]["status"] == "running"\n'
            'assert report["commands"][0]["name"] == "fixture"\n'
            'assert report["passed"] is False\n'
            'print("running evidence persisted before child executed")\n'
        )
        output = self.run_command([sys.executable, '-c', program])
        self.assertIn('running evidence persisted', output)

    @unittest.skipUnless(os.name == 'posix', 'owned POSIX build process groups')
    def test_timeout_kills_a_term_resistant_child_and_preserves_partial_log(self):
        # The child survives a plain parent kill and ignores SIGTERM. Its output
        # pipe remains open until the runner kills its own complete process group.
        child_program = (
            'import os, pathlib, signal, time\n'
            'signal.signal(signal.SIGTERM, signal.SIG_IGN)\n'
            'pathlib.Path("child.pid").write_text(str(os.getpid()))\n'
            'while True:\n'
            ' pathlib.Path("heartbeat").write_text(str(time.monotonic_ns()))\n'
            ' time.sleep(0.02)\n'
        )
        parent_program = (
            'import os, pathlib, subprocess, sys, time\n'
            'pathlib.Path("command.pid").write_text(str(os.getpid()))\n'
            f'child = subprocess.Popen([sys.executable, "-c", {child_program!r}])\n'
            'while not pathlib.Path("child.pid").exists(): time.sleep(0.01)\n'
            'print("fixture-child-ready", flush=True)\n'
            'child.wait()\n'
        )
        # Keep an independent watchdog outside the implementation under test.
        # A buggy runner stuck draining an inherited pipe must not hang the suite
        # before its elapsed assertion or fixture cleanup can execute.
        driver_program = (
            'import pathlib, subprocess, sys\n'
            f'sys.path.insert(0, {str(Path(evidence.__file__).parent)!r})\n'
            'import build_evidence\n'
            'report = dict(passed=False, commands=[])\n'
            'try:\n'
            f' build_evidence.run_command("fixture", {[sys.executable, "-c", parent_program]!r}, report, '
            f'output=pathlib.Path({str(self.output)!r}), cwd=pathlib.Path({str(self.root)!r}), timeout=2)\n'
            'except subprocess.TimeoutExpired:\n'
            ' raise SystemExit(0)\n'
            'raise SystemExit("fixture did not time out")\n'
        )
        started = time.monotonic()
        try:
            with (self.root / 'supervisor.log').open('w') as log:
                result = subprocess.run([sys.executable, '-c', driver_program],
                    cwd=self.root, stdout=log, stderr=subprocess.STDOUT, timeout=12)
            self.assertEqual(result.returncode, 0, (self.root / 'supervisor.log').read_text())
            self.assertLess(time.monotonic() - started, 10,
                'timeout cleanup waited for a descendant indefinitely')
            self.report = json.loads((self.output / 'check.json').read_text())
            entry = self.saved_command()
            self.assertEqual(entry['status'], 'timed_out')
            self.assertEqual(entry['timeoutSeconds'], 2)
            self.assertTrue(entry['cleanup']['processGroup'])
            self.assertIn('SIGKILL', entry['cleanup']['signals'])
            self.assertIn('fixture-child-ready', Path(entry['logPath']).read_text())
            self.assertTrue((self.root / 'child.pid').is_file(), 'child never started')
            first = (self.root / 'heartbeat').read_bytes()
            time.sleep(0.12)
            self.assertEqual((self.root / 'heartbeat').read_bytes(), first,
                'timed-out command left its compiler child running')
        finally:
            # Fixture cleanup must also be safe against the buggy implementation.
            for name in ('child.pid', 'command.pid'):
                pid_file = self.root / name
                if pid_file.exists():
                    try:
                        os.kill(int(pid_file.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass


class SealedBuildTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'build-test@example.invalid')
        self.git('config', 'user.name', 'Build fixture')
        for relative, content in {
            '.gitignore': '/target\n/output\n',
            'Cargo.toml': '[workspace]\n',
            'Cargo.lock': '# locked dependency graph\n',
            'apps/desktop/package-lock.json': '{"lockfileVersion":3}\n',
            'crates/node/src/main.rs': 'fn main() {}\n',
        }.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture inputs')
        self.configuration = dict(kind='desktop', profile='release', features=[], env={},
            tools=dict(rustc='rustc fixture 1', cargo='cargo fixture 1'))
        self.artifacts = {'bundle': self.root / 'target/release/bundle/macos/Kaiki Chat.app'}
        bundle = self.artifacts['bundle'] / 'Contents/MacOS'
        bundle.mkdir(parents=True)
        for name in ('agentic-desktop', 'kaiki-agentic-node', 'agentic-mcp', 'agentic-cli'):
            binary = bundle / name
            binary.write_bytes(('built ' + name).encode())
            binary.chmod(0o755)
        self.manifest = self.root / 'output/prepared-artifacts.json'

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.root, text=True).strip()

    def test_sealed_manifest_records_source_locks_configuration_and_artifacts(self):
        draft = evidence.begin_build(self.root, self.configuration, self.artifacts)
        sealed = evidence.seal_build(self.root, draft, self.manifest)
        self.assertEqual(json.loads(self.manifest.read_text()), sealed)
        self.assertEqual(sealed['status'], 'prepared')
        self.assertEqual(sealed['configuration'], self.configuration)
        self.assertEqual(sealed['source']['head'], self.git('rev-parse', 'HEAD'))
        self.assertIn('Cargo.lock', sealed['source']['files'])
        self.assertIn('apps/desktop/package-lock.json', sealed['source']['files'])
        self.assertEqual(set(sealed['artifacts']), set(self.artifacts))
        self.assertRegex(sealed['artifacts']['bundle']['sha256'], '^[0-9a-f]{64}$')
        with self.assertRaisesRegex(ValueError, 'status'):
            evidence.seal_build(self.root, sealed, self.manifest)

    def test_source_lock_or_ref_change_during_build_cannot_be_sealed(self):
        changes = {
            'tracked source': lambda: (self.root / 'crates/node/src/main.rs').write_text('fn main() { panic!(); }\n'),
            'new untracked source': lambda: (self.root / 'crates/node/src/new_module.rs').write_text('pub fn changed() {}\n'),
            'lockfile content': lambda: (self.root / 'Cargo.lock').write_text('# changed dependency graph\n'),
            'lockfile deletion': lambda: (self.root / 'apps/desktop/package-lock.json').unlink(),
            'ref with identical files': lambda: self.git('commit', '--allow-empty', '-qm', 'different review ref'),
        }
        # Each build snapshots the state left by the previous change.
        for name, change in changes.items():
            with self.subTest(change=name):
                draft = evidence.begin_build(self.root, self.configuration, self.artifacts)
                change()
                with self.assertRaisesRegex(ValueError, 'source'):
                    evidence.seal_build(self.root, draft, self.manifest)
                self.assertFalse(self.manifest.exists())


if __name__ == '__main__':
    unittest.main()
