import datetime
import json
import pathlib
import subprocess
import sys
import uuid

ROOT = pathlib.Path('/Users/glebk/Code/chat/output/ar2-wallet-flow/skill-host-smoke/host-state')
CTX = json.loads((ROOT.parent / 'connection-context.json').read_text())
STATE = ROOT / 'task-state.json'
def save(obj):
    STATE.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')
def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()
state = json.loads(STATE.read_text()) if STATE.exists() else {
    'startedAt': now(), 'recipientNetworkId': CTX['recipientNetworkId'],
    'send': {'operationId': str(uuid.uuid4()), 'text': 'Checking the conversation via the shipped CLI skill.'},
    'polls': [], 'processedMessageIds': [], 'runs': [],
}
action = sys.argv[1]
recipient = state['recipientNetworkId']
stdin = None
if action == 'context':
    args = ['context']
elif action in ('send', 'retry'):
    args = ['messages', 'send', '--to', recipient, '--operation-id', state['send']['operationId'], '--text-stdin']
    stdin = state['send']['text'].encode('utf-8')
elif action == 'delivery':
    args = ['delivery', 'get', '--to', recipient, '--operation-id', state['send']['operationId']]
elif action == 'poll':
    maximum = int(sys.argv[2]) if len(sys.argv) > 2 else 4096
    poll = {'operationId': str(uuid.uuid4()), 'from': recipient, 'limit': 10, 'maxBytes': maximum, 'leaseSeconds': 60}
    state['polls'].append(poll)
    args = ['inbox', 'poll', '--from', recipient, '--operation-id', poll['operationId'], '--limit', str(poll['limit']), '--max-bytes', str(poll['maxBytes']), '--lease-seconds', str(poll['leaseSeconds'])]
elif action == 'ack':
    lease = sys.argv[2]
    args = ['inbox', 'ack', '--from', recipient, '--lease-id', lease]
else:
    raise SystemExit('Unknown action')
index = len(state['runs']) + 1
name = f'{index:02d}-{action}'
run = {'action': action, 'startedAt': now(), 'args': args, 'stdoutPath': str(ROOT / (name + '.stdout.json')), 'stderrPath': str(ROOT / (name + '.stderr.txt'))}
state['runs'].append(run)
save(state)
completed = subprocess.run([CTX['cliConfig']['command'], *CTX['cliConfig']['args'], *args], input=stdin, capture_output=True, timeout=35)
pathlib.Path(run['stdoutPath']).write_bytes(completed.stdout)
pathlib.Path(run['stderrPath']).write_bytes(completed.stderr)
run.update(exitCode=completed.returncode, finishedAt=now())
envelope = json.loads(completed.stdout)
if action == 'poll':
    state['polls'][-1]['page'] = envelope
    state['polls'][-1]['stdoutPath'] = run['stdoutPath']
if action in ('send', 'retry') and 'result' in envelope:
    state['send'].setdefault('messageId', envelope['result']['id'])
save(state)
print(json.dumps({'cliExitCode': completed.returncode, 'stdoutPath': run['stdoutPath'], 'envelope': envelope}, ensure_ascii=False))
