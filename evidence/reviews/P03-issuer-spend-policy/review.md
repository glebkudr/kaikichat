# Independent backend test review

Baseline: d4719ac. The existing no-context reviewer
`/root/common_context_test_critic` received a standalone request covering the ten
frozen inputs in accepted-inputs.json, the domain contract and the RED logs.
Its FINAL verdict was ACCEPT before production code was changed. Tests and helper
hashes remained unchanged through implementation and validation.

The critic found no blockers or material missing scenarios for this module. It
confirmed actual EIP-1186 proofs, the initializer and pre-funding gate, preserved
funding layout/accounting, one-time policy binding, chain-domain separation, real
rollback/restart and the independent P256 selection oracle. Its nonblocking
suggestion was a direct underpayment check on the derived contract. The unchanged
base issuer suite already covers that behavior; the implementation shares that
exact purchase path and all base tests passed.

An intermediate concern about combining an unbound issuer with an expected zero
policy was withdrawn by the critic: the genuine unbound state also has a zero
stored issuer domain, and existing domain negatives separately cover that guard.
No unreachable synthetic storage state was added. Production explicitly rejects
a zero expected policy before proof verification.

Both RED commands failed because CanonicalPostageIssuer.sol did not yet exist.
That establishes the missing module, not execution of the later assertions. No
stub contract or positive fake proof verifier was introduced. The two initial
Foundry invocations accidentally overlapped, and force-build removed its generated
output directory on failure; the existing build-storage setup command restored
the empty managed directory before further checks. Subsequent Foundry runs were
sequential. Source files, storage configuration and artifact links were preserved.

After ACCEPT, implementation added the derived contract, reused the base purchase
body through an internal method, reused the existing bounded storage verifier and
added a strict CLI command. No dependency installation or version change occurred.

## Fixed guest compatibility regression

The first release build changed the compiled postage image from
ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de to
dd9a7b1d9e4dc8fdec2b90889afc30306b7db71dfb19e38071732c514ef2f405.
The one-off package compatibility check failed; this run is not accepted. A new
permanent compatibility test was written before the fix. Its executable RED failed
on this exact image mismatch. The test fixes neither expected image nor receipts:
it uses both genuine receipts and their independently verified public statements
from d4719ac, checking context, nullifier, resources, operation binding and expiry.
The compile-time type inference error encountered while writing the test was fixed
before review and before the recorded assertion-based RED.

The same no-context reviewer independently confirmed all four extra frozen hashes,
unchanged public evidence, upstream verification of both receipts under the old
image and full independently encoded CBOR journals. It returned FINAL ACCEPT with
no blockers; receipt-name assertion diagnostics were a nonblocking suggestion.
Only then was the host-only policy module/export excluded from target_os=zkvm.
The original funding relation and image expectation were not changed. The targeted
regression passed, restoring the previous image and both real receipt checks.
Final validation includes this test and repeats the backend/frontend and actual
issuer-policy EVM gates after the fix.
