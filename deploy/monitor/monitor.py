#!/usr/bin/env python3
"""Production's watch on the chain RPC; its alerts go to the owner's Telegram.

Every ROUND seconds it checks:
- the RPC: CHAIN_RPC, the address the nodes read the chain at, answers
  eth_chainId with CHAIN_ID, and its latest block is at most HEAD_AGE old;
- the nodes: each answers node_info over its IPC socket. run-nodes.sh links
  every node's socket and owner token into the `monitor` directory of the
  nodes' volume; this container mounts only that directory, none of the
  nodes' keys;
- the nodes' own reads (node_info's `chain`, which counts the identity
  server's too): over the last WINDOW seconds, at least MIN_FAILURES and at
  least a quarter of them failed. A counter that went down means the node
  restarted, and its count begins again there. A round no node answers
  says nothing about the reads.

A check failing FAIL_ROUNDS rounds in a row sends an alert, a reminder every
REMIND seconds while it fails, and a note when it passes again. What was
told survives a restart in STATE_FILE, so a deployment that mends a problem
still brings its note. A message Telegram did not take goes again on the
next round, in order; one it refuses for good is dropped. Messages and logs
name the RPC's host, never its path or user, which may hold a key, and
never the bot's token.

    monitor.py          watch until stopped
    monitor.py --test   send one message and exit

Environment: CHAIN_RPC, CHAIN_ID, TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID;
NODES_DIR (/nodes by default) and STATE_FILE (none by default).
"""
import http.client
import json
import os
import re
import signal
import socket
import struct
import sys
import time
import traceback
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROUND = 60
FAIL_ROUNDS = 5
REMIND = 3600
HEAD_AGE = 120
WINDOW = 180
MIN_FAILURES = 5
TIMEOUT = 10
# Messages kept while Telegram takes none; the oldest go first.
OUTBOX = 50
# Longer texts (such as an RPC's own error message) are cut.
MAX_TEXT = 1000
TELEGRAM = "https://api.telegram.org"
TOKEN = re.compile(r"\d+:[A-Za-z0-9_-]+")
# The nodes' reads when no node answered.
UNKNOWN = object()


class Problem(Exception):
    """What went wrong, in words for the owner."""


class SendError(Exception):
    """Telegram did not take a message; `lasting` if it never will."""

    def __init__(self, reason, lasting=False):
        super().__init__(reason)
        self.lasting = lasting


def log(text):
    print(text, flush=True)


def describe(error):
    """An error in words that never carry the address asked: RPC and
    Telegram addresses may hold keys."""
    if isinstance(error, urllib.error.URLError):
        error = error.reason
        if isinstance(error, str):
            return error
    if isinstance(error, TimeoutError):
        return "no answer in time"
    if isinstance(error, json.JSONDecodeError):
        return "an answer that is not JSON"
    if isinstance(error, OSError):
        return str(error) or type(error).__name__
    return type(error).__name__


def rpc_call(url, method, params, timeout):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    headers = {"Content-Type": "application/json", "User-Agent": "kaiki-monitor"}
    try:
        with urllib.request.urlopen(urllib.request.Request(url, body, headers), timeout=timeout) as response:
            answer = json.loads(response.read())
    except urllib.error.HTTPError as error:
        error.close()
        raise Problem(f"{method}: HTTP {error.code}") from None
    except (OSError, ValueError, http.client.HTTPException) as error:
        raise Problem(f"{method}: {describe(error)}") from None
    if not isinstance(answer, dict):
        raise Problem(f"{method}: not a JSON-RPC answer")
    if answer.get("error") is not None:
        error = answer["error"]
        raise Problem(f"{method}: {error.get('message') if isinstance(error, dict) else error}")
    return answer.get("result")


def rpc_problem(url, chain_id, now, timeout):
    """What is wrong with the RPC now, or None."""
    try:
        chain = int(rpc_call(url, "eth_chainId", [], timeout), 16)
        if chain != chain_id:
            return f"chain {chain}, not {chain_id}"
        head = rpc_call(url, "eth_getBlockByNumber", ["latest", False], timeout)
        if head is None:
            return "no latest block"
        number, stamp = int(head["number"], 16), int(head["timestamp"], 16)
    except Problem as problem:
        return str(problem)
    except (TypeError, ValueError, KeyError):
        return "answers that are not a chain's"
    age = now - stamp
    if age > HEAD_AGE:
        return f"its latest block {number} is {age:.0f} s old"
    return None


def receive(s, size):
    data = bytearray()
    while len(data) < size:
        chunk = s.recv(size - len(data))
        if not chunk:
            raise ConnectionError("the node closed the connection")
        data += chunk
    return bytes(data)


def node_call(path, token, method, timeout):
    """A call over a node's IPC socket (the framing of node-info.py)."""
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(timeout)
        s.connect(str(path))
        wire = json.dumps({"token": token, "method": method, "request": {}}).encode()
        s.sendall(struct.pack(">I", len(wire)) + wire)
        (size,) = struct.unpack(">I", receive(s, 4))
        return json.loads(receive(s, size))


class Nodes:
    """The nodes' sockets and the recent samples of their read counters."""

    def __init__(self, directory, timeout):
        self.directory = Path(directory)
        self.timeout = timeout
        self.samples = {}  # name: [(now, reads, failures)], oldest first

    def round(self, now):
        """(what is wrong with the nodes' answers, with their reads); the
        reads are UNKNOWN when no node answered."""
        names = sorted(
            (path.stem for path in self.directory.glob("node-*.sock")),
            key=lambda name: (len(name), name),
        )
        if not names:
            return f"no node sockets in {self.directory}", UNKNOWN
        silent, reads, failures = [], 0, 0
        for name in names:
            try:
                token = (self.directory / f"{name}.token").read_text().strip()
                path = self.directory / f"{name}.sock"
                chain = node_call(path, token, "node_info", self.timeout)["result"]["chain"]
                sample = (now, int(chain["reads"]), int(chain["failures"]))
            except (OSError, ValueError, KeyError, TypeError, struct.error):
                silent.append(name)
                continue
            history = self.samples.setdefault(name, [])
            if history and (sample[1] < history[-1][1] or sample[2] < history[-1][2]):
                history.clear()
            history.append(sample)
            # Half a round of slack: rounds come a little later than ROUND.
            history[:] = [old for old in history if old[0] >= now - WINDOW - ROUND / 2]
            reads += sample[1] - history[0][1]
            failures += sample[2] - history[0][2]
        answers = None
        if silent:
            answers = f"{len(silent)} of {len(names)} do not answer: {', '.join(silent)}"
        if len(silent) == len(names):
            return answers, UNKNOWN
        failing = None
        if failures >= MIN_FAILURES and failures * 4 >= reads:
            failing = f"{failures} of {reads} reads failed in the last {WINDOW // 60} min"
        return answers, failing


class Check:
    """One check's run of failed rounds and what the owner was told."""

    def __init__(self, title):
        self.title = title
        self.failed = 0
        self.since = None  # the first failed round of the run
        self.told = None  # the last alert of the run

    def update(self, problem, now):
        """The message this round's outcome calls for, or None."""
        if problem is None:
            told, since = self.told, self.since
            self.failed, self.since, self.told = 0, None, None
            if told is None:
                return None
            return f"✅ {self.title}: fine again after {minutes(now - since)}"
        if not self.failed:
            self.since = now
        self.failed += 1
        if self.failed < FAIL_ROUNDS:
            return None
        if self.told is None:
            self.told = now
            return f"🔴 {self.title}: {problem}"
        if now - self.told >= REMIND:
            self.told = now
            return f"🔴 {self.title}, for {minutes(now - self.since)}: {problem}"
        return None


def minutes(seconds):
    return f"{round(seconds / 60)} min"


def host_of(url):
    parts = urllib.parse.urlsplit(url)
    return f"{parts.hostname}:{parts.port}" if parts.port else str(parts.hostname)


class Monitor:
    def __init__(self, rpc_url, chain_id, nodes_dir, send, timeout=TIMEOUT, state=None):
        self.rpc_url, self.chain_id, self.send, self.timeout = rpc_url, chain_id, send, timeout
        self.host = host_of(rpc_url)
        self.nodes = Nodes(nodes_dir, timeout)
        self.checks = {
            "rpc": Check(f"RPC {self.host}"),
            "nodes": Check("Nodes"),
            "reads": Check("Nodes' chain reads"),
        }
        self.outbox = []
        self.state = Path(state) if state else None
        self.load()

    def round(self, now):
        # The RPC first: slow nodes would make its head look younger.
        problems = {"rpc": self.checked(lambda: rpc_problem(self.rpc_url, self.chain_id, now, self.timeout))}
        nodes = self.checked(lambda: self.nodes.round(now))
        problems["nodes"], problems["reads"] = nodes if isinstance(nodes, tuple) else (nodes, UNKNOWN)
        told = {key: check.told for key, check in self.checks.items()}
        for key, problem in problems.items():
            if problem is UNKNOWN:
                continue
            if problem:
                log(f"{self.checks[key].title}: {problem}")
            message = self.checks[key].update(problem, now)
            if message:
                self.outbox.append(message[:MAX_TEXT])
        del self.outbox[:-OUTBOX]
        if told != {key: check.told for key, check in self.checks.items()}:
            self.save()
        self.flush()

    def checked(self, check):
        """A check's outcome; its own failure is its problem."""
        try:
            return check()
        except Exception as error:  # noqa: BLE001 - a bug must not stop the watch
            return f"the monitor failed to check: {type(error).__name__}"

    def flush(self):
        while self.outbox:
            try:
                self.send(self.outbox[0])
            except SendError as error:
                if error.lasting:
                    log(f"telegram refused, dropped: {error}: {self.outbox.pop(0)}")
                    continue
                log(f"telegram: {error}; {len(self.outbox)} message(s) wait")
                return
            log(f"sent: {self.outbox.pop(0)}")

    def load(self):
        if not self.state:
            return
        try:
            saved = json.loads(self.state.read_text())
        except (OSError, ValueError):
            return
        for key, run in saved.items() if isinstance(saved, dict) else ():
            check = self.checks.get(key)
            try:
                since, told = float(run["since"]), float(run["told"])
            except (KeyError, TypeError, ValueError):
                continue
            if check:
                check.failed, check.since, check.told = FAIL_ROUNDS, since, told

    def save(self):
        if not self.state:
            return
        told = {
            key: {"since": check.since, "told": check.told}
            for key, check in self.checks.items()
            if check.told is not None
        }
        try:
            fresh = self.state.with_name(self.state.name + ".new")
            fresh.write_text(json.dumps(told))
            fresh.replace(self.state)
        except OSError as error:
            log(f"state: {describe(error)}")


def telegram_sender(token, chat_id, api=TELEGRAM, timeout=TIMEOUT):
    """send(text) to the chat; a SendError names Telegram's reason, never
    the token."""
    url = f"{api}/bot{token}/sendMessage"

    def send(text):
        form = {"chat_id": chat_id, "text": text, "disable_web_page_preview": "true"}
        body = urllib.parse.urlencode(form).encode()
        try:
            with urllib.request.urlopen(url, body, timeout=timeout) as response:
                answer = json.loads(response.read())
        except urllib.error.HTTPError as error:
            try:
                reason = json.loads(error.read()).get("description", "")
            except (OSError, ValueError, AttributeError, http.client.HTTPException):
                reason = ""
            finally:
                error.close()
            # A refused text or chat stays refused; too many requests pass.
            lasting = 400 <= error.code < 500 and error.code != 429
            raise SendError(f"HTTP {error.code} {reason}".strip(), lasting) from None
        except Exception as error:  # noqa: BLE001 - its text may carry the URL
            raise SendError(describe(error)) from None
        if not isinstance(answer, dict) or not answer.get("ok"):
            raise SendError("not taken")

    return send


def main():
    env = {name: os.environ.get(name, "").strip() for name in
           ("CHAIN_RPC", "CHAIN_ID", "TELEGRAM_BOT_TOKEN", "TELEGRAM_CHAT_ID", "NODES_DIR", "STATE_FILE")}
    missing = [name for name in list(env)[:4] if not env[name]]
    if missing:
        sys.exit(f"monitor.py: set {', '.join(missing)}")
    if not TOKEN.fullmatch(env["TELEGRAM_BOT_TOKEN"]):
        sys.exit("monitor.py: TELEGRAM_BOT_TOKEN is not a bot token")
    if sys.argv[1:] not in ([], ["--test"]):
        sys.exit("usage: monitor.py [--test]")
    send = telegram_sender(env["TELEGRAM_BOT_TOKEN"], env["TELEGRAM_CHAT_ID"])
    nodes_dir = Path(env["NODES_DIR"] or "/nodes")
    monitor = Monitor(env["CHAIN_RPC"], int(env["CHAIN_ID"]), nodes_dir, send, state=env["STATE_FILE"] or None)
    if sys.argv[1:] == ["--test"]:
        try:
            send(f"Kaiki monitor: a test message. Watching RPC {monitor.host} and the nodes.")
        except SendError as error:
            sys.exit(f"telegram: {error}")
        log("sent")
        return
    # As PID 1 the process has no default action for SIGTERM.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    log(f"watching RPC {monitor.host} (chain {monitor.chain_id}) and the nodes in {nodes_dir}")
    while True:
        started = time.monotonic()
        try:
            monitor.round(time.time())
        except Exception as error:  # noqa: BLE001 - the watch goes on
            where = traceback.extract_tb(error.__traceback__)[-1]
            # No message: it may carry an address with a key.
            log(f"round failed: {type(error).__name__} at line {where.lineno}")
        time.sleep(max(0.0, ROUND - (time.monotonic() - started)))


if __name__ == "__main__":
    main()
