After ACCEPT add two real Core tests to mandatory scripts/check-postage.sh after its unchanged nine genuine proof/CLI tests:
cargo test --locked --release -p agentic-core --features postage-process-tests --test registry process_acceptance -- --nocapture --test-threads=1
The sourced build-postage-cli.sh already supplies mandatory separate prover, default verifier and upstream oracle binaries, plus external scratch. Tests fail if these environment variables are absent; no skip/fallback. No new dependencies/versions or guest/image changes.
