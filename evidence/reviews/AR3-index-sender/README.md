# AR3: accepted ordinary sender index publication

This slice adds automatic paid-index promises and data-holder location publication
for ordinary sender jobs. Candidate 13 passes the complete index fault/restart
gate. The same application source/binary also passes the original ordinary
sender, single-claim index server, hostile history ingress and full cumulative
epoch handover gates. Current affected checks pass 75 backend/21 frontend tests,
Clippy and fmt with 587 matching inputs. The base is `6ee0009`.

- [Ordinary sender regression](regression-sender-candidate-4.json): owner and
  signed agent inputs, real SQL failure/cold restart, wrong-Noise offer rejection,
  offline/cold recipient retrieval; six independently checked QC signatures.
- [Index compatibility](regression-paid_index_network.json): genuine primary and
  copy claims, cold index/copy recovery and original MLS retrieval; three signatures.
- [Hostile history](regression-public_epoch_ingress.json): invalid peer pages
  cannot alter SQL or enable signing; three spends, two closings, 69 signatures.
- [Full epoch handover](regression-public_epoch_handover.json): 133 spends, two
  closings, original historical evidence after lease expiry, 1605 signatures.

All native runs have zero cleanup errors. The handover command itself passed in
1106.14 seconds; its collector initially read the wrong directory. That wrapper
failure is preserved in `regression-handover-collector-path-error.json`. Only the
collector changed after the runs. Initial recovery is separately recorded in
`regression-handover-collector-recovery.json`. `--collect` requires exact equality
with the saved native report and trace, as well as the original successful
command/log, current native source/binary and all inputs. The independent critic
accepted this correction; affected checks were rerun after the final change.

The sender retains a descriptor once, compact verified index promises, and
per-index location ACK positions alongside its existing shared outgoing payment
and ciphertext. SQL failures cannot publish successful progress. Both metadata
growth and later outgoing admission use the same quota. The native acceptance
requires two real ordinary messages, 20 selected index promises, 200 location
ACKs, cold sender/index recovery, unchanged original payment and no owner work
RPCs. Automatic recipient index discovery and successful job retirement remain
open; see [NEXT.md](NEXT.md) and the [specification](../../../spec/postage/public-sender-index-v1.md).

[Test contract and independent critic decisions](TEST_CONTRACT.md) document tests
before production and the refinements. [Native evidence](native.json) records
20 promises, 200 location ACKs, both sender SQL faults and cold recovery.
[Affected checks](checks.json) pass 75 backend tests, 21 frontend tests, Clippy
and fmt with 587 unchanged input fingerprints for candidate 13. Candidate-11
reports are preserved separately. This is not a full-workspace gate.

The preserved `candidate-1.json` through `candidate-10.json` and `candidate-12.json`
are failed runs, not
acceptance. Earlier CPU samples motivated same-call authentication reuse and
background scheduling changes, but these did not fix the native failure. Static
inspection subsequently found location preparation reading incoming storage from
an outgoing-only sender, and a loop able to perform 100 failed preparations in one
advance. The correction exports verified claims from the outgoing ledger and
bounds failed work. A round-robin transfer cursor distributes attempts across
index peers; pending results are consumed before new work can evict the bounded
result cache. Candidate 11 completed in 404.11 seconds with no cleanup errors.
It then failed the original sender deadline; the location cache alone did not
resolve that regression. Batching passed it on candidate 3's source. Candidate
12 stalled at 19/20 promises; missing-primary-only resolution lets candidate 13
pass its index gate (221.35 seconds) and the original sender candidate 4 gate.
[profile.json](profile.json) and [profile-5.json](profile-5.json)
contain diagnostic counts and hashes only; local raw logs/samples are not tracked.

The bounded location cache preserves full-carrier provenance checks. Batching
then replaces 100 location requests per message with ten requests containing the
missing compact receipts for each paid index. Receiver persistence and sender
ACKs are atomic for each batch. The independent critic accepted the tests before
production; all 26 paid-index tests, the node custody codec test, 21 frontend
tests and production Clippy/fmt pass. Protocol stream, byte and admission limits
and native deadlines remain unchanged. These are affected checks and native
regressions, not a full-workspace or full-AR3/V1 release gate.
