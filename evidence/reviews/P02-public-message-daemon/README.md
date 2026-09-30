# Public message preparation through the daemon — accepted

The new strict owner route reuses Core's original-message preparation, native
public wallet, current authority authenticator and common error/serialization
mapping. No caller-provided key, stamp, envelope or operation can replace the
actual saved message. Messaging grants remain unable to control owner funding.

The targeted real-process test and all **194 daemon tests** pass, zero failed or
ignored, on 530 unchanged frozen source files. Workspace fmt/Clippy, 59 frontend
tests, seven model tests, twelve EVM-model tests, TypeScript and Vite also pass.
The unchanged Core dependency is the preceding `d92a957` checkpoint, where all
250 Core tests and 194 daemon tests passed.

The fresh live-EVM/network gate obtains its first actual mark through
`prepare_public_postage_message`. It checks the exact pre-existing MLS envelope,
unrelated policy operation, actual funded random key/signature and unchanged
balance after invalid message/retention requests. Exact preparation persists
through restart and the updated checkpoint after retrieval. Raw reservation
agrees with the same already allocated ticket.

Three independently verified QC signatures authorize storage of the actual
872-byte encrypted message. The full existing SQL-failure/copy/holder-inspection
gate passes. With sender and original storage sources absent, the recipient
automatically discovers, reads and imports the exact original message once.
There are zero owner retrieval calls, zero workflow ZK worker invocations and
no cleanup errors. One ticket remains allocated, three available. Source and
daemon binary hashes are unchanged across the live run.

`paid-message-protocol.json` retains the fresh preparation, signed public mark,
context, finalized record, public envelope, portable obligation and recipient
result. `paid-network-evidence.json` and `module-checks.json` record scope/results.
Full raw trace and execution logs are retained locally and ignored by Git, per
repository storage rules. Their locations and the trace hash remain recorded.
The authorization timing is a test segment including verification/retries and
negative controls, not a network latency benchmark.

```sh
python3 scripts/build-storage.py run python3 tests/evm/public_paid_ciphertext.py
```

Run the live gate after other Cargo commands finish; it checks the executable
hash. It still uses owner IPC for sender orchestration. Automatic sender policy,
UI/CLI/MCP wallet lifecycle and budgets, R10, autonomous repair, independent durable
indexes, handover and remaining V1/desktop/platform acceptance are open. No new
packaged application or native UI run is claimed.
