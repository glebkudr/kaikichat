# Native public book daemon IPC

Accepted module: six owner routes over the existing Core wallet. No new keys,
wallet database, funding rules, worker or peer protocol. Contract:
`spec/postage/public-book-daemon-v1.md`.

Separate test critic R1 FINAL ACCEPT preceded production. Actual Rust process RED
and fresh canonical EVM RED both reached the missing `public_postage_intents`
route. Reviewed test/helper/spec inputs remained unchanged throughout acceptance.

GREEN: all 194 node tests in three nonempty suites, no failures or ignored tests;
workspace fmt/Clippy; 59 frontend tests, TypeScript and Vite. The backend run
holds 394 source/test/manifest inputs unchanged. The previous full workspace
760-test result remains evidence for its earlier Core checkpoint, not a new
whole-workspace run for this adapter.

Fresh Anvil chains 31360 and 31361 each purchase a four-ticket public batch on the
canonical issuer using the daemon-generated key's actual commitment. Four IPC
clients concurrently submit one duplicate and two distinct operations. The
duplicate returns the exact stamp, while three unique operations allocate indices
0–2. A separately owned receiving daemon and independent Python Ed25519/CBOR
oracle verify the resulting evidence. Re-import cannot replenish the balance.
After refresh, source shutdown, historical certificate expiry and process restart,
exact reservations survive and the fourth index is allocated once. Exhaustion
and current checkpoint expiry reject use. Invalid funding and altered signatures,
operation/index and request authority cannot create new valid allocation.

Both nonexistent worker paths and executable tripwires pass. The tripwire's
separate self-test records one invocation; the full public workflow adds none.
Proof/verification job counts remain zero. See `evm-checks.json` and
`evm-green-serial.log` for the accepted run, including unchanged source and binary
fingerprints. No private key is exported to the funding test/oracle.

`evm-concurrent-build-rejected.*` is NOT accepted evidence: both behavioral
scenarios finished, but a parallel Cargo regression build replaced the executable
and the final immutable-binary check failed. No assertion was weakened. The full
EVM gate was repeated serially after Cargo finished. A transient wrapper refusal
while Foundry's force-build recreated `foundry/out` was retried after that build;
no storage guard was bypassed or layout changed.

Run through managed build storage:

```sh
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --all-targets -- --test-threads=4
python3 scripts/build-storage.py run python3 tests/evm/public_postage_daemon.py
```

Run the EVM gate without concurrent builds that replace the daemon executable.
Owner funding routes remain unavailable to messaging grants. The process test
also confirms actual scoped MLS delivery. This does not yet claim automatically
paid MLS delivery, desktop wallet UI, MCP budget policy, epoch handover, R10 or
full V1 completion. No new packaged app/native UI E2E was produced in this module.
