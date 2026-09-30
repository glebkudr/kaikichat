"""Runtime IPC capability, failure attribution and cleanup through doctor/CLI."""
import errno
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import build_evidence

spec = importlib.util.spec_from_file_location('unix_ipc_sockets',
    Path(__file__).parent / 'fixtures/unix_ipc_sockets.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class UnixIpcPreflightTests(unittest.TestCase):
    def probe(self, failure=None, error_number=errno.EPERM):
        sockets = fixture.Sockets(failure, error_number)
        directories, create_directory = [], tempfile.TemporaryDirectory

        def temporary_directory(*args, **kwargs):
            directory = create_directory(*args, **kwargs)
            directories.append(Path(directory.name))
            return directory

        with mock.patch.object(build_evidence.socket, 'socket', side_effect=sockets.socket), \
                mock.patch.object(build_evidence.tempfile, 'TemporaryDirectory',
                    side_effect=temporary_directory):
            report = build_evidence._unix_ipc_probe()
        audit = sockets.audit()
        self.assertEqual(audit['closed'], audit['opened'], audit)
        self.assertEqual(audit['pathsRemaining'], [])
        self.assertEqual(audit['directoriesRemaining'], [])
        self.assertTrue(all(not directory.exists() for directory in directories), directories)
        self.assertEqual(report['cleanup']['status'], 'passed')
        self.assertEqual(report['cleanup']['socketsClosed'], audit['opened'])
        self.assertTrue(report['cleanup']['temporaryDirectoryRemoved'])
        self.assertTrue(all(0 < timeout <= 1 for timeout in audit['timeouts']), audit)
        return report, audit

    def test_allowed_socket_lifecycle_is_required_and_cleaned(self):
        report, audit = self.probe()
        self.assertEqual(report['name'], 'unix-ipc')
        self.assertEqual(report['status'], 'passed')
        self.assertEqual(report['operation'], 'accept')
        self.assertIsNone(report['errno'])
        self.assertEqual([item['operation'] for item in report['operations']],
            ['socket', 'temporary_directory', 'bind', 'listen', 'socket', 'connect', 'accept'])
        self.assertTrue(all(item['status'] == 'passed' for item in report['operations']))
        self.assertEqual(audit['opened'], 3)
        self.assertEqual(audit['operations'][:6],
            ['listener:socket', 'listener:bind', 'listener:listen',
             'client:socket', 'client:connect', 'listener:accept'])

    def test_eperm_at_each_socket_boundary_blocks_with_exact_operation_and_cleanup(self):
        for endpoint, operation in (('listener', 'socket'), ('listener', 'bind'),
                ('listener', 'listen'), ('client', 'socket'), ('client', 'connect'),
                ('listener', 'accept')):
            with self.subTest(endpoint=endpoint, operation=operation):
                report, _ = self.probe(endpoint + ':' + operation)
                self.assertEqual(report['status'], 'blocked')
                self.assertEqual(report['operation'], operation)
                self.assertEqual(report['endpoint'], endpoint)
                self.assertEqual(report['errno'], errno.EPERM)
                self.assertEqual(report['errnoName'], 'EPERM')
                self.assertEqual(report['operations'][-1]['status'], 'blocked')
                self.assertEqual(report['operations'][-1]['errno'], errno.EPERM)

    def test_cleanup_refusal_cannot_report_success_and_closes_remaining_sockets(self):
        sockets = fixture.Sockets('accepted:close')
        with mock.patch.object(build_evidence.socket, 'socket', side_effect=sockets.socket):
            report = build_evidence._unix_ipc_probe()
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['operation'], 'cleanup')
        self.assertEqual(report['errno'], errno.EPERM)
        self.assertEqual(report['cleanup']['status'], 'failed')
        self.assertEqual(report['cleanup']['errors'][0]['operation'], 'close')
        self.assertTrue(report['cleanup']['temporaryDirectoryRemoved'])
        self.assertEqual(sockets.audit()['closed'], 3)
        self.assertEqual(sockets.audit()['directoriesRemaining'], [])

    def test_connect_and_accept_timeouts_are_bounded_blockers_with_cleanup(self):
        for identity in ('client:connect', 'listener:accept'):
            with self.subTest(operation=identity):
                report, _ = self.probe(identity, None)
                self.assertEqual(report['status'], 'blocked')
                self.assertIsNone(report['errno'])
                self.assertEqual(report['errorType'], 'TimeoutError')
                self.assertEqual(report['operation'], identity.split(':')[1])

    def doctor(self, suite, phase):
        with mock.patch.object(build_evidence.sys, 'platform', 'linux'), \
                mock.patch.object(build_evidence.shutil, 'which', return_value='/fixture/tool'), \
                mock.patch.object(build_evidence.subprocess, 'run', return_value=
                    subprocess.CompletedProcess([], 0, 'fixture 100.0.0', '')):
            return build_evidence.doctor(ROOT, suite, profile='portable-linux', phase=phase, env={})

    def test_runtime_requirement_blocks_doctor_without_requiring_build_tools(self):
        sockets = fixture.Sockets('listener:socket')
        with mock.patch.object(build_evidence.socket, 'socket', side_effect=sockets.socket):
            report = self.doctor('native-runtime', 'runtime')
        self.assertEqual(report['status'], 'blocked')
        self.assertFalse(report['passed'])
        self.assertEqual([item['name'] for item in report['requirements']],
            ['python3', 'git', 'unix-ipc'])
        self.assertEqual(report['requirements'][-1]['errno'], errno.EPERM)

    def test_build_and_other_suites_do_not_probe_runtime_ipc(self):
        with mock.patch.object(build_evidence, '_unix_ipc_probe',
                side_effect=AssertionError('unselected runtime capability was probed')):
            for suite, phase in (('backend-unit', 'runtime'), ('native-build', 'runtime'),
                    ('build-tooling', 'runtime'), ('evm', 'runtime')):
                with self.subTest(suite=suite, phase=phase):
                    report = self.doctor(suite, phase)
                    self.assertTrue(report['passed'], report)
                    self.assertNotIn('unix-ipc', [item['name'] for item in report['requirements']])
            # The runtime suite has no build phase and the removed native-spend
            # suite has no fallback: neither probes tools or IPC.
            for suite, phase in (('native-runtime', 'build'), ('native-spend', 'runtime')):
                with self.subTest(suite=suite, phase=phase):
                    report = self.doctor(suite, phase)
                    self.assertEqual(report['status'], 'unsupported', report)
                    self.assertEqual(report['requirements'], [])


@unittest.skipUnless(sys.platform == 'linux', 'portable-linux wrapper requires Linux')
class UnixIpcCliTests(unittest.TestCase):
    def test_actual_wrapper_reports_allowed_and_eperm_and_removes_probe_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            scripts = root / 'scripts'
            scripts.mkdir()
            for name in ('build-storage.py', 'build_evidence.py'):
                shutil.copyfile(ROOT / 'scripts' / name, scripts / name)
            binary_dir = root / 'bin'
            binary_dir.mkdir()
            for name in ('python3', 'git'):
                path = binary_dir / name
                path.write_text(f'#!{sys.executable}\nimport sys\n'
                    'assert sys.argv[1:] == ["--version"], "doctor attempted execution"\n'
                    'print("fixture 100.0.0")\n')
                path.chmod(0o755)
            driver = root / 'driver.py'
            driver.write_text('import json, runpy, socket, sys\nfrom pathlib import Path\n'
                f'sys.path.insert(0, {str(Path(__file__).parent / "fixtures")!r})\n'
                'from unix_ipc_sockets import Sockets\n'
                'mode, audit_path, wrapper, *arguments = sys.argv[1:]\n'
                'fixture = Sockets(None if mode == "allowed" else "listener:socket")\n'
                'socket.socket = fixture.socket\n'
                'sys.path.insert(0, str(Path(wrapper).parent))\n'
                'sys.argv = [wrapper, *arguments]\n'
                'try:\n    runpy.run_path(wrapper, run_name="__main__")\n'
                'finally:\n    Path(audit_path).write_text(json.dumps(fixture.audit()))\n')
            for mode, expected, code in (('allowed', 'passed', 0), ('eperm', 'blocked', 2)):
                with self.subTest(mode=mode):
                    output, audit = root / mode, root / (mode + '-sockets.json')
                    result = subprocess.run([sys.executable, str(driver), mode, str(audit),
                        str(scripts / 'build-storage.py'), '--profile', 'portable-linux',
                        '--output', str(output), 'doctor', '--suite', 'native-runtime',
                        '--phase', 'runtime'], cwd=root, env=dict(PATH=str(binary_dir)),
                        text=True, capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, code, result.stdout + result.stderr)
                    report = json.loads(result.stdout)
                    self.assertEqual(report['status'], expected)
                    self.assertEqual(report['passed'], code == 0)
                    self.assertEqual(json.loads((output / 'check.json').read_text()), report)
                    capability = report['requirements'][-1]
                    self.assertEqual(capability['name'], 'unix-ipc')
                    self.assertEqual(capability['status'], expected)
                    self.assertEqual(capability['errno'], None if code == 0 else errno.EPERM)
                    lifecycle = json.loads(audit.read_text())
                    self.assertEqual(lifecycle['opened'], lifecycle['closed'])
                    self.assertEqual(lifecycle['pathsRemaining'], [])
                    self.assertEqual(lifecycle['directoriesRemaining'], [])
                    self.assertEqual(result.stderr, '')


if __name__ == '__main__':
    unittest.main()
