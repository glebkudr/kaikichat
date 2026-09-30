"""scripts/network-preset.py builds the preset kaikichat.com serves.

The signer is a stand-in that wraps the payload as it is, so these tests see
exactly what would be signed: the welcome agent and lobby, and the channels
and groups the network recommends to a new profile, taken from the welcome
agent's welcome.json (deploy/node/welcome.py).
"""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "network-preset.py"
AGENT = "ain1" + "ab" * 32
LOBBY = "cd" * 32
NEWS = "ef" * 32
SIGNER = """#!/usr/bin/env python3
import json, sys
if sys.argv[1] == "sign":
    print(json.dumps({"preset": open(sys.argv[3]).read(), "signature": "00"}))
else:
    print("ok")
"""


class NetworkPreset(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="ain-preset-script-"))
        self.signer = self.dir / "signer"
        self.signer.write_text(SIGNER)
        self.signer.chmod(0o700)
        (self.dir / "seed").write_text("00" * 32)
        (self.dir / "published.json").write_text(json.dumps([
            {"node": n, "route": f"/ip4/203.0.113.1/udp/{4100 + n}/quic-v1/p2p/peer{n}"}
            for n in (1, 2)]))
        (self.dir / "manifest.json").write_text(json.dumps({"nodeFlags": [
            "--chain-rpc", "https://rpc.example", "--chain-id", "8453",
            "--book-shop", "0x" + "01" * 20, "--grant-issuer", "0x" + "02" * 20,
            "--registry", "0x" + "03" * 20, "--chain-confirmations", "5"]}))

    def tearDown(self):
        for path in sorted(self.dir.rglob("*"), reverse=True):
            path.unlink() if path.is_file() else path.rmdir()
        self.dir.rmdir()

    def preset(self, welcome):
        (self.dir / "welcome.json").write_text(json.dumps(welcome))
        out = self.dir / "network.json"
        subprocess.run(
            [sys.executable, str(SCRIPT), "--published", str(self.dir / "published.json"),
             "--seed", str(self.dir / "seed"), "--signer", str(self.signer),
             "--manifest", str(self.dir / "manifest.json"), "--out", str(out),
             "--welcome", str(self.dir / "welcome.json"),
             "--release", str(self.dir / "no-release.json")],
            check=True, capture_output=True, text=True)
        return json.loads(json.loads(out.read_text())["preset"])

    def test_the_news_channel_and_the_lobby_are_recommended_in_that_order(self):
        preset = self.preset({"agent": AGENT, "name": "Kaiki welcome", "lobby": LOBBY,
                              "lobbyName": "Kaiki Lobby", "news": NEWS,
                              "newsName": "Kaiki News"})
        self.assertEqual(preset["recommended"], [
            {"kind": "channel", "ref": NEWS, "owner": AGENT, "name": "Kaiki News"},
            {"kind": "group", "ref": LOBBY, "owner": AGENT, "name": "Kaiki Lobby"},
        ])
        # `welcome` keeps its own four fields: the news is a recommendation.
        self.assertEqual(preset["welcome"], {"agent": AGENT, "name": "Kaiki welcome",
                                             "lobby": LOBBY, "lobbyName": "Kaiki Lobby"})

    def test_without_a_news_channel_the_lobby_alone_is_recommended(self):
        preset = self.preset({"agent": AGENT, "name": "Kaiki welcome", "lobby": LOBBY,
                              "lobbyName": "Kaiki Lobby"})
        self.assertEqual(preset["recommended"], [
            {"kind": "group", "ref": LOBBY, "owner": AGENT, "name": "Kaiki Lobby"},
        ])


if __name__ == "__main__":
    unittest.main()
