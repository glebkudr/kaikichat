# Verifiable finalized ancestry

Finalizer deliveries now carry a bounded canonical proof from the exact delivered
entry to a real stored Simplex finalization. Both Ed25519 and P256 verify target,
hash links, heights, committee and current finite authority using the existing
certificate verifier. Running and embedded service delivery share one exporter
that reads finalized archive data. A direct certificate remains optional; no
parent QC is manufactured and acknowledgement still follows the application effect.

Tests preceded production and received independent no-context FINAL ACCEPT; all
nine [accepted input hashes](accepted-inputs.json) remained unchanged. The
missing-interface RED and nonblocking coverage limits are recorded in
[review.md](review.md). The first ordinary run stopped at a Clippy loop-counter
lint; [that failure](validation-initial.json) and its source manifest are retained.
The code was corrected without changing tests or acceptance limits.

The final run passed 515 ordinary Rust tests (zero failed/ignored), seven model
and nine independent oracle tests, formatting/workspace Clippy, forty frontend
tests, TypeScript and Vite. The targeted 42 tests include both scheme suites,
actual Marshal descendant-QC recovery from synced storage without peers and real
four-service consensus/fault/restart scenarios. The maximum supported 128-entry
proof with full-size payloads also verifies; malformed and substituted paths fail.

The actual selected-daemon EVM gate passed over TCP/Noise and QUIC. Each transport
has four selected profiles with the same thirteen effects; 81 and 87 genuine
certificate signatures were independently verified. Invalid proposers, cross-scope
packet replay, unchanged quorum, role/head/network revocation, storage-effect
failure, held durable cursor, cold unacknowledged replay and real lease expiry
passed. The source chain stopped during voting. There were 850 owner calls and
no cleanup errors. This gate runs production node code with a local signed-work
fixture application; it does not exercise production postage admission.
[Report](daemon-evm.json), [public registry/history evidence](daemon-public-evidence.json).
The public snapshot seed in that evidence is an on-chain beacon, not a signing key.
Raw engine/application logs and disposable profiles remain outside Git.

The ordinary release Tauri app was rebuilt, passed deep/strict ad-hoc codesign
and excludes the automation driver. The bundled verifier accepts both prior genuine
receipts at historical time with unchanged fixed image, independently matching full
journals, and operation/expiry denials. No new proof was generated; the app is not
notarized. All 420 final source inputs remained unchanged.
[Ordinary gates](validation.json), [release](release.json),
[receipt compatibility](compatibility.json), [counts and scope](summary.json).

Portable finality is not a spend/admission certificate. Issuer-global nullifier
state, epoch handover, paid custody/repair and full V1 acceptance remain unfinished.
No owner/MCP endpoint or consensus-frame limit changed. Native UI, Linux network
matrix, full remaining EVM and new-proof process suites were not repeated here.
The domain contract is [ancestry-proof-v1.md](../../../spec/finalizer/ancestry-proof-v1.md).
