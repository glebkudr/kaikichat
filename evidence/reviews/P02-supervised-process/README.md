# Supervised local postage process evidence

The real Core funded wallet now feeds an explicitly supervised proving CLI. The
length-prefixed private request is followed by an open stdin lifetime channel;
EOF or any extra byte terminates the worker. Existing prove/verify behavior and
fixed guest image are preserved. No daemon owner/MCP job command is added here.

Tests preceded production. The first independent review required complete invalid
frames to retain stdin open, isolating parsing from EOF rejection, and required
observation of actual worker stdout after parent SIGKILL. Both were fixed. Actual
CPU activity is now observed before EOF and parent-death interruption; this is an
active proving pipeline observation, not identification of an internal STARK phase.
The second critic returned FINAL ACCEPT; the earlier CLI produces a runtime RED.
See accepted-tests.json, accepted-contract.md, test-review1/2.json and raw log hashes.

Both new process tests pass. EOF, an unexpected byte and SIGKILL of a separate real
parent leave no receipt. The actual worker PID disappears after parent death. A
fresh SQLCipher reopen preserves the paid mark, exact request and nullifier. A real
subsequent proof with stdin open succeeds at the 4 MiB request boundary; separate
default and independent upstream verifiers confirm the complete public statement.
Proof time 327921 ms, receipt 584987 bytes; image
ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de.

All component runs passed: 471 ordinary Rust, 9 genuine proof/CLI and 2 Core process
checks (482 Rust total), 7 models, 9 oracles, 40 frontend tests, formatting, default/
proving/process-feature Clippy and TS/Vite. The 215 source inputs and 4 binary files
remained unchanged. The two Core checks were run before the nine proof regression
checks; all check-postage command components were exercised, without claiming a
single whole-script or full-EVM/native/Linux/release gate. The corpus is historical
public deterministic test data, not current-authority admission.

Implementation is in crates/postage-zk/src/main.rs; the mandatory driver is
scripts/check-postage.sh. The actual daemon supervisor, funding-anchor retention,
common interval policy, current-authority admission, global spend and full V1 remain
open. Abrupt worker exit does not run Rust destructors; no OS-wide zeroization claim
is made. Product proving uses no explicit private execution-segment files.
