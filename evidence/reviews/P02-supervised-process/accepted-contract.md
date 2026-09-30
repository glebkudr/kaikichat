# Proposed supervised local postage process v1

Add an explicit `agentic-postage prove-supervised` mode to the existing prove-enabled
binary. Keep `prove` and `verify` unchanged; the default verifier rejects the new
command. No new public owner/MCP/network API or spend capability is added here.

The supervisor sends a four-byte unsigned big-endian length, followed by exactly
that many bytes of the existing strict {context,witness} JSON request. Length must
be nonzero and at most 4 MiB, before allocation. Reject short headers/bodies, unknown
or duplicate JSON fields and malformed requests with empty stdout and the existing
safe failure diagnostic. No private values in argv, environment, files or errors.

After parsing the entire request and installing the stdin lifetime watcher, emit
exactly `postage worker ready\n` to stderr. This acknowledges framing and an installed
watcher, not a validated relation or proof. Continue genuine local proving while the
supervisor holds stdin open. Any further stdin byte, EOF or read error aborts the
entire worker with failure, including after the parent is SIGKILLed. Do not rely on
a graceful parent handler or an in-process cancellation flag. On success stdout
contains only the existing bounded genuine receipt; the supervisor keeps stdin open
until worker exit. Ready plus the fixed safe error is allowed after cancellation.

Use the current private-input zeroization and in-memory proving implementation.
The existing fixed guest/image and independent verifier remain unchanged. There
is no reimplemented cryptography or test prover substitute. This protocol is for
trusted local subprocesses; the future daemon supervisor still owns concurrency,
wall-clock deadlines, bounded output, receipt authentication and durable job state.

Core integration tests must obtain input from the newly implemented encrypted
funded wallet API. Actual worker cancellation by EOF and unexpected lifeline byte
must produce no receipt. An actual separate parent process must be SIGKILLed after
worker readiness, and the actual worker PID must disappear, not merely be reported
as cancelled. A fresh Core reopen after interruption must preserve paid/intent
rows, exact prepared input and nullifier. A real subsequent supervised proof with
stdin still open must succeed, and the separate default verifier and independent
upstream oracle must check its complete public journal/context/nullifier. This
uses public deterministic real-payment fixtures and historical relation time; it
does not claim fresh runtime admission or a production daemon job controller.

Boundaries include exactly 4 MiB valid JSON accepted and one-byte-over-limit valid
JSON rejected, incomplete frames and the default verifier. Parent/worker cleanup
must be bounded even when an assertion fails. Complete malformed/oversized frames
retain stdin open until actual rejection with FAILED only, isolating parsing from
EOF cancellation. The SIGKILL case captures the actual worker stdout and requires
it empty. Before EOF and parent-death interruption, observe at least one second
of worker CPU time, so an idle process waiting for input cannot satisfy the test.
This observes an active local proving pipeline, not a particular internal STARK
phase. Preserve all existing one-shot proof/CLI regressions. No paid state is
spent or rewritten by a local process.
