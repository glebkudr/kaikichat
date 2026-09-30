"""Fresh targeted checks; immutable input fingerprints and local hashed logs."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys
import time

root = Path.cwd()
label, phase = sys.argv[1:3]
assert re.fullmatch(r"[a-z0-9-]+", label)
assert phase in ("baseline", "candidate")
suite = sys.argv[3] if len(sys.argv) > 3 else "AR3-history-automatic"
assert suite in ("AR3-history-automatic", "AR3-history-books")
evidence = root / "evidence/reviews" / suite
output = root / "output" / suite.lower()
output.mkdir(parents=True, exist_ok=True)
assert not (evidence / f"{label}.json").exists(), "fresh labels only"

def inputs():
    paths = subprocess.check_output([
        "git", "ls-files", "--cached", "--others", "--exclude-standard", "--",
        "crates", "apps", "scripts", "tests", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    ], text=True).splitlines()
    paths.append(str(Path(__file__).relative_to(root)))
    paths.extend(str(p.relative_to(root)) for p in Path(__file__).parent.glob('run_*.py'))
    return {p: hashlib.sha256((root / p).read_bytes()).hexdigest()
            for p in sorted(set(paths)) if (root / p).is_file()}

commands = [
    ("core", ["cargo", "test", "--locked", "-p", "agentic-core", "--test", "conversations", "custody_progress::history"]),
    ("store", ["cargo", "test", "--locked", "-p", "agentic-postage-spend", "--test", "paid_index"]),
]
if phase == "candidate":
    commands += [
        ("node-mailbox", ["cargo", "test", "--locked", "-p", "agentic-node", "--lib", "mailbox"]),
        ("node-custody", ["cargo", "test", "--locked", "-p", "agentic-node", "--lib", "paid_custody"]),
        ("node-mailbox-processes", ["cargo", "test", "--locked", "-p", "agentic-node", "--test", "processes", "mailboxes::", "--", "--test-threads=1"]),
        ("frontend", ["node", "apps/desktop/node_modules/vitest/vitest.mjs", "run", "--root", "apps/desktop", "tests/chat-shell.test.tsx"]),
        ("clippy", ["cargo", "clippy", "--locked", "-p", "agentic-core", "-p", "agentic-postage-spend", "-p", "agentic-node", "--lib", "--bins", "--", "-D", "warnings"]),
        ("fmt", ["cargo", "fmt", "--all", "--check"]),
    ]
    if suite == "AR3-history-books":
        commands.insert(2, ("node-connections", ["cargo", "test", "--locked", "-p", "agentic-node", "--lib", "reserved_connection_tests"]))
before = inputs()
(evidence / f"{label}-inputs.json").write_text(json.dumps(before, indent=2) + "\n")
checks = []
for name, command in commands:
    log = output / f"{label}-{name}.log"
    started = time.monotonic()
    with log.open("w") as sink:
        run = subprocess.run(command, stdout=sink, stderr=subprocess.STDOUT)
    content = log.read_text()
    counts = re.findall(r"test result: ok\. (\d+) passed", content)
    if name == "frontend":
        counts = re.findall(r"Tests\s+(\d+) passed", content)
    checks.append(dict(name=name, command=command, exit_code=run.returncode,
                       seconds=round(time.monotonic() - started, 2),
                       tests_passed=sum(map(int, counts)),
                       log_local=str(log.relative_to(root)),
                       log_sha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(checks[-1]), flush=True)
after = inputs()
changed = sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p))
report = dict(phase=phase, wrapper="python3 scripts/build-storage.py run",
              fingerprinted_inputs=len(before), checks=checks, source_changes=changed,
              passed=not changed and all(c["exit_code"] == 0 for c in checks))
(evidence / f"{label}.json").write_text(json.dumps(report, indent=2) + "\n")
raise SystemExit(0 if report["passed"] else 1)
