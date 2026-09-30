# Node capacity: connections and request processing (V1)

Every node bounds the connections it keeps and the request/response work it
does, so that no peer or protocol can exhaust it. These are wire-size and
operation bounds, not a limit on total heap or CPU. Code:
`crates/node/src/reserved_connections.rs`, `processing.rs`,
`processing_budget.rs`, `processing_gate.rs`.

Both layers have two classes, *ordinary* and *selected*. The selected class
was built for the committee of the replaced storage path; today nothing in
production grants it (`Reservation` is kept for the holders' swarms, "phase
2 directory"), so every peer is ordinary. The selected limits below stay in
code and in `node_info` and are exercised only by tests.

## Connections

Admission runs before any stateful protocol creates a connection handler;
established, closed and failed events keep the counts exact, and a pending
close still occupies its slot.

| Bound | Value |
|---|---|
| Ordinary established connections | 64; 128 on a DHT server that is not relay-only |
| Established connections in all | 192 |
| Peers with a selected reservation | 64 |
| Pending outgoing dials | 16 ordinary, 48 in all |
| Pending incoming handshakes | 32 |
| Connections per peer | 2; 1 on a relay server without an AutoNAT server |

- A reservation is private to the runtime: a peer id, a lease of at most
  60 seconds and a revocation signal. A configured address, a claimed role
  or an unverified peer id cannot create one, and no owner or agent method
  can.
- When selected sockets turn ordinary and the ordinary class is over its
  limit, the newest excess sockets are closed, keeping established ordinary
  clients.
- `node_info.connectionCapacity`: established, ordinary and reserved counts,
  the reserved peers and the limits.

## Request processing

All request/response protocols of the node share one budget per swarm:
delivery, bootstrap and the mailbox swarm, inbound and outbound streams
alike. Kademlia, Identify, AutoNAT and the relay keep their own bounds.

| Bound | Ordinary | Selected |
|---|---|---|
| Active operations | 16 | 48 (64 in all) |
| Per peer | 4 (16 across both classes) | 16 |
| Bytes charged | 4 × (16 MiB + 1 KiB) per class; 2 × (16 MiB + 1 KiB) per peer across both | same |
| Operations started per second | 256, 64 per peer | 1024, 256 per peer |

- An operation is charged the larger of its protocol's request and response
  bounds before its codec reads or writes, and holds the charge until the
  worker is dropped, including while an inbound request waits for the
  application's answer. Finishing releases the slot and bytes, not the rate
  allowance.
- There is no wait queue: without capacity or rate a request is refused
  before its body is read. The protocols' own deadlines cancel stalled
  workers.
- Codecs are bound to the connection's authenticated peer id when libp2p
  builds the handler; an unbound codec fails closed. A lease keeps the
  reservation sources it started with; a later grant cannot revive one.
- The mailbox protocol also passes the access gate before its bytes are read
  (units, peers let in by a book's pass, and a small separate budget for
  showing a credential): [access-by-book-v1.md](access-by-book-v1.md).
- `node_info.processingCapacity`: active, peak and byte charges per class,
  the limits, started counts, capacity and rate refusals and per-peer
  usage. Reading it acquires nothing. Replacing the network settings builds
  a new budget.

## Tests

`reserved_connection_tests.rs` and `dht_server_capacity_tests.rs` drive the
real admission and lifecycle (ordinary and reserved sockets, per-peer
limits, pending dials, retirements, lease expiry, the DHT server's
headroom); `processing_codec_tests.rs` and `processing_gate_tests.rs` cover
the budget and the gate, including refusal before the body is read and
release on completion or cancellation.
