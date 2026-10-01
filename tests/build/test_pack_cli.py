"""The command line's archive (scripts/pack-cli.sh): what install.sh unpacks and
every kaiki that updates itself checks, the ones from before the daemon was
named kaiki-agentic-node included."""
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / "scripts/pack-cli.sh"
CLI = ("kaiki", "kaiki-agentic-node", "agentic-cli", "agentic-mcp")


def work_directory(test):
    (ROOT / "output").mkdir(exist_ok=True)
    directory = tempfile.TemporaryDirectory(dir=ROOT / "output")
    test.addCleanup(directory.cleanup)
    return Path(directory.name)


def release(work, names):
    """A cargo release directory with `names` built; it still holds the
    daemon's binary under its former name from an earlier build."""
    build = work / "release"
    build.mkdir()
    (build / "agentic-node").write_text("#!/bin/sh\necho stale build\n")
    for name in names:
        (build / name).write_text(f"#!/bin/sh\necho {name}\n")
        (build / name).chmod(0o755)
    return build


def pack(build, out):
    return subprocess.run(["bash", str(PACK), "macos-arm64", str(build), str(out)],
                          capture_output=True, text=True)


class PackCli(unittest.TestCase):
    def test_the_archive_holds_the_command_line_and_the_name_older_updaters_check(self):
        work = work_directory(self)
        result = pack(release(work, CLI), work / "out")
        self.assertEqual(result.returncode, 0, result.stderr)
        archive = work / "out/kaiki-macos-arm64.tar.gz"
        with tarfile.open(archive) as tar:
            members = {member.name: member for member in tar.getmembers()}
        self.assertEqual(sorted(members), ["kaiki", *sorted(f"kaiki/{name}" for name in
                                                            (*CLI, "agentic-node", "install.json"))])
        for name in CLI:
            self.assertTrue(members[f"kaiki/{name}"].isfile(), name)
            self.assertEqual(members[f"kaiki/{name}"].mode & 0o777, 0o755, name)
        # kaiki 0.2.4 and older refuse an update without agentic-node: the
        # name stays, as a link to the daemon, not the stale build beside it.
        former = members["kaiki/agentic-node"]
        self.assertTrue(former.issym())
        self.assertEqual(former.linkname, "kaiki-agentic-node")

        # Unpacked as such a kaiki unpacks it, the files it checks are there.
        unpacked = work / "unpacked"
        unpacked.mkdir()
        subprocess.run(["tar", "-xzf", str(archive), "-C", str(unpacked)], check=True)
        kaiki = unpacked / "kaiki"
        for name in ("kaiki", "agentic-node", "install.json"):
            self.assertTrue((kaiki / name).is_file(), name)
        self.assertEqual((kaiki / "agentic-node").read_text(), "#!/bin/sh\necho kaiki-agentic-node\n")
        self.assertEqual(json.loads((kaiki / "install.json").read_text()), {"build": "cli-macos-arm64"})

    def test_publishing_packs_the_command_line_with_this_script(self):
        publish = (ROOT / "scripts/publish-cli.sh").read_text()
        self.assertIn("pack-cli.sh", publish)
        # No packing of its own: the binaries' names are this script's.
        for name in ("kaiki-agentic-node", "agentic-node", "agentic-cli", "agentic-mcp"):
            self.assertNotIn(name, publish)


if __name__ == "__main__":
    unittest.main()
