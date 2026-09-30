"""Reject toolchains older than the MSRV required by the locked native graph.

Version probes and toolkit bytes are small fixtures. These tests never compile
Rust or execute any supplied toolkit payload.
"""
import hashlib
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import build_evidence


class RustMinimumTests(unittest.TestCase):
    def test_doctor_rejects_old_cargo_or_rustc_and_accepts_1_91(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / 'bin'
            binary.mkdir()
            env = dict(PATH=str(binary), HOME=str(root))

            def tool(name, version):
                path = binary / name
                path.write_text(f'#!{sys.executable}\nimport sys\n'
                    'assert sys.argv[1:] == ["--version"], "doctor attempted a build"\n'
                    f'print({(name + " " + version)!r})\n')
                path.chmod(0o755)

            for name, version in dict(cargo='1.91.0', rustc='1.91.0', cc='1.0.0',
                                     **{'pkg-config': '1.8.1'}).items():
                tool(name, version)
            with mock.patch.object(build_evidence.sys, 'platform', 'linux'):
                for name in ('cargo', 'rustc'):
                    for old_version in ('1.85.0', '1.90.9'):
                        with self.subTest(tool=name, version=old_version):
                            tool(name, old_version)
                            try:
                                report = build_evidence.doctor(root, 'backend-unit',
                                    profile='portable-linux', env=env)
                                self.assertEqual(report['status'], 'failed', report)
                                self.assertIs(report['passed'], False)
                                item = next(item for item in report['requirements'] if item['name'] == name)
                                self.assertEqual(item['status'], 'failed')
                                self.assertIn('>= 1.91.0', item['reason'])
                            finally:
                                tool(name, '1.91.0')
                report = build_evidence.doctor(root, 'backend-unit', profile='portable-linux', env=env)
                self.assertEqual(report['status'], 'passed', report)
                self.assertIs(report['passed'], True)

    def toolkit(self, root, *, cargo='1.91.0', rustc='1.91.0'):
        """Make a complete checksum-bound archive so only the MSRV varies."""
        def write(relative, contents, executable=False):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
            path.chmod(0o755 if executable else 0o644)
            return path

        def sha(path):
            return hashlib.sha256(path.read_bytes()).hexdigest()

        def identity(relative):
            return dict(path=relative, sha256=sha(root / relative))

        versions = dict(node='26.0.0', npm='11.0.0', cargo=cargo, rustc=rustc)
        for name, version in versions.items():
            write(f'payload/bin/{name}', f'#!/bin/sh\necho "{name} {version}"\n', True)
        native = ('cc', 'pkg-config', 'openssl', 'webkit2gtk-4.1', 'gtk+-3.0')
        for name in native:
            write(f'payload/native/{name}', f'Native dependency fixture: {name}\n')
        write('payload/Cargo.lock', 'version=3\n[[package]]\nname="fixture"\nversion="0.1.0"\n'
            'source="registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum="{"a" * 64}"\n')
        write('payload/vendor/fixture/Cargo.toml', '[package]\nname="fixture"\nversion="0.1.0"\n')
        write('payload/vendor/fixture/src/lib.rs', 'pub fn fixture() {}\n')
        write('payload/vendor/fixture/.cargo-checksum.json', json.dumps(dict(package='a' * 64,
            files={name: sha(root / 'payload/vendor/fixture' / name)
                for name in ('Cargo.toml', 'src/lib.rs')})))
        write('payload/cargo/config.toml', '[source.crates-io]\nreplace-with="vendored-sources"\n'
            '[source.vendored-sources]\ndirectory="vendor"\n[net]\noffline=true\n')
        packages = {'vitest': ('vitest.mjs', 'vitest'), 'jsdom': ('lib/api.js', None),
            'typescript': ('bin/tsc', 'tsc'), 'vite': ('bin/vite.js', 'vite'),
            '@tauri-apps/cli': ('tauri.js', 'tauri')}
        lock = dict(lockfileVersion=3, packages={'': dict(devDependencies={name: '1.0.0' for name in packages})})
        for name, (entry, executable) in packages.items():
            metadata = dict(name=name, version='1.0.0', main=entry)
            locked = dict(version='1.0.0')
            if executable:
                metadata['bin'] = locked['bin'] = {executable: entry}
                write(f'payload/frontend/node_modules/.bin/{executable}', '// launcher fixture\n', True)
            write(f'payload/frontend/node_modules/{name}/{entry}', '// package fixture\n', bool(executable))
            write(f'payload/frontend/node_modules/{name}/package.json', json.dumps(metadata))
            lock['packages']['node_modules/' + name] = locked
        write('payload/frontend/package-lock.json', json.dumps(lock))

        def inventory(relative):
            directory = root / relative
            return {path.relative_to(directory).as_posix(): sha(path)
                for path in directory.rglob('*') if path.is_file()}

        archive = root / 'toolkit.tar'
        with tarfile.open(archive, 'w') as output:
            for path in sorted((root / 'payload').rglob('*')):
                output.add(path, arcname=path.relative_to(root).as_posix(), recursive=False)
        manifest = dict(schema=1, platform='linux', target='x86_64-unknown-linux-gnu', execution='native',
            archive=identity('toolkit.tar'), tools=versions,
            toolFiles={name: identity(f'payload/bin/{name}') for name in versions},
            nativeDependencies=list(native), nativeFiles={name: identity(f'payload/native/{name}') for name in native},
            frontend=dict(packageLockPath='payload/frontend/package-lock.json',
                packageLockSha256=sha(root / 'payload/frontend/package-lock.json'),
                nodeModulesPath='payload/frontend/node_modules',
                nodeModulesFiles=inventory('payload/frontend/node_modules')),
            cargo=dict(locked=True, offline=True, lockPath='payload/Cargo.lock',
                lockSha256=sha(root / 'payload/Cargo.lock'), vendorPath='payload/vendor',
                vendorFiles=inventory('payload/vendor'), configPath='payload/cargo/config.toml',
                configSha256=sha(root / 'payload/cargo/config.toml')))
        path = root / 'toolkit.json'
        path.write_text(json.dumps(manifest))
        return path, manifest

    def test_toolkit_rejects_old_cargo_or_rustc_and_accepts_1_91(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path, manifest = self.toolkit(root)
            self.assertEqual(build_evidence.validate_toolkit(root, path,
                target='x86_64-unknown-linux-gnu'), manifest)
            for name in ('cargo', 'rustc'):
                for old_version in ('1.85.0', '1.90.9'):
                    with self.subTest(tool=name, version=old_version):
                        path, _ = self.toolkit(root, **{name: old_version})
                        with self.assertRaisesRegex(ValueError, 'toolkit requires ' + name):
                            build_evidence.validate_toolkit(root, path, target='x86_64-unknown-linux-gnu')
            path, manifest = self.toolkit(root)
            self.assertEqual(build_evidence.validate_toolkit(root, path,
                target='x86_64-unknown-linux-gnu'), manifest)


if __name__ == '__main__':
    unittest.main()
