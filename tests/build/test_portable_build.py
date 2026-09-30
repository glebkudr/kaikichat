"""Portable storage, desktop build identity and doctor contracts; no build tools."""
import importlib.util
import contextlib
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import build_evidence

spec = importlib.util.spec_from_file_location('build_storage', ROOT / 'scripts/build-storage.py')
build_storage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build_storage)


class PortableBuildTests(unittest.TestCase):
    def test_explicit_portable_run_needs_no_mac_config_but_default_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'target').mkdir()
            (root / 'output').mkdir()
            config = root / '.local/build-storage.json'
            command = [sys.executable, '-c', 'print("portable fixture")']
            with mock.patch.multiple(build_storage, ROOT=root, CONFIG=config), \
                    mock.patch.dict(os.environ, {'PATH': str(root / 'bin')}, clear=True), \
                    mock.patch.object(build_storage.sys, 'platform', 'linux'), \
                    mock.patch.object(build_storage, 'mount_image') as mount, \
                    mock.patch.object(build_storage, 'plist') as plist, \
                    mock.patch.object(build_storage.os, 'execvpe') as execute:
                build_storage.main(['--profile', 'portable-linux', 'run', *command])
                execute.assert_called_once()
                executable, arguments, environment = execute.call_args.args
                self.assertEqual((executable, arguments), (command[0], command))
                self.assertEqual(Path(environment['CARGO_TARGET_DIR']).resolve(), (root / 'target').resolve())
                mount.assert_not_called()
                plist.assert_not_called()
                self.assertFalse(config.exists())
                execute.reset_mock()
                with mock.patch.object(build_storage.sys, 'platform', 'darwin'):
                    with self.assertRaisesRegex(RuntimeError, 'config|Config|storage'):
                        build_storage.main(['run', *command])
                    with self.assertRaises((RuntimeError, ValueError, SystemExit)):
                        build_storage.main(['--profile', 'portable-linux', 'run', *command])
                with self.assertRaises((RuntimeError, ValueError, SystemExit)):
                    build_storage.main(['--profile', 'unknown', 'run', *command])
                (root / 'target').rmdir()
                (root / 'target').symlink_to(root / 'unavailable-volume/target')
                with self.assertRaises((RuntimeError, OSError, ValueError)):
                    build_storage.main(['--profile', 'portable-linux', 'run', *command])
                self.assertTrue((root / 'target').is_symlink())
                execute.assert_not_called()

    def test_portable_run_rejects_external_storage_links_and_cross_target(self):
        cases = ['target', 'output', 'cache', '.local/verification-workspaces',
            'CARGO_BUILD_TARGET', 'CARGO_BUILD_TARGET_DIR']
        for case in cases:
            with self.subTest(storage=case), tempfile.TemporaryDirectory() as temporary:
                root, external = Path(temporary) / 'checkout', Path(temporary) / 'external'
                root.mkdir()
                external.mkdir()
                marker = external / 'keep.txt'
                marker.write_text('caller-owned storage')
                env = {'PATH': str(root / 'bin')}
                link = None
                if case == 'CARGO_BUILD_TARGET':
                    env[case] = 'aarch64-unknown-linux-gnu'
                elif case == 'CARGO_BUILD_TARGET_DIR':
                    env[case] = str(external)
                else:
                    link = root / case
                    link.parent.mkdir(parents=True, exist_ok=True)
                    link.symlink_to(external, target_is_directory=True)
                with mock.patch.multiple(build_storage, ROOT=root, CONFIG=root / '.local/build-storage.json'), \
                        mock.patch.dict(os.environ, env, clear=True), \
                        mock.patch.object(build_storage.sys, 'platform', 'linux'), \
                        mock.patch.object(build_storage.os, 'execvpe') as execute:
                    try:
                        with self.assertRaises((RuntimeError, ValueError)):
                            build_storage.main(['--profile', 'portable-linux', 'run', sys.executable, '-c', 'pass'])
                    finally:
                        if link is not None:
                            self.assertTrue(link.is_symlink(), 'rejection replaced caller-owned storage')
                            self.assertEqual(link.resolve(), external.resolve())
                        self.assertEqual(marker.read_text(), 'caller-owned storage')
                        self.assertEqual(list(external.iterdir()), [marker], 'wrapper wrote outside checkout')
                    execute.assert_not_called()

    def test_mac_default_verifies_volume_identity_image_ownership_and_managed_links(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            volume, image = root / 'volume', root / 'external/build.sparsebundle'
            volume.mkdir()
            image.mkdir(parents=True)
            config_path = root / '.local/build-storage.json'
            config_path.parent.mkdir()
            config = dict(mount=str(volume), image=str(image), volume_uuid='fixture-apfs-uuid')
            config_path.write_text(json.dumps(config))
            for relative, destination in build_storage.LINKS.items():
                target, link = volume / destination, root / relative
                target.mkdir(parents=True, exist_ok=True)
                link.parent.mkdir(parents=True, exist_ok=True)
                link.symlink_to(target, target_is_directory=True)
            disk = dict(FilesystemType='apfs', VolumeUUID=config['volume_uuid'])
            images = {'images': [{'image-path': str(image),
                'system-entities': [{'mount-point': str(volume)}]}]}

            def utility(arguments, **options):
                name = Path(arguments[0]).name
                self.assertIn(name, ('diskutil', 'hdiutil'))
                self.assertEqual(arguments[1], 'info', 'check unexpectedly mounted/detached an image')
                return subprocess.CompletedProcess(arguments, 0,
                    stdout=plistlib.dumps(disk if name == 'diskutil' else images), stderr=b'')

            with mock.patch.multiple(build_storage, ROOT=root, CONFIG=config_path), \
                    mock.patch.object(build_storage.sys, 'platform', 'darwin'), \
                    mock.patch.object(build_storage.os.path, 'ismount', return_value=True) as mounted, \
                    mock.patch.object(build_storage.subprocess, 'run', side_effect=utility) as run, \
                    mock.patch.object(build_storage.os, 'execvpe') as execute:
                build_storage.main(['check'])
                self.assertEqual({Path(call.args[0][0]).name for call in run.call_args_list},
                    {'diskutil', 'hdiutil'})
                for key, bad in [('FilesystemType', 'hfs'), ('VolumeUUID', 'another-volume')]:
                    with self.subTest(identity=key):
                        original = disk[key]
                        disk[key] = bad
                        with self.assertRaises(RuntimeError):
                            build_storage.main(['run', sys.executable, '-c', 'pass'])
                        disk[key] = original
                images['images'][0]['image-path'] = str(root / 'unowned.sparsebundle')
                with self.assertRaises(RuntimeError):
                    build_storage.main(['check'])
                images['images'][0]['image-path'] = str(image)
                link = root / 'target'
                link.unlink()
                link.mkdir()
                with self.assertRaises(RuntimeError):
                    build_storage.main(['check'])
                self.assertTrue(link.is_dir())
                self.assertFalse(link.is_symlink(), 'failure replaced a caller-owned directory')
                link.rmdir()
                link.symlink_to(volume / build_storage.LINKS['target'])
                for unavailable in ('diskutil', 'hdiutil'):
                    def missing(arguments, **options):
                        if Path(arguments[0]).name == unavailable:
                            raise FileNotFoundError(unavailable)
                        return utility(arguments, **options)
                    with self.subTest(utility=unavailable), \
                            mock.patch.object(build_storage.subprocess, 'run', side_effect=missing), \
                            self.assertRaises((RuntimeError, OSError)):
                        build_storage.main(['check'])
                mounted.return_value = False
                with self.assertRaises(RuntimeError):
                    build_storage.main(['check'])
                config['image'] = str(root / 'disconnected/build.sparsebundle')
                config_path.write_text(json.dumps(config))
                with self.assertRaises(RuntimeError):
                    build_storage.main(['run', sys.executable, '-c', 'pass'])
                config_path.unlink()
                with self.assertRaises(RuntimeError):
                    build_storage.main(['check'])
                execute.assert_not_called()

    def test_mac_checkout_with_a_storage_directory_uses_only_its_own_part_of_the_volume(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, volume = Path(temporary) / 'checkout', Path(temporary) / 'volume'
            image = Path(temporary) / 'external/build.sparsebundle'
            image.mkdir(parents=True)
            config_path = root / '.local/build-storage.json'
            config_path.parent.mkdir(parents=True)
            config = dict(mount=str(volume), image=str(image), volume_uuid='fixture-apfs-uuid',
                directory='second')
            config_path.write_text(json.dumps(config))
            for destination in build_storage.LINKS.values():
                # Another checkout's directories at the volume's root.
                (volume / destination).mkdir(parents=True, exist_ok=True)
            disk = dict(FilesystemType='apfs', VolumeUUID=config['volume_uuid'])
            images = {'images': [{'image-path': str(image),
                'system-entities': [{'mount-point': str(volume)}]}]}

            def utility(arguments, **options):
                name = Path(arguments[0]).name
                self.assertIn(arguments[1], ('info', 'attach'))
                return subprocess.CompletedProcess(arguments, 0,
                    stdout=plistlib.dumps(disk if name == 'diskutil' else images), stderr=b'')

            with mock.patch.multiple(build_storage, ROOT=root, CONFIG=config_path), \
                    mock.patch.object(build_storage.sys, 'platform', 'darwin'), \
                    mock.patch.object(build_storage.os.path, 'ismount', return_value=True), \
                    mock.patch.object(build_storage.subprocess, 'run', side_effect=utility):
                build_storage.main(['setup'])
                for relative, destination in build_storage.LINKS.items():
                    self.assertEqual((root / relative).resolve(),
                        (volume / 'second' / destination).resolve())
                build_storage.main(['check'])
                self.assertEqual(build_storage.environment(config)['npm_config_cache'],
                    str(volume / 'second/npm/cache'))
                link = root / 'target'
                link.unlink()
                link.symlink_to(volume / build_storage.LINKS['target'], target_is_directory=True)
                with self.assertRaises(RuntimeError, msg="accepted the other checkout's target"):
                    build_storage.main(['check'])
                for escape in ('..', '../elsewhere', str(Path(temporary) / 'elsewhere')):
                    with self.subTest(directory=escape):
                        config_path.write_text(json.dumps(dict(config, directory=escape)))
                        with self.assertRaises(RuntimeError):
                            build_storage.main(['check'])

    @contextlib.contextmanager
    def managed_mac_checkout(self):
        """Healthy mac-apfs fixture: a checkout whose managed links enter a mounted volume."""
        with tempfile.TemporaryDirectory() as temporary:
            root, volume = Path(temporary) / 'checkout', Path(temporary) / 'volume'
            image = Path(temporary) / 'external/build.sparsebundle'
            image.mkdir(parents=True)
            (root / '.git').mkdir(parents=True)
            config_path = root / '.local/build-storage.json'
            config_path.parent.mkdir()
            config = dict(mount=str(volume), image=str(image), volume_uuid='fixture-apfs-uuid')
            config_path.write_text(json.dumps(config))
            for relative, destination in build_storage.LINKS.items():
                target, link = volume / destination, root / relative
                target.mkdir(parents=True, exist_ok=True)
                link.parent.mkdir(parents=True, exist_ok=True)
                link.symlink_to(target, target_is_directory=True)
            disk = dict(FilesystemType='apfs', VolumeUUID=config['volume_uuid'])
            images = {'images': [{'image-path': str(image),
                'system-entities': [{'mount-point': str(volume)}]}]}

            def utility(arguments, **options):
                self.assertEqual(arguments[1], 'info', 'worktree run mounted/detached an image')
                return subprocess.CompletedProcess(arguments, 0, stdout=plistlib.dumps(
                    disk if Path(arguments[0]).name == 'diskutil' else images), stderr=b'')

            caller = Path.cwd()
            with mock.patch.multiple(build_storage, ROOT=root, CONFIG=config_path), \
                    mock.patch.object(build_storage.sys, 'platform', 'darwin'), \
                    mock.patch.object(build_storage.os.path, 'ismount', return_value=True), \
                    mock.patch.object(build_storage.subprocess, 'run', side_effect=utility) as run, \
                    mock.patch.object(build_storage.os, 'execvpe') as execute:
                try:
                    yield root, volume, disk, run, execute
                finally:
                    os.chdir(caller)

    def test_mac_run_in_named_verification_worktree_uses_its_sources_and_target(self):
        with self.managed_mac_checkout() as (root, volume, disk, run, execute):
            worktree = (volume / 'verification-workspaces/feature').resolve()
            (worktree / 'crates').mkdir(parents=True)
            (worktree / '.git').write_text('gitdir: fixture\n')
            command = [sys.executable, '-c', 'pass']
            seen = []
            execute.side_effect = lambda *arguments: seen.append(Path.cwd())
            caller = Path.cwd()
            # The path may name the managed link from the main checkout, or be
            # relative to the caller's own directory inside the worktree.
            for label, path, start in [
                    ('managed link', root / '.local/verification-workspaces/feature', caller),
                    ('relative', Path('.'), worktree),
                    ('relative from checkout', Path('.local/verification-workspaces/feature'), root)]:
                with self.subTest(path=label):
                    os.chdir(start)
                    execute.reset_mock()
                    seen.clear()
                    build_storage.main(['--worktree', str(path), 'run', *command])
                    executable, arguments, environment = execute.call_args.args
                    self.assertEqual((executable, arguments), (command[0], command))
                    self.assertEqual(seen, [worktree], 'command did not start in the worktree')
                    self.assertEqual(Path(environment['CARGO_TARGET_DIR']), worktree / 'target')
                    self.assertEqual(environment['AIN_BUILD_STORAGE_PROFILE'], 'mac-apfs')
                    self.assertEqual(Path.cwd(), start.resolve(), 'failed exec did not restore cwd')
            # Without --worktree the main checkout remains the only run directory.
            os.chdir(caller)
            execute.reset_mock()
            seen.clear()
            build_storage.main(['run', *command])
            self.assertEqual(seen, [root.resolve()])
            self.assertEqual(Path(execute.call_args.args[2]['CARGO_TARGET_DIR']), root / 'target')
            self.assertFalse((worktree / 'target').exists(), 'the wrapper created a worktree target')

            # Durable evidence records the same worktree cwd and target.
            output = root / 'output/worktree-evidence'
            probe = 'import os; print(os.getcwd()); print(os.environ["CARGO_TARGET_DIR"])'
            self.assertEqual(build_storage.main(['--output', str(output), '--worktree', str(worktree),
                'run', sys.executable, '-c', probe]), 0)
            report = json.loads((output / 'check.json').read_text())
            self.assertEqual(report['status'], 'passed')
            self.assertEqual(Path(report['commands'][0]['cwd']), worktree)
            printed = (output / 'command.log').read_text().splitlines()
            self.assertEqual([Path(line).resolve() for line in printed],
                [worktree, (worktree / 'target').resolve()])
            self.assertEqual(execute.call_count, 1, 'evidence run replaced the wrapper process')

    def test_mac_worktree_run_fails_closed_before_executing(self):
        with self.managed_mac_checkout() as (root, volume, disk, run, execute):
            workspaces = volume / 'verification-workspaces'
            worktree = workspaces / 'feature'
            (worktree / 'crates').mkdir(parents=True)
            (worktree / '.git').write_text('gitdir: fixture\n')
            # Every rejected path below except the main checkout would be a
            # checkout root if containment were not enforced.
            (workspaces / '.git').mkdir()
            elsewhere = root.parent / 'elsewhere'
            (elsewhere / '.git').mkdir(parents=True)
            sibling = volume / 'verification-workspaces-other/feature'
            (sibling / '.git').mkdir(parents=True)
            (workspaces / 'plain').mkdir()
            (workspaces / 'notes.txt').write_text('not a checkout')
            (workspaces / 'escape').symlink_to(elsewhere, target_is_directory=True)
            shared = workspaces / 'shared'
            (shared / '.git').mkdir(parents=True)
            (shared / 'target').symlink_to(volume / 'rust-target', target_is_directory=True)
            (workspaces / 'donor/target').mkdir(parents=True)
            (workspaces / 'donor/.git').mkdir()
            borrower = workspaces / 'borrower'
            (borrower / '.git').mkdir(parents=True)
            (borrower / 'target').symlink_to(workspaces / 'donor/target', target_is_directory=True)
            command = ['run', sys.executable, '-c', 'pass']
            managed = root / '.local/verification-workspaces'

            # Other actions and profiles reject the option before any storage action.
            for action in ('check', 'mount', 'setup', 'install', 'unmount', 'doctor'):
                with self.subTest(action=action), self.assertRaisesRegex(RuntimeError, '--worktree'):
                    build_storage.main(['--worktree', str(worktree), action])
            with mock.patch.object(build_storage.sys, 'platform', 'linux'), \
                    self.assertRaisesRegex(RuntimeError, '--worktree'):
                build_storage.main(['--profile', 'portable-linux', '--worktree', str(worktree), *command])
            run.assert_not_called()

            # Storage identity is checked before the worktree, even for a bad one.
            disk['VolumeUUID'] = 'another-volume'
            for path in (worktree, managed / 'missing', elsewhere):
                with self.subTest(identity=path.name), \
                        self.assertRaisesRegex(RuntimeError, 'Unexpected filesystem or volume'):
                    build_storage.main(['--worktree', str(path), *command])
            output = root / 'output/bad-identity'
            with self.assertRaisesRegex(RuntimeError, 'Unexpected filesystem or volume'):
                build_storage.main(['--output', str(output), '--worktree', str(elsewhere), *command])
            self.assertFalse(output.exists(), 'storage rejection wrote evidence')
            disk['VolumeUUID'] = 'fixture-apfs-uuid'

            # Managed links are still required: an internal workspace directory
            # is refused even though it holds a checkout.
            managed.unlink()
            (managed / 'feature/.git').mkdir(parents=True)
            with self.assertRaisesRegex(RuntimeError, 'must be a symlink'):
                build_storage.main(['--worktree', str(managed / 'feature'), *command])
            shutil.rmtree(managed)
            managed.symlink_to(workspaces, target_is_directory=True)

            rejected = [
                ('outside workspaces', elsewhere),
                ('main checkout', root),
                ('workspaces root', managed),
                ('prefix sibling', sibling),
                ('missing', managed / 'missing'),
                ('subdirectory', managed / 'feature/crates'),
                ('regular file', managed / 'notes.txt'),
                ('not a checkout', managed / 'plain'),
                ('escaping link', managed / 'escape'),
                ('traversal', managed / 'feature/../../../checkout'),
                ('shared main target', managed / 'shared'),
                ('borrowed worktree target', managed / 'borrower')]
            for label, path in rejected:
                with self.subTest(worktree=label), self.assertRaisesRegex(RuntimeError, '--worktree'):
                    build_storage.main(['--worktree', str(path), *command])
            # The evidence branch validates before writing its first report.
            for number, (label, path) in enumerate(rejected):
                output = root / 'output' / f'bad-{number}'
                with self.subTest(evidence=label), self.assertRaisesRegex(RuntimeError, '--worktree'):
                    build_storage.main(['--output', str(output), '--worktree', str(path), *command])
                self.assertFalse(output.exists(), f'rejected {label} run wrote evidence')
            self.assertEqual((shared / 'target').resolve(), (volume / 'rust-target').resolve(),
                'rejection changed a caller-owned link')
            self.assertEqual((borrower / 'target').resolve(), (workspaces / 'donor/target').resolve())

            # A caller inside a workspace must name it: the former
            # "env CARGO_TARGET_DIR=$PWD/target" form ran the main checkout.
            for start in (worktree, worktree / 'crates'):
                with self.subTest(caller=start.name):
                    os.chdir(start)
                    with self.assertRaisesRegex(RuntimeError, '--worktree'):
                        build_storage.main(['run', 'env', f'CARGO_TARGET_DIR={start}/target',
                            'cargo', 'test', '--workspace'])
                    output = root / 'output' / f'guard-{start.name}'
                    with self.assertRaisesRegex(RuntimeError, '--worktree'):
                        build_storage.main(['--output', str(output), *command])
                    self.assertFalse(output.exists(), 'rejected run wrote evidence')
            execute.assert_not_called()
            self.assertFalse((worktree / 'target').exists())

    def test_desktop_build_identity_binds_cargo_inputs_and_only_desktop_env(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'checkout'
            external_config = Path(temporary) / 'builder/flags.toml'
            external_config.parent.mkdir()
            external_config.write_text('[build]\nrustflags=["--cfg", "desktop_fixture"]\n')
            # Cargo resolves recursive include paths relative to each including file:
            # https://doc.rust-lang.org/cargo/reference/config.html#include
            included_config = root / '.local/includes/builder.toml'
            optional_config = included_config.parent / 'optional.toml'
            inputs = {
                '.cargo/config.toml': 'include=["../.local/includes/builder.toml"]\n'
                    '[source.crates-io]\nreplace-with="vendored-sources"\n'
                    '[source.vendored-sources]\ndirectory="vendor"\n[net]\noffline=true\n',
                '.local/includes/builder.toml': 'include=[' + json.dumps(str(external_config)) +
                    ', {path="optional.toml", optional=true}]\n',
                'vendor/fixture/src/lib.rs': 'pub fn fixture() {}\n',
                '.local/cargo-home/config.toml': '[net]\noffline=true\n',
            }
            for relative, content in inputs.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            tools = root / '.local/producer-tools'
            tools.mkdir()
            for name in ('cargo', 'rustc', 'node'):
                path = tools / name
                path.write_text(f'#!{sys.executable}\nprint("{name} fixture")\n')
                path.chmod(0o755)
            # Values are bound as hashes: a signing key must not reach the report.
            bound = dict(PATH=str(tools), CARGO_HOME=str(root / '.local/cargo-home'),
                CARGO_TARGET_DIR=str(root / 'target'), RUSTFLAGS='-Cdebuginfo=1',
                TAURI_SIGNING_PRIVATE_KEY='fixture signing key', VITE_FIXTURE='1',
                NODE_OPTIONS='--max-old-space-size=4096', MACOSX_DEPLOYMENT_TARGET='13.0')
            env = dict(bound, HOME=str(root))
            # The desktop graph has no Solidity or RISC Zero build step.
            unrelated = dict(AIN_SOLC=str(root / 'solc'), AIN_FOUNDRY_BIN=str(root / 'foundry'),
                FOUNDRY_PROFILE='ci', DAPP_SOLC_VERSION='0.8.36', RISC0_DEV_MODE='1')

            def identity(env):
                with mock.patch.object(build_evidence, 'ROOT', root):
                    return build_evidence.desktop_configuration(False, False, env=env)

            recorded = identity(dict(env, **unrelated))
            self.assertEqual(recorded['env'], {name: hashlib.sha256(value.encode()).hexdigest()
                for name, value in sorted(bound.items())})
            self.assertEqual(identity(env), recorded)
            self.assertEqual((recorded['kind'], recorded['profile'], recorded['features']),
                ('desktop', 'release', []))
            self.assertEqual(set(recorded['tools']), {'cargo', 'rustc', 'node'})
            self.assertEqual(recorded['targetDir'], str((root / 'target').resolve()))
            self.assertEqual(set(recorded['cargoConfigs']), {str(root / '.cargo/config.toml'),
                str(included_config), str(external_config), str(optional_config),
                str(root / '.local/cargo-home/config.toml')})
            self.assertIsNone(recorded['cargoConfigs'][str(optional_config)])
            self.assertEqual(set(recorded['cargoVendors']), {str(root / 'vendor')})
            original_external = external_config.read_bytes()
            original_included = included_config.read_bytes()
            for change in ('external-flags', 'optional-created', 'vendor', 'required-missing', 'recursive-cycle'):
                with self.subTest(cargo_input=change):
                    vendored = root / 'vendor/fixture/src/lib.rs'
                    original_vendored = vendored.read_bytes()
                    try:
                        if change == 'external-flags':
                            external_config.write_text('[build]\nrustflags=["--cfg", "different_build"]\n')
                        elif change == 'optional-created':
                            optional_config.write_text('[build]\nrustflags=["--cfg", "new_optional_input"]\n')
                        elif change == 'vendor':
                            vendored.write_text('pub fn changed() {}\n')
                        elif change == 'required-missing':
                            external_config.unlink()
                        else:
                            external_config.write_text('include=[' + json.dumps(str(included_config)) + ']\n')
                        if change in ('required-missing', 'recursive-cycle'):
                            with self.assertRaisesRegex(ValueError, 'include|config|cycle'):
                                identity(env)
                        else:
                            self.assertNotEqual(identity(env), recorded)
                    finally:
                        external_config.write_bytes(original_external)
                        included_config.write_bytes(original_included)
                        optional_config.unlink(missing_ok=True)
                        vendored.write_bytes(original_vendored)
            self.assertEqual(identity(env), recorded)
            # A different artifact directory would seal stale files from ./target.
            without_target = {name: value for name, value in env.items() if name != 'CARGO_TARGET_DIR'}
            for selector in [dict(env, CARGO_TARGET_DIR=str(root / 'other-target')),
                             dict(without_target, CARGO_BUILD_TARGET_DIR=str(root / 'other-target')),
                             dict(env, CARGO_BUILD_TARGET='x86_64-unknown-linux-gnu')]:
                with self.subTest(selector=sorted(set(selector) - set(env))), self.assertRaises(ValueError):
                    identity(selector)

    def test_doctor_checks_only_the_selected_suite_and_distinguishes_statuses(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary_dir = root / 'bin'
            binary_dir.mkdir()
            (binary_dir / 'python3').symlink_to(sys.executable)
            (binary_dir / 'git').symlink_to(shutil.which('git'))
            env = dict(PATH=str(binary_dir), HOME=str(root), CARGO_TARGET_DIR=str(root / 'target'))
            for relative in ('target', 'output', 'apps/desktop/node_modules/vitest',
                             'apps/desktop/node_modules/jsdom'):
                (root / relative).mkdir(parents=True, exist_ok=True)
            (root / 'apps/desktop/package.json').write_text(json.dumps({'engines': {'node': '>=26.0.0'}}))
            (root / 'apps/desktop/node_modules/vitest/vitest.mjs').write_text('// installed fixture\n')
            (root / 'apps/desktop/node_modules/jsdom/package.json').write_text('{"name":"jsdom"}\n')

            def tool(name, version):
                path = binary_dir / name
                path.write_text(f'#!{sys.executable}\nimport sys\n'
                    'assert sys.argv[1:] in (["--version"], ["-V"]), "doctor invoked a build"\n'
                    f'print({version!r})\n')
                path.chmod(0o755)

            def status(suite, expected, missing=None, phase='build'):
                report = build_evidence.doctor(root, suite, profile='portable-linux', env=env, phase=phase)
                self.assertEqual(report['status'], expected, report)
                self.assertEqual(report['suite'], suite)
                if missing:
                    self.assertIn(missing, json.dumps(report).lower())
                return report

            with mock.patch.object(build_evidence.sys, 'platform', 'linux'):
                with self.subTest(profile='mac-apfs'), \
                        mock.patch.object(build_evidence.shutil, 'which',
                            side_effect=AssertionError('unsupported profile probed tools')):
                    report = build_evidence.doctor(root, 'build-tooling', profile='mac-apfs', env=env)
                    self.assertEqual(report['status'], 'unsupported', report)
                    self.assertFalse(report['passed'])
                status('build-tooling', 'passed')
                status('backend-unit', 'blocked', 'cargo')
                tool('node', 'v26.0.0')
                tool('npm', '11.0.0')
                # Neither frontend dependency is supplied by Cargo, Tauri or Foundry.
                status('frontend-unit', 'passed')
                tool('cargo', 'cargo 1.95.0')
                tool('rustc', 'rustc 1.95.0')
                tool('cc', 'cc fixture 1')
                tool('pkg-config', '1.8.1')
                status('backend-unit', 'passed')
                status('desktop-build', 'blocked', 'tauri')
                status('evm', 'blocked', 'forge')
                status('native-macos', 'unsupported')
                status('unknown-suite', 'unsupported')
                tool('node', 'v25.9.0')
                status('frontend-unit', 'failed', '26')
                for name in ('forge', 'anvil', 'cast'):
                    tool(name, f'{name} 1.4.0')
                # check-evm.sh passes an explicit AIN_SOLC to Forge; the evm
                # preflight does not require it, even when the path is missing.
                env['AIN_SOLC'] = str(root / 'configured-solc')
                report = status('evm', 'passed')
                self.assertEqual([item['name'] for item in report['requirements']],
                    ['forge', 'anvil', 'cast'])
                with mock.patch.object(build_evidence, '_unix_ipc_probe',
                        return_value=dict(name='unix-ipc', status='passed')):
                    report = status('native-runtime', 'passed', phase='runtime')
                    self.assertEqual([item['name'] for item in report['requirements']],
                        ['python3', 'git', 'unix-ipc'])
                    status('native-runtime', 'unsupported', 'native-build')

    def test_portable_wrapper_executes_exact_python_suites_and_retains_command_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            # Copy only the two accepted patterns and their imports: discovering
            # these fixtures cannot recursively execute this portable test file.
            for relative in ('scripts/build-storage.py', 'scripts/build_evidence.py', 'tests/build/test_build_evidence.py'):
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, destination)
            for relative in ('target', 'output', 'bin', 'home', 'tmp', 'network-guard'):
                (root / relative).mkdir()
            (root / 'bin/python3').symlink_to(sys.executable)
            trap = root / 'forbidden-tool.log'
            git = shutil.which('git')
            for name in ('cargo', 'rustc', 'rustup', 'forge', 'anvil', 'curl', 'wget', 'docker', 'pip', 'pip3', 'git'):
                path = root / 'bin' / name
                forward = (f'if sys.argv[1] in {{"init", "config", "add", "commit", "rev-parse", "ls-files"}}:\n'
                    f' os.execv({git!r}, [{git!r}, *sys.argv[1:]])\n') if name == 'git' else ''
                path.write_text(f'#!{sys.executable}\nimport os, pathlib, sys\n' + forward +
                    f'pathlib.Path({str(trap)!r}).write_text({name!r})\nraise SystemExit(97)\n')
                path.chmod(0o755)
            (root / 'network-guard/sitecustomize.py').write_text(
                'import pathlib, socket\n'
                'def forbidden(*args, **kwargs):\n'
                f' pathlib.Path({str(trap)!r}).write_text("network")\n'
                ' raise RuntimeError("Python acceptance attempted network access")\n'
                'socket.socket.connect = forbidden\n'
                'socket.create_connection = forbidden\n'
                'socket.getaddrinfo = forbidden\n')
            env = dict(PATH=str(root / 'bin'), HOME=str(root / 'home'), TMPDIR=str(root / 'tmp'),
                PYTHONPATH=str(root / 'network-guard'), PYTHONDONTWRITEBYTECODE='1', GIT_CONFIG_NOSYSTEM='1')

            def run_wrapper(command, output):
                wrapper = root / 'scripts/build-storage.py'
                arguments = [str(wrapper), '--profile', 'portable-linux',
                    '--output', str(output), 'run', *command]
                # Simulate only host selection when these contracts run on
                # macOS. Wrapper dispatch, children and evidence remain real.
                bootstrap = ('import runpy, sys\nsys.platform = "linux"\n'
                    f'sys.argv = {arguments!r}\nrunpy.run_path({str(wrapper)!r}, run_name="__main__")\n')
                return subprocess.run([sys.executable, '-c', bootstrap], cwd=root,
                    env=env, text=True, capture_output=True, timeout=60)

            cases = [('build-tooling', 'tests/build', 'test_build_evidence.py', 7)]
            for name, directory, pattern, count in cases:
                with self.subTest(suite=name):
                    output = root / 'output' / name
                    command = [sys.executable, '-m', 'unittest', 'discover',
                        '-s', directory, '-p', pattern, '-v']
                    result = run_wrapper(command, output)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    saved = json.loads((output / 'check.json').read_text())
                    self.assertEqual(len(saved['commands']), 1)
                    entry = saved['commands'][0]
                    self.assertEqual((entry['status'], entry['exitCode']), ('succeeded', 0))
                    self.assertEqual(entry['args'], command)
                    self.assertEqual(Path(entry['cwd']).resolve(), root.resolve())
                    log = Path(entry['logPath']).read_text()
                    self.assertRegex(log, rf'\bRan {count} tests\b')
                    self.assertRegex(log, r'\nOK\s*$')
                    self.assertEqual(saved.get('status'), 'passed', saved)
                    self.assertIs(saved['passed'], True)
            with self.subTest(suite='failing-child'):
                output = root / 'output/failing-child'
                command = [sys.executable, '-c',
                    'print("retained intentional failure"); raise SystemExit(23)']
                result = run_wrapper(command, output)
                self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
                saved = json.loads((output / 'check.json').read_text())
                self.assertEqual(len(saved['commands']), 1)
                entry = saved['commands'][0]
                self.assertEqual((entry['status'], entry['exitCode']), ('failed', 23))
                self.assertEqual(entry['args'], command)
                self.assertEqual(Path(entry['cwd']).resolve(), root.resolve())
                self.assertIn('retained intentional failure', Path(entry['logPath']).read_text())
                self.assertEqual(saved.get('status'), 'failed', saved)
                self.assertIs(saved['passed'], False)
            self.assertFalse(trap.exists(), trap.read_text() if trap.exists() else '')
            self.assertTrue((root / 'output/build-tooling/check.json').is_file())

    def test_offline_linux_toolkit_requires_native_dependencies_and_verified_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'bundle-root'
            root.mkdir()
            # Tiny text payloads exercise a real archive/layout without creating
            # or installing a compiler, native library or distributable toolkit.
            contents = {
                'payload/bin/node': '#!/bin/sh\nprintf "v26.0.0\\n"\n',
                'payload/bin/npm': '#!/bin/sh\nprintf "11.0.0\\n"\n',
                'payload/bin/rustc': '#!/bin/sh\nprintf "rustc 1.95.0\\n"\n',
                'payload/bin/cargo': '#!/bin/sh\nprintf "cargo 1.95.0\\n"\n',
                'payload/Cargo.lock': 'version = 3\n[[package]]\nname = "fixture"\nversion = "0.1.0"\n'
                    'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                    f'checksum = "{"a" * 64}"\n',
                'payload/vendor/fixture/Cargo.toml': '[package]\nname="fixture"\nversion="0.1.0"\n',
                'payload/vendor/fixture/src/lib.rs': 'pub fn fixture() {}\n',
                'payload/cargo/config.toml': '[source.crates-io]\nreplace-with="vendored-sources"\n'
                    '[source.vendored-sources]\ndirectory="vendor"\n[net]\noffline=true\n',
            }
            native = ['cc', 'pkg-config', 'openssl', 'webkit2gtk-4.1', 'gtk+-3.0']
            frontend_packages = {
                'vitest': ('4.0.0', 'vitest.mjs'), 'jsdom': ('27.0.0', 'lib/api.js'),
                'typescript': ('5.9.3', 'bin/tsc'), 'vite': ('7.1.5', 'bin/vite.js'),
                '@tauri-apps/cli': ('2.8.4', 'tauri.js'),
            }
            package_lock = dict(name='offline-desktop-fixture', version='0.1.0', lockfileVersion=3,
                packages={'': dict(name='offline-desktop-fixture', version='0.1.0',
                    devDependencies={name: version for name, (version, _) in frontend_packages.items()})})
            frontend_bins = {'vitest': 'vitest', 'typescript': 'tsc', 'vite': 'vite', '@tauri-apps/cli': 'tauri'}
            for name, (version, entry) in frontend_packages.items():
                package_lock['packages'][f'node_modules/{name}'] = dict(version=version)
                prefix = f'payload/frontend/node_modules/{name}'
                metadata = dict(name=name, version=version, main=entry)
                if name in frontend_bins:
                    metadata['bin'] = {frontend_bins[name]: entry}
                    package_lock['packages'][f'node_modules/{name}']['bin'] = metadata['bin']
                contents[f'{prefix}/package.json'] = json.dumps(metadata)
                contents[f'{prefix}/{entry}'] = '// offline installed package fixture\n'
            contents['payload/frontend/package-lock.json'] = json.dumps(package_lock)
            contents['payload/vendor/fixture/.cargo-checksum.json'] = json.dumps(dict(package='a' * 64,
                files={name: hashlib.sha256(contents[f'payload/vendor/fixture/{name}'].encode()).hexdigest()
                    for name in ('Cargo.toml', 'src/lib.rs')}))
            contents.update({f'payload/native/{name}.fixture': f'{name} offline package fixture\n'
                for name in native})
            for relative, content in contents.items():
                payload = root / relative
                payload.parent.mkdir(parents=True, exist_ok=True)
                payload.write_text(content)
                if relative.startswith('payload/bin/'):
                    payload.chmod(0o755)
            for name, executable in frontend_bins.items():
                entry = frontend_packages[name][1]
                (root / f'payload/frontend/node_modules/{name}/{entry}').chmod(0o755)
                relative = f'payload/frontend/node_modules/.bin/{executable}'
                launcher = root / relative
                launcher.parent.mkdir(exist_ok=True)
                launcher.symlink_to(f'../{name}/{entry}')
                contents[relative] = None  # Pack the actual symlink, never a replacement text file.
            archive = root / 'archives/toolkit.tar'
            archive.parent.mkdir()

            def sha(path):
                return hashlib.sha256(path.read_bytes()).hexdigest()

            def file_identity(relative):
                return dict(path=relative, sha256=sha(root / relative))

            def pack(extra_members=()):
                with tarfile.open(archive, 'w') as package:
                    for relative in sorted(contents):
                        package.add(root / relative, arcname=relative, recursive=False)
                    for member in extra_members:
                        package.addfile(member)
                return sha(archive)

            manifest = dict(schema=1, platform='linux', target='x86_64-unknown-linux-gnu',
                execution='native',
                archive=dict(path='archives/toolkit.tar', sha256=pack()),
                tools=dict(node='26.0.0', npm='11.0.0', rustc='1.95.0', cargo='1.95.0'),
                toolFiles={name: file_identity(f'payload/bin/{name}') for name in ('node', 'npm', 'rustc', 'cargo')},
                nativeDependencies=native,
                nativeFiles={name: file_identity(f'payload/native/{name}.fixture') for name in native},
                frontend=dict(packageLockPath='payload/frontend/package-lock.json',
                    packageLockSha256=sha(root / 'payload/frontend/package-lock.json'),
                    nodeModulesPath='payload/frontend/node_modules',
                    nodeModulesFiles={str(Path(relative).relative_to('payload/frontend/node_modules')): sha(root / relative)
                        for relative in contents if relative.startswith('payload/frontend/node_modules/')}),
                cargo=dict(locked=True, offline=True,
                    lockPath='payload/Cargo.lock', lockSha256=sha(root / 'payload/Cargo.lock'),
                    vendorPath='payload/vendor',
                    vendorFiles={str(Path(relative).relative_to('payload/vendor')): sha(root / relative)
                        for relative in contents if relative.startswith('payload/vendor/')},
                    configPath='payload/cargo/config.toml', configSha256=sha(root / 'payload/cargo/config.toml')))
            path = root / 'toolkit.json'

            def validate(value):
                path.write_text(json.dumps(value))
                return build_evidence.validate_toolkit(root, path,
                    target='x86_64-unknown-linux-gnu')

            # The accepted manifest needs neither Docker nor a Foundry payload.
            self.assertEqual(validate(manifest), manifest)
            launcher_relative = 'payload/frontend/node_modules/.bin/vitest'
            launcher = root / launcher_relative
            original_target = os.readlink(launcher)
            outside_launcher = Path(temporary) / 'external-entry.mjs'
            outside_launcher.write_bytes(launcher.read_bytes())
            outside_launcher.chmod(0o755)
            for change in ('changed-target', 'escaping-target', 'omitted'):
                with self.subTest(frontend_launcher=change):
                    launcher.unlink()
                    candidate = copy.deepcopy(manifest)
                    try:
                        if change == 'changed-target':
                            launcher.symlink_to('../vite/bin/vite.js')
                        elif change == 'escaping-target':
                            launcher.symlink_to(outside_launcher)
                        else:
                            del contents[launcher_relative]
                            del candidate['frontend']['nodeModulesFiles']['.bin/vitest']
                            candidate['archive']['sha256'] = pack()
                        with self.assertRaises(ValueError):
                            validate(candidate)
                    finally:
                        launcher.unlink(missing_ok=True)
                        launcher.symlink_to(original_target)
                        contents[launcher_relative] = None
                        manifest['archive']['sha256'] = pack()
            self.assertEqual(validate(manifest), manifest)
            invalid = []
            for key, value in [('platform', 'darwin'), ('target', 'aarch64-unknown-linux-gnu'),
                               ('execution', 'docker-only'), ('nativeDependencies', []),
                               ('nativeDependencies', [name for name in native if name != 'openssl']),
                               ('nativeDependencies', [name for name in native if name != 'gtk+-3.0'])]:
                candidate = copy.deepcopy(manifest)
                candidate[key] = value
                invalid.append(candidate)
            for key, value in [('node', '25.9.0'), ('npm', None), ('rustc', None), ('cargo', None)]:
                candidate = copy.deepcopy(manifest)
                if value is None:
                    del candidate['tools'][key]
                else:
                    candidate['tools'][key] = value
                invalid.append(candidate)
            for section, key in [('toolFiles', 'npm'), ('nativeFiles', 'gtk+-3.0'),
                                 ('frontend', 'packageLockPath'), ('frontend', 'packageLockSha256'),
                                 ('frontend', 'nodeModulesPath'), ('frontend', 'nodeModulesFiles')]:
                candidate = copy.deepcopy(manifest)
                del candidate[section][key]
                invalid.append(candidate)
            candidate = copy.deepcopy(manifest)
            del candidate['frontend']
            invalid.append(candidate)
            candidate = copy.deepcopy(manifest)
            candidate['frontend']['nodeModulesFiles'] = {}
            invalid.append(candidate)
            for field in ('packageLockSha256', 'nodeModulesFiles'):
                candidate = copy.deepcopy(manifest)
                if field == 'nodeModulesFiles':
                    candidate['frontend'][field]['vitest/vitest.mjs'] = '0' * 64
                else:
                    candidate['frontend'][field] = '0' * 64
                invalid.append(candidate)
            for key in ('locked', 'offline'):
                candidate = copy.deepcopy(manifest)
                candidate['cargo'][key] = False
                invalid.append(candidate)
            for key in ('vendorFiles', 'lockSha256', 'configSha256'):
                candidate = copy.deepcopy(manifest)
                del candidate['cargo'][key]
                invalid.append(candidate)
            for index, candidate in enumerate(invalid):
                with self.subTest(invalid_manifest=index), self.assertRaises(ValueError):
                    validate(candidate)
            for relative in ('payload/Cargo.lock', 'payload/cargo/config.toml',
                             'payload/vendor/fixture/src/lib.rs', 'payload/bin/rustc',
                             'payload/bin/npm', 'payload/native/openssl.fixture',
                             'payload/native/gtk+-3.0.fixture', 'payload/frontend/package-lock.json',
                             'payload/frontend/node_modules/vitest/vitest.mjs'):
                with self.subTest(missing_or_changed=relative):
                    payload = root / relative
                    original = payload.read_bytes()
                    mode = payload.stat().st_mode & 0o7777
                    try:
                        payload.unlink()
                        with self.subTest(state='missing'), self.assertRaises(ValueError):
                            validate(manifest)
                        payload.write_bytes(original + b'changed')
                        payload.chmod(mode)
                        with self.subTest(state='changed'), self.assertRaises(ValueError):
                            validate(manifest)
                    finally:
                        payload.write_bytes(original)
                        payload.chmod(mode)
            self.assertEqual(validate(manifest), manifest, 'negative cases failed to restore the valid fixture')
            # An internally consistent archive still cannot omit a locked
            # frontend dependency, or silently use a different checkout lock.
            for name, (_, entry) in frontend_packages.items():
                relative = f'payload/frontend/node_modules/{name}/{entry}'
                with self.subTest(omitted_frontend=name):
                    payload = root / relative
                    original = contents.pop(relative)
                    mode = payload.stat().st_mode & 0o7777
                    payload.unlink()
                    try:
                        candidate = copy.deepcopy(manifest)
                        del candidate['frontend']['nodeModulesFiles'][f'{name}/{entry}']
                        candidate['archive']['sha256'] = pack()
                        with self.assertRaises(ValueError):
                            validate(candidate)
                    finally:
                        contents[relative] = original
                        payload.write_text(original)
                        payload.chmod(mode)
            manifest['archive']['sha256'] = pack()
            checkout = Path(temporary) / 'checkout'
            (checkout / 'apps/desktop').mkdir(parents=True)
            shutil.copyfile(root / 'payload/Cargo.lock', checkout / 'Cargo.lock')
            shutil.copyfile(root / 'payload/frontend/package-lock.json', checkout / 'apps/desktop/package-lock.json')
            path.write_text(json.dumps(manifest))
            self.assertEqual(build_evidence.validate_toolkit(root, path,
                target='x86_64-unknown-linux-gnu', checkout=checkout), manifest)
            (checkout / 'apps/desktop/package-lock.json').write_text('{"lockfileVersion":3,"packages":{}}\n')
            with self.subTest(frontend_lock='different-checkout'), self.assertRaises(ValueError):
                build_evidence.validate_toolkit(root, path, target='x86_64-unknown-linux-gnu', checkout=checkout)
            # A matching checksum must not turn a network-enabled Cargo config
            # into a reproducible offline toolkit.
            config = root / 'payload/cargo/config.toml'
            original = config.read_text()
            for altered in [original.replace('offline=true', 'offline=false'),
                            original.replace('replace-with="vendored-sources"', '')]:
                with self.subTest(cargo_config=altered):
                    config.write_text(altered)
                    candidate = copy.deepcopy(manifest)
                    candidate['cargo']['configSha256'] = sha(config)
                    candidate['archive']['sha256'] = pack()
                    with self.assertRaises(ValueError):
                        validate(candidate)
            config.write_text(original)
            manifest['archive']['sha256'] = pack()
            self.assertEqual(validate(manifest), manifest)
            # Every link must resolve to an ordinary member of this archive,
            # rather than relying on leftovers from an earlier unpack. Matching
            # bytes cannot substitute for the archived lstat type or mode.
            archive_cases = [
                ('member-symlink', tarfile.SYMTYPE, 'bin/node', 'symlink', None, True),
                ('member-hardlink', tarfile.LNKTYPE, 'payload/bin/node', 'hardlink', 0o755, True),
                ('member-directory', tarfile.DIRTYPE, '', 'directory', 0o755, True),
                ('stale-symlink', tarfile.SYMTYPE, 'stale-node', 'symlink', None, False),
                ('stale-hardlink', tarfile.LNKTYPE, 'payload/stale-node', 'hardlink', 0o755, False),
                ('self-hardlink', tarfile.LNKTYPE, 'payload/archive-fixture', 'file', 0o755, False),
                ('special-member', tarfile.FIFOTYPE, '', 'file', 0o755, False),
                ('directory-is-link', tarfile.DIRTYPE, 'bin', 'symlink', 0o755, False),
                ('directory-mode', tarfile.DIRTYPE, '', 'private-directory', 0o755, False),
                ('hardlink-is-symlink', tarfile.LNKTYPE, 'payload/bin/node', 'symlink', 0o755, False),
                ('symlink-is-file', tarfile.SYMTYPE, 'bin/node', 'file', 0o777, False),
                ('symlink-mode', tarfile.SYMTYPE, 'bin/node', 'symlink', None, False),
                ('hardlink-mode', tarfile.LNKTYPE, 'payload/bin/node', 'hardlink', 0o700, False),
            ]
            alias, stale = root / 'payload/archive-fixture', root / 'payload/stale-node'
            for name, member_type, link_target, unpacked_type, member_mode, accepted in archive_cases:
                with self.subTest(archive_identity=name):
                    try:
                        if name in ('stale-symlink', 'stale-hardlink'):
                            shutil.copy2(root / 'payload/bin/node', stale)
                        if unpacked_type == 'symlink':
                            alias.symlink_to('bin/node' if member_type == tarfile.LNKTYPE else link_target)
                        elif unpacked_type == 'hardlink':
                            os.link(root / link_target, alias)
                        elif unpacked_type in ('directory', 'private-directory'):
                            alias.mkdir(mode=0o700 if unpacked_type == 'private-directory' else 0o755)
                            alias.chmod(0o700 if unpacked_type == 'private-directory' else 0o755)
                        else:
                            shutil.copy2(root / 'payload/bin/node', alias)
                        entry = tarfile.TarInfo('payload/archive-fixture')
                        entry.type, entry.mode = member_type, member_mode
                        if member_mode is None:
                            entry.mode = alias.lstat().st_mode & 0o7777
                        if name == 'symlink-mode':
                            entry.mode ^= 0o100  # A genuine mismatch on every host.
                        if member_type in (tarfile.SYMTYPE, tarfile.LNKTYPE):
                            entry.linkname = link_target
                        candidate = copy.deepcopy(manifest)
                        candidate['archive']['sha256'] = pack([entry])
                        if accepted:
                            self.assertEqual(validate(candidate), candidate)
                        else:
                            with self.assertRaises(ValueError):
                                validate(candidate)
                    finally:
                        if alias.is_symlink() or alias.is_file():
                            alias.unlink()
                        elif alias.is_dir():
                            alias.rmdir()
                        stale.unlink(missing_ok=True)
            manifest['archive']['sha256'] = pack()
            outside = Path(temporary) / 'outside.tar'
            shutil.copyfile(archive, outside)
            for unsafe in ('../outside.tar', str(outside)):
                candidate = copy.deepcopy(manifest)
                candidate['archive']['path'] = unsafe
                with self.subTest(archive_path=unsafe), self.assertRaises(ValueError):
                    validate(candidate)
            for name, link in [('../escaped.txt', None), ('payload/external-link', '/outside')]:
                with self.subTest(archive_member=name):
                    pack()
                    with tarfile.open(archive, 'a') as package:
                        entry = tarfile.TarInfo(name)
                        if link:
                            entry.type, entry.linkname = tarfile.SYMTYPE, link
                            package.addfile(entry)
                        else:
                            entry.size = 1
                            package.addfile(entry, io.BytesIO(b'x'))
                    candidate = copy.deepcopy(manifest)
                    candidate['archive']['sha256'] = sha(archive)
                    with self.assertRaises(ValueError):
                        validate(candidate)
            manifest['archive']['sha256'] = pack()
            archive.write_bytes(b'changed toolkit download')
            with self.assertRaisesRegex(ValueError, 'checksum|sha256|archive'):
                validate(manifest)


if __name__ == '__main__':
    unittest.main()
