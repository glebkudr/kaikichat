"""Run the unchanged ordinary sender gate with passive status timing samples.

No additional IPC calls, changed assertions, deadlines or status return values.
Invoke from the repository root through scripts/build-storage.py run.
"""
from pathlib import Path
import json
import sys
import time

sys.path.insert(0, str(Path.cwd() / 'tests/evm'))
import public_sender_daemon as sender

original_status = sender.status
last_sample = {}


def observed_status(node, message):
    started = time.monotonic()
    result = original_status(node, message)
    if started - last_sample.get(message['id'], 0) >= 10:
        last_sample[message['id']] = started
        fields = ('state', 'error', 'replicas', 'indexReplicas', 'indexLocations')
        print(json.dumps(dict(at=started, statusSeconds=time.monotonic()-started,
            messageId=message['id'], **{key: result.get(key) for key in fields})), flush=True)
    return result


sender.status = observed_status
sender.main()
