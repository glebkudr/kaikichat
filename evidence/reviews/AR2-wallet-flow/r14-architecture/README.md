# R14 architecture review intake — 2026-09-14

`architect-review.md`, `analyze_r14.py`, `R14_FOCUS.json` and
`architect-measurements-README.md` preserve the user-supplied analysis.
Their advice is source material; the selected implementation plan is
[V1_HISTORY_LIFECYCLE_R14.md](../../../../Docs/V1_HISTORY_LIFECYCLE_R14.md).
Links beginning with `sandbox:` in the preserved review refer to the author's
environment and are not local evidence paths.

The script was read before execution. It uses only standard-library JSON,
CBOR decoding, hashing and file reads, with its result written to the explicit
output path. A temporary directory linked its three expected inputs to:

| Script input | Actual input relative to repository root |
| --- | --- |
| `review/runtime/hr-r14/trace.json` | `output/hr-r14/trace.json` |
| `review/R14_FOCUS.json` | this directory's `R14_FOCUS.json` |
| `project/evidence/reviews/AR2-wallet-flow/history-range-native-r14.json` | `evidence/reviews/AR2-wallet-flow/history-range-native-r14.json` |

The invocation was `python3 analyze_r14.py --root <temporary-directory>
--out <temporary-directory>/measured.json`. Parsed output matched every field
of the supplied measurement JSON and is retained as `structural-measurements.json`.
The temporary directory was removed. The large original runtime trace stays in
the existing build/output storage.

Separately, the complete `postStop` object was compared with existing R14
evidence; original31 data/index holders were found by its exact operation in
retained `remaining` rows, not by indexing the graph-ordered survivor arrays.
The 21 supplied code files were compared byte-for-byte with the repository.
Hashes and limits are recorded in `recheck.json`.

This reproduces the author's structural algorithm; it does not provide a second
cryptographic oracle, execute a native scenario, or establish the cause of the
original31 failure. No production source or backend test was changed during
this review intake. Existing R14 failures and all V1 acceptance requirements
remain open.
