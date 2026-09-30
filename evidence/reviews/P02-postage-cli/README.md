# P02 local CLI and memory-backed execution

Validation passed: 475 Rust tests (466 ordinary and nine actual proof/CLI),
formatting, default/proving Clippy and nested guest formatting, plus 40 frontend
tests, TypeScript and Vite. The backend driver exited 0 and the 25-file source
snapshot stayed unchanged. The proof suite completed in 670.14 seconds.

This increment follows the accepted recursive-STARK
backend at 07dae6d and its clean aggregate; it does not complete P02 or V1.

The no-context test critic required moving the helper below tests/support and
checking the DEFAULT CLI exact receipt-string cap separately from its outer JSON
cap. Revised tests received FINAL ACCEPT before production. The existing real
positive proof test then failed on unchanged LocalProver with TMPDIR/TMP/TEMP set
to an ordinary file: an executed RED, not a mock or missing compiler. A separate
CLI build RED confirmed that the binary target did not yet exist.

The implementation uses upstream in-memory segments and the local proving server,
rejects RISC0_PPROF_OUT, bounds stdio and preserves receipt bytes exactly. The CLI
clears its raw input buffer and typed witness on drop, without claiming to zeroize
upstream allocations or control OS swap/core dumps. Verification asserts a relation
at supplied context and explicitly returns admission:false. Funding checkpoint
authentication, persisted wallet recovery and global spending remain separate.
The correctness vectors use explicit historical verification times and a 35-second
context lifetime; a roughly six-minute proof is not a live admission demonstration.
Core must derive and recheck actual time when integration is implemented.

The mandatory driver builds and preserves a DEFAULT-feature verifier before the
proving CLI and separate upstream oracle. Nine tests retain all previous genuine
proof controls, generate the shared proof through the actual CLI with blocked temp,
and exercise valid JSON at prove4MiB, receipt4MiB and verify-request8MiB boundaries.
Raw logs remain in managed external output; exact paths and hashes are recorded
in evidence.json. No current native/Linux rerun or new packaged app is claimed.

The actual CLI proof took 360038 ms and contained 584675 JSON bytes, with unchanged
image ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de.
Observed prover RSS reached 7192224 KiB in 30 samples beginning after proving had
started; this is not an exact whole-run peak. The default verifier binary is
2772368 bytes, versus 88695296 bytes for the prover; hashes are retained separately.
The proof was accepted by both the default product verifier and upstream oracle
without private input. Exact inner/outer JSON bounds and negative controls passed.

EVM/contract/daemon/UI source did not change. The preceding complete aggregate at
07dae6d passed all 19 EVM reports; it is historical evidence for this new CLI
increment, not a newly executed full aggregate. Native/Linux and app bundling were
not repeated. Shared-root ownership, persisted wallet interruption and real-time
admission remain open; neither the full P02 milestone nor V1 is complete.
