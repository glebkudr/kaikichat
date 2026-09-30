# Durable public sender admission

Accepted Core module; the full V1 application remains unfinished.

An ordinary owner or authenticated runtime send in an enabled conversation now
commits the original MLS message/outbox, separate custody job and one explicit
sponsorship reservation in the same SQLCipher transaction. The job survives
direct acknowledgment and restart. Operation retries preserve the original job
and usage, including after exhaustion/full queue. Runtime budgets are keyed by
the grant verified by the existing broker; SendMessage alone cannot sponsor work.
Policy updates, removing/readding grants and restored book aliases retain usage.

Ten accepted tests cover genuine MLS delivery/decryption, scoped signed calls,
actual SQL faults and recovery, quota/queue saturation, preserved retries,
missing/corrupt records and positive recovery controls. The 128-message case
fills the real queue despite acknowledging every original message.

Final frozen R4 verification: **779 Rust tests**, zero failed/ignored,
50 nonempty suites, 612 unchanged source inputs;
workspace fmt/Clippy; 59 frontend tests, 7 model and 12 EVM-model tests,
TypeScript and Vite. Full Rust run: 885.864 seconds.
See [module-checks.json](module-checks.json), backend/frontend checks and input
manifests. [Critic decisions](critic-review.md) distinguish accepted results from
earlier compilation/runtime failures. Raw execution logs remain local and ignored.

Queue reservations are sponsorship units, not already allocated stamp indices.
Jobs report queued, never paid/stored/delivered. Native ticket allocation and
its current balance remain the existing public-message preparation API's job.
The daemon scheduling/configuration adapter, spend finality, selected custody,
private pointer publication and UI/CLI/MCP flow remain open; see [next gate](NEXT.md).
R10/repair, independent durable indexes, handover and remaining V1/platform gates
are not accepted by this module. No new packaged application is claimed.

Fresh regression on the current daemon also passed the complete existing public
paid-MLS EVM/network gate: 3 independently verified QC signatures,
872 actual encrypted bytes, storage/copy/holder inspection,
restart/fault recovery and automatic recipient retrieval after sender/original
source loss. One ticket is allocated, three remain available; worker invocations
and owner retrieval calls are zero. The source and binary hashes remained fixed.
See paid-network-evidence.json and the curated paid-message-protocol.json.
Full raw trace is retained locally and ignored; its SHA256 is in module-checks.

This regression still uses owner calls to orchestrate the sender. It does not
exercise an enabled automatic admission policy or establish an automatic sender.

```sh
python3 scripts/build-storage.py run cargo test --locked --workspace --all-targets -- --test-threads=4
python3 scripts/build-storage.py run python3 tests/evm/public_paid_ciphertext.py
```

Run the live EVM gate only after other Cargo builds finish; it binds its results
to the actual daemon executable.
