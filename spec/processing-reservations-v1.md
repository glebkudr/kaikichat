# Shared processing capacity for selected services

Shared request/response processing capacity alongside connection reservations.
A passing resource module is not a finished product.

## Scope and authority

All daemon request/response behaviours share one per-swarm admission budget:
delivery, bootstrap and the mailbox swarm protocol. Outbound streams count against
the same budget (2026-09-26: the finalizer, checkpoint, operator-proof,
announcement, postage and custody protocols were removed with the replaced path).
No protocol or connection gets an independently refillable copy. Preserve existing
wire codecs, payload bounds, timeouts, outbound-job bounds and all Core verification.
Keep Kademlia, Identify, AutoNAT and relay on their existing independent bounded paths.
The selected frame behaviour must be polled before ordinary application behaviours.

Reuse the current authenticated connection-reservation catalog, including original
signed key/transport hints, newer checked routes, public/local authority and pending
client-request signals. A handler's actual authenticated PeerID determines eligibility;
a requested protocol, supplied key, owner pin or bootstrap address does not. A signed
selected hint may reserve proof-discovery processing before the full route exchange,
which itself still must pass Core verification before any service frame is accepted.

Derive all codec instances for a connection from that connection's PeerID. The locked
libp2p request-response implementation synchronously clones its codec when constructing
each inbound/outbound handler, then clones that bound handler codec for request workers.
A wrapper may bind its codec factory for exactly that synchronous construction call,
clear the factory afterwards and preserve the immutable bound peer in worker clones.
An unbound codec must fail closed. Do not edit or fork the dependency or infer the peer
from request content. All access to a connection factory is internal to its behaviour.

In-flight service processing retains the original independent reservation sources.
Any original live source keeps its reservation; losing all original sources invalidates
subsequent IO polls. A new authority/request grant cannot revive an old token. Its
physical slot and byte charge remain occupied until the owning operation is dropped.
Selection grants capacity, never signer or application validity.

## Bounded resources

Across all request/response protocols and connections:

- 16 ordinary and 48 selected operations, at most 64 active operations total.
- Ordinary operations: at most 4 per peer; all classes together at most 16 per peer.
- Each class reserves at most 4 × (16 MiB + 1024 bytes); each peer at most
  2 × (16 MiB + 1024 bytes) across both classes. This does not let one peer own
  the selected pool.
- An operation conservatively charges the maximum of its existing request and response
  bounds before its codec reads/writes. Charges cover incomplete reads and inbound
  requests waiting for the application's response, not just a completed decode call.
- Acquire at most 256 ordinary operations per second, at most 64 per ordinary peer;
  selected operations have an independent 1024/second and 256/peer allowance. Dropping
  a finished/error/cancelled operation releases its active/byte charge, not its rate
  allowance. Monotonic one-second windows bound retained per-peer rate accounting.
- No unbounded wait queue for processing slots. Refuse before reading a body when
  capacity/rate is unavailable. The existing finite request deadlines cancel stalled
  workers and release allocations. Keep the underlying per-connection stream limits.

These are codec-operation and wire-size reservations, not a claim about total process
heap or a hard percentage of CPU time. Decoded outbound responses remain subject to
existing bounded local job counts. Runtime scope/transport/receipt checks, engine
queues and application admission remain mandatory. Unverified ordinary frame claims
must not populate the selected frame-admission table before transport validation.

Owner node_info.processingCapacity reports actual/peak per-class active and byte
charges, fixed limits, total admitted counts, capacity/rate refusals and bounded active
peer accounting. This read-only observation must not acquire slots, create grants,
dial, verify a proof or reset counters. Network replacement creates a new budget and
retires the old swarm; it cannot transfer old in-flight reservations into new authority.

## Required evidence

Six focused resource tests cover shared class/per-peer slots, weighted largest-response bounds,
shared protocol handles, resource reuse only after actual token drop, independent
source revocation/non-revival, completion versus rate refill and real signed expiry.
They test the resource component, not cryptographic authority or a network by themselves.
A seventh test uses the production bootstrap behaviour between two real TCP/Noise
swarms: four requests finish decoding while their response channels are held; a fifth
is refused before the server timeout. Response completion and cancellation each
release a charge and allow reuse. Dropping the swarm drains all remaining workers
while application response channels remain held. The prior eight connection-lifecycle
tests remain unchanged except registration of the new test modules. Tests and critic ACCEPT precede all production changes.

Extend the existing actual four-profile TCP/Noise and QUIC service runner, keeping
its original replay, authority/role/network replacement, durable-effect/cursor failures,
slow replay and connection-capacity assertions. Existing ordinary fan-in keeps64 real
peers; its optional load mode now negotiates actual frame and bootstrap protocols.
Half its held requests use each protocol. Real partial CBOR is written without EOF;
regular small writes observe remote refusal, not simulated counters or hidden owner
request injection. An external baseline first requires at least32 distinct ordinary peers with failures occurring
within3 seconds of their individual request start, earlier than the unchanged5-second
server deadlines and7-second test-client deadline. Old per-connection limits admit all64
held streams and fail this control before any new diagnostic-field assertion.

The live test must then observe16 shared ordinary readers, both protocols represented,
no selected charge for ordinary identities and actual timeout/reuse after more than5
seconds. Effect13 must independently certify on all4 selected profiles. Observe capacity
and effect counts inside the convergence loop, including during queuing. Require
pending-finalization observations and at least8 held readers in a majority of these
samples, allowing brief timeout/reacquisition gaps. Next all64 ordinary peers send complete frames claiming a
real selected P256 key from unrelated Noise identities. Demonstrate transmitted work,
actual unverified responses from at least32 peers and rate rejection; no such request
may enqueue or reserve selected processing. Effect14 must certify under this traffic. Capture load and effect counts inside
the same convergence loop; require pending observations and fresh completed wire
writes between the first pending observation and convergence.
The held-read phase retains the original200ms request cadence. Only the completed
traffic phase uses a20ms cadence, staggered across the64 active independent peers,
with at most one outstanding request per peer. A200ms synchronized burst can hit
the16-reader ceiling while never admitting256 operations per second; increasing
offered completed work exercises the independent rate boundary without lowering
any production limit or weakening any test assertion. Retain the latest capacity
and raw-counter snapshot when this phase has not yet met all conditions.
Stopping load must drain ordinary charges to zero while all64 sockets remain. The
original short-head control becomes effect15; after its actual expiry, queued effect16
must still be refused. Independent QC verification and cleanup remain unchanged.

Preserve the carrier's idle default for client-discovery and connection tests; rate/
load observations use bounded aggregate records. Run complete backend/frontend,
fmt/Clippy, the client genuine-spend TCP/QUIC gate, service TCP/QUIC and announcement/
DHT/genuine-spend regressions, then ordinary Tauri packaging and receipt compatibility.
Freeze exact inputs throughout gates. Exclude all test-only carriers/helpers from the app.
