# P01 P-256 voting runtime and TCP recovery

Baseline:735d7f2. Full V1 remains incomplete. Typed public P-256 committees now run the
shared actual Simplex engine. A separate configured batch executable also runs P-256
validators over authenticated TCP. Daemon-owned selection policy, current-head handling
and enabled operator keys are the next integration boundary; these fixtures do not provide
owner/MCP spend, independent ciphertext custody or the full E01–E26 product contract.

## Tests before implementation

All13 existing engine scenarios moved into support/engine_cases.rs without changing their
assertions or logic. Normalizing whitespace gives the same SHA256 before and after:
51a16d3aeb7603060cc82f5435c6edc2fed26bc20f0d0411a0ee923da4b8a5a3.
Two entry points supply concrete crypto types and independent fixture keys/namespaces.
A14th scenario seeds the actual metadata store with independently encoded ID/genesis/key
bytes; the correct record must allow a verified own vote, while a foreign genesis must
fail before any activity. It covers the existing96-byte Ed25519 value and97-byte P-256 value.

The P-256 fixture uses upstream real signatures and an independent minimal-CBOR committee
ID. Its selected renewal scenario consumes retained actual FinalizerRegistry/Anvil proofs,
requires the shorter proof deadline, then resumes the same WAL and archives using a new
head. Old entries remain byte-exact; entries4–6 require the changed signed work variant77.
Ed25519 passed14/14 on unchanged production. P-256 RED was the absent generic runtime API.
The separate no-fork backend-test-critic returned FINAL ACCEPT before engine implementation.

Next, all3 existing TCP subprocess scenarios moved into support/tcp_cases.rs. The Ed25519
key helper preserves its previous derivation; P-256 uses independent fixed test scalars.
Both negative private keys and the foreign public key are valid curve keys outside the
roster. The old executable passed3/3 before production changes. P-256 RED was solely the
missing new executable. The same independent critic gave FINAL ACCEPT before the shared
executable implementation. A later two-line `.as_slice()` fix for Clippy also received FINAL
ACCEPT: the exact comparison bytes and every assertion were preserved. Original failure,
RED and GREEN logs are retained beside this report.

## Implementation

The sealed EngineCommittee trait supplies concrete committee/public/private/scheme/binding
types and delegates to each committee's existing finite authority. RuntimeConfig and Running
default to the old Ed25519 committee. One start/initialize path owns supervision, startup
checks, lease shutdown, guards, five channels, actual Simplex, Marshal, broadcast, backfill,
voting WAL and immutable archives. There is no separate P-256 consensus implementation.

Storage prefix remains `ain-finalizer-{logHex}-{publicKeyHex}`. The binding is raw
committeeID32 || genesis32 || publicKey, preserving the old Ed25519 record byte for byte.
P-256 adds its concrete33-byte compressed key. No length prefix or reset/import is added.
Signatures use the existing P-256 committee namespace and upstream Commonware2026.9.0 with
an identity BiMap. No dependency or lock file changed; no custom curve code was introduced.

Both executable wrappers compile the same validator.rs against concrete crypto types.
`agentic-finalizer-validator-p256` accepts the existing strict configured batch format with
P-256 committee members/peers and a32-byte scalar via stdin. It validates exact peer coverage,
private storage permissions, interprocess locking, batch identity and finite authority.
Authenticated TCP has a distinct P-256 namespace; the Ed25519 executable keeps its old one.
Only durably processed entries are emitted, including the recovered prefix. These are
configured batch executables; the selected-proof JSON is not silently promoted to authority.

## Focused evidence

Both suites pass14 engine scenarios: actual quorum, Byzantine application rejection,
structural guards,2+2 partition and healing, offline catch-up, unacknowledged redelivery,
crash after own vote without equivocation, bounded128-entry epoch, lease/binding/membership,
descendant finality and proof renewal over the same journal. Both also pass3 actual Unix TCP
scenarios: SIGKILL after a real QC before all24 entries, progress by three survivors, cold
recovery without peers, complete catch-up after restarting peers, invalid configuration
before readiness, and a storage lock whose rejection cannot be explained by a port conflict.

An additional four-process P-256 run retained12 identical entries per process. Independent
Python/OpenSSL decoding verified144 actual signatures and rejected144 wrong-vote-subject
signatures. It checks canonical CBOR, complete parent chain, proposal fields, signer bitmap,
quorum, low-S and full certificate consumption. All process stderr files are empty; cleanup
is empty. The exact executable hash, test-only inputs, wires and verifier are retained in
P01-p256-runtime-demo/. This is independent certificate checking of a live configured run,
separate from the fault tests and from the selected-proof deterministic runtime tests.

Three isolated compiling mutations are detected: changed fixed binding bytes, an incorrect
signing namespace, and canonical committee end substituted for the shorter proof shutdown.
The scratch workspace first passed14/14, then each mutation failed its intended accepted
scenario. Every modified scratch file was restored. Actual product sources remained unchanged
throughout the aggregate run. Logs and P01-p256-runtime-mutations.json retain the results.

## Remaining product integration

The finalizer crate is currently an independent workspace target: no desktop/Core/node
package depends on it. Application production sources and dependency locks were unchanged.
This increment does not add P-256 voting to the packaged desktop app. The previous735d7f2
native5-flow/Linux7-outcome evidence concerns those unchanged application sources; no fresh
native, Linux or release/notarization claim is made for the new P-256 runtime here.

Next, persist the finalizer-specific registry/policy beside the existing custody registry,
reuse the durable checkpoint clock and current head, and keep P-256 operator secrets in the
encrypted profile under explicit owner controls. Feed typed authority into the service and
stop/revalidate it on head, lease or role changes. Historical funding retention, bounded
spending, custody/repair, group/jobs/trust and all remaining V1 acceptance remain required.

## Complete aggregate for these sources

`scripts/check.sh` exited0:422 Rust +29 Solidity +7 model +40 frontend test functions,
498 total. All16 actual EVM reports are fresh and pass; cleanup lists are empty. Formatting,
workspace all-targets Clippy, TypeScript and frontend build pass. Custody resolution verifies
295 positions through2619 owner calls, with required lifetime/cancellation controls; the
legacy selection CLI passes36 checks and the public P-256 CLI passes28. Source hashes stayed
unchanged through the run. P01-p256-runtime-validation.json and the hashed EVM manifest retain
the scope and results. No additional native/release/Linux run is claimed for this increment.
