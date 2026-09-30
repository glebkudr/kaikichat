# Separate backend test critic

Reviewer: `/root/common_context_test_critic`, initially spawned with no inherited
context, reused for this bounded test review. HEAD before production: af760bf.

1. **FINAL ACCEPT before production:** no blockers. New tests exercise owner and
   agent IPC boundaries, MLS availability, actual fixed-verifier receipts without
   sender wallet/prover, shared capacity, cancellation/reaping, restart, finite
   authority and pending/completed revocation. The reviewer checked the genuine
   short certificate's sequence, predecessor and signed time construction, and
   that the callback preserves the original paid scenario's owner head.
2. **FINAL ACCEPT before production for small test improvements:** add elapsed
   time guards to distinguish shared-pool eviction from sixty-second expiry,
   delayed exact-view comparison to pin checkedAt, and invoke the new wrapper from
   check-evm.sh. The wrapper executes all original daemon scenarios plus the new
   callback, so the regression gate does not need duplicate expensive proving.
3. **ACCEPT for test-harness correction after partial actual execution:** the
   first EVM run produced and independently verified a genuine receipt, then
   passed the new foreign receiver, shared worker and I/O scenarios. Starting an
   auxiliary receiver failed because its Unix socket path was 107–110 bytes on
   macOS. Two directory-name substitutions reduce all auxiliary sockets to 96
   bytes. The reviewer reversed exactly these substitutions and reproduced the
   accepted test hash; assertions, deadlines and fixture semantics are unchanged.
   This failed run does not establish the later authority/retention scenarios.
4. **ACCEPT for isolating the authority-change fixture:** the second actual run
   passed every new remote-verification scenario, then the old proving regression
   refused its pinned assignment after restart. A fresh real-EVM diagnostic showed
   that renewing the authenticated conversation peer correctly causes the owner
   to synchronize that newer checkpoint after restart. The unchanged authority
   and retention block now runs on an independent receiver; the connected peer
   remains in the responsiveness tests. New assertions preserve both original
   conversation heads. The reviewer found one missing extracted-function argument,
   proving_request, which was passed explicitly before rerunning. Reversing the
   extraction reproduces the second accepted hash; all existing cases and deadlines
   remain intact. The combined scenario subsequently passed completely on the
   release daemon; see evm-release.json and validation.json.

The initial Rust baseline exited 101: the existing proof scenario passed, while
the new scenario failed at the missing postageVerifications capability. The
independent Python entry point also failed at that absent runtime field before
starting Anvil/proving. These are actual missing-interface failures; they do not
claim that later cryptographic success/negative scenarios were executed before
implementation. Positive verification uses the genuine fixed verifier; delaying
helpers exec that same verifier, and I/O-error helpers are negative controls only.

A genuine certificate signed under a foreign profile remains directly covered by
Core tests, not isolated by the new daemon callback. No new daemon coverage claim
is made for that particular case.
