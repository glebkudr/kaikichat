#!/usr/bin/env python3
"""Extract small factual observations from the published native trace revision.

This does not run the product and does not assert a production fix is correct.
Inputs must be the exact blobs published at TRACE_REVISION.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import statistics

TRACE_REVISION = "1daf8fbbab25b8b798757f9a28582ee94fa3234e"
INPUTS = {
    "h11-macos-failed": {
        "trace.json": "f02c61b4ae1014a9a2a4c48d37ba50dcd44aeba7",
        "check.json": "70018459b99b0a6d817aacc1fa1fc2aa0826047b",
    },
    "a04-linux-failed/run": {
        "trace.json": "5e04269938842ed7b199800c2f4a07f8bb31a025",
        "check.json": "f4b97a7f8be2e1efae481b7c00bfeab38a7826b1",
    },
    "a04-macos-failed": {
        "trace.json": "2285b4105b9e39a74124964f276f6c6f0f3eb189",
        "check.json": "a9e7bdaeb33bfe3e65d5a187e4ca0255b411764b",
    },
}


def counts(jobs):
    return [
        {"state": state, "error": error, "count": count}
        for (state, error), count in sorted(
            Counter((job.get("state"), job.get("error")) for job in jobs).items(),
            key=lambda item: str(item[0]),
        )
    ]


def totals(jobs):
    return {key: sum(job.get(key, 0) or 0 for job in jobs)
            for key in ("replicas", "indexReplicas", "indexLocations")}


def extract(root):
    result = {"schema": 1, "task": "publication-liveness-native-trace-analysis",
              "kind": "static-extraction", "traceRevision": TRACE_REVISION,
              "productionBase": "684dbb5a3a800fa4c380db221b26b2b8722e4260",
              "runs": {}, "limitations": [
                  "statusSeconds is the duration of one sequential sweep over all batch jobs, not the duration of a single IPC command.",
                  "samples[].at is monotonic time at sweep start; each job and network snapshot is read at a different instant.",
                  "All 22 runtime logs in each failed run have zero bytes; no internal SQL counts, fence duration, permit transitions or finalizer result timing was captured.",
                  "A final postageClient.pending snapshot does not establish pending counts at earlier samples.",
                  "The H11 sample ranges are inferred from replica counters resetting after each completed batch; completedBatches and final envelope sequences independently confirm three completed batches and a fourth stalled batch.",
              ]}
    for directory, expected in INPUTS.items():
        parsed = {}
        hashes = {}
        for filename, expected_blob in expected.items():
            path = Path(directory) / filename
            raw = (root / path).read_bytes()
            blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            if blob != expected_blob:
                raise ValueError(f"Git blob mismatch for {path}: {blob} != {expected_blob}")
            hashes[str(path)] = {"gitBlobSha1": blob, "sha256": hashlib.sha256(raw).hexdigest(),
                                  "bytes": len(raw), "sourcePath": str(path),
                                  "url": "https://github.com/glebkudr/kaikichat"}
            parsed[filename] = json.loads(raw)
        trace, check = parsed["trace.json"], parsed["check.json"]
        samples, last = trace.get("samples", []), trace.get("lastSender", [])
        actor = trace["failureActor0"]
        run = {"inputs": hashes, "failure": check["failure"],
               "attemptCheckPath": check["attemptCheckPath"],
               "sourceHash": check["sourceHash"], "binarySha256": check["binarySha256"],
               "runtimeLogCount": len(check["runtimeLogs"]),
               "runtimeLogBytes": sum(log["bytes"] for log in check["runtimeLogs"]),
               "completedBatches": trace.get("completedBatches", []),
               "completedSeries": trace.get("a04Capacity", {}).get("completedSeries"),
               "initialPolicy": trace["ordinaryWallet"]["initialPolicy"],
               "authorityRenewals": [{key: renewal[key] for key in (
                   "issuedAt", "expiresAt", "blockNumber", "snapshotsUnchanged", "observed")}
                   for renewal in trace["authorityRenewals"]],
               "lastSenderCount": len(last), "lastSenderStates": counts(last),
               "lastSenderTotals": totals(last),
               "lastSenderFundingStates": dict(Counter(
                   (((job.get("delivery") or {}).get("postage") or {}).get("funding") or {}).get("state")
                   for job in last)),
               "failureActor0": {key: actor[key] for key in (
                   "pendingOutbox", "postageClient", "postageAuthoritySync", "custodyResolution", "checkpoints")},
               "samples": []}
        for index, sample in enumerate(samples):
            network = sample["network"]
            capacity = network["processingCapacity"]
            run["samples"].append({
                "index": index, "at": sample["at"],
                "sinceFirstSampleSeconds": sample["at"] - samples[0]["at"],
                "statusSweepSeconds": sample["statusSeconds"], "jobCount": len(sample["jobs"]),
                "states": counts(sample["jobs"]), "totals": totals(sample["jobs"]),
                "processingCapacity": {key: capacity[key] for key in (
                    "active", "ordinaryActive", "selectedActive", "capacityRejected", "rateRejected",
                    "startedOrdinary", "startedSelected")},
                "custodyResolution": network["custodyResolution"],
                "peerConnections": len(network["peerConnections"]),
            })
        if directory == "h11-macos-failed":
            run["lastSequences"] = [job["prepared"]["envelope"]["sequence"] for job in last]
            run["lastEnvelopeExpiryRange"] = [
                min(job["prepared"]["envelope"]["expiresAt"] for job in last),
                max(job["prepared"]["envelope"]["expiresAt"] for job in last)]
            run["lastPlanValidUntil"] = sorted(set(job["plan"]["validUntil"] for job in last))
            run["lastDurabilityObservedAtRange"] = [
                min(job["delivery"]["postage"]["durability"]["observedAt"] for job in last),
                max(job["delivery"]["postage"]["durability"]["observedAt"] for job in last)]
            run["inferredBatchSamples"] = []
            for batch, (start, end) in enumerate(((0, 19), (19, 41), (41, 69), (69, 100)), 1):
                part = samples[start:end]
                observed = next((item for item in part if totals(item["jobs"])["replicas"] > 0), None)
                run["inferredBatchSamples"].append({
                    "batch": batch, "sampleStartInclusive": start, "sampleEndExclusive": end,
                    "maxStatusSweepSeconds": max(item["statusSeconds"] for item in part),
                    "medianStatusSweepSeconds": statistics.median(item["statusSeconds"] for item in part),
                    "sampledStatusSweepSecondsSum": sum(item["statusSeconds"] for item in part),
                    "firstObservedNonzeroDataSecondsFromFirstBatchSample":
                        None if observed is None else observed["at"] - part[0]["at"],
                    "lastSampleTotals": totals(part[-1]["jobs"]),
                })
        result["runs"][directory] = run
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace_root", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    encoded = json.dumps(extract(args.trace_root), ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
