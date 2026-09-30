# P01/N05 authenticated fixed epoch selection

Full V1 remains OPEN. This adds a real proof-to-consensus integration under the existing
explicit trusted-attestor L2 profile, not native chain finality or a daemon voting service.

## Implementation

- A strict locally selected policy pins network, checkpoint trust, registry domain and n.
  One fixed domain per epoch has no per-request wallet, salt, log ID or preferred peers.
- The existing real Ethereum account/storage verifier authenticates full root/count/seed.
  Custody and finalizer reuse one bounded sparse Fisher–Yates sampler with distinct domains;
  custody's exact existing output stays unchanged. Memory follows requested size, not population.
- All selected paid-unit ordinals and opened signing keys must match. Order is irrelevant;
  duplicate/missing/unselected entries and repeated/weak keys fail. No resampling or n/q decrease.
  Multiple paid units from the same owner remain eligible; no operator-independence claim.
- A private proof authority interval augments Committee without changing its canonical ID.
  Both certificate verification and supervised runtime enforce it. A later accepted checkpoint
  can extend local evidence while preserving the same committee, archives and voting WAL.
- `agentic-finalizer select-committee` produces the verified roster and complete provenance
  through a strict bounded CLI. Existing configured verifier and TCP fixture stay compatible.

Contract: `spec/finalizer/registry-selection-v1.md`. Only existing local workspace crates
were added as dependencies. Existing locked external package versions remain unchanged.

## Test-first review

Six new selection tests and one actual-engine integration test were written before
production. RED logs show absent selection/authority APIs. The independent no-fork
`/root/node_test_critic` required four fixes: a distinct resumed application payload to expose
history reset, valid boundary-sized JSON controls, correctly typed forbidden overrides,
and a bounded shared CLI helper with complete successful input writes. All were applied.
FINAL ACCEPT: selection test SHA256 a74994b4353ed3c165182705dce1b65de50ce1da863d04fabbd321fb974c3e30;
engine test file ff741513f0f6a233829aba27ca299c2714ea9132b5b32c2fb2167e567b27c190.
The existing CLI test retains all assertions and shares its extracted deadline/kill/reap helper.

Independent Python vectors cover n=4,7,10,16 over actual retained Anvil proofs on two chains.
They use a literal full-list sampler and independent CBOR encoding. Reversed member input
must produce identical IDs and quorum; same-owner units are explicitly exercised. Selected
engines finalize a signed history, stop at proof expiry while the registry lease remains live,
then accept a longer correctly signed successor checkpoint and resume the same archives/WAL.
Old entries1–3 retain exact variant0 bytes and entries4–6 require signed variant77 bytes.
Fresh proposal regeneration cannot hide loss of the original history.

The focused run passes all6 selection,13 engine and1 existing CLI tests; all-targets Clippy
passes for finalizer and L2 adapter. Logs: `P01-selection-green.log`, `P01-selection-clippy.log`.

## Fresh actual EVM/CLI gate

`tests/evm/finalizer_selection.py` deploys actual local issuer/registry contracts on two
chains, stops the chain source, then invokes the current compiled CLI36 times. Full output
is compared with the independent oracle for four committee sizes and shuffled input.
Valid nonselected membership, duplicate units, wrong policy domain, insufficient population
and the actual unseeded pre-beacon state are rejected. Successful controls bracket the attacks;
each must run while its evidence is live. The final phase uses actual elapsed wall time to
verify old and renewed proof expiry. The renewed head preserves exact committee identity.

The supplemental gate received separate FINAL ACCEPT after adding live-lease controls,
eager report invalidation and complete dependency/source fingerprints. Input, raw output,
errors and exit code are saved before assertions, including failed attempts. Fingerprints
are checked after builds and scenarios. The oracle refactor was independently verified to
reproduce stored vectors byte for byte. The gate is now part of `scripts/check-evm.sh`.
Its accepted script hash is c6ac66d062c3f4710afd88014451e9e23be0ad742859f6fb33cc2a8afc6765eb.
Focused result: `P01-selection-evm-focused.json`; full proof/transcripts:
`output/evm-e2e/finalizer-selection/`. Cleanup errors are empty.

## Isolated negative controls

The mutation workspace and APFS cloned target are separate from the application build.
Compiling controls deliberately permit an unselected valid unit, remove proof authority,
bind the log to a changing checkpoint ID, ignore the runtime's short proof deadline, and
reset the storage namespace on refresh. Each is detected. The reset control reaches actual
finalization of different bytes and fails the exact old-prefix assertion. A further direct
certificate-expiry control keeps the advertised deadline while accepting beyond it; its
result is recorded separately. Logs and details: `P01-selection-mutations.json`.
All scratch sources are restored after experiments. Current source list/hash:
`P01-selection-source.json`.

## Aggregate verification

The full native macOS gate exited0:395 Rust,24 Solidity,7 model and40 frontend test functions
(466 total), all14 actual EVM runners,5 actual packaged hidden WKWebView flows, release build,
strict deep signature and default-driver-exclusion checks pass. The app remains macOS arm64,
ad-hoc signed and not notarized. Native chat and restored-trust screenshots were viewed
beside the retained2d56a44 references; only expected timestamps differ, with no visible regression.
Terminal log: `P01-selection-native.log`; native result: `P01-selection-native.json`.

The aggregate's fresh finalizer gate passes36 actual CLI checks across chains31337/31338
with all required outcome flags and empty cleanup. Its source hash is
33a10c91dcd88f5d896dd4e4ed45da80a386cca1d353c5d8270005d36af50d49;
compiled binary SHA256 d19c0bede8586febbcb9bf3d7fe2d6e3bfac90b2555982d7b8d80a74115dd62d.
The custody-resolution gate passes295 verified positions through2625 actual owner calls,
including all required lifetime/cancellation controls and empty cleanup. Retained results:
`P01-selection-evm.json`, `P01-selection-custody-resolution.json`.

The separate Linux node transport gate exits0 with7 outcomes and empty cleanup
(run ain-nat-e51bc2a6, source c8e7b699e167b526594e7c99a9035164edfafa229a5e986b71cace9efbd91df9).
It checks existing daemon network behavior, not the selected finalizer runtime on Linux.
Evidence: `P01-selection-network.json`, `P01-selection-network.log`.

## Remaining integration

Core/daemon must own the pinned policy, accepted current head and monotonic time, discover
selected operator endpoints, obtain explicitly enabled own operator keys, and host the voting
service through authenticated channels. The CLI accepts explicit verification inputs and
is not a live daemon current-head authority. Existing TCP executable remains a configured
batch fixture.

A complete authenticated signing-key roster must remain available even when one selected
operator withholds its commitment opening. This API verifies a supplied complete roster;
NodeRegistry currently authenticates opaque owner-bound commitments, so it does not guarantee
the availability of all openings. Repeated or malformed selected keys fail closed. Authenticated
roster availability and eligible finalizer key registration before the beacon are necessary
before claiming Byzantine liveness for an automatically selected service. The fixtures know
all openings and do not establish this property.

A complete Byzantine equivocation/withholding campaign, epoch handover,
product-specific transaction validity and effects, spend, ciphertext custody/R=10 repair,
groups/jobs/trust and full V1 acceptance remain open. A policy change is not authorization
to create another spend domain or reissue tickets. 4/3 remains a test size; stake/ASN/owner
labels do not establish independent honest operators, and beacon manipulation remains a
separate risk. This gate does not claim to solve complete malicious voting-state rollback.
