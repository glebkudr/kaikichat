#!/usr/bin/env python3
"""Kaiki Chat's network preset (Docs/V1_NETWORK_PRESET_2026_09_28_RU.md).

Builds the preset of the public network from deployments/base.json
(`nodeFlags`; Base mainnet since 2026-09-30, `--manifest
deployments/base-sepolia.json --network kaiki-testnet-base-sepolia --name
"Kaiki testnet (Base Sepolia)"` for the former testnet), the nodes' routes (their /data/published.json), the identity
server and the app's latest release (deployments/release.json, written by
scripts/publish-cli.sh), with a serial one above the file it replaces, and
signs it with `kaiki-preset` and the offline key:

    network-preset.py --published published.json --seed SEED \\
        [--directory URL --directory-key HEX] [--welcome welcome.json] \\
        [--release deployments/release.json] \\
        [--signer target/debug/kaiki-preset] [--out deploy/site/network.json]
"""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NETWORK = "kaiki-base"
NAME = "Kaiki Chat (Base)"
IDENTITY = "https://id.kaikichat.com"
MIN_VERSION = "0.1.0"
FLAGS = {
    "--chain-rpc": "chainRpc",
    "--chain-id": "chainId",
    "--book-shop": "bookShop",
    "--grant-issuer": "grantIssuer",
    "--registry": "registry",
    "--chain-confirmations": "chainConfirmations",
    "--operator-pool": "operatorPool",
}


def chain(manifest):
    flags = manifest["nodeFlags"]
    fields = {}
    for flag, value in zip(flags[::2], flags[1::2]):
        field = FLAGS[flag]
        fields[field] = int(value) if field in ("chainId", "chainConfirmations") else value
    return fields


def serial_after(out):
    try:
        envelope = json.loads(out.read_text())
        return json.loads(envelope["preset"])["serial"] + 1
    except FileNotFoundError:
        return 1


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--published", type=Path, required=True)
    parser.add_argument("--seed", type=Path, required=True)
    parser.add_argument("--signer", type=Path, default=ROOT / "target/debug/kaiki-preset")
    parser.add_argument("--manifest", type=Path, default=ROOT / "deployments/base.json")
    parser.add_argument("--network", default=NETWORK, help="the network's id; another id is a switch")
    parser.add_argument("--name", default=NAME, help="the network's name shown to people")
    parser.add_argument("--chain-rpc", help="the public JSON-RPC clients use (default: the manifest's)")
    parser.add_argument("--out", type=Path, default=ROOT / "deploy/site/network.json")
    parser.add_argument("--directory", help="the discovery service's URL")
    parser.add_argument("--directory-key", help="the key it signs bindings with (64 hex)")
    parser.add_argument("--welcome", type=Path,
                        help="the welcome agent's welcome.json (deploy/node/welcome.py)")
    parser.add_argument("--release", type=Path, default=ROOT / "deployments/release.json",
                        help="the latest release and its builds, when the file exists")
    args = parser.parse_args()
    routes = [node["route"] for node in json.loads(args.published.read_text())]
    preset = {
        "network": args.network,
        "name": args.name,
        "serial": serial_after(args.out),
        "minVersion": MIN_VERSION,
        "bootstrap": routes,
        **chain(json.loads(args.manifest.read_text())),
        "identityServer": IDENTITY,
    }
    if args.chain_rpc:
        preset["chainRpc"] = args.chain_rpc
    if args.directory:
        preset["directory"] = args.directory
    if args.directory_key:
        preset["directoryKey"] = args.directory_key
    if args.welcome:
        welcome = json.loads(args.welcome.read_text())
        preset["welcome"] = {key: welcome[key] for key in ("agent", "name", "lobby", "lobbyName")}
    if args.release.exists():
        release = json.loads(args.release.read_text())
        preset["release"] = {"version": release["version"], "builds": release["builds"]}
    with tempfile.NamedTemporaryFile("w", suffix=".json") as payload:
        payload.write(json.dumps(preset, separators=(",", ":")))
        payload.flush()
        signed = subprocess.run(
            [str(args.signer), "sign", str(args.seed), payload.name],
            check=True, capture_output=True, text=True,
        ).stdout
    args.out.write_text(signed)
    checked = subprocess.run(
        [str(args.signer), "verify", str(args.out)],
        check=True, capture_output=True, text=True,
    ).stdout
    print(checked, file=sys.stderr)
    released = preset.get("release", {}).get("version", "none")
    print(f"{args.out}: serial {preset['serial']}, {len(routes)} routes, release {released}")


if __name__ == "__main__":
    main()
