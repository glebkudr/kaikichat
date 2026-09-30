# P01/F04 — configured Simplex certificates and durable application history

Status: focused implementation, full aggregate, native release and Linux gates pass. Full P01/V1 remains open.

## Contract and independent test review

`spec/finalizer/certificate-journal-v1.md` defines this bounded library/CLI boundary.
It verifies actual Commonware Simplex Ed25519 finalization certificates, then persists
an ordered application log. It neither runs the voting engine nor establishes an L2-selected
committee, global spend, replica custody or economic admission.

Tests were written before production; initial RED was missing planned APIs. The separate
existing no-context-fork backend test critic returned REVISE for five masked or incomplete
negative controls. Revisions isolated the signing namespace with fresh exact-payload
signatures, reopened each corrupted signature and both historical lease boundaries,
used authentic sorted keys for committee size failures, checked validate(allow) followed
by append(deny), and padded a valid CLI request to its exact bound with subprocess deadlines.
The optional genuine nonzero first parent-view case was also added.

The critic then returned FINAL ACCEPT before production. A subsequent narrow FINAL ACCEPT
approved only replacing two unsupported upstream Set collectors with TryFrom<Vec<T>>;
key membership/order, cryptography and assertions did not change. Production edits were
paused while each verdict was pending. Logs preserve RED, initial compiler integration
errors and the successful focused run.

## Implementation

New `agentic-finalizer` workspace crate uses pinned Commonware2026.9.0, verified as current
through official docs.rs and cargo info before installation. Existing Rust1.97.1 compiles
it. All19 resolved Commonware packages have the same version. `Cargo.lock` retains exact
resolved versions/checksums; no production deployment occurred.

Committee validation enforces nonzero domain/log/epoch, finite lease, strict ordered unique
non-weak Ed25519 members and n=3f+1 for4..64 members. Quorum is derived. The complete
configuration binds the signing namespace and genesis. Entry CBOR has a single canonical
encoding, fixed context/operation/parent hashes and bounded opaque payload. Finalization
verification reuses the upstream bounded codec and signature checker. Notarization is
not finalization; two signatures never stand in for the4/3 fixture's required three.

The library reuses `ProfileStore::commit_states`, its CAS and FULL-synchronous SQLCipher
transaction. A returned appended=true record follows the commit. Reopen authenticates the
entire bounded128-entry history, including signatures, accepted time, sequence, both parent
representations, operation uniqueness and actual profile identity. A different committee
cannot reset an existing log. Reads/historical exact-proposal retries survive expiry, while
new appends require a live lease and nondecreasing accepted time. Alternate valid quorum
subsets on the same stored proposal return the original receipt without rewriting state.

Application validation is called on candidate validation and again before new append. The
library does not claim atomic effects in an unrelated database. It holds no voting key and
cannot detect complete valid filesystem rollback/deletion; durable engine-floor protection
and actual application transactions are still required for voting/spending integration.
The128-entry limit fails closed without pruning. This is deliberately a bounded log.

`agentic-finalizer verify` exposes the actual verifier with strict JSON/hex fields and a
256KiB total input bound. Output explicitly says configured committee. It exposes no spend,
key export or reset command and is not bundled into the desktop as an available feature.

## Focused results and sensitivity

`P01-certificate-journal-compile.log`:5 certificate tests,1 actual CLI test,7 journal test
functions pass. The7 journal functions include the child entry point used by the actual
SIGKILL test. Tests use independent minimal CBOR/digest construction and real upstream
Ed25519 keys/certificates. Persistent effects are checked through a second SQLCipher
connection and actual process reopen, not only through in-memory counters.

`P01-certificate-journal-clippy.log`: locked all-targets Clippy with warnings denied passes.

`P01-certificate-journal-mutations.json` and four corresponding logs preserve isolated,
compiling negative controls. Each fails an existing approved behavioral test:
- skipping finalization signature verification;
- skipping the external validity predicate;
- skipping certificate parent/view constraints;
- skipping historical live-lease validation during reopen.

Mutation sources live in an isolated APFS recovery directory; production source was not
mutated for these runs. The aggregate rebuilds from the authoritative workspace afterwards.

## Remaining P01 integration

Actual Simplex Engine/Automaton/Relay/Reporter, authenticated peer channels, voting WAL,
durable floor/recovery, withholding/partition/equivocation and reconfiguration are not
implemented by this module. The current append contract deliberately requires a direct
finalization for every parent; Simplex also supports recursive finalization through a
descendant. Engine integration must implement authenticated ancestry/catch-up for that
case before claiming liveness, not treat a missing direct parent certificate as invalid
application data. Locally configured committee certificates must not be passed
off as L2-selected operator consensus. Full spend state machine, ZK tickets, reserve/consume/
cancel, retained funding evidence, physical ciphertext storage and autonomous repair remain
open, as do the outstanding desktop and agent-work requirements.

## Aggregate evidence

The actual Linux gate exits0 with7 outcomes and cleanupErrors=[]. Run ID
`ain-nat-e53d1116`; source hash
`5120adc02e8c65458afc530f1dbcb39b09bf449a86f25ec7b13d2aa2f44ec526`.
The retained network log and JSON cover actual LAN/NAT/relay/QUIC/AutoNAT behavior.
The aggregate passes373 Rust,24 Solidity,7 Python model and40 frontend test functions
(444 total). All previous actual EVM runners pass. The final custody-resolution runner
checks288 position outcomes through2676 owner calls across two chains, with all mandatory
flags true and cleanupErrors=[]. Source hash:
`44a5cfda4e97a15510d0a565e7c0e32688c408a6e4497ee4f148163b81a35ead`.
The call/position totals vary with bounded expiry observations; the fixed selected positions
and mandatory cancellation assertions are unchanged.

All5 actual hidden macOS WKWebView flows pass. New `alice-chat.png` and
`checkpoint-restored.png` were visually inspected together with86b31b2 references:
layout, messages, delivery state, trust profile/key data and controls match; only generated
timestamps differ. Native JSON and tracked screenshots are retained. The normal macOS arm64 release bundle, strict deep codesign verification and default
driver-exclusion check all pass. The bundle remains ad-hoc signed and not notarized.
The final aggregate log exits0 and ends with Native macOS gate passed.

The finalizer source/lock/spec input digest is
`5641b807fd18bb01bc2039ee21b63c094a40c05940dc29b1ad83bdbdafd5b572`;
`P01-certificate-journal-source.json` lists the exact files. No implementation source
changed during the aggregate runs. The Linux network gate verifies the existing daemon;
the new finalizer library tests in this module were executed on macOS. Linux engine
integration/testing belongs to the next P01 runtime stage.

`P01-certificate-journal-cli-input.json` uses the approved deterministic public fixture keys,
not a user profile. `P01-certificate-journal-cli-output.json` is the actual verifier response.
Committee ID: `3d52f619c32b69ffebfaef891638f1de9568534f05887f1c0e855001b51b893c`;
entry digest: `c2a587e7be264d8d907cb7f28cfd836a92065bcf1c80f5967d129b54e509e7b7`.
The input was emitted from the approved helper in an isolated build, and passed to the
actual workspace binary. The fixture build log is retained.

The retained CLI artifact was additionally checked using the independent existing Python
CBOR reference and Python cryptography Ed25519, without importing the Rust encoder or
Commonware verifier. All3 literal fixture signatures verify; changing the subject to
notarization rejects all3. Both configuration and entry digests match. This is a manual
cross-implementation fixture check, not a general alternate consensus verifier; report:
`P01-certificate-journal-independent.json`.
