# P02 local recursive-STARK backend

This increment implements local proving and bounded verification of the existing
private-postage statement. It does not complete P02, integrate postage into the
desktop/daemon, or close the V1 goal. Machine-readable outcomes, hashes and limits
are in `evidence.json`. Raw run logs stay in managed external `output/postage-proof/`
under the project storage rule; their exact paths and hashes are retained here.

The separate no-context backend critic first required a genuine same-program
Composite control below the receipt size limit, a bounded independent verifier,
relation-specific guest failures and the last valid authorization second. The
revised tests received ACCEPT before host/guest production code. The retained API
RED is a compiler type-check with kernels deliberately omitted; it demonstrates
missing API symbols, not a failing cryptographic execution. The earlier missing
Metal failure is also retained as an environment failure.

The seven release tests subsequently generated actual proofs. They check an
independent process using upstream verification, exact independent Python journal
and nullifier vectors, all changed context fields and time boundaries, tampered
seal/journal, a validly encoded fake claim even with the dev environment enabled,
a genuine different program committing the same journal, a genuine same-program
Composite within the input cap, direct invalid guest execution without host
preflight, and valid JSON at exactly4MiB versus one extra whitespace byte.

The first actual proof took313205ms and serialized to584671bytes on an Apple M4 Pro
with48GiB RAM. Its image ID was
`ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de`.
All seven tests passed in602.31s. Sampling started after the echo control and
observed up to7679232KiB RSS for the test process; this is not an exact whole-session
peak or a measurement of application send latency. The proving cost needs further
work before integration into an interactive sending flow.

That first driver exited127 after the successful tests: it had been edited while
the shell was executing it. It is explicitly not a clean gate result. A fresh
unchanged aggregate then exposed Clippy's host compiler wrapper leaking into the
nested RISC Zero guest build. After a separate ACCEPT of the real failing gate,
the build script was changed to run its guest builder child with only compiler
wrappers removed, preserving build settings, failure propagation and no-skip
guards. A subsequent aggregate reached 466 passing ordinary Rust tests and then
failed a strict Clippy check on the test oracle environment lookup. The equivalent
`unwrap_or_else` correction received separate ACCEPT. Both failed aggregates are
retained. Final aggregate results are recorded separately in `evidence.json`.

`scripts/check.sh` now necessarily runs `scripts/check-postage.sh`; ordinary
workspace tests alone omit its feature-required proof test. The proof driver
checks nested guest formatting, proving-feature Clippy, and release proofs.
`.cargo/config.toml` forces locked nested Cargo builds. The SDK3.0.6 is pinned;
the actual local guest toolchain is r0.1.97.0. There is no remote witness upload,
sender-selected guest identity, dev receipt acceptance or Groth16 proving path.

The final evidence critic rejected an earlier claim that the prover made no
filesystem writes. Upstream LocalProver serializes execution segments to temporary
files; these may include guest memory and private witness input. Normal temporary
directory cleanup does not cover crashes or SIGKILL. Current tests use public
deterministic seeds. Private temporary storage and crash cleanup must be resolved
before exposing this prover to real wallet secrets; this increment does not claim
that protection.

The final unchanged aggregate exited 0: 466 ordinary Rust tests plus all seven
actual proof tests, 29 Solidity, 7 model, 9 fixture/oracle and 40 frontend tests.
Formatting, both Clippy configurations, TypeScript and Vite passed. All 19 freshly
generated EVM reports passed with no reported cleanup errors, including actual
selected daemon services over TCP/Noise and QUIC (867 owner calls). The final proof
took 360279 ms, serialized to 584596 bytes and retained the same image ID; the
seven-proof-test phase took 645.48 seconds. The 23-file source snapshot did not
change during the aggregate. Native/Linux acceptance and app bundling were not
repeated for this unconnected library increment.

The new RISC Zero tools and exported Metal copy were initially placed internally,
then relocated to the existing external APFS image. All419 manifest entries and
2408675049 bytes were verified, sources and Git backed up and preserved. A separate
storage reviewer required registration of the links in the existing storage
manager, then accepted after internal-directory rejection and missing-link recovery
checks. Existing Node/Foundry, general Rustup and Apple's installed system asset
were left in place. rzup was updated to0.5.2 after the primary index showed the
initial0.5.1 documentation result was stale.

Still required: a real shared-root anonymity demonstration with independently
owned deposits, finalized-mark persistence, private temporary-data handling and
proving interruption recovery,
Core/daemon integration, canonical global BFT spending, private future-beacon
custody selection and all remaining E01–E26 product acceptance. Deterministic test
seeds are public fixtures. Neither an external cryptographic audit nor a formal
anonymity theorem is claimed.
