# Authenticated replies for durable sender history pages

The [sender contract](../../../spec/history-page-sender-network-v2.md) now persists
an exact page's ACK only after the ordinary request ID,
actual peer and connection, relay policy, timeout and clock checks. One bounded
client handles the existing leaf wire and typed leaf/branch/root requests. It
reads committed bytes and resolves the actual retained paid position. Wire mode
and all commitment fields are part of request identity. A late reply can confirm
its old live body without confirming a successor or any missing child.

The ordinary flat-directory publisher uses this immutable page ledger already.
It retains the existing leaf exchange and current-directory/pointer check.
Legacy ACKs are not copied into new pages: an upgraded sender obtains fresh
matched replies. Local SQL or quota failure cannot report a stored page or
advance progress; capacity stays distinct. The existing four pending requests,
sixteen jobs, protocol limits, paid allowance and trust rules are unchanged.

Six new tests use genuine public paid originals and real TCP/Noise Runtime swarms.
They cover five independently confirmed leaf/branch/root bodies, old-root replies,
cold retries, both response modes, every mismatched commitment field and legacy
operation/hash, an actually connected wrong peer, unknown connection, timeout,
missing clock, relay-only policy, completed request IDs, SQL body/entry/head faults,
quota, identity conflicts, bounded pending work, trust and expiry. The private
shared response path accepts the signed fixture clock; ordinary events still use
wall time. There is no IPC clock override or authorization bypass.

Independent backend review first requested an ordinary-worker guard, independent
legacy operation/hash negatives and an actually connected wrong peer. All were
added and accepted before production. The ordinary native lifecycle now requires
the completed exact leaf/hash and all ten ACKs in this ledger, unchanged after a
cold sender restart and still present after a later send. A worker left on the
legacy ledger fails this guard.

## Validation

Final results and source hashes are recorded in
[targeted checks](history-page-sender-checks.json). The 27 backend scenarios include
the six new sender tests, six existing page ingress tests, nine transport
regressions and six sender-store tests. All 31 targeted frontend tests, node
all-target Clippy and formatting pass. Counts overlap prior reports and are not
additional full-product coverage.

Two rebuilt ordinary native scenarios pass on 790 unchanged captured inputs:

- [CLI lifecycle and recovery](history-page-sender-native-c3.json): actual purchase,
  two sends, exact retries and budget refusal, the exact-page ACK/cold-restart
  guard, completion/retirement faults, matching CLI/MCP/desktop API status, and
  recovery with the sender absent after deleting 18 data and 18 index copies.
  Two distinct surviving data holders and two distinct index holders preserve the
  originals. Initial rosters overlap; this run is not the disjoint-roster or
  cross-book gate. It verifies six actual finality signatures.
- [Four sender SQL faults](history-page-sender-fault-native-c3.json): index
  promises, holder locations, history preparation and history ACK commits all
  block publication before and after a cold restart. Removing the faults allows
  completion. All ten ACK peers are independently read; cold retry does not
  rewrite stored index content. Six actual finality signatures are verified.

Both reports identify the same pinned daemon binary; the CLI lifecycle also pins
the actual CLI and MCP binaries. Both exit 0, cleanup reports are empty, and no
owned native process remains. Runs use ordinary daemons/CLI and local Anvil,
without visible windows or Keychain. This renews only the named native scenarios;
it is not current GUI, WAN, >128-message or full-release acceptance. API status
equality does not claim a new desktop rendering check. Local logs remain under
`output/ar2-wallet-flow/`; their hashes and exact reproduction commands are in
the checks file. Full raw traces and temporary credentials are not exported.

The first native attempt reached the actual loss fixture, which still assumed
the old aggregate store. The second removed 18 data copies with exact survivor
checks, then failed an index snapshot that included background maintenance clocks.
Both failures are retained. Independently reviewed test-only corrections now
delete the selected body, metadata, expiry and conflict rows, reconcile quota,
and compare every surviving and unrelated row byte-for-byte with its revision.
The index snapshot covers every retained row and normalized head, excluding only
the head's maintenance clock and expiry cursor. The existing data snapshot API
is preserved. Four sender SQL fault stages likewise target both legacy aggregates
and the new operation/page rows. These helpers are never linked into the daemon.

## Remaining ordinary graph work

The publisher and recipient still use the v1 flat directory. Typed body transport
and durable ACKs do not establish complete graph publication. Next connect bounded
durable graph publication and traversal, exact current-root/pointer fences and
atomic per-reference recipient imports. Every required live child needs its own
paid confirmations; a root ACK cannot cover missing descendants. Cold ACK reads
do not create new availability observations or portable storage proofs.

The ordinary >128 simultaneously live paid native loss/recovery gate remains
required, with cold restarts, disjoint rosters, short leases and missing/corrupt
pages. No allowance or reference limit was raised. Full V1 remains incomplete
within the unchanged 67-card /22-E2E /three-platform scope.
