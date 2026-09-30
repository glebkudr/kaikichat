# Finite selected connection reservations

Contract for the connection part of service-discovery-v1 step4. Test-first review,
RED/GREEN and release evidence are recorded separately under evidence/reviews.
Stream/CPU isolation and DHT client/server roles remain separate required work.

## Admission and resource contract

Keep the existing64 ordinary established connections and ordinary16 pending outgoing
dials. Add a separate bounded reserve: at most64 eligible PeerIDs, at most192 total
established connections and48 total pending outgoing dials. Incoming unauthenticated
handshakes retain their32 total bound. All peers, including reserved ones, retain the
existing per-peer bound (normally2; relay-server profile without AutoNAT server uses1).
The192 ceiling is64 ordinary plus up to2 connections for each of64 eligible peers;
it does not choose a consensus committee size or change a quorum.

Reuse the existing libp2p absolute/pending/per-peer limit implementation. Add only the
ordinary-class accounting it lacks. Never use its peer bypass as an absolute-limit
exception. Admission runs before every stateful protocol creates a connection handler.
ConnectionEstablished/Closed and DialFailure/ListenFailure update real bookkeeping;
a pending close does not release physical capacity. No new public owner/MCP method
can create a reservation or register arbitrary authority.

## Source and lifetime

A private Rust reservation contains PeerID, the current observation time, an expiry
at most60seconds later and a shared revocation signal. Runtime grants it only for a
currently selected key needed by a live Core-approved local service. Reuse the current
authority/fence, signed service hint and actual
checked route types. A configured bootstrap address, claimed role/key, unverified
client-supplied PeerID or an unbound discovery result alone is insufficient.

For each selected key choose the current finite signed hint or newer verified route;
do not reserve both old and new transport identities during a rotation. The permit's
expiry is no later than either the signed binding or local authority. Reservations
for a shared peer remain valid while any current source remains live. Bound input
work at256 grants (four local registrations, each with at most64 keys), coalesce
duplicate peer grants and retain at most64 distinct peers. An invalid
replacement does not replace the previous valid bounded catalog. Revoked source
signals still invalidate its old reservations immediately.

Role disable, head replacement, network replacement, actual expiry and a clock earlier
than a reservation's validated observation cannot preserve that reservation. Propagate
the current Core time; never mask rollback with a fabricated future observation.
Relay-only policy still governs every route and dial; a reservation grants capacity,
not a new transport path, vote, signer, application effect or custody admission.

When formerly reserved sockets become ordinary and exceed64, retire only the excess
ordinary sockets, newest first, preferring to retain the established ordinary clients.
Poll emits a bounded exact-connection close once per outstanding retirement. Counts
remain occupied until the corresponding ConnectionClosed. Re-evaluate classification
at admission and runtime polling so a revoked signal needs no extra wire message.

Owner node_info.connectionCapacity exposes established/ordinary/reserved occupancy,
the numeric caps and a bounded reservedPeers list. It reads observations and cannot
create authority, request proofs or dial. Its absence of reserved peers is explicit.

## Test-first evidence required

Trait-level tests drive actual NetworkBehaviour admission and FromSwarm lifecycle:
64 ordinary plus128 reserved sockets; per-peer policy; a193rd attempt under a valid
rotation before old sockets close; exact retirements and positive reuse; ordinary16
versus total48 pending dials and32 pre-auth inbound; failures freeing pending slots;
catalog overflow refusal; shared source revocation; real finite lease expiry.
These synthetic lifecycle events are not claimed as a network or Core authority test.

The existing full daemon TCP/Noise and QUIC runner additionally keeps its genuine
registry, four selected profiles, certificate oracle, scope replay, revocation,
crash/cursor, slow effect and expiry cases. After cold replay it removes configured
bootstrap addresses and only the ordinary bootstrap cache from one stopped profile,
verifying every other state is unchanged. Other selected nodes remain stopped until
64 independent ordinary swarms occupy the target. They then start and must reconnect
while all64 clients stay connected. A65th ordinary dial is refused, then succeeds
after one actual ordinary socket closes. Role disable retires excess demoted selected
sockets; re-enable restores them; a fresh thirteenth operation finalizes on all four
nodes under the same ordinary fan-in. No new selected endpoint is injected.

Full backend/frontend, fmt/Clippy, previous announcement/DHT/genuine-spend scenarios
and ordinary packaging remain required after production edits. Test-only fan-in code
must be excluded from the ordinary app. This increment does not establish the full
V1 outcome. In particular, current service-cache/verified-route capacities16 still
need alignment with larger configured committees; the guard's64-peer ceiling alone
does not prove a64-member daemon mesh. Quorum must never be lowered to fit resources.

Pending ordinary postage-client reservations are deferred to their own required
test-first module. This implementation must not use Client::targets() or its
owner-supplied PeerID as a reservation source. That module needs a real selected key
paired with an unrelated PeerID as a negative control, and a pending client with an
authenticated binding progressing under a full ordinary pool as its positive control.
