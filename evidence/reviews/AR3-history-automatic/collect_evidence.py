"""Retain named public evidence, with lossless factoring of ten paid responses."""
from pathlib import Path
import hashlib
import json
import sys

root = Path.cwd()
evidence = Path(__file__).resolve().parent
sender_label, recipient_label = sys.argv[1:]


def read(label):
    report = json.loads((evidence / f"{label}.json").read_text())
    assert report["passed"] and not report["source_changes"]
    trace = root / report["trace_local"]
    assert hashlib.sha256(trace.read_bytes()).hexdigest() == report["trace_sha256"]
    return report, json.loads(trace.read_text())


sender_report, sender = read(sender_label)
recipient_report, recipient = read(recipient_label)
assert sender_report["native"]["binarySha256"] == recipient_report["native"]["binarySha256"]
assert json.loads((evidence / f"{sender_label}-inputs.json").read_text()) == json.loads(
    (evidence / f"{recipient_label}-inputs.json").read_text()
)


def provenance(report):
    return {k: report[k] for k in ("trace_local", "trace_sha256", "log_local", "log_sha256")}


def sender_views(values):
    fields = ("messageId", "state", "error", "history", "replicas", "indexReplicas", "indexLocations")
    return [{k: value.get(k) for k in fields} for value in values]


audits = sender["historyAckPeerAudits"]
assert len(audits) == 10 and sorted(a["position"] for a in audits) == list(range(10))
first = audits[0]["response"]
common = {k: v for k, v in first["anchor"].items()
          if all(a["response"]["anchor"].get(k) == v for a in audits)}
variants = []
for audit in audits:
    response = audit["response"]
    assert response["status"] == "history" and response["manifest"] == first["manifest"]
    differing = {k: v for k, v in response["anchor"].items() if k not in common}
    reconstructed = dict(status="history", manifest=first["manifest"], anchor=dict(common, **differing))
    assert reconstructed == response
    variants.append(dict(position=audit["position"], peer=audit["peer"], anchor=differing))

sender_evidence = dict(
    provenance=provenance(sender_report),
    publication=sender["historyPublication"],
    originals=[v["prepared"]["envelope"] for v in sender["completedSender"]],
    stages={name: sender_views(sender[name]) for name in (
        "promiseCommitFailure", "locationCommitFailure", "historyPrepareFailure",
        "historyAckFailure", "completedSender", "restoredSender")},
    remoteReads=dict(status="history", manifest=first["manifest"], commonAnchor=common, peers=variants),
)
recipient_fields = (
    "publication", "originalHolders", "survivingHolders", "pointerPeers", "dataLoss",
    "indexLoss", "survivingIndexes", "withoutPublicTrust", "failedSync", "rollbackBefore",
    "rollbackAfter", "secondOriginalFailure", "partialProgress", "recipientHead", "recipient",
    "firstSync", "restartedSync", "recipientObserverMethods", "recipientObserverCalls",
    "senderObserverMethods", "committedReferenceProgress", "coldVerifiedHistoryPass",
)
recipient_evidence = dict(
    provenance=provenance(recipient_report),
    originals=[v["prepared"]["envelope"] for v in recipient["completedSender"]],
    completedSender=sender_views(recipient["completedSender"]),
    **{k: recipient[k] for k in recipient_fields},
)
for name, result in (("sender-evidence", sender_evidence), ("recipient-evidence", recipient_evidence)):
    # Compact wire byte arrays avoid expanding identical proof encodings into
    # hundreds of thousands of lines. Values remain unchanged and machine-readable.
    path = evidence / f"{name}.json"
    path.write_text(json.dumps(result, separators=(",", ":")) + "\n")
    print(json.dumps(dict(path=str(path.relative_to(root)), bytes=path.stat().st_size,
                         sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
