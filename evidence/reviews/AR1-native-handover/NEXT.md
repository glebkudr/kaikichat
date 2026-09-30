# Remaining integration checks

The full native transfer gate passed on attempt 7. Public trace evidence and
source/binary fingerprints are retained. Full backend/frontend regression,
TypeScript/Vite, model tests and workspace Clippy/fmt pass; consolidated evidence
is in checks.json. Continue with the remaining acceptance gates below.

The additional configuration/bootstrap failure window is now fixed and its native
SQL-fault test passed. Both selected and ordinary-client configuration use one
state-batch commit. Same-revision live/cold retry, paid-input preservation and
absence of premature successor journals are recorded in configuration-atomic-check.json.

The additional hostile closing/history ingress gate now passes on a real Noise
connection: authentic wrong-target closing, altered original QC and mixed pages
all preserve exact SQL state and prevent an early signer. Completed responses
and repeated queries prove transport delivery. Exact original answers restore
all three records after crash and cold reads. The independently reviewed fixture,
three genuine spends/two closings/69 signatures and targeted regression are in
[AR1-hostile-history](../AR1-hostile-history/). This supplements the large native
gate without replacing its cumulative length, expiry or SQL-fault coverage.

Outage closure and automatic current-authority/context renewal remain necessary
for continuous use. Successful manual publication/configuration in the handover
gate does not establish these lifecycle transitions. Sender retirement/fairness,
AR2–AR5, 67 mandatory cards, 22 E2E and all three platforms also remain open.

The AR2 build boundary described below has now been implemented and verified in
[AR2-verifier-build](../AR2-verifier-build/). The clean verifier, actual macOS
debug/release bundles, explicit legacy tests and full regression all pass.
The following paragraphs retain the original test contract for that work.

An AR2 build boundary was directly visible in the accepted AR1 source:
crates/postage-zk/build.rs always calls risc0_build::embed_methods, including the
wrong-image test guest, even when only legacy verification is requested. The
existing compatibility test retains a real old receipt and fixed expected image
ID. Splitting the frozen verifier identity from optional guest/prover generation
can therefore be checked against actual existing evidence. This should remove
the guest compiler from ordinary V1 build inputs without changing the accepted
legacy relation or allowing caller-selected images. It is planned work, not part
of the handover acceptance.

For that build split, retain the existing compatibility test's original receipts;
do not replace them with newly generated proofs. Add a clean verifier-build gate
with guest/kernel generation disabled that still authenticates both old receipts,
their exact context/nullifier/resources, and refuses expired or changed contexts.
Use an isolated target inside the managed build volume. The default workspace
dependency graph must omit the guest builder; selecting only default members is
insufficient. The explicit proving feature must still compile the real guest,
bind its image to the same frozen identity and pass the existing genuine-proof
and wrong-image rejection checks. Review the new tests independently before
changing the build graph. No new dependency version is needed for this boundary.

The handover trace also exposes avoidable repeated requests after source admission
is exhausted (418 writes for 32 useful answers in full6). A later bounded retry
policy should preserve revocation/fence checks and progress without increasing
source quotas. The immediate correction is only to the test's transfer deadline;
retry pacing is not claimed implemented or accepted by this gate.
