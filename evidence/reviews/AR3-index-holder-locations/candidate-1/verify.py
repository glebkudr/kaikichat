"""Run affected clusters only; invoke through scripts/build-storage.py run."""
from pathlib import Path
import concurrent.futures
import hashlib
import json
import re
import subprocess
import time

ROOT = Path.cwd()
EVIDENCE = ROOT / "evidence/reviews/AR3-index-holder-locations"
OUTPUT = ROOT / "output/ar3-index-holder-locations"
OUTPUT.mkdir(parents=True, exist_ok=True)
paths = subprocess.check_output([
    "git", "ls-files", "--cached", "--others", "--exclude-standard", "--",
    "crates", "apps", "scripts", "tests", "Cargo.toml", "Cargo.lock",
    "rust-toolchain.toml",
], text=True).splitlines()
paths.append(str(Path(__file__).relative_to(ROOT)))
inputs = {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest()
          for p in sorted(set(paths)) if (ROOT / p).is_file()}
(EVIDENCE / "source-inputs.json").write_text(json.dumps(inputs, indent=2) + "\n")


def run(name, command, cwd=ROOT):
    started = time.monotonic()
    log = OUTPUT / (name + ".log")
    with log.open("w") as sink:
        process = subprocess.run(command, cwd=cwd, stdout=sink, stderr=subprocess.STDOUT)
    raw = log.read_text()
    suites = [tuple(map(int, m)) for m in re.findall(
        r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", raw)]
    changed = [p for p, h in inputs.items()
               if not (ROOT / p).is_file() or hashlib.sha256((ROOT / p).read_bytes()).hexdigest() != h]
    result = dict(name=name, command=command, cwd=str(cwd),
                  wrapper="python3 scripts/build-storage.py run", exit_code=process.returncode,
                  elapsed_seconds=round(time.monotonic() - started, 2),
                  log_local=str(log.relative_to(ROOT)), log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),
                  source_changes=changed, fingerprinted_inputs=len(inputs))
    if suites:
        result.update(passed=sum(s[0] for s in suites), failed=sum(s[1] for s in suites),
                      ignored=sum(s[2] for s in suites))
    if name == "frontend-history":
        counts = re.findall(r"Tests\s+(\d+) passed", raw)
        result["passed"] = int(counts[-1]) if counts else None
    (EVIDENCE / (name + ".json")).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result), flush=True)
    return result


with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    frontend = pool.submit(run, "frontend-history", [
        "node", "node_modules/vitest/vitest.mjs", "run", "tests/chat-shell.test.tsx",
    ], ROOT / "apps/desktop")
    results = [run("paid-custody-index", [
        "cargo", "test", "--locked", "-p", "agentic-postage-spend",
        "--test", "paid_index", "--test", "paid_custody",
    ])]
    results.append(run("public-custody", [
        "cargo", "test", "--locked", "-p", "agentic-postage-spend",
        "--test", "public_spend", "custody",
    ]))
    results.append(run("clippy", [
        "cargo", "clippy", "--locked", "-p", "agentic-postage-spend",
        "--lib", "--test", "paid_index", "--test", "paid_custody", "--test", "public_spend",
        "--", "-D", "warnings",
    ]))
    results.append(run("fmt", ["cargo", "fmt", "--all", "--", "--check"]))
    results.append(frontend.result())
passed = all(r["exit_code"] == 0 and not r["source_changes"] for r in results)
(EVIDENCE / "checks.json").write_text(json.dumps({
    "passed": passed, "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
    "scope": "targeted custody/index plus frontend history; no complete workspace or native network gate",
    "fingerprinted_inputs": len(inputs), "checks": results,
}, indent=2) + "\n")
raise SystemExit(0 if passed else 1)
