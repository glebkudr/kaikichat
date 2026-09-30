"""Pinned npm publication omissions never exempt payload bytes from checksums.

The package.json fixture is copied byte-for-byte from the official 7.29.7 npm
tarball after verifying its SHA512 against the checkout's package-lock. Other
package files are tiny integrity fixtures; this suite does not run a frontend.
"""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import build_evidence

BABEL = '@babel/helper-validator-identifier'
INTEGRITY = 'sha512-qehxGkRj55h/ff8EMaJ+cYhyaKlHIxqYDn682wQD7RNp9UujOQsHog2uS0r2vzr4pW+sXf90NeeayjcNaX3fFg=='
METADATA = Path(__file__).with_name('fixtures') / 'babel-helper-validator-identifier-7.29.7.package.json'


class FrontendPackageContractTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.modules = self.root / 'payload/frontend/node_modules'
        self.lock_path = self.root / 'payload/frontend/package-lock.json'
        self.lock = dict(lockfileVersion=3, packages={'': dict(devDependencies={})})
        entries = dict(vitest='vitest.mjs', jsdom='lib/api.js', typescript='bin/tsc',
            vite='bin/vite.js', **{'@tauri-apps/cli': 'tauri.js'})
        for name, entry in entries.items():
            self.write(name + '/package.json', json.dumps(dict(name=name, version='1.0.0', main=entry)))
            self.write(name + '/' + entry, '// Bound package entry fixture.\n')
            self.lock['packages']['node_modules/' + name] = dict(version='1.0.0')
            self.lock['packages']['']['devDependencies'][name] = '1.0.0'
        self.metadata = self.modules / BABEL / 'package.json'
        self.write(BABEL + '/package.json', METADATA.read_bytes())
        self.write(BABEL + '/lib/index.js', '// Bound Babel runtime fixture.\n')
        self.lock['packages']['node_modules/' + BABEL] = dict(version='7.29.7', integrity=INTEGRITY)
        self.frontend = self.bind()

    def write(self, name, content):
        path = self.modules / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content.encode() if isinstance(content, str) else content)

    def bind(self):
        self.lock_path.write_text(json.dumps(self.lock))
        return dict(packageLockPath='payload/frontend/package-lock.json',
            packageLockSha256=hashlib.sha256(self.lock_path.read_bytes()).hexdigest(),
            nodeModulesPath='payload/frontend/node_modules',
            nodeModulesFiles={path.relative_to(self.modules).as_posix():
                hashlib.sha256(path.read_bytes()).hexdigest()
                for path in self.modules.rglob('*') if path.is_file()})

    def validate(self, frontend=None):
        build_evidence._check_frontend(self.root, frontend or self.frontend,
            'x86_64-unknown-linux-gnu', checkout=None)

    def test_accepts_only_the_published_missing_type_declaration(self):
        self.assertEqual(hashlib.sha256(METADATA.read_bytes()).hexdigest(),
            '1ad6aeced8b186ac259da45fea50ab9d65d3d958f6458c80e3c8013649d1b12d')
        self.assertFalse((self.modules / BABEL / 'lib/index.d.ts').exists())
        self.validate()

    def test_requires_exact_locked_publication_integrity(self):
        entry = self.lock['packages']['node_modules/' + BABEL]
        for integrity in (None, 'sha512-' + 'A' * 88, INTEGRITY.replace('qehx', 'Qehx')):
            with self.subTest(integrity=integrity):
                if integrity is None:
                    entry.pop('integrity', None)
                else:
                    entry['integrity'] = integrity
                with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
                    self.validate(self.bind())

    def test_requires_original_package_metadata_even_after_inventory_rebinding(self):
        self.metadata.write_bytes(METADATA.read_bytes() + b'\n')
        with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
            self.validate(self.bind())

    def test_does_not_apply_to_another_version(self):
        metadata = json.loads(METADATA.read_text())
        metadata['version'] = '7.29.8'
        self.metadata.write_text(json.dumps(metadata))
        self.lock['packages']['node_modules/' + BABEL]['version'] = '7.29.8'
        with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
            self.validate(self.bind())

    def test_missing_runtime_entry_still_fails_after_inventory_rebinding(self):
        (self.modules / BABEL / 'lib/index.js').unlink()
        with self.assertRaisesRegex(ValueError, 'frontend package entry is missing'):
            self.validate(self.bind())

    def test_other_missing_export_is_rejected(self):
        metadata = json.loads((self.modules / 'jsdom/package.json').read_text())
        metadata['exports'] = {'.': {'types': './lib/index.d.ts', 'default': './lib/api.js'}}
        self.write('jsdom/package.json', json.dumps(metadata))
        with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
            self.validate(self.bind())

    def test_changed_bound_bytes_fail_before_the_publication_exception(self):
        for relative in (BABEL + '/package.json', BABEL + '/lib/index.js'):
            with self.subTest(file=relative):
                path = self.modules / relative
                original = path.read_bytes()
                try:
                    path.write_bytes(original + b'changed')
                    with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                        self.validate()
                finally:
                    path.write_bytes(original)

    def test_incomplete_inventory_is_not_excused(self):
        del self.frontend['nodeModulesFiles'][BABEL + '/lib/index.js']
        with self.assertRaisesRegex(ValueError, 'inventory does not match'):
            self.validate()

    def directory_mapping(self, target='./regenerator/', key='./regenerator/'):
        metadata = json.loads((self.modules / 'jsdom/package.json').read_text())
        metadata['exports'] = {key: target}
        self.write('jsdom/package.json', json.dumps(metadata))

    def test_legacy_folder_export_requires_a_nonempty_bound_directory(self):
        self.directory_mapping()
        self.write('jsdom/regenerator/index.js', '// Published legacy folder export.\n')
        self.validate(self.bind())
        (self.modules / 'jsdom/regenerator/index.js').unlink()
        with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
            self.validate(self.bind())

    def test_folder_export_does_not_allow_an_ordinary_missing_file_target(self):
        self.write('jsdom/regenerator/index.js', '// Bound directory contents.\n')
        for key, target in (('./regenerator', './regenerator/'),
                            ('./regenerator/', './regenerator')):
            with self.subTest(key=key, target=target):
                self.directory_mapping(target, key)
                with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
                    self.validate(self.bind())

    def test_folder_export_keeps_file_checksums_and_exact_inventory(self):
        self.directory_mapping()
        self.write('jsdom/regenerator/index.js', '// Bound directory contents.\n')
        self.frontend = self.bind()
        self.write('jsdom/regenerator/index.js', '// Altered directory contents.\n')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.validate()
        self.frontend = self.bind()
        del self.frontend['nodeModulesFiles']['jsdom/regenerator/index.js']
        with self.assertRaisesRegex(ValueError, 'inventory does not match'):
            self.validate()

    def test_published_source_exports_require_exact_lock_metadata_and_route(self):
        cases = [('@standard-schema/spec', 'standard-schema-spec-1.1.0.package.json',
                  'sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==',
                  './src/index.ts'),
                 ('vitest', 'vitest-4.1.11.package.json',
                  'sha512-fhACrNXUidIbGSBr5FlbuBkO7VWC1ZyLl0DO4CU2DrQoAPxX84Ysxs+HeGQpii5lZWV1Q4gBZTTu49mF+A6Edw==',
                  './src/*')]
        for name, filename, integrity, absent in cases:
            with self.subTest(package=name):
                raw = (METADATA.parent / filename).read_bytes()
                metadata = json.loads(raw)
                self.write(name + '/package.json', raw)
                targets = list(metadata['exports'].values())
                while targets:
                    value = targets.pop()
                    if isinstance(value, dict):
                        targets.extend(value.values())
                    elif isinstance(value, str) and value != absent:
                        relative = value.removeprefix('./').replace('*', 'fixture')
                        if relative != 'package.json':
                            self.write(name + '/' + relative, '// Published export fixture.\n')
                for executable, entry in metadata.get('bin', {}).items():
                    self.write(name + '/' + entry.removeprefix('./'), '// Bound executable.\n')
                    self.write('.bin/' + executable, '#!/bin/sh\nexit 0\n')
                    (self.modules / '.bin' / executable).chmod(0o755)
                locked = dict(version=metadata['version'], integrity=integrity)
                self.lock['packages']['node_modules/' + name] = locked
                self.validate(self.bind())
                for mutation in ('integrity', 'metadata', 'other_export'):
                    with self.subTest(mutation=mutation):
                        if mutation == 'integrity':
                            locked['integrity'] = 'sha512-' + 'A' * 88
                        elif mutation == 'metadata':
                            self.write(name + '/package.json', raw + b'\n')
                        else:
                            changed = json.loads(raw)
                            changed['exports']['./also-missing'] = './also-missing.js'
                            self.write(name + '/package.json', json.dumps(changed))
                        with self.assertRaisesRegex(ValueError, 'frontend package export is missing'):
                            self.validate(self.bind())
                        locked['integrity'] = integrity
                        self.write(name + '/package.json', raw)


if __name__ == '__main__':
    unittest.main()
