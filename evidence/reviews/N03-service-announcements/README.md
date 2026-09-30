# N03: bounded connected-peer service announcements

The ordinary daemon queries each connected interested peer for one bounded active-key
batch. Untrusted key labels filter local interest before Core verifies current membership,
finite signatures and the actual authenticated transport. The shared prepared batch also
feeds DHT publication and deduplicates a key used by several local applications.
Automatic selected-key × connected-neighbor probing has been removed. Explicit owner
requests and renewal for an already known route retain the existing full-proof protocol.

Bounds remain explicit: four pending five-second requests, twenty-second refresh,
sixty-four connected peers, four entries per response and per-peer/global admission.
The batch limit reflects four running local service slots and does not reduce registry
selection or quorum. Disabling a role invalidates the batch; re-enabling triggers discovery
on an unchanged connection. Relay-only cannot install a route from a direct connection.

Tests preceded implementation. The independent no-context critic initially required
actual ordinary-daemon producer verification, eighth/ninth/other-peer admission controls
and deduplication of a shared key across two applications. R2 accepted the revised tests;
actual missing-module compile RED preceded production. Four focused Rust tests then passed.

The first live attempt passed TCP and QUIC preflight phases but failed when its relay-only
readiness predicate observed a TCP listener before the asynchronous QUIC listener appeared.
That failed run is preserved. R3 independently accepted only the required-listener predicate
change with the same ten-second deadline; all production and other frozen sources stayed
unchanged. The second complete live attempt passed. The specification is retained as the
original reviewed test contract; this report records the implemented and verified state.

Validation on the same 481 frozen source inputs:

- 569 Rust tests in 73 suites, zero failed or ignored; 40 frontend tests; fmt and Clippy.
- Actual TCP/QUIC producer responses independently verify. Thirteen ordinary neighbors
  include one silent peer. The TCP receiver sends fourteen initial queries to fourteen
  actual neighbors, checks only two interested entries, rejects a valid signature borrowed
  from another PeerID, and installs the subsequent valid selected route. Uninterested labels
  do not invoke Core binding verification. A direct peer under relay-only produces no
  announcement query, binding check or route; successful relay delivery is not claimed here.
- Two fresh genuine fixed-image receipts. The unchanged DHT/finality scenario uses only two
  ordinary seeds, discovers four selected daemons without supplied selected endpoints,
  recovers address movement and cold restarts, and finalizes the paid operation with twelve
  independently checked QC signatures. 2,428 owner calls; no cleanup errors.
  The source chain is offline during voting.
- The extended TCP/Noise and QUIC service gate retains every previous assertion and adds
  actual shared-key batch checks on all four selected daemons per transport. Thirteen durable
  effects per selected profile, 78/78 independently verified signatures,
  914 owner calls, no cleanup errors. Existing real expiry,
  role/head/network revocation, replay, crash and effect-before-ACK checks remain intact.
- The ordinary Tauri application is rebuilt from those sources. Deep/strict ad-hoc codesign,
  automation-driver exclusion and both historical genuine receipt checks pass, including
  operation-substitution and expiry refusal. The fixed proof image is unchanged.

`test-review.json`, all three reviews and their inputs, `red.json`, `failed-attempt-1/`,
`preflight.json`, `live-run.json`, the discovery and network traces, full test/package reports
and source manifest preserve the evidence. Detailed command logs remain in ignored
`output/service-discovery-planning/`, referenced by hashes.

This checkpoint does not complete V1. Persistent service-address and binding rollback
protection, reserved committee connections, DHT client/server roles and ordinary-client
unknown-peer inputs remain open. Ciphertext custody/admission/repair and the remaining
product scenarios remain open. Native UI and Linux were not rerun in this slice; the app
is not notarized.
