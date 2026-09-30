"""Safety boundaries for completing a staged toolkit; no network or builds."""
import importlib.util
import io
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
SPEC = importlib.util.spec_from_file_location('prepare_linux_toolkit', ROOT / 'scripts/prepare-linux-toolkit.py')
prepare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare)


class RustComponentSafetyTests(unittest.TestCase):
    def layout(self, root):
        for relative in ['payload/bin', 'payload/lib', 'payload/frontend/node_modules',
                         'payload/native', 'payload/cargo', 'payload/vendor']:
            (root / relative).mkdir(parents=True, exist_ok=True)

    def archive(self, root, entries):
        archive = root / 'component.tar.gz'
        with tarfile.open(archive, 'w:gz') as bundle:
            for name, kind in entries:
                member = tarfile.TarInfo(name)
                if kind == 'link':
                    member.type = tarfile.SYMTYPE
                    member.linkname = '../outside'
                    bundle.addfile(member)
                else:
                    member.size = 4
                    member.mode = 0o755
                    bundle.addfile(member, io.BytesIO(b'rust'))
        return archive

    def test_rejects_traversal_before_creating_destination(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = self.archive(root, [('component/bin/rustc', 'file'), ('../escaped', 'file')])
            destination = root / 'extract'
            with self.assertRaises(ValueError):
                prepare.safe_extract_rust(archive, destination)
            self.assertFalse(destination.exists())
            self.assertFalse((root.parent / 'escaped').exists())

    def test_rejects_symlink_members_before_extraction(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = self.archive(root, [('component/lib', 'link')])
            with self.assertRaises(ValueError):
                prepare.safe_extract_rust(archive, root / 'extract')
            self.assertFalse((root / 'extract').exists())

    def test_preserves_regular_component_bytes_and_executable_mode(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = self.archive(root, [('component/bin/rustc', 'file')])
            destination = root / 'extract'
            prepare.safe_extract_rust(archive, destination)
            binary = destination / 'component/bin/rustc'
            self.assertEqual(binary.read_bytes(), b'rust')
            self.assertEqual(binary.stat().st_mode & 0o777, 0o755)

    def test_rejects_old_or_nonexact_rust_versions(self):
        for version in ['1.90.9', 'stable', '1.91', '../1.91.0', '1.91.0?query']:
            with self.subTest(version=version), self.assertRaises(ValueError):
                prepare.checked_rust_version(version)
        self.assertEqual(prepare.checked_rust_version('1.91.0'), '1.91.0')

    def test_external_tool_link_is_rejected_before_execution(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'toolkit'
            self.layout(root)
            external = Path(temp) / 'external-node'
            external.write_text('untrusted binary')
            (root / 'payload/bin/node').symlink_to(external)
            with patch.object(prepare.subprocess, 'check_output') as execute:
                with self.assertRaises(ValueError):
                    prepare.manifest_for(root, prepare.TARGET)
            execute.assert_not_called()

    def test_linked_install_parent_is_rejected_before_download(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'toolkit'
            self.layout(root)
            external = Path(temp) / 'external-lib'
            external.mkdir()
            (root / 'payload/lib').rmdir()
            (root / 'payload/lib').symlink_to(external)
            with patch.object(prepare, 'download') as download:
                with self.assertRaises(ValueError):
                    prepare.complete_rust(root, '1.91.0', prepare.TARGET)
            download.assert_not_called()
            self.assertEqual(list(external.iterdir()), [])

    def test_existing_accepted_manifest_and_archive_are_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'toolkit'
            self.layout(root)
            (root / 'archives').mkdir()
            archive = root / 'archives/linux-toolkit.tar.gz'
            manifest = root / 'toolkit.json'
            archive.write_bytes(b'previous archive')
            manifest.write_bytes(b'previous manifest')
            with patch.object(prepare, 'manifest_for') as inventory:
                with self.assertRaises(ValueError):
                    prepare.seal(root, prepare.TARGET, partial=False)
            inventory.assert_not_called()
            self.assertEqual(archive.read_bytes(), b'previous archive')
            self.assertEqual(manifest.read_bytes(), b'previous manifest')


if __name__ == '__main__':
    unittest.main()
