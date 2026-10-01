"""Test-only control inside a private container; never a production daemon API."""
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import time

ROOT = Path('/tmp/ain-network-profile')
BINARY = '/usr/local/bin/kaiki-agentic-node'


def private_json(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(value, stream)


def rpc(request):
    bootstrap = json.loads((ROOT / 'bootstrap.json').read_text())
    payload = json.dumps({'token': bootstrap['ownerToken'], 'method': request['method'],
                          'request': request.get('request', {})}).encode()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
        stream.settimeout(5)
        stream.connect(str(ROOT / 'node.sock'))
        stream.sendall(struct.pack('>I', len(payload)) + payload)

        def read_exact(size):
            result = bytearray()
            while len(result) < size:
                chunk = stream.recv(size - len(result))
                if not chunk:
                    raise RuntimeError('daemon disconnected during IPC')
                result.extend(chunk)
            return bytes(result)

        size = struct.unpack('>I', read_exact(4))[0]
        if size > 16 * 1024 * 1024:
            raise RuntimeError('unbounded daemon response')
        return json.loads(read_exact(size))


def own_pid():
    path = ROOT / 'pid'
    if not path.exists():
        return None
    pid = int(path.read_text())
    try:
        command = Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
        if command[0] == BINARY.encode():
            return pid
    except (FileNotFoundError, ProcessLookupError):
        # procfs can lose a process between opening cmdline and reading the descriptor.
        pass
    return None


def start(request):
    ROOT.mkdir(mode=0o700, exist_ok=True)
    if own_pid() is not None:
        raise RuntimeError('owned daemon already running')
    if request:
        private_json(ROOT / 'start.json', request)
    request = json.loads((ROOT / 'start.json').read_text())
    if not (ROOT / 'bootstrap.json').exists():
        private_json(ROOT / 'bootstrap.json', {'masterKey': os.urandom(32).hex(), 'ownerToken': os.urandom(32).hex()})
    args = [BINARY, 'serve', '--profile', str(ROOT / 'profile.db'), '--ipc', str(ROOT / 'node.sock'), '--secrets-stdin']
    for address in request['listen']:
        args.extend(['--listen', address])
    args.extend(request.get('arguments', []))
    with (ROOT / 'daemon.log').open('ab') as log:
        child = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=log, stderr=log, start_new_session=True)
    child.stdin.write((ROOT / 'bootstrap.json').read_bytes() + b'\n')
    child.stdin.close()
    (ROOT / 'pid').write_text(str(child.pid))
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if child.poll() is not None:
            raise RuntimeError(f'daemon exited during startup ({child.returncode})')
        try:
            result = rpc({'method': 'node_info'}).get('result')
            if result and len(result['listeners']) == len(request['listen']):
                return result
        except (OSError, RuntimeError):
            pass
        time.sleep(0.05)
    raise RuntimeError('daemon startup timeout')


def stop():
    pid = own_pid()
    if pid:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass  # Already exited after the ownership check; stop is idempotent.
        deadline = time.monotonic() + 5
        while own_pid() and time.monotonic() < deadline:
            time.sleep(0.02)
        if own_pid():
            raise RuntimeError('owned daemon did not stop')
    return {'stopped': True}


def command(*args):
    return subprocess.check_output(args, text=True, timeout=15).strip()


def interface(address):
    for item in json.loads(command('ip', '-j', 'address')):
        if any(a.get('local') == address for a in item['addr_info']):
            return item['ifname']
    raise RuntimeError('test interface missing')


def main(kind, request):
    if kind == 'start':
        return start(request)
    if kind == 'stop':
        return stop()
    if kind == 'rpc':
        return rpc(request)
    if kind == 'lan_sockets':
        count = 0
        for name in ('udp', 'udp6'):
            for line in Path('/proc/net/' + name).read_text().splitlines()[1:]:
                if int(line.split()[1].split(':')[1], 16) == 5353:
                    count += 1
        return {'count': count}
    if kind == 'state_failure':
        secret = json.loads((ROOT / 'bootstrap.json').read_text())
        data = {'path': str(ROOT / 'profile.db'), 'key': secret['masterKey'], 'enabled': request['enabled']}
        subprocess.run(['/usr/local/bin/ain-test-state-failure'], input=json.dumps(data), text=True, timeout=10, check=True)
        return {'enabled': request['enabled']}
    if kind == 'advertise_mdns':
        # Independent DNS-SD PTR/TXT encoder from the published libp2p mDNS wire spec.
        def name(value):
            return b''.join(bytes([len(part)]) + part.encode() for part in value.split('.')) + b'\x00'
        service = name('_p2p._udp.local')
        instance = name('testadvertiser00000000000000000000._p2p._udp.local')
        def record(owner, kind, data):
            return owner + struct.pack('>HHIH', kind, 1, 120, len(data)) + data
        text = ('dnsaddr=' + request['address']).encode()
        if len(text) > 255:
            raise RuntimeError('oversized fixture TXT')
        packet = struct.pack('>HHHHHH', 0, 0x8400, 0, 1, 0, 1)
        packet += record(service, 12, instance) + record(instance, 16, bytes([len(text)]) + text)
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as stream:
            # Internal test LANs have no default route. Bind multicast to their owned interface.
            local = next(address['local'] for item in json.loads(command('ip', '-j', 'address'))
                         for address in item['addr_info'] if address['family'] == 'inet' and address['scope'] == 'global')
            stream.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF, socket.inet_aton(local))
            stream.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 1)
            for _ in range(3):
                stream.sendto(packet, ('224.0.0.251', 5353))
                time.sleep(0.1)
        return {'sent': 3, 'bytes': len(packet)}
    if kind == 'inspect_invitation':
        return json.loads(subprocess.check_output(['/usr/local/bin/ain-inspect-invitation'], input=json.dumps(request['invitation']), text=True, timeout=10))
    if kind == 'dns':
        # An internal network's embedded DNS answers no outside names; a
        # client behind its router asks a public resolver through it.
        Path('/etc/resolv.conf').write_text(''.join(f'nameserver {server}\n' for server in request['servers']))
        return {'servers': request['servers']}
    if kind == 'route':
        command('ip', 'route', 'replace', 'default', 'via', request['gateway'])
        return json.loads(command('ip', '-j', 'route'))
    if kind == 'router':
        lan, wan = interface(request['lan']), interface(request['wan'])
        command('iptables', '-P', 'FORWARD', 'DROP')
        command('iptables', '-F', 'FORWARD')
        command('iptables', '-t', 'nat', '-F', 'POSTROUTING')
        command('iptables', '-A', 'FORWARD', '-i', lan, '-d', request['blockedSubnet'], '-m', 'comment', '--comment', 'ain-block-direct', '-j', 'DROP')
        if request.get('blockedPublic'):
            command('iptables', '-A', 'FORWARD', '-i', lan, '-d', request['blockedPublic'], '-p', 'udp', '-m', 'comment', '--comment', 'ain-block-upgrade', '-j', 'DROP')
        command('iptables', '-A', 'INPUT', '-i', wan, '-p', 'udp', '--dport', '4001', '-m', 'comment', '--comment', 'ain-unsolicited-udp', '-j', 'DROP')
        command('iptables', '-A', 'FORWARD', '-i', lan, '-o', wan, '-j', 'ACCEPT')
        command('iptables', '-A', 'FORWARD', '-i', wan, '-o', lan, '-m', 'conntrack', '--ctstate', 'ESTABLISHED,RELATED', '-j', 'ACCEPT')
        command('iptables', '-t', 'nat', '-A', 'POSTROUTING', '-o', wan, '-j', 'MASQUERADE')
        return {'lan': lan, 'wan': wan}
    if kind == 'block':
        # A router stops forwarding its LAN's traffic to one public endpoint,
        # as if that provider were gone for this network only.
        lan = interface(request['lan'])
        for protocol in request.get('protocols', ['udp', 'tcp']):
            port = ['--dport', str(request['port'])] if 'port' in request else []
            rule = ['FORWARD', '-i', lan, '-d', request['host'], '-p', protocol, *port,
                    '-m', 'comment', '--comment', request['comment'], '-j', 'DROP']
            command('iptables', '-I', *rule)
        return {'blocked': request['host'], 'port': request.get('port')}
    if kind == 'backup':
        # A copy of the stopped profile, as an owner's backup would be.
        if own_pid() is not None:
            raise RuntimeError('stop the daemon before a backup')
        target = Path('/tmp/ain-network-backup')
        subprocess.run(['rm', '-rf', str(target)], check=True)
        subprocess.run(['cp', '-a', str(ROOT), str(target)], check=True)
        return {'backup': str(target)}
    if kind == 'restore':
        if own_pid() is not None:
            raise RuntimeError('stop the daemon before a restore')
        subprocess.run(['rm', '-rf', str(ROOT)], check=True)
        subprocess.run(['cp', '-a', '/tmp/ain-network-backup', str(ROOT)], check=True)
        return {'restored': str(ROOT)}
    if kind == 'metrics':
        return {'rules': command('iptables-save', '-c'), 'connections': command('conntrack', '-L', '-o', 'extended')}
    if kind == 'reachability_firewall':
        operation = '-A' if request['blocked'] else '-D'
        command('iptables', operation, 'INPUT', '-p', 'tcp', '--dport', '4001', '-m', 'conntrack', '--ctstate', 'NEW', '-m', 'comment', '--comment', 'ain-autonat-new-inbound', '-j', 'DROP')
        return {'blocked': request['blocked']}
    if kind == 'udp_probe':
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as stream:
            # An unrelated source port; this never opens a client's peer-to-peer mapping.
            stream.bind(('0.0.0.0', 45001))
            stream.sendto(b'ain-negative-network-control', (request['host'], request['port']))
        return {'sent': True}
    if kind == 'probe':
        try:
            with socket.create_connection((request['host'], request['port']), timeout=1):
                return {'reachable': True}
        except OSError:
            return {'reachable': False}
    raise RuntimeError('unknown fixture command')


if __name__ == '__main__':
    os.umask(0o077)
    value = json.loads(sys.stdin.read() or '{}')
    print(json.dumps(main(sys.argv[1], value)))
