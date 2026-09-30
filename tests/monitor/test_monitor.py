"""Production's watch on the chain RPC (deploy/monitor/monitor.py).

The monitor asks the RPC the nodes use for its chain id and latest block,
and asks every node for node_info over its IPC socket. What the owner gets
in Telegram: nothing while all is well or after a short blip; an alert once
a problem has lasted FAIL_ROUNDS rounds, a reminder every REMIND seconds
while it lasts, and a note when it is over. A message Telegram did not take
goes on the next round, in order; one it refuses for good is dropped. What
was told survives the monitor's restart (a deployment). An RPC or a node
that never answers, or an answer cut short, is a problem too, not a stuck
monitor; silent nodes say nothing about how their reads go.

The RPC is a local HTTP server that answers single JSON-RPC requests as a
node does (the monitor sends no batches); the nodes are Unix sockets that
answer node_info with the IPC framing of deploy/node/node-info.py (a 4-byte
big-endian length, then JSON). The fake Telegram takes a form-encoded
sendMessage.
"""
import importlib.util
import json
import socket
import struct
import tempfile
import threading
import unittest
from unittest import mock
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("monitor", ROOT / "deploy" / "monitor" / "monitor.py")
monitor = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(monitor)

T0 = 1_790_000_000.0
BASE = 8453
ROUND = monitor.ROUND
# Long enough for a loaded machine; short for peers that never answer.
TIMEOUT = 3
HANG = 0.05
# An RPC's address may carry a key in its path, as hosted RPCs do.
KEY = "SECRETKEY"


class Rpc(BaseHTTPRequestHandler):
    """A JSON-RPC node: `state` says what it answers."""

    state = {}

    def log_message(self, *args):
        pass

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        state = Rpc.state
        if state.get("status"):
            self.send_response(state["status"])
            self.end_headers()
            return
        if state.get("cut"):
            self.send_response(200)
            self.send_header("Content-Length", "100")
            self.end_headers()
            self.wfile.write(b'{"jsonrpc": "2.0"')
            return
        if state.get("html"):
            body = b"<html>Bad gateway</html>"
        elif state.get("error"):
            body = json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": state["error"]}).encode()
        else:
            if request["method"] == "eth_chainId":
                result = hex(state.get("chain", BASE))
            elif request["method"] == "eth_getBlockByNumber" and not state.get("no_block"):
                result = {"number": hex(40_000_000), "timestamp": hex(int(state["head"]))}
            else:
                result = None
            body = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Telegram(BaseHTTPRequestHandler):
    """Telegram's sendMessage; `ok` false answers as a refused chat does."""

    posted = []
    ok = True

    def log_message(self, *args):
        pass

    def do_POST(self):
        form = parse_qs(self.rfile.read(int(self.headers["Content-Length"])).decode())
        Telegram.posted.append((self.path, form))
        if Telegram.ok:
            code, body = 200, {"ok": True, "result": {}}
        else:
            code, body = 400, {"ok": False, "description": "Bad Request: chat not found"}
        wire = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Length", str(len(wire)))
        self.end_headers()
        self.wfile.write(wire)


def serve(handler):
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def free_port():
    closed = socket.socket()
    closed.bind(("127.0.0.1", 0))
    port = closed.getsockname()[1]
    closed.close()
    return port


def close(server):
    # shutdown wakes an accept() blocked in another thread on Linux too.
    try:
        server.shutdown(socket.SHUT_RDWR)
    except OSError:
        pass
    server.close()


class Node:
    """One node's IPC socket answering node_info with its chain counters,
    or, with `answer` false, taking connections and never answering."""

    def __init__(self, directory, name, token, answer=True):
        self.reads, self.failures, self.token = 0, 0, token
        # A real node_info carries much more than `chain`.
        self.pad = 0
        (directory / f"{name}.token").write_text(token + "\n")
        self.server = socket.socket(socket.AF_UNIX)
        self.server.bind(str(directory / f"{name}.sock"))
        self.server.listen()
        if answer:
            threading.Thread(target=self.serve, daemon=True).start()

    def serve(self):
        while True:
            try:
                conn, _ = self.server.accept()
            except OSError:
                return
            with conn:
                (size,) = struct.unpack(">I", conn.recv(4, socket.MSG_WAITALL))
                request = json.loads(conn.recv(size, socket.MSG_WAITALL))
                if request["token"] != self.token or request["method"] != "node_info":
                    answer = {"error": {"code": "unauthorized"}}
                else:
                    chain = {"configured": True, "reads": self.reads, "failures": self.failures}
                    answer = {"result": {"peerId": "12D3Koo", "peerConnections": "x" * self.pad, "chain": chain}}
                wire = json.dumps(answer).encode()
                conn.sendall(struct.pack(">I", len(wire)) + wire)

    def stop(self):
        close(self.server)


class MonitorTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rpc = serve(Rpc)
        cls.telegram = serve(Telegram)

    @classmethod
    def tearDownClass(cls):
        cls.rpc.shutdown()
        cls.telegram.shutdown()

    def setUp(self):
        quiet = mock.patch.object(monitor, "log")
        quiet.start()
        self.addCleanup(quiet.stop)
        Rpc.state = {"head": T0}
        # A short path: a Unix socket's must fit in about 100 bytes.
        temp = tempfile.TemporaryDirectory(prefix="mon", dir="/tmp")
        self.addCleanup(temp.cleanup)
        self.nodes_dir = Path(temp.name)
        self.nodes = [self.node(n) for n in (1, 2)]
        self.sent = []
        self.refuse = 0
        self.now = T0
        self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/v2/{KEY}")

    def node(self, n, answer=True):
        node = Node(self.nodes_dir, f"node-{n}", f"token-{n}", answer)
        self.addCleanup(node.stop)
        return node

    def monitor(self, url, timeout=TIMEOUT, state=None):
        return monitor.Monitor(url, BASE, self.nodes_dir, self.send, timeout=timeout, state=state)

    def send(self, text):
        if self.refuse:
            self.refuse -= 1
            raise monitor.SendError("HTTP 502")
        if "refused for good" in text:
            raise monitor.SendError("HTTP 400 Bad Request: message is too long", lasting=True)
        self.sent.append(text)

    def rounds(self, count, reads=0, failures=0):
        """`count` rounds a minute apart; each node reads `reads` times and
        fails `failures` of them in every round. The RPC's head keeps up."""
        for _ in range(count):
            self.now += ROUND
            if Rpc.state.get("fresh", True):
                Rpc.state["head"] = self.now - 2
            for node in self.nodes:
                node.reads += reads
                node.failures += failures
            self.watch.round(self.now)

    def test_all_well_sends_nothing(self):
        # Larger than a socket's buffer: it comes in several reads.
        self.nodes[0].pad = 300_000
        self.rounds(10, reads=20, failures=0)
        self.assertEqual(self.sent, [])

    def test_rpc_outage_alerts_reminds_and_recovers(self):
        self.rounds(3)
        Rpc.state["status"] = 429
        self.rounds(monitor.FAIL_ROUNDS - 1)
        self.assertEqual(self.sent, [], "a problem shorter than FAIL_ROUNDS is a blip")
        self.rounds(1)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("127.0.0.1", self.sent[0])
        self.assertIn("429", self.sent[0])
        # A reminder once REMIND has passed, none before.
        self.rounds(monitor.REMIND // ROUND - 1)
        self.assertEqual(len(self.sent), 1)
        self.rounds(2)
        self.assertEqual(len(self.sent), 2)
        self.assertIn("429", self.sent[1])
        del Rpc.state["status"]
        self.rounds(1)
        self.assertEqual(len(self.sent), 3)
        self.assertIn("127.0.0.1", self.sent[2])
        self.assertNotIn("429", self.sent[2])
        self.rounds(10)
        self.assertEqual(len(self.sent), 3, "recovered once, then quiet")
        for text in self.sent:
            self.assertNotIn(KEY, text)

    def test_blip_shorter_than_fail_rounds_sends_nothing(self):
        for _ in range(2):
            Rpc.state["status"] = 503
            self.rounds(monitor.FAIL_ROUNDS - 1)
            del Rpc.state["status"]
            self.rounds(1)
        self.assertEqual(self.sent, [])

    def test_wrong_chain_is_a_problem(self):
        Rpc.state["chain"] = 84532
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("84532", self.sent[0])

    def test_stale_head_is_a_problem(self):
        self.rounds(2)
        Rpc.state["fresh"] = False
        # The head stops; it is stale once it is HEAD_AGE old.
        self.rounds(monitor.HEAD_AGE // ROUND + monitor.FAIL_ROUNDS + 1)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("old", self.sent[0])

    def test_rpc_errors_in_a_good_http_answer_are_problems(self):
        cases = {
            "rate limit": {"error": {"code": -32005, "message": "rate limit"}},
            "no block": {"no_block": True},
            "not json": {"html": True},
            "cut short": {"cut": True},
        }
        for name, state in cases.items():
            with self.subTest(name):
                self.sent = []
                self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/")
                Rpc.state = {"head": self.now, **state}
                self.rounds(monitor.FAIL_ROUNDS)
                self.assertEqual(len(self.sent), 1)
                self.assertIn("127.0.0.1", self.sent[0])
                if name == "rate limit":
                    self.assertIn("rate limit", self.sent[0])

    def test_unreachable_rpc_is_a_problem(self):
        self.watch = self.monitor(f"http://127.0.0.1:{free_port()}/v2/{KEY}")
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("127.0.0.1", self.sent[0])
        self.assertNotIn(KEY, self.sent[0])

    def test_hanging_rpc_and_node_are_problems(self):
        # A server that takes the connection and never answers.
        hanging = socket.socket()
        hanging.bind(("127.0.0.1", 0))
        hanging.listen()
        self.addCleanup(close, hanging)
        self.watch = self.monitor(f"http://127.0.0.1:{hanging.getsockname()[1]}/", HANG)
        self.node(3, answer=False)
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 2)
        text = "\n".join(self.sent)
        self.assertIn("127.0.0.1", text)
        self.assertIn("1 of 3 do not answer: node-3", text)

    def test_nodes_failing_reads_alert_and_recover(self):
        self.rounds(3, reads=10)
        # The RPC answers the monitor, but half of the nodes' own reads fail.
        self.rounds(monitor.FAIL_ROUNDS + monitor.WINDOW // ROUND, reads=10, failures=5)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("reads", self.sent[0])
        self.rounds(monitor.WINDOW // ROUND + 1, reads=10, failures=0)
        self.assertEqual(len(self.sent), 2, "a note once the window is clean")

    def test_silent_nodes_leave_a_reads_alert_standing(self):
        self.rounds(3, reads=10)
        self.rounds(monitor.FAIL_ROUNDS + monitor.WINDOW // ROUND, reads=10, failures=5)
        self.assertEqual(len(self.sent), 1)
        # The container stops: no node answers, which is no sign of health.
        for node in self.nodes:
            node.stop()
        self.rounds(monitor.FAIL_ROUNDS - 1)
        self.assertEqual(len(self.sent), 1)

    def test_alert_state_survives_a_restart(self):
        state = self.nodes_dir / "state.json"
        self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/", state=state)
        Rpc.state["status"] = 429
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 1)
        # A deployment restarts the monitor while the RPC still fails: no
        # alert anew before REMIND.
        self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/", state=state)
        self.rounds(3)
        self.assertEqual(len(self.sent), 1)
        # Another deployment mends it: the note comes, once.
        self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/", state=state)
        del Rpc.state["status"]
        self.rounds(3)
        self.assertEqual(len(self.sent), 2)
        self.assertIn("✅", self.sent[1])
        self.watch = self.monitor(f"http://127.0.0.1:{self.rpc.server_port}/", state=state)
        self.rounds(3)
        self.assertEqual(len(self.sent), 2)

    def test_rare_or_few_node_failures_are_not_alerted(self):
        # Three failed reads in fifty, on one node, all the time.
        for _ in range(monitor.WINDOW // ROUND + monitor.FAIL_ROUNDS + 1):
            self.rounds(1, reads=50)
            self.nodes[0].failures += 3
        # A quiet network: every read fails, but there are only a few.
        for _ in range(monitor.FAIL_ROUNDS + monitor.WINDOW // ROUND):
            self.rounds(1, reads=0)
            self.nodes[0].reads += 1
            self.nodes[0].failures += 1
        self.assertEqual(self.sent, [])

    def test_one_minute_burst_of_failed_reads_is_not_alerted(self):
        self.rounds(3, reads=10)
        self.rounds(1, reads=10, failures=10)
        self.rounds(monitor.FAIL_ROUNDS + monitor.WINDOW // ROUND, reads=10)
        self.assertEqual(self.sent, [])

    def test_restarted_nodes_count_from_their_restart(self):
        # The monitor starts beside nodes that have run for long, failures
        # of long ago included: those are no problem now.
        for node in self.nodes:
            node.reads, node.failures = 10_000, 500
        self.rounds(monitor.FAIL_ROUNDS + 2, reads=10)
        self.assertEqual(self.sent, [])
        # The nodes restart (the container stops with any one of them), and
        # now most of their reads fail.
        for node in self.nodes:
            node.reads, node.failures = 0, 0
        self.rounds(monitor.FAIL_ROUNDS + 2, reads=10, failures=8)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("reads", self.sent[0])

    def test_silent_node_alerts_and_recovers(self):
        self.rounds(2)
        self.nodes[1].stop()
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("node-2", self.sent[0])
        self.assertNotIn("node-1", self.sent[0])
        # The node is back: a new socket at the same place, as run-nodes.sh
        # links a restarted node's.
        (self.nodes_dir / "node-2.sock").unlink()
        self.nodes[1] = self.node(2)
        self.rounds(1)
        self.assertEqual(len(self.sent), 2)

    def test_no_nodes_at_all_is_a_problem(self):
        for node in self.nodes:
            node.stop()
        for path in self.nodes_dir.iterdir():
            path.unlink()
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(len(self.sent), 1)

    def test_message_telegram_refused_goes_next_round_in_order(self):
        Rpc.state["status"] = 500
        self.refuse = 1
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(self.sent, [])
        del Rpc.state["status"]
        self.rounds(1)
        self.assertEqual(len(self.sent), 2)
        self.assertIn("500", self.sent[0])
        self.assertNotIn("500", self.sent[1])

    def test_message_refused_for_good_is_dropped(self):
        Rpc.state["error"] = {"code": -32005, "message": "refused for good"}
        self.rounds(monitor.FAIL_ROUNDS)
        self.assertEqual(self.sent, [])
        del Rpc.state["error"]
        self.rounds(1)
        self.assertEqual(len(self.sent), 1)
        self.assertIn("✅", self.sent[0])

    def test_telegram_sender_posts_to_the_chat_and_hides_the_token(self):
        api = f"http://127.0.0.1:{self.telegram.server_port}"
        token = "123456:SECRET-token"
        send = monitor.telegram_sender(token, "4242", api)
        Telegram.posted, Telegram.ok = [], True
        send("hello")
        path, form = Telegram.posted[0]
        self.assertEqual(path, f"/bot{token}/sendMessage")
        self.assertEqual(form["chat_id"], ["4242"])
        self.assertEqual(form["text"], ["hello"])
        Telegram.ok = False
        with self.assertRaises(monitor.SendError) as refused:
            send("hello")
        self.assertIn("chat not found", str(refused.exception))
        self.assertNotIn("SECRET", str(refused.exception))
        with self.assertRaises(monitor.SendError) as unreachable:
            monitor.telegram_sender(token, "4242", f"http://127.0.0.1:{free_port()}")("hello")
        self.assertNotIn("SECRET", str(unreachable.exception))
        # A token pasted with a space in it.
        with self.assertRaises(monitor.SendError) as malformed:
            monitor.telegram_sender("123456:SECRET token", "4242", api)("hello")
        self.assertNotIn("SECRET", str(malformed.exception))


if __name__ == "__main__":
    unittest.main()
