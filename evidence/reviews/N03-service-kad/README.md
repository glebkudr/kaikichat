# N03: shared Kademlia queries and automatic selected-service discovery

The ordinary daemon now publishes and resolves signed service address records through
the existing bounded Kademlia transport. It installs a selected route only after the
existing Core verifier checks current membership and the actual authenticated connection.
Mailbox and service queries share two active slots and sixteen retained results, with
separate typed consumers. Publication requires two actual distinct matching receivers;
records are checked again when a completed lookup is consumed.

The independent no-context critic first required an actual singleton-ACK refusal and
expiry of an already completed, undrained lookup. Both were added before FINAL ACCEPT.
Actual compile RED then preceded production changes. Six focused tests passed, including
independent raw TCP/Noise and QUIC Kademlia peers, publisher shutdown, hostile records,
capacity refusal and finite expiry. Four accepted files remain byte-exact; runtime.rs
is compared after removing only its four documented production additions. Its accepted
test registration remains exact. All six fixture helpers stayed unchanged.

Validation on the same 477 frozen source inputs:

- 565 Rust tests in 73 suites, zero failed or ignored; 40 frontend tests; fmt and Clippy.
- Two fresh genuine fixed-image receipts. Four ordinary selected daemons know only two
  ordinary DHT seeds, find the exact selected keys and PeerIDs, recover a changed address,
  finalize a paid operation and restore connections/results after all four cold restarts.
  Twelve QC signatures are independently checked; 1,696 owner calls; no cleanup errors.
  The source chain is offline during voting. No selected endpoint is supplied by the test.
- The unchanged selected-service TCP/Noise and QUIC gate passes thirteen durable effects
  per selected profile, 81/72 independently checked signatures and 899 owner calls, without
  cleanup errors. Existing role/head revocation, same-connection binding renewal, real
  crashes, replay rejection and effect-before-ACK checks remain intact.
- The ordinary Tauri application is rebuilt from those sources. Deep/strict ad-hoc codesign,
  automation-driver exclusion and both genuine historical receipt checks pass, including
  operation substitution and expiry refusal. The fixed proof image is unchanged.

`test-review.json`, both review revisions, `red.json`, `live-run.json`, `discovery.json`,
`discovery-trace.json`, `run.json`, `finalizer-network.json` and its two traces, the source
manifest and package reports preserve the evidence. Detailed command logs remain in
ignored `output/service-discovery-planning/`, referenced by hashes.

This checkpoint does not complete V1. Legacy key/peer probing is still present alongside
the new DHT path. Bounded announcements, persistent service rollback protection, reserved
committee connections, DHT client/server roles and unknown-peer ordinary client inputs
remain open. Ciphertext custody/admission/repair and remaining product scenarios remain
open. Native UI and Linux were not rerun in this slice; the app is not notarized.
