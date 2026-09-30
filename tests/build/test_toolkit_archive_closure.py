"""A verified toolkit archive binds the complete unpacked payload topology."""
import io
import os
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
import build_evidence as evidence


class ToolkitArchiveClosureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / 'toolkit'
        self.root.mkdir()
        self.archive = self.root / 'archives/toolkit.tar'
        self.archive.parent.mkdir()
        self.library = self.root / 'payload/native/lib/libfixture.so.1'
        self.library.parent.mkdir(parents=True)
        self.library.write_bytes(b'checksum-bound native library\n')
        self.library.chmod(0o644)
        self.link = self.library.with_name('libfixture.so')
        self.link.symlink_to(self.library.name)
        self.required = {self.library.relative_to(self.root).as_posix()}

    def pack(self, *, implicit_directories=False, extra_members=()):
        with tarfile.open(self.archive, 'w') as bundle:
            if implicit_directories:
                for path in (self.library, self.link):
                    bundle.add(path, arcname=path.relative_to(self.root).as_posix(), recursive=False)
            else:
                bundle.add(self.root / 'payload', arcname='payload', recursive=True)
            for path in extra_members:
                bundle.add(path, arcname=path.relative_to(self.root).as_posix(), recursive=False)

    def validate(self):
        evidence._check_toolkit_archive(self.root, self.archive, self.required)

    def assert_unbound(self, path):
        with self.assertRaisesRegex(ValueError, 'absent from.*archive') as caught:
            self.validate()
        self.assertIn(path.relative_to(self.root).as_posix(), str(caught.exception))

    def test_accepts_complete_files_directories_and_relative_links(self):
        (self.root / 'payload/empty').mkdir()
        self.pack()
        self.validate()

    def test_accepts_implicit_parent_directories_in_small_archives(self):
        self.pack(implicit_directories=True)
        with tarfile.open(self.archive) as bundle:
            self.assertFalse(any(member.isdir() for member in bundle))
        self.validate()

    def test_rejects_extra_native_library_without_changing_the_archive(self):
        self.pack()
        original = self.archive.read_bytes()
        extra = self.library.with_name('libinjected.so')
        extra.write_bytes(b'unbound dynamic-library candidate\n')
        self.assert_unbound(extra)
        self.assertEqual(self.archive.read_bytes(), original)
        self.assertEqual(extra.read_bytes(), b'unbound dynamic-library candidate\n')

    def test_rejects_extra_symlink_even_when_it_points_to_an_archived_file(self):
        self.pack()
        extra = self.library.with_name('libinjected.so')
        extra.symlink_to(self.library.name)
        self.assert_unbound(extra)
        self.assertEqual(os.readlink(extra), self.library.name)

    def test_rejects_extra_dangling_symlink(self):
        self.pack()
        extra = self.library.with_name('libabsent.so')
        extra.symlink_to('missing-target.so')
        self.assert_unbound(extra)
        self.assertTrue(extra.is_symlink())

    def test_rejects_extra_empty_directory(self):
        self.pack()
        extra = self.root / 'payload/native/injected-search-path'
        extra.mkdir()
        self.assert_unbound(extra)
        self.assertTrue(extra.is_dir())

    def test_implicit_parent_is_a_closed_subtree_not_only_an_access_path(self):
        self.pack(implicit_directories=True)
        extra = self.root / 'payload/other-runtime'
        extra.mkdir()
        self.assert_unbound(extra)

    def test_checks_each_archived_top_level_subtree(self):
        support = self.root / 'support/activation.sh'
        support.parent.mkdir()
        support.write_bytes(b'# bound activation fixture\n')
        self.pack(extra_members=(support,))
        self.validate()
        extra = support.with_name('injected.sh')
        extra.write_bytes(b'# unbound activation candidate\n')
        self.assert_unbound(extra)

    def test_does_not_walk_archived_directory_symlinks(self):
        # Following this accepted link would traverse native again indefinitely.
        alias = self.root / 'payload/native/alias'
        alias.symlink_to('.', target_is_directory=True)
        self.pack()
        scandir = os.scandir

        def guarded_scandir(path):
            path = Path(path)
            self.assertFalse(path == alias or alias in path.parents,
                'archive validation followed a directory symlink')
            return scandir(path)

        with mock.patch.object(evidence.os, 'scandir', side_effect=guarded_scandir):
            self.validate()

    def test_mutable_cache_archive_and_manifest_outside_payload_are_not_scanned(self):
        cache = self.root / 'cache/cargo'
        cache.mkdir(parents=True)
        (cache / 'mutable-index').write_bytes(b'not part of the distributed toolkit\n')
        (self.root / 'toolkit.json').write_text('{"fixture": true}\n')
        (self.root / 'archives/another.tar').write_bytes(b'unrelated previous download\n')
        # A textual prefix is not a path boundary of the archived payload.
        sibling = self.root / 'payload-backup'
        sibling.mkdir()
        (sibling / 'unbound-but-outside').write_bytes(b'outside archived subtree\n')
        self.pack()
        scandir = os.scandir

        def guarded_scandir(path):
            path = Path(path)
            self.assertTrue(path == self.root / 'payload' or self.root / 'payload' in path.parents,
                f'archive validation scanned outside its bound payload: {path}')
            return scandir(path)

        with mock.patch.object(evidence.os, 'scandir', side_effect=guarded_scandir):
            self.validate()
            (cache / 'mutable-index').write_bytes(b'cache changed after validation\n')
            self.validate()

    def test_top_level_regular_file_does_not_bind_the_entire_toolkit_root(self):
        bound = self.root / 'activation.sh'
        bound.write_bytes(b'# standalone bound file\n')
        self.pack(extra_members=(bound,))
        (self.root / 'unrelated.txt').write_bytes(b'not beneath an archived directory\n')
        with mock.patch.object(evidence.os, 'scandir', wraps=os.scandir) as walks:
            self.validate()
        self.assertNotIn(self.root, [Path(call.args[0]) for call in walks.call_args_list])

    def test_malformed_member_boundaries_cannot_select_a_broader_root(self):
        for name in ('.', '/', '../outside', '/payload', 'payload/../toolkit.json',
                     'payload//native', 'payload/./native', 'payload\\native'):
            with self.subTest(member=name):
                self.pack()
                with tarfile.open(self.archive, 'a') as bundle:
                    member = tarfile.TarInfo(name)
                    member.size = 1
                    bundle.addfile(member, io.BytesIO(b'x'))
                with self.assertRaises(ValueError), \
                        mock.patch.object(evidence.os, 'scandir',
                            side_effect=AssertionError('malformed archive started a filesystem walk')):
                    self.validate()

    def test_existing_member_byte_mode_and_link_identity_checks_remain_mandatory(self):
        self.pack()
        original = self.library.read_bytes()
        self.library.write_bytes(original + b'changed')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.validate()
        self.library.write_bytes(original)
        self.library.chmod(0o755)
        with self.assertRaisesRegex(ValueError, 'type/mode'):
            self.validate()
        self.library.chmod(0o644)
        self.link.unlink()
        self.link.write_bytes(original)
        with self.assertRaisesRegex(ValueError, 'type/mode'):
            self.validate()


if __name__ == '__main__':
    unittest.main()
