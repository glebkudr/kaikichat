# DHT roles (V1)

Kademlia (`/agentic-internet/kad/1`, libp2p-kad 0.48.0 with the local patch
in [vendor/libp2p-kad/README-PATCH.md](../vendor/libp2p-kad/README-PATCH.md))
only finds peers: it neither stores nor serves records. Each node is a DHT
client or, by the owner's choice, a DHT server. Code:
`crates/node/src/routing.rs`, `crates/core/src/network_preferences.rs`,
`network_settings.rs`.

## Choosing the role

- A new profile, and a saved preference record from before this setting, is
  a client. The owner's preference `dhtServer` is a boolean, false by
  default, kept in the encrypted network preferences with their revision
  check; loading an old record does not rewrite it.
- `agentic-node serve --dht-server` makes a node without saved preferences
  a server. Once the owner saves preferences, the saved value wins over the
  flag, a saved `false` included.
- The mode is set explicitly (`Some(Client)` or `Some(Server)`), so a
  confirmed external address, for example through a relay reservation, never
  promotes a client ([the upstream contract](https://docs.rs/libp2p-kad/0.48.0/libp2p_kad/struct.Behaviour.html#method.set_mode)).
- Relay-only networking turns the whole DHT off in either role. A saved
  server preference survives and resumes when direct networking returns.
- The network's own holders run as DHT servers (`deploy/node/run-nodes.sh`).
  The desktop's network settings have a separate checkbox for it.

## Behaviour

- A client sends bounded FIND_NODE queries and offers no inbound Kademlia
  service.
- A server answers FIND_NODE: an exact lookup of a known peer returns only
  that peer's current addresses, others get the closest peers of its routing
  table. Keys longer than 64 bytes and lookups over the admission (32 per
  peer, 1024 in all per minute) are refused.
- GET_RECORD, PUT_RECORD, GET_PROVIDERS and ADD_PROVIDER are refused (a
  Reset, or ignored). Records, caching, publication and replication are off.
- Only authenticated signed node records populate the routing table;
  unsolicited address announcements are ignored. At most 128 candidate
  peers, two searches at a time, 32 requests per search, a 15-second query
  timeout, 3-second substreams and 8 KiB packets.
- Refused requests close their inbound substream with Reset; the vendored
  patch wakes the waiting stream so its slot is released.

## Status

`node_info.routing` and `network_settings.status.routing` report `enabled`,
the actual `mode` (`client`, `server` or `disabled`) and `blockedByPolicy`,
with the lookup counters (`lookupRejections`, `repliesDropped`, query errors
by kind and peer). The DHT role says nothing about reachability or trust.

## Tests

`crates/node/tests/support/dht_roles.rs` (real TCP/Noise and QUIC daemons:
a default client refuses, an explicit server answers, the owner's toggle
changes the service only once it is saved, relay-only suppresses it, and a
confirmed relay address never promotes a client), `routing_tests.rs`
and `dht_server_capacity_tests.rs`.
