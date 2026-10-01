"""Release signing order and refusal to publish a rejected/not Developer ID app."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MacosNotarize(unittest.TestCase):
    def run_signing(self, identity=True, status="Accepted"):
        (ROOT / "output").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "output") as directory:
            work = Path(directory)
            app = work / "Kaiki Chat.app"
            binaries = app / "Contents" / "MacOS"
            binaries.mkdir(parents=True)
            for name in ("agentic-desktop", "kaiki-agentic-node", "agentic-cli", "agentic-mcp", "kaiki"):
                path = binaries / name
                path.write_text("binary")
                path.chmod(0o755)
            tools = work / "tools"
            tools.mkdir()
            mock = tools / "mock"
            mock.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ["CALLS"], "a") as log:
    log.write(json.dumps([name, *args]) + "\\n")
if name == "security" and os.environ["HAS_IDENTITY"] == "1":
    print('1) ABCDEF "Developer ID Application: Test (TESTTEAM)"')
elif name == "xcrun" and args[:2] == ["notarytool", "submit"]:
    print(json.dumps({"id": "submission-id", "status": os.environ["NOTARY_STATUS"]}))
elif name == "xcrun" and args[:2] == ["stapler", "staple"]:
    pathlib.Path(args[2], "ticket").write_text("stapled")
elif name == "ditto":
    app, archive = map(pathlib.Path, args[-2:])
    archive.write_text("stapled" if (app / "ticket").exists() else "unstapled")
''')
            mock.chmod(0o755)
            for name in ("security", "codesign", "xcrun", "ditto", "spctl"):
                (tools / name).symlink_to(mock)
            calls = work / "calls.jsonl"
            out = work / "release"
            env = {**os.environ, "PATH": f"{tools}:{os.environ['PATH']}",
                   "CALLS": str(calls), "HAS_IDENTITY": str(int(identity)),
                   "NOTARY_STATUS": status}
            env.pop("APPLE_SIGNING_IDENTITY", None)
            result = subprocess.run(["bash", str(ROOT / "scripts/macos-notarize.sh"),
                                     str(app), str(out)], env=env, capture_output=True, text=True)
            commands = [json.loads(line) for line in calls.read_text().splitlines()]
            archive = out / "kaiki-chat-macos-arm64.zip"
            return result, commands, archive.read_text() if archive.exists() else None

    def test_signs_every_binary_then_bundle_and_archives_after_stapling(self):
        result, commands, archive = self.run_signing()
        self.assertEqual(result.returncode, 0, result.stderr)
        signs = [call for call in commands if call[:2] == ["codesign", "--force"]]
        self.assertEqual([Path(call[-1]).name for call in signs],
                         ["agentic-desktop", "kaiki-agentic-node", "agentic-cli", "agentic-mcp", "kaiki", "Kaiki Chat.app"])
        for call in signs:
            self.assertIn("--timestamp", call)
            self.assertEqual(call[call.index("--options") + 1], "runtime")
            self.assertNotIn("--deep", call)
        # Stable identifiers; kaiki shares the app's for the Keychain.
        self.assertEqual({Path(call[-1]).name: call[call.index("--identifier") + 1] for call in signs[:-1]},
                         {"agentic-desktop": "net.agenticinternet.desktop", "kaiki": "net.agenticinternet.desktop",
                          "kaiki-agentic-node": "net.agenticinternet.agentic-node",
                          **{name: f"net.agenticinternet.{name}" for name in ("agentic-cli", "agentic-mcp")}})
        self.assertEqual(archive, "stapled")
        self.assertTrue(any(call[:2] == ["spctl", "--assess"] for call in commands))

    def test_missing_developer_id_does_not_sign_or_package(self):
        result, commands, archive = self.run_signing(identity=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Developer ID Application", result.stderr)
        self.assertFalse(any(call[0] in ("codesign", "ditto") for call in commands))
        self.assertIsNone(archive)

    def test_rejected_notarization_does_not_staple_or_create_release_archive(self):
        result, commands, archive = self.run_signing(status="Invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(any(call[:3] == ["xcrun", "notarytool", "log"] for call in commands))
        self.assertFalse(any(call[:2] == ["xcrun", "stapler"] for call in commands))
        self.assertIsNone(archive)


if __name__ == "__main__":
    unittest.main()
