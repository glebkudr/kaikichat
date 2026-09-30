"""Explicit socket lifecycle fixture; it never opens a host socket."""
import errno
from pathlib import Path
import socket


class Sockets:
    def __init__(self, failure=None, error_number=errno.EPERM):
        self.failure, self.error_number = failure, error_number
        self.operations, self.opened, self.paths = [], [], []

    def check(self, endpoint, operation):
        identity = endpoint + ':' + operation
        self.operations.append(identity)
        if identity == self.failure:
            if self.error_number is None:
                raise TimeoutError('Unix IPC probe timed out')
            raise OSError(self.error_number, 'injected Unix IPC capability refusal')

    def socket(self, family, kind):
        assert family == socket.AF_UNIX and kind == socket.SOCK_STREAM
        endpoint = 'listener' if not self.opened else 'client'
        self.check(endpoint, 'socket')
        return self.open(endpoint)

    def open(self, endpoint):
        result = Stream(self, endpoint)
        self.opened.append(result)
        return result

    def audit(self):
        return dict(operations=self.operations, opened=len(self.opened),
            closed=sum(stream.closed for stream in self.opened),
            timeouts=[stream.timeout for stream in self.opened if stream.endpoint != 'accepted'],
            paths=[str(path) for path in self.paths],
            pathsRemaining=[str(path) for path in self.paths if path.exists()],
            directoriesRemaining=[str(path.parent) for path in self.paths if path.parent.exists()])


class Stream:
    def __init__(self, owner, endpoint):
        self.owner, self.endpoint = owner, endpoint
        self.closed, self.timeout, self.listening = False, None, False

    def settimeout(self, seconds):
        self.timeout = seconds

    def bind(self, path):
        self.owner.check(self.endpoint, 'bind')
        path = Path(path)
        path.touch(exist_ok=False)
        self.owner.paths.append(path)

    def listen(self, backlog):
        self.owner.check(self.endpoint, 'listen')
        assert backlog == 1
        self.listening = True

    def connect(self, path):
        self.owner.check(self.endpoint, 'connect')
        assert Path(path).exists() and self.owner.opened[0].listening

    def accept(self):
        self.owner.check(self.endpoint, 'accept')
        return self.owner.open('accepted'), None

    def close(self):
        self.closed = True
        self.owner.check(self.endpoint, 'close')
