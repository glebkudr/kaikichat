# L02 authenticated registry proofs — integration evidence

This increment connects the committed NodeRegistry contract to the existing Rust Ethereum
verifier, explicit trusted-attestor checkpoint profile and a real CLI. It does not select
keepers, grant role possession, prove independent copies or close the full V1 goal.

## Test-first history

New Rust API and adapter tests, independent Anvil fixtures and an actual CLI gate preceded
production. RED evidence is retained in L02-registry-proof-rust-red.log,
L02-registry-checkpoint-red.log and L02-registry-proof-cli-red.log. The first two show absent
Rust APIs; the last force-built the existing CLI and received a controlled refusal for the
new command where verified registry output was required.

The separate context-free node_test_critic first returned REVISE because both positive
membership examples selected only the final active entry. A verifier summing every sibling
could return the right ordinal by accident. The revised corpus preserves index2/ordinal1
and adds index1/ordinal0 belonging to another owner/key, with a real nonempty right sibling.
Both are verified against the full root/count, including after subsequent mutable-tree edits.
The actual CLI gate checks both exact members. The critic then returned FINAL ACCEPT.

A nonblocking helper suggestion was also addressed: old and new checkpoint CLI tests reuse
one helper with a ten-second deadline, child kill/reap and the existing exit/stdout/stderr
assertions. A separate stdin writer includes input blocking in that deadline. The prior ten
checkpoint tests passed after extraction, before new production work began.

## Implementation

RegistryProfile strictly parses the selected chain, address, genesis, deployed runtime code
hash and immutable parameters. Domain/config encoding matches Solidity. The shared Ethereum
account/storage proof function now accepts a const-sized slot list; funding keeps its exact
four words and registry uses five words from snapshots mapping slot6. Limits, account/code
membership, storage paths, absence and duplicate-key rejection are reused, not reimplemented.

The verified snapshot checks count/range, captured seed, exact finite admission deadline,
future block relation and supplied checkpoint time. Its lifetime is the earlier of admission
and checkpoint lease. Ordered membership verifies every sibling hash/count, the complete
root/count, owner-bound key opening and active ordinal. Cached membership cannot outlive the
snapshot or move its check time backward. Opening is not a node-role possession proof.

TrustedCheckpoint::verify_registry rechecks lease, the pinned issuer and matching registry
chain/genesis, then supplies its exact root/block/time/lease. The registry deployment remains
an explicit local choice. Existing immutable trust manifest encoding is unchanged.

The command agentic-l2-adapter verify-checkpoint-registry accepts bounded strict JSON and
returns explicit trusted-attestor provenance, snapshot fields and one verified member.
It never returns finalized, availableBalance, independent-copy claims or spend authority.
Existing funding command behavior is retained through the common parser/verification context.

## Real evidence and limits

The independent generator force-builds current contracts, checks the actual compiler storage
layout, deploys real bytecode on local chains31337/31338 and fetches eth_getProof. It retains
real absent, unseeded, post-seal and rolled-back states; local storage injection creates real
MPT paths for separately tested invalid semantics. Python signs the actual headers with an
explicit test quorum. Isolated wrong-chain/genesis fixtures retain valid registry MPT and valid
quorum/manifest/issuer binding so those adapter checks cannot hide behind unrelated failures.

Direct checks passed: eight registry Rust tests, four adapter/CLI tests, the prior ten
checkpoint tests, workspace Clippy and the live two-chain Rust CLI gate. The final native gate completed with exit 0: 300 Rust tests, 24 Solidity tests
(256 fuzz runs each), 40 frontend tests, all five packaged hidden WKWebView flows,
release bundling, deep/strict ad-hoc signature and production-driver exclusion. The live
registry gate made 28 real Rust CLI verifications across two Anvil chains with empty cleanup
errors. Current native chat and restored-checkpoint screenshots were viewed alongside the
existing component references; text, controls and columns remain readable. The older trust
reference predates automatic peer fetching and therefore has different explanatory copy.

The first nine-mutation run found a test gap: omitting the final total-count comparison
survived, while ordinal, root, cached expiry, checkpoint lease, chain/genesis, deployed code
and storage MPT mutations were killed. This is not a missing production check: it already
exists. Initial report/logs and the pre-revision test file are retained with initial/before-count
names. The revised fixture authenticates a root containing two units together with an in-range
scalar count of three. Snapshot authentication must succeed; both valid member paths must
then fail specifically with Membership. The live CLI also rejects a valid quorum over that
state. The separate critic returned FINAL ACCEPT for the revision. The final run killed all
nine compiling mutations, with a passing unmutated baseline and empty cleanup errors.
Production code did not need a change for the added count oracle.

No registry deployment is public, no dependency was installed, and no live chain observer or
node placement was added by this verifier. Daemon/Core integration, node-role possession,
committed selection, paid admission, actual R=10 custody and repair remain required work.

## Open aggregate verification and storage incident

An initial native and Linux run both passed before the additional count fixture/test.
The final native run passed after that revision, but the final Linux run
`ain-nat-791772a6` failed after six successful outcomes at
“previously public address withdrawn after firewall change”. Its exact source hash was
`5a38c944e748e13747700162507417e71bb835bcadc5c3f499720fbfe8d8fbe2`;
cleanup errors were empty. Failed report/logs are retained as
`L02-registry-proof-network-withdrawal-failed.*`. The complete diagnostic retry `ain-nat-b9b71a36` passed all seven outcomes with
empty cleanup errors on the identical source hash/image. Four subsequent isolated executions
of the unchanged AutoNAT assertions also passed: `ain-nat-9c142fe0`, `ain-nat-eb4f3d50`,
`ain-nat-5c51af83`, `ain-nat-b5ddf3fc`, all with empty cleanup errors. The diagnostic scripts
only restrict scenario selection and record status/firewall observations; production and
acceptance assertions were unchanged. In the traced full retry, firewall denial caused
private/no-public-address after a failed fresh probe, removal restored public success, and
service loss withdrew the route again. The original failure remains unreproduced, not fixed;
retain it for investigation if it recurs. These current passing runs do not close full V1.

During final Git checks WD4000 disappeared from /Volumes. Git returned I/O errors and exited
139; no commit or canonical synchronization of this increment was completed. The APFS
worktree remains available. A separately verified source/evidence copy was created at
`/Users/glebk/Library/Caches/agentic-internet/recovery/registry-proof-20260905T212045Z`.
WD4000 subsequently returned mounted at the original path without disk repair by this task.
Both branches still point to 855a736, the canonical working tree is clean and the changed
APFS worktree is intact. Git diff validation passed. A full shared-repository fsck reported
invalid AppleDouble `._*` sidecar ref names on exFAT; no missing-object error appeared.
The two real branches were exported to a standalone APFS bundle to preserve their history.
The original failure and recovery evidence do not imply that fsck passed on the exFAT repo.

The exported bundle was cloned into an independent bare APFS repository; `git fsck --full
--no-dangling` there passed with exit 0, checking the actual preserved branch history and objects
without the exFAT sidecar ref names. Mutation-verified production hashes remain unchanged.
