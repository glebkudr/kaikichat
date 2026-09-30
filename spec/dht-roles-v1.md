# Owner-controlled DHT roles

Implements step 5 of service-discovery-v1.md. The full V1 goal remains open.

- A fresh profile and a previous-version preference record without `dhtServer`
  use client mode. The owner JSON preference is a boolean, defaults to false and
  is returned explicitly. The existing encrypted version-1 state, revision CAS,
  exact retry and commit-before-live-change boundary remain in use. Loading old
  bytes does not rewrite them. Malformed values fail validation.
- `agentic-node serve --dht-server` opts an unsaved node into server mode. Once
  preferences are saved, their value wins over startup flags, including an
  explicitly saved false. Server mode does not enroll an operator, enable a
  finalizer, change a committee or authorize any application work. Finalizer
  enable/disable does not change either DHT preference or live DHT role.
- Client mode may issue bounded FIND_NODE, GET_RECORD and signed publication
  queries; it does not offer inbound Kademlia service. Local records used for
  the client's own publication are permitted. Servers answer requests and cache
  only the existing validated, finite mailbox/service records. DHT hints still
  require the existing independent signature, scope, lease and transport checks.
- Explicit libp2p `Some(Client)` / `Some(Server)` modes prevent an external-address
  observation from silently promoting a client. No new routing implementation
  or package is required. See the locked libp2p-kad 0.48.0 API and
  [the upstream mode contract](https://docs.rs/libp2p-kad/latest/libp2p_kad/struct.Behaviour.html#method.set_mode).
- Relay-only suppresses the entire direct DHT path for both preferences. A saved
  server preference survives suppression and resumes when the owner restores
  direct networking. No direct fallback is introduced. Owner `node_info.routing`
  and `network_settings.status.routing` report `enabled`, actual `mode`
  (`client`, `server`, `disabled`) and `blockedByPolicy`, alongside existing
  bounded lookup diagnostics. DHT service says nothing about reachability or trust.
- Both transports retain existing limits: 128 remembered peers, four addresses
  per peer, two active queries, 32 network requests per query, existing finite
  query/record budgets and rate admission. This module does not enlarge committee
  route caches or weaken consensus thresholds.
- The network panel exposes a separate owner checkbox with its resource purpose,
  saved/live distinction and relay suppression. Polling must not replace edited
  intent or its revision. Errors, explicit reload and lost-response retry reuse
  the existing form workflow.

## Acceptance

Tests precede production and require independent no-context backend critic ACCEPT.
Real TCP/Noise and QUIC probes must connect to the authenticated target and count
actual decoded Kad replies: default client refuses, explicit server answers,
failed SQL/CAS/unauthorized changes preserve the old role, successful demotion
refuses, and cold restart honors stored settings over CLI. Relay-only blocks
requests and returns explicit policy status; restoring direct mode restores service.
A relay server without DHT opt-in must remain a DHT client after a real circuit
reservation exposes confirmed external addresses, including after enable/demote.

An exact old SQLCipher preference payload loads as client without rewriting bytes.
Toggle, retry, failure and cold recovery preserve a real queued MLS operation,
which must still deliver and obtain its receipt. Existing daemon routing tests
continue to traverse two explicitly enabled independent servers as ordinary
clients, recover moved recipients, survive one seed loss and preserve application
authorization. Existing mailbox publication/read scenarios must retain independent
remote caches and sender-offline recovery with clients at both ends.

The live signed-announcement/DHT/genuine-spend gates use explicit ordinary DHT
servers and selected clients. They retain actual enrollment/proofs, moved-address
and unseeded cold recovery, role revocation and independent QC verification.
Finalizer-role toggling is checked in both DHT modes. Backend/frontend, fmt,
Clippy, actual application rebuild and headless visual inspection are required.
