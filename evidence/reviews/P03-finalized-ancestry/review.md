# Independent backend test review

Baseline: 6b95a22. The existing no-context backend-test-critic received a standalone
request covering nine frozen test/contract inputs and the missing-interface RED.
The FINAL verdict was ACCEPT before any production changes. It independently
confirmed all hashes and unchanged production, real quorum signatures for both
schemes, independent CBOR, target/digest/tip assertions, actual Marshal archive
recovery without peers, and preservation of application/delivery/acknowledgement
checks. The RED demonstrates missing APIs, not executed assertions.

Nonblocking suggestions were a fixture with two consecutive stored certificates
to distinguish the first suitable descendant from the last, and checking the
initial manually held Delivery proof in the unacknowledged-delivery scenario.
The current fixture has one descendant QC; the live suite checks proofs through
its normal delivery path, including replay. These suggestions were not represented
as additional executed cases. The implementation walks ascending heights and returns
at the first stored QC, preserving direct certificate() semantics.

The critic also noted that mutated height/committee inputs break hash links at the
same time; they do not isolate every semantic guard under a coherently re-signed
invalid chain. There is no separate wrong-genesis fixture. These were explicitly
nonblocking for the bounded module; no claim of complete mutation coverage is made.
Production checks the full height/link/scope relation and height-one genesis parent.

After ACCEPT, one shared bounded canonical ancestry codec/validator was implemented,
with thin adapters reusing the existing Ed25519 and P256 certificate/lease checks.
One generic exporter reads finalized Marshal records and the real descendant QC;
it serves both Running::finality_proof and service::Delivery::proof. It creates no
votes, certificate, network fetch, archive mutation, admission or handover state.
No dependency was installed or changed. Accepted tests were not edited afterwards.

The extended Marshal case uses actual archive machinery and explicitly assembled
real quorum signatures, not a live quorum producing that particular descendant.
The unchanged four-service fault/restart suite separately runs real consensus and
now verifies each normally consumed delivery proof. The actual daemon TCP/QUIC
regression uses a local signed-work fixture application, never production spend
admission. See the accompanying reports for executed gates and limitations.

The first ordinary validation stopped at Clippy's explicit_counter_loop lint in
ancestry validation. It is retained as validation-initial.json with its original
source manifest. The production loop now uses enumerate and checked height
subtraction; no test, expectation, wire rule or acceptance limit changed. The full
ordinary sequence is rerun after that correction.
