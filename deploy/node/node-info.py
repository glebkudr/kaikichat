#!/usr/bin/env python3
"""Ask a running kaiki-agentic-node for `node_info` over its IPC socket and print
what the operator needs: its public route and its unit commitment.

    node-info.py DATA_DIR PUBLIC_IP PORT

Waits up to a minute for the node. Prints one JSON line:
{"peerId", "route", "commitment", "account"}.
"""
import json
import socket
import struct
import sys
import time


def call(path: str, token: str, method: str) -> dict:
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(5)
        s.connect(path)
        wire = json.dumps({"token": token, "method": method, "request": {}}).encode()
        s.sendall(struct.pack(">I", len(wire)) + wire)
        (size,) = struct.unpack(">I", s.recv(4, socket.MSG_WAITALL))
        return json.loads(s.recv(size, socket.MSG_WAITALL))


def main() -> None:
    data, ip, port = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(f"{data}/secrets.json") as f:
        token = json.load(f)["ownerToken"]
    deadline = time.monotonic() + 60
    while True:
        try:
            info = call(f"{data}/ipc.sock", token, "node_info")["result"]
            break
        except (OSError, KeyError, ValueError, struct.error):
            if time.monotonic() > deadline:
                sys.exit(f"{data}: the node did not answer")
            time.sleep(1)
    peer = info["peerId"]
    # The holder's receipt account, which pays the gas of its payouts, is in
    # `operator_earnings` (node_info does not carry it).
    try:
        account = call(f"{data}/ipc.sock", token, "operator_earnings")["result"]["account"]
    except (OSError, KeyError, ValueError, struct.error):
        account = None
    print(json.dumps({
        "peerId": peer,
        "route": f"/ip4/{ip}/udp/{port}/quic-v1/p2p/{peer}",
        "commitment": info.get("directory", {}).get("ownCommitment"),
        "account": account,
    }))


if __name__ == "__main__":
    main()
