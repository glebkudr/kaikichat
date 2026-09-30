# Daemon public receipt verification

The owner can start, read and cancel a foreign receipt verification using public
checkpoint history and a current registry proof. Core derives the common statement
and supplies an opaque current-authority context; the daemon never accepts a
caller-selected clock, interval, context, image or executable in IPC. A node with
only the fixed verifier can perform this operation without sender custody state.

The verifier runs outside the daemon actor using the existing bounded child I/O,
deadline, cancellation and reaping helper. Proving and verification share one
active slot, sixteen retained jobs and sixty-second terminal retention. Old proof
response shapes remain compatible. Verification responses expose successful
nullifier/resources only after fixed verification and a fresh Core check. Reading
or retrying a completed result rechecks authority and clears those outputs after
revocation; checkedAt remains the original successful completion time. All views
have admission:false and no verification spends or modifies paid-wallet records.

Test review and the real missing-interface RED preceded production. The ordinary
gate passed 492 Rust tests, seven models, nine independent-oracle tests, formatting
and workspace Clippy. All forty frontend tests, TypeScript and Vite passed. The
accepted test inputs and 328 source inputs are frozen for validation. The wrapper
is included in check-evm.sh and preserves all original paid daemon scenarios.

The combined current-time EVM scenario passed on the release daemon. It generated
a genuine 584820-byte proof in 309400 ms and checked the entire journal with the
independent upstream oracle. A receiver without sender wallet or prover verified
the same receipt. Malformed input, wrong operation and altered seal were rejected;
MLS continued during held workers; cancellation, graceful shutdown, bounded pipes,
deadline, shared proving capacity, exact retries and process-local retention passed.
Both pending and completed results were revoked after a real head change; renewed
public authority accepted the identical receipt. Actual short signed leases expired
for pending and completed jobs. No paid-wallet mutation occurred.

All original daemon proving checks also passed, including real worker cancellation,
daemon SIGKILL and wallet recovery, independent proof verification, wrong verifier
outputs and fresh authority at completion. This was the third combined run: the
first exposed an overlong auxiliary Unix socket path, and the second exposed
automatic checkpoint synchronization between connected fixtures. Their incomplete
reports, the confirming fresh-EVM synchronization diagnostic, and separately
accepted test-only corrections are retained. Production stayed unchanged through
all three runs. The final 328 source inputs and five accepted test inputs matched
their frozen hashes. The two ordinary IPC scenarios also pass in release mode.

The current packaged debug application passed all five hidden WKWebView scenarios:
messaging and delivery receipts, close/reopen history recovery, scoped MCP with
revocation, relay preferences/restart and checkpoint trust/recovery. Chat and agent
permission screenshots were visually compared with the previous verified native
screenshots; their layout remains consistent. The same five native gate source
files stayed unchanged. A fresh ordinary release Tauri app was then built, its
deep/strict ad-hoc signature verified, and its default dependency graph excludes
the automation plugin. It is not notarized. The real receipt scenario ran the
release daemon before bundling; the native application scenarios used the hidden
debug automation bundle. These are distinct validation results.

Scope remains receipt verification. Public epoch-history availability, daemon
common-policy proving, issuer-global canonical spend and complete V1 acceptance
remain unfinished. The one-MiB collective IPC limit is unchanged; individual
history/registry/receipt ceilings do not imply their maxima fit simultaneously.
No new guest, verifier image, schema or dependency is introduced. This increment
does not repeat the Linux network matrix, full EVM remainder or separate full
real-proof suites. Raw execution logs stay in the ignored output directory, as
required by AGENTS.md; their paths and hashes are in validation.json. Compact
results, review corrections, incomplete reports, the genuine public receipt and
native screenshots remain alongside the successful final evidence.

Contract: spec/postage-circuit/daemon-verification-v1.md.
