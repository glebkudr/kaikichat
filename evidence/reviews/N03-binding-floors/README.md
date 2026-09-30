# N03: durable Core transport binding floors

Both compact binding verification and full selected-peer presentation verification
now share a signed binding version barrier in the existing encrypted profile store.
Current membership and actual transport authentication precede the barrier. Its
change commits atomically with the checkpoint clock before CheckedFinalizerPeer
is returned. Replayed older and conflicting equal-issued bindings are refused,
including after cold restart and a genuine checkpoint/roster renewal.

The same signed duplicate remains valid within its original lease without rewriting
the floor. A newer short-lived binding keeps its barrier until the maximum possible
older-signature deadline, rather than allowing the old transport to revive when
the newer lease expires. The implementation reuses the existing sixty-second
binding lifetime constant, verifier, SQLCipher states/CAS and checkpoint transaction.
At most sixty-four live floors are retained; a live barrier is never evicted for
capacity. Persisted bytes are bounded and signed observations are revalidated on load.

Five new tests and the reconciled existing deadline/replay test preceded production.
R1 critic review required preserving the old deadline check before observing a newer
binding, plus an actual checkpoint/roster renewal test carrying fresh full proofs.
R2 accepted these changes. A first compile attempt exposed a test-signing API mismatch:
installed p256 normalize_s returns the signature directly. That failure is retained;
R3 accepted exactly its one-line correction before production. It was not counted as
behavioral RED. The subsequent actual baseline run had 33 passed and six failed tests
at the intended replay/persistence assertions; after implementation all 39 passed.

The first full network run failed during TCP at a checkpoint revision conflict;
QUIC was not started. Its trace does not distinguish a clock observation from a
binding commit as the intervening revision change. R4 independently accepted a
deterministic Core interleaving proving that a floor commit advances revision at
unchanged observed time/head, plus a three-attempt live-harness retry restricted to
the exact checkpoint_conflict response. The live gate deliberately forces a real
intervening daemon revision and requires the requested head on retry. All production
stayed unchanged from the failed run; all existing authority/recovery assertions
remain. The failed run is retained, followed by a complete successful rerun.

Validation on the same 483 frozen source inputs:

- 574 Rust tests across 73 suites, zero failed/ignored; 40 frontend tests; fmt and Clippy.
- Two canonical chain fixtures, actual encrypted profile reopen, both API directions,
  equal-issued conflict, exact duplicate/newer recovery, wrong-key/transport/signature
  poisoning controls and a genuinely signed shorter lease. Real SQL INSERT/UPDATE
  failures cover both floor and checkpoint writes. All l2/% states are compared,
  preserving the complete transaction under test.
- The actual TCP/Noise and QUIC selected-service gate, with that reviewed CAS correction,
  passed thirteen durable
  effects per selected profile, 81/75 independently checked signatures and
  936 owner calls, with no cleanup errors. It retains actual
  authority expiry, head/role/network revocation, shared-key announcements, scope replay,
  process crashes and effect-before-ACK recovery.
- The ordinary Tauri application was rebuilt from the same sources. Deep/strict ad-hoc
  codesign, automation-driver exclusion and both historical genuine receipt checks passed,
  including wrong-operation and expiry rejection. The fixed proof image remains unchanged.

All four final accepted test/spec files stayed byte-exact through the final validation
and packaging. The original pre-implementation baseline contains 96 production inputs;
R4 verifies all 97 implemented production inputs stayed unchanged after the failure.
Six helpers remain unchanged. Exact R3 test snapshots preserve the original RED inputs.
All four review revisions, compile-attempt and failed TCP reports, actual RED/focused
GREEN, full gates, network traces, package reports and source manifest are retained here. Detailed logs
remain in ignored output/service-discovery-planning and are referenced by hashes.

The critic's non-blocking suggested tests for the actual sixty-four-floor boundary and
corrupt persisted floor restoration have not been added; no coverage claim is made for
those cases. Fresh genuine proof creation and the complete announcements/DHT paid gate
were not repeated in this slice. Native UI and Linux were not rerun; the app is not
notarized. Persistent DHT addresses, reserved committee connections, DHT roles,
ordinary-client unknown-peer inputs, ciphertext custody/admission/repair and the
remaining V1 product scenarios are still open.
