"""macOS binaries are published only with the Developer ID and stable identifiers."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SIGN = ROOT / "scripts/macos-sign-cli.sh"
CLI = ("kaiki", "kaiki-agentic-node", "agentic-cli", "agentic-mcp")
APP = ("agentic-desktop", *CLI)
IDENTIFIERS = {"agentic-desktop": "net.agenticinternet.desktop", "kaiki": "net.agenticinternet.desktop",
               # The daemon's file is named for Activity Monitor; its identifier stays.
               "kaiki-agentic-node": "net.agenticinternet.agentic-node",
               "agentic-cli": "net.agenticinternet.agentic-cli",
               "agentic-mcp": "net.agenticinternet.agentic-mcp"}
DEVELOPER_ID = "Developer ID Application: Test (TESTTEAM)"

# codesign and security as far as the script uses them: signatures live in a
# JSON file, so nothing reaches the Keychain.
MOCK = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
name, args = pathlib.Path(sys.argv[0]).name, sys.argv[1:]
with open(os.environ["CALLS"], "a") as log:
    log.write(json.dumps([name, *args]) + "\n")
state = pathlib.Path(os.environ["SIGNATURES"])
signatures = json.loads(state.read_text()) if state.exists() else {}
if name == "security":
    print(f'  1) ABCDEF "{os.environ["IDENTITY"]}"')
elif "--sign" in args:
    signatures[args[-1]] = {"identifier": args[args.index("--identifier") + 1],
                            "identity": args[args.index("--sign") + 1],
                            "runtime": "runtime" in args, "timestamp": "--timestamp" in args}
    state.write_text(json.dumps(signatures))
elif args[-1] not in signatures:
    sys.exit(f"{args[-1]}: code object is not signed at all")
elif args[0] == "-dvv":
    signature = signatures[args[-1]]
    flags = "0x10000(runtime)" if signature["runtime"] else "0x0(none)"
    lines = [f"Identifier={signature['identifier']}", f"CodeDirectory v=20500 flags={flags}"]
    if signature["identity"] == "-":
        lines += ["Signature=adhoc", "TeamIdentifier=not set"]
    else:
        team = signature["identity"].rsplit("(", 1)[1].rstrip(")")
        lines += [f"Authority={signature['identity']}", "Authority=Apple Root CA",
                  "Timestamp=30. Sep 2026 at 23.20.34" if signature["timestamp"] else "Signed Time=now",
                  f"TeamIdentifier={team}"]
    print("\n".join(lines), file=sys.stderr)
'''


def work_directory(test):
    (ROOT / "output").mkdir(exist_ok=True)
    directory = tempfile.TemporaryDirectory(dir=ROOT / "output")
    test.addCleanup(directory.cleanup)
    return Path(directory.name)


class IdentifierTable(unittest.TestCase):
    def test_kaiki_shares_the_apps_identifier_the_others_keep_their_own(self):
        app = json.loads((ROOT / "apps/desktop/src-tauri/tauri.conf.json").read_text())["identifier"]
        self.assertEqual(IDENTIFIERS["kaiki"], app)
        for name, identifier in IDENTIFIERS.items():
            printed = subprocess.run(["bash", str(SIGN), "--identifier", name],
                                     capture_output=True, text=True, check=True).stdout.strip()
            self.assertEqual(printed, identifier, name)


class MockedSigning(unittest.TestCase):
    def setUp(self):
        self.work = work_directory(self)
        tools = self.work / "tools"
        tools.mkdir()
        (tools / "mock").write_text(MOCK)
        (tools / "mock").chmod(0o755)
        for name in ("codesign", "security"):
            (tools / name).symlink_to(tools / "mock")
        self.calls = self.work / "calls.jsonl"
        self.signatures = self.work / "signatures.json"
        self.env = {**os.environ, "PATH": f"{tools}:{os.environ['PATH']}", "CALLS": str(self.calls),
                    "SIGNATURES": str(self.signatures), "IDENTITY": DEVELOPER_ID}
        self.env.pop("APPLE_SIGNING_IDENTITY", None)

    def binaries(self, directory, names):
        directory.mkdir(parents=True)
        for name in names:
            (directory / name).write_text("binary")
            (directory / name).chmod(0o755)
        return directory

    def run_script(self, *args):
        return subprocess.run(["bash", str(SIGN), *map(str, args)], env=self.env, capture_output=True, text=True)

    def test_signs_each_binary_with_its_identifier_then_passes_the_check(self):
        release = self.binaries(self.work / "release", CLI)
        result = self.run_script(release)
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()]
        signs = [call for call in calls if call[:2] == ["codesign", "--force"]]
        self.assertEqual([Path(call[-1]).name for call in signs], list(CLI))
        for call in signs:
            self.assertEqual(call[call.index("--identifier") + 1], IDENTIFIERS[Path(call[-1]).name])
            self.assertEqual(call[call.index("--sign") + 1], DEVELOPER_ID)
            self.assertEqual(call[call.index("--options") + 1], "runtime")
            self.assertIn("--timestamp", call)
        self.assertTrue(any(call[:2] == ["codesign", "-dvv"] for call in calls))

    def test_check_refuses_a_developer_id_binary_with_another_identifier(self):
        release = self.binaries(self.work / "release", CLI)
        app = self.work / "Kaiki Chat.app"
        executables = self.binaries(app / "Contents/MacOS", APP)
        signatures = {str(directory / name): {"identifier": IDENTIFIERS[name], "identity": DEVELOPER_ID,
                                              "runtime": True, "timestamp": True}
                      for directory, names in ((release, CLI), (executables, APP)) for name in names}
        signatures[str(app)] = signatures[str(executables / "agentic-desktop")]
        signatures[str(release / "kaiki")]["identifier"] = "net.agenticinternet.kaiki"
        self.signatures.write_text(json.dumps(signatures))
        result = self.run_script("--check", release, app)
        self.assertNotEqual(result.returncode, 0)
        refusals = [line for line in result.stderr.splitlines() if line.startswith("refused")]
        self.assertEqual(len(refusals), 1, result.stderr)
        self.assertIn(f"{release}/kaiki", refusals[0])
        self.assertIn("net.agenticinternet.desktop", refusals[0])


@unittest.skipUnless(sys.platform == "darwin" and shutil.which("cc") and shutil.which("codesign"),
                     "needs macOS cc and codesign")
class AdHocBinaries(unittest.TestCase):
    """Real codesign, ad-hoc only (`--sign -`): no identity, no Keychain."""

    def test_publish_refuses_ad_hoc_binaries_before_packing_or_the_server(self):
        work = work_directory(self)
        (work / "main.c").write_text("int main(void) { return 0; }\n")
        subprocess.run(["cc", "-o", str(work / "dummy"), str(work / "main.c")], check=True)
        release, app = work / "release", work / "Kaiki Chat.app"
        for directory, names in ((release, CLI), (app / "Contents/MacOS", APP)):
            directory.mkdir(parents=True)
            for name in names:
                shutil.copy2(work / "dummy", directory / name)  # the linker's ad-hoc signature
        # Ad-hoc even with the right identifier, and one not signed at all.
        for name in ("kaiki", "kaiki-agentic-node", "agentic-cli"):
            subprocess.run(["codesign", "--force", "--sign", "-", "--identifier", IDENTIFIERS[name],
                            str(release / name)], check=True, capture_output=True)
        subprocess.run(["codesign", "--remove-signature", str(release / "agentic-mcp")], check=True)
        tools = work / "tools"
        tools.mkdir()
        for name in ("ssh", "scp", "curl"):
            (tools / name).write_text(f'#!/bin/sh\necho {name} >> "{work}/network"\nexit 1\n')
            (tools / name).chmod(0o755)
        (work / "env").write_text("PROD_SSH_HOST=127.0.0.1\nSITE_URL=http://127.0.0.1:9\n")
        release_json = ROOT / "deployments/release.json"
        before = release_json.read_bytes() if release_json.exists() else None
        env = {**os.environ, "PATH": f"{tools}:{os.environ['PATH']}", "KAIKI_ENV_FILE": str(work / "env")}
        result = subprocess.run(["bash", str(ROOT / "scripts/publish-cli.sh"), "macos-arm64", str(release),
                                 "app-macos-arm64", str(app)], env=env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        refusals = [line for line in result.stderr.splitlines() if line.startswith("refused")]
        expected = [release / name for name in CLI] + [app / "Contents/MacOS" / name for name in APP]
        for path in expected:
            self.assertTrue(any(f"{path}:" in line for line in refusals), f"{path} not refused:\n{result.stderr}")
        self.assertIn("ad-hoc", "\n".join(line for line in refusals if f"{release}/kaiki:" in line))
        self.assertNotIn("packed", result.stdout)
        self.assertFalse((work / "network").exists())
        self.assertEqual(release_json.read_bytes() if release_json.exists() else None, before)


if __name__ == "__main__":
    unittest.main()
