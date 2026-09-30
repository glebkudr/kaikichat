from pathlib import Path
import json
import sys
import time

root = Path('/Users/glebk/Code/chat')
sys.path.insert(0, str(root / 'tests/evm'))
import public_history_prefetch_probe as suite

suite.OUT = suite.paid.OUT = suite.cli.OUT = suite.cli.wallet.OUT = root / 'output/hpp-r9'
suite.paid.postage_author = suite.postage_author

# Log only status results the unchanged scenario already reads. No extra RPC,
# changed return value, worker call, injected data or altered wait/oracle.
delivery = suite.cli.wallet.recipient.sender.delivery
original_sync = delivery.sync
last_report = {}
def observed_sync(node):
    value = original_sync(node)
    current = time.monotonic()
    profile = node.directory.name
    if current - last_report.get(profile, 0) >= 30:
        last_report[profile] = current
        print(json.dumps({'observedSync': value, 'profile': profile,
                          'at': current, 'checkedAt': int(time.time())}), flush=True)
    return value

delivery.sync = observed_sync
# Observe only after the original cleanup has stopped every custody actor.
# No added RPC, imported data, changed deadline or work call during the scenario.
original_cleanup = suite.cleanup

def observed_cleanup(report):
    recipients = [node for node in suite.custody.actors
                  if node.directory.name == 'ciphertext-bob']
    original_cleanup(report)
    try:
        snapshot = {'phase': 'after original custody cleanup', 'captured': False,
                    'errors': {}}
        assert len(recipients) == 1
        node = recipients[0]
        assert node.process is None
        snapshot['processStopped'] = True
        recipient = suite.cli.wallet.recipient
        conversation = suite.custody.message['recipientConversation']
        for name, fields in [
            ('progress', {'recipientProgressSnapshot': True}),
            ('deferred', {'recipientDeferredSnapshot': True}),
            ('prefetch', {'recipientPrefetchSnapshot': True, 'conversation': conversation})
        ]:
            try:
                snapshot[name] = recipient.fixture(node, **fields)
            except Exception as error:
                snapshot['errors'][name] = type(error).__name__
        if 'progress' in snapshot:
            try:
                snapshot['imports'] = recipient.import_entries(snapshot['progress'])
            except Exception as error:
                snapshot['errors']['imports'] = type(error).__name__
        snapshot['captured'] = not snapshot['errors']
        (suite.OUT / 'recipient-stop-snapshot.json').write_text(
            json.dumps(snapshot, indent=2) + '\n')
    except Exception as error:
        # New capture/serialization/write errors must not replace the original
        # scenario exception or prevent the provider-stop loop from running.
        try:
            print(json.dumps({'recipientSnapshotError': type(error).__name__}), flush=True)
        except Exception:
            pass

suite.cleanup = observed_cleanup
suite.paid.main(suite)
