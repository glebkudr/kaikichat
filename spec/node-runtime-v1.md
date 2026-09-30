# Node runtime (V1)

`agentic-node serve` is the daemon of one profile: it holds the profile's
encrypted store ([store-v1.md](store-v1.md)), runs the shared application
core ([application-core-v1.md](application-core-v1.md)) and the libp2p
swarm, and serves its owner over a local socket. The owner CLI `kaiki`, the
desktop window and the scoped agent clients are its clients
([owner-cli-v1.md](owner-cli-v1.md), [desktop-gui-v1.md](desktop-gui-v1.md),
[agent-grants-v1.md](agent-grants-v1.md)). Code: `crates/node/src/main.rs`,
`lib.rs`, `runtime.rs`, `network_settings.rs`, `ipc.rs`, `bootstrap*.rs`,
`crates/core/src/transport_binding.rs`.

## Start and secrets

- `agentic-node serve --profile DB --ipc SOCKET --secrets-stdin [flags]`
  (`agentic-node serve --help`): listen and public addresses, bootstrap
  peers, relays, relay-only, AutoNAT, relay server, LAN discovery, DHT
  server, chain, identity server, directory. `kaiki daemon start` saves the
  listen, bootstrap, chain, identity-server and directory flags in
  `daemon.json`. Once the owner saves network preferences, they override the
  flags for relays, relay-only, AutoNAT peers, LAN discovery and the DHT
  role, and for bootstrap peers only when the owner named them:
  preferences without `bootstrapPeers` (the key absent, or `null` in
  `configure_network`) take the `--bootstrap` flags of this start, for a
  profile that follows the network preset the preset's routes; a list,
  `[]` included, replaces them. Such preferences are stored without the
  key, which an older daemon on the same profile reads as no routes.
  `network_settings.status.bootstrap.routes` (also in `node_info`) shows the
  routes in use. The default listener is
  `/ip4/0.0.0.0/udp/0/quic-v1`; TCP with Noise and Yamux is supported too.
- The secrets come as one JSON line of at most 4096 bytes on an anonymous
  stdin pipe, `{"masterKey", "ownerToken"}`, each 32 bytes in hex: the
  SQLCipher key and the owner IPC token. The daemon never puts them into
  arguments, the environment, logs or its profile; keeping them is its
  launcher's job (the keychain or a sealed file for `kaiki` and the window,
  [desktop-host-v1.md](desktop-host-v1.md); the network's own holders keep a
  plain `secrets.json` beside each profile in their container,
  `deploy/node/run-nodes.sh`).
- The profile's root key is created inside the store. The libp2p Ed25519
  transport key is generated separately and kept in the encrypted state
  `transport/identity`, so the peer id survives restarts and crashes.
- The network domain is SHA-256 of `AgenticInternet/network/v1`
  (`ae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18`,
  `NETWORK_DOMAIN`); every signed document names it.

## Node records

A node record binds the root to its transport key: a signed document of kind
`Identity`, root epoch 0, living at most 86 400 seconds, at most 4096 bytes,
body `[1, peer_id, addresses]` or `[2, peer_id, addresses, sequence]` with a
sequence of 1 or more. It carries at most 8 addresses of at most 256 bytes;
each must end in the same `/p2p/` peer id. A record is accepted only when its
peer id equals the one the Noise or QUIC session authenticated and its author
is the expected root; an invalid record changes nothing. The first Welcome
stores the peer's signed routes together with the contact and its MLS state.

## Protocols

| Protocol | Use |
|---|---|
| `/agentic-internet/delivery/1` | Direct delivery between online peers: request and response `{nodeRecord, envelope, stamped?}` in CBOR, frames of at most 131 072 bytes, 5-second timeout, 16 concurrent streams |
| `/agentic-internet/bootstrap/1` | Exchange of signed node records with bootstrap peers |
| `/agentic-internet/mailbox/1` | The mailbox swarm ([access-by-book-v1.md](access-by-book-v1.md), [Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md](../Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md)) |
| `/agentic-internet/kad/1` | Peer routing only ([dht-roles-v1.md](dht-roles-v1.md)) |
| `/agentic-internet/1` | Identify (feeds hole punching; never becomes a signed route) |

Circuit Relay v2 (client, optionally server), AutoNAT and DCUtR run beside
them; an optional mDNS finds peers on a LAN. Connections time out after
6 seconds and close after 60 idle seconds. Admission and processing limits
are in [node-capacity-v1.md](node-capacity-v1.md).

- An `envelope` carries a control message (a Welcome, a receipt) or an
  unpaid message; `stamped` carries an application message paid like a
  swarm store, so one stamp pays for both the direct and the mailbox path.
  A stamp of a granted book also carries its grant, checked as holders
  check it. When the recipient cannot check a stamp (its book is unknown
  and the chain did not answer the read of that book or of its grant's
  issuer rules), a one-to-one contact's message is taken with low trust and
  shown so; a stranger's request never is.
  A response with neither is a refusal, never an acknowledgment.
- A queued message leaves the outbox only on a valid receipt signed by its
  recipient or on a quorum of holder receipts. A lost response keeps it
  queued; retries resend the same saved ciphertext and the receiver keeps
  one copy.
- Delivery dials with a new outbound port, so a quick restart does not hit
  the old TCP tuple (`AddrInUse` on macOS); listeners keep their ports.

## Owner IPC

- A Unix socket in an owner-only (0700) directory, mode 0600; one request
  and one response per connection, each a big-endian u32 length and JSON.
  The owner sends `{token, method, request}`; the scoped agents' `{proof}`
  envelope is in [agent-grants-v1.md](agent-grants-v1.md).
- Bounds: a request's length prefix is checked (≤ 1 MiB) before its body is
  read; 32 connections at a time; 64 commands queued for the daemon; 5
  seconds for the whole exchange; responses ≤ 16 MiB.
- The 32-byte token is compared in constant time before dispatch; a wrong
  one is `unauthorized` and changes nothing. Answers are `{result}` or
  `{error: {code, message}}`; an unknown method is `unknown_method`.
  Closing a client leaves the daemon running.
- The methods are the owner's commands of `kaiki` and the desktop window.
  `node_info` reports the peer id, listeners, transports and bounded
  counters of every subsystem (routing, bootstrap, relay, mailbox swarm,
  capacity), never secrets.

## Shutdown

On SIGTERM the daemon removes its listeners, disconnects its peers and keeps
the transport running for at most 500 ms, so QUIC close packets leave before
the process exits; no application request is served meanwhile. A remote peer
then drops the connection promptly instead of waiting for the QUIC idle
timeout. Process tests (`crates/node/tests/support/shutdown.rs`) stop a node
with SIGTERM within five seconds, require the survivors to drop it within two
and restart it on the same endpoint and identity for three cycles with one
copy of every message, over TCP and QUIC. Nothing is promised for a crashed
machine or a lossy network.

## Bootstrap schedule

- At most 4 explicit bootstrap hints (a fifth is refused before the network
  starts), up to 64 cached signed records and 32 LAN hints; 4 exchanges at a
  time.
- After a successful exchange a peer is refreshed in 30 seconds. A failure
  retries after 500 ms, doubling up to 30 seconds.
- Losing the last connection to a verified peer that is idle and healthy
  moves its next attempt to 500 ms, once: repeated closes do not reset
  failure backoff, free a slot or add hints. Without this a client behind
  NAT with only a bootstrap route waited out the 30-second refresh after
  its bootstrap peer restarted.
- Tests: `bootstrap_limits_tests.rs` for the schedule, and the process tests
  `bootstrap_{tcp,quic}_reconnects_after_verified_provider_restart_without_message_trigger`.
- In relay-only mode only hints with a circuit address are dialed; the
  others are counted as blocked.
