# Kaiki Chat

Source: [glebkudr/kaikichat](https://github.com/glebkudr/kaikichat).
The repository includes the desktop application, agent CLI and MCP, network
node, identity and directory servers, and smart contracts.

Licensed under [MIT](LICENSE). Vendored dependencies retain their original
copyright notices and licenses.

A decentralized messenger and an environment for agent work. V1 is in
implementation: the current build runs direct conversations between real
clients over libp2p and OpenMLS. The full V1 goal **has not been reached
yet** — the boundaries and outstanding requirements are listed in
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

The V1 scope is [agent-first](Docs/V1_AGENT_FIRST_SCOPE_2026_09_25.md): the
agent CLI and skill first, the GUI in phase B; macOS and Linux. Offline
messages are stored by a swarm of recipient mailboxes: 10 holders, quorum 7,
a stamp from one of the user's books on every message, notaries catch double
spending, mailboxes live for 30 days
([decision](Docs/V1_STORAGE_REDESIGN_2026_09_24.md),
[plan and phase status](Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md)). Books are
bought in the contract or issued by the identity server
([services/identity-server](services/identity-server)) with a daily issuance
cap in `contracts/src/GrantIssuer.sol`.

On 2026-09-26 the former path was removed completely: custody/history storage,
finalizers and checkpoints, private ZK stamps, the L2 adapter, the stamp
issuance and operator payout contracts, agent orders and their panels. Their
descriptions and evidence remain in Git history. The mailbox swarm is verified
on a single-process rig with managed time
([acceptance spike](evidence/reviews/mailbox-swarm-spike-2026-09-26/));
native delivery of offline messages arrives after phases 1b (buying books and
reading the issuance policy from the chain) and 2 (the signed directory of
holders).

## Running the desktop app

On the current Mac connect WD4000 and run the build from the main repository.
The disk image is attached automatically:

```sh
cd /Users/glebk/Code/chat
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
```

If the npm dependencies need to be reinstalled, run
`python3 scripts/build-storage.py install`. The storage commands are described
in [AGENTS.md](AGENTS.md). On another machine without
`.local/build-storage.json` you can use a regular Rust/Node >=26 install,
`npm ci` in `apps/desktop` and `node scripts/build-desktop.mjs` from the
repository root.

The app appears at `target/release/bundle/macos/Kaiki Chat.app`. The build
includes the daemon, MCP, CLI, SQLCipher and the frontend; the app's user
needs no Node, Cargo or Vite. The bundle is signed with a local ad-hoc
signature, without Apple notarization. At this stage macOS arm64 is verified;
acceptance of other platforms remains open.

Create a profile, press "My invitation" and hand the full code to your peer.
The second client adds the contact from that invitation. By default the
desktop app uses direct IP reachability. In "Network settings" you can connect
your own Circuit Relay v2 and AutoNAT nodes; the daemon supports upgrading
from relay to direct QUIC across NAT. Offline message storage in the mailbox
swarm connects to native delivery after phases 1b and 2. "Delivered" appears
after the recipient's signed acknowledgement.

Keys live in the system Keychain, data in a private Application Support
directory. Closing the window keeps the daemon running; starting the app again
connects to it and restores the history. Full daemon lifecycle settings are
not implemented yet.

## Network settings

"Find nodes on the local network" enables mDNS over IPv4: two members of the
same LAN can find each other without a known address. Discovery is off by
default, including for old profiles. Press "Save and reconnect" to apply the
change. Discovery shares the device's Peer ID and addresses with the local
network; an announcement alone creates no contact and no right to read
messages. A found node still passes the usual signed-record check.

"Relay only" turns multicast off regardless of the saved discovery switch.
Turning LAN discovery off survives a restart; previously verified addresses
remain usable for communication. The CLI flag `serve --lan-discovery` applies
to a profile without saved settings. Actual discovery, fake announcements and
socket shutdown were verified in an isolated Linux LAN; the native macOS run
verifies that the switch persists and is suppressed in relay-only mode.
Verifying multicast and macOS system permissions on a real LAN remains a
separate platform step.

Open "Network settings", paste up to 4 independent relay provider addresses,
one per line, and, if needed, the addresses of AutoNAT verifier nodes. A full
address with a Peer ID is required: `/ip4/<IP>/tcp/<PORT>/p2p/<PEER_ID>` or
`/ip4/<IP>/udp/<PORT>/quic-v1/p2p/<PEER_ID>`. The node operator must enable
the service separately; saving an address does not by itself confirm
reachability.

"Save and reconnect" stores the settings in the encrypted profile and
re-establishes the network connections. The name, keys, assigned ports,
history and queue persist, including across a daemon restart. The panel shows
the actual reservations and probe results. If a reply is lost, save again; on
a conflict load the current values with the "Refresh saved settings" button.
Background state updates never replace the addresses you typed.

"Relay only" disables upgrading and closes the previous connections when
switched on. Outgoing messages use an allowed relay route or an already open
relay connection; otherwise they wait in the queue. After changing providers,
create new invitations for new contacts. Finding addresses beyond the known
hints and the local network is still being developed. The mode claims no
anonymity.

An existing contact's address updates after a successful exchange of messages
or acknowledgements. The update is signed by the node owner and stored with a
version number: after a restart old records do not displace the new address.
A message already queued uses the updated route without the user sending it
again. While no confirmed address is present, the last verified hint is kept
for a connection attempt; this does not mean the node is reachable. When the
known addresses are lost, the enabled LAN discovery can be used; finding an
external node without working hints through a private rendezvous is not
implemented yet.

The "Nodes for joining the network" field accepts up to 4 bootstrap addresses
of participants. The saved list replaces the CLI hints on the next start;
clearing the list does not block already verified addresses in the cache. Old
profiles load with an empty list, without touching history or keys.

The daemon exchanges its own signed records over bootstrap. For a profile
without saved settings, `serve --bootstrap <multiaddr>` sets up to 4 distinct
peer hints: direct IP/PeerID or a route through one relay. A working hint is
used independently of unreachable ones; no DNS and no company-mandated server
are needed for this exchange. Up to 64 valid records are kept in the encrypted
cache and reused after a restart without passing the flags again. The contact
list itself and third-party records are not shared by this protocol. Relay-only
mode applies to discovery as well; bootstrap creates no trusted chat.

After a delivery failure the daemon automatically looks up the recipient's new
address through Kademlia, using independent bootstrap nodes and the signed
cache. A chain of two intermediaries, an address change, delivery of the
original message and recovery after a restart without intermediaries are
verified. A found node must confirm its NodeRecord; a DHT answer creates no
contact. For manual diagnostics, owner IPC accepts `lookup_peer` with
`peerId`, and `node_info.routing.lookups` shows the progress and the result.
At most 2 lookups of 32 queries each run at the same time; automatic starts
are limited to one per 5 seconds and a repeat for the same recipient to once
per 60 seconds.

By default the node runs as a DHT client: it performs lookups and publishes
its own signed records. To serve other participants' queries, enable "Help
the network find nodes" in the app settings or pass `serve --dht-server` for
a node without saved settings. For lookups through intermediaries at least one
reachable intermediary must explicitly enable the DHT server. The owner's
saved choice, including the off state, takes priority over the CLI and
survives a restart. This role grants no validator powers; confirmation of an
external address does not enable it automatically. `node_info.routing.mode`
and `network_settings.status.routing.mode` show the actual mode. Details:
[DHT client/server](spec/dht-roles-v1.md).

In relay-only mode this first Kademlia adapter is off, and the previous relay
delivery keeps working. The DHT role choice persists and applies again when
direct connectivity returns. Lookup requests reveal the target's PeerID and
network addresses to the intermediate nodes. Private rendezvous is not
implemented yet; the offline mailboxes are described above. Details:
[routing boundaries and verification](evidence/reviews/N03-peer-routing.md).

Owner IPC `node_info.bootstrap` and `network_settings.status.bootstrap` show
the actually connected peers with verified signatures, the network domain and
the `bootstrap-needed` status when there are no such connections. Retries are
limited to four parallel requests with a delay of up to 30 seconds; incoming
checks to 64 per minute overall and 8 per peer. The protocol confirms the
signed membership in the network domain but does not verify the node's
registration in the registry yet. The panel shows the number of peers with
verified signatures and reports when known hints are forbidden by relay-only
mode. A signature confirms the key, not a reputation. Unverified LAN hints
live no longer than 60 seconds without a new announcement; up to 32 peers are
kept, with up to 4 addresses per peer. They share the bootstrap queue and do
not remove the expected-key check from the signed cache. Lookup through a
private rendezvous remains in progress.

## Connecting an agent

Open "Agents", choose a name, conversations, a lifetime and a text limit.
Reading incoming messages is allowed for the selected conversations; sending
must be enabled separately. Press "Grant access" and copy the shown
configuration into your MCP client. `agentic-mcp` is included in the app
bundle; the runtime key is created locally, owner/master keys never enter the
configuration.

The agent reads the MCP resource `agentic://runtime`: it returns only the
allowed conversation IDs/titles, actions, the lifetime and the text limit.
Metadata is capped at 16 KiB and contains no history. Every read checks the
current permissions; refresh the resource data when needed.

The messaging tools: `inbox.poll`, `inbox.ack`, `messages.send`,
`delivery.get`. They accept a `conversationId` from the resource. Repeating an
operation reuses the previous `operationId`; after processing the inbox,
acknowledge the issued lease. "Revoke access" blocks further calls of an
already running client, including reading the resource. Groups and
subscriptions are not served over MCP yet. The old signed agent `snapshot` now
returns only these metadata; messages are read through the bounded inbox. The
owner snapshot for the UI is unchanged.


`delivery.get` accepts a `conversationId` and the original `operationId` of
the send. It returns the message ID and the current status, without the text.
Only operations of this runtime with an active right to send are available.
After a restart the previous operationId keeps working. `delivered` means the
recipient's signed acknowledgement; the replicas field is 0 for now: copies
in the mailbox swarm are not reflected in it yet.

Tool errors contain `structuredContent.error` with `code`, `message`,
`retryable` and, when needed, `details`. Resource read errors use the same
object in the JSON-RPC `error.data`. `retryable: true` allows a retry after
the cause is gone; there is no need to retry the request continuously.

| Code | Agent action |
| --- | --- |
| `inbox_busy` | Wait for the active lease to be acknowledged or to expire, then repeat the previous request. |
| `inbox_lease_expired` | Start a new poll with a new operationId; the old ack no longer applies. |
| `inbox_item_too_large` | Adjust maxBytes from details.requiredBytes within the grant. |
| `idempotency_conflict` | Restore the original request body or give the new operation a new ID. |
| `unavailable` | After the daemon/storage recovers, retry the same operationId. |
| `unauthorized` | Check permissions and parameters; an automatic retry does not fix a refusal. |
| `cancelled` | The request was cancelled; cancellation does not roll back an atomic commit that already started. |

## Verification

For a standalone daemon, `serve --relay <multiaddr>` (up to 4 distinct
intermediaries) and `--relay-only` are available. A relay address has the form
`/ip4/<IP>/tcp/<PORT>/p2p/<RELAY_PEER_ID>` or the QUIC equivalent. Confirmed
relay routes are automatically included in the signed invitation. In
relay-only mode it contains only them; after an intermediary fails a fallback
works, and the queue is restored after a restart without resending from the
UI/MCP. This mode claims no anonymity.

A node becomes an intermediary only with `--relay-server`;
`--relay-capacity 1..16` sets the reservation limit, 16 by default.
`--relay-reservation-seconds 4..3600` sets the TTL (300 by default),
`--relay-circuit-seconds 2..120` the circuit lifetime (120 by default). The
circuit byte threshold is 1 MiB; this is a technical quota, exact financial
accounting of traffic is not implemented yet. Owner IPC `node_info` shows the
confirmed `relayRoutes`, the real `peerConnections`, renewals and expiries in
`relayServer`. TCP/QUIC, failure/recovery, renewal at the full limit, freeing
slots by TTL and continued delivery after quotas are verified. With `--relay`
a regular client automatically tries DCUtR: after successful QUIC hole
punching the conversation continues even when the only relay is stopped. With
the direct route blocked, relay works. Owner `node_info.holePunch` shows the
enablement and the success/failure counters. `--relay-only` disables the
upgrade. TCP hole punching and native UI acceptance behind a real NAT remain
open; configuring the client through the native UI is already available.

`--autonat-peer <multiaddr>` sets up to 4 independent verifier nodes; the
format is the same as for relay. `--autonat-probe-seconds 10..900` sets the
interval, 60 seconds by default. With verifier nodes set, new signed
invitations and the NodeRecord publish only the confirmed direct address and
the working relay routes. A positive answer from the service is not enough: a
new incoming connection from its PeerID to the probed port is required. When
blocked, the address is revoked and rechecked after recovery; when the service
is unreachable the status becomes `unknown`. Without a reachable route,
creating an invitation returns `network_unavailable`; existing communication
keeps working. Without `--autonat-peer` the previous direct/LAN address mode
is kept. Old already issued invitations are immutable. On a delivery failure
a contact's new direct route may be found through Kademlia and stored after a
signature check; route lookup through relay is not implemented yet.

The verification service is enabled separately with `--autonat-server`. By
default it verifies only clients with a global IP; `--autonat-allow-local`
allows a private/LAN network with the service explicitly enabled. Limits: 64
accepted probes per minute, 8 per peer, up to 8 addresses per request;
temporary connections are released and do not cancel the relay reservation.
Owner `node_info.autoNat` shows the status and counters,
`advertisedAddresses` the current addresses for new invitations. One node may
provide both services. A reachability check grants no rights over messages or
agent work.

```sh
python3 scripts/build-storage.py run scripts/check.sh
python3 scripts/build-storage.py run node scripts/check-native.mjs
python3 scripts/build-storage.py run node scripts/check-network.mjs
```

The contract gate needs Foundry 1.8.1 (`forge`, `cast`, `anvil` in PATH) and
Solidity 0.8.36; the compiler version is pinned in `contracts/foundry.toml`.
The tools directory can be set with `AIN_FOUNDRY_BIN` and the installed solc
path with `AIN_SOLC`. The separate contract gate:
`python3 scripts/build-storage.py run scripts/check-evm.sh`.

The first command checks Rust, Solidity/Anvil, React, types and the
production frontend. The second additionally builds a test `.app`, runs hidden
WKWebView windows with separate profiles, verifies chat, restart, MCP grant
and revocation, and relay/AutoNAT configuration through the UI with a real
daemon/Keychain, saves screenshots to `output/native-e2e`, then builds and
verifies the signature of the regular release `.app`. WebDriver is present
only in the `e2e` feature; a release with this feature is rejected by the
compiler. An additional native scenario stops the recipient's node entirely,
then verifies the restoration of settings/ports and delivery of the original
message from the sender's queue. The tests create separate profiles and delete
only their own processes and Keychain entries.

The third command runs Linux network E2E through OrbStack/Docker: it builds
the current daemon, verifies direct delivery, two private networks behind
NAT, TCP/QUIC relay, failure of the active intermediary and queue recovery
after a restart, direct QUIC after DCUtR and the shutdown of the only relay,
and relay delivery with the upgrade blocked. AutoNAT is verified against a
real firewall: publishing, revoking and restoring the signed address, service
failure, a closed NAT with relay, and a joint provider of both services. The
report and packet counters are in `output/network-e2e`. The rig deletes only
its own containers/networks, including after SIGINT/SIGTERM; shared network
settings are not touched. `--no-build` is allowed only when the image
fingerprint matches the current sources. TCP hole punching and the native
desktop behind NAT remain separate unfinished checks.

The main repository with its own Git lives at `/Users/glebk/Code/chat`
(internal APFS). The heavy build directories are symlinked to
`/Volumes/ChatBuild`; that volume is stored in the image
`/Volumes/WD4000/Code2/chat/.storage/build.sparsebundle`. Sources and Git are
available without WD4000; builds need the external disk. The old external
checkout is not a working source.

The local configuration lives in `.local/build-storage.json` and is not part
of Git. A regular run attaches the existing image in the background
automatically and verifies the volume UUID and all symlinks:

```sh
cd /Users/glebk/Code/chat
python3 scripts/build-storage.py run scripts/check.sh
python3 scripts/build-storage.py run node scripts/check-native.mjs
python3 scripts/build-storage.py run node scripts/check-network.mjs
```

To restore the npm dependencies use `python3 scripts/build-storage.py
install`: the lock-file install runs inside the image and preserves the
`node_modules` symlink. Before unplugging WD4000, finish builds and run
`python3 scripts/build-storage.py unmount`, then eject the external disk. When
the volume is missing or different, the launcher stops without creating a
fallback `target` on the internal disk. On other machines without this local
configuration, the usual build commands remain available.

### Fast diagnostic loop (V1-C05)

A single-process reproducer: several logical nodes inside one test, with real
`Runtime`, SQLCipher and a libp2p swarm under shared managed time
(`clock::Virtual`, tests only) and deterministic RNG. The driver advances the
clock to the nearest announced `next_due` and waits for real requests and
dials in real time. The mailbox swarm scenarios are
`reproducer_tests::mailbox_swarm::`:

```sh
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib reproducer_tests:: -- --nocapture
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib runtime::clock -- --nocapture
```

The acceptance spike sends 258 messages (ignored by default;
`AIN_SPIKE_MESSAGES=<n>` reduces the count). The method and results:
[evidence/reviews/mailbox-swarm-spike-2026-09-26](evidence/reviews/mailbox-swarm-spike-2026-09-26/):

```sh
python3 scripts/build-storage.py run cargo test --locked -p agentic-node --lib -- --ignored acceptance_spike --nocapture
```

What to read on a hang: `node_info.mailboxSwarm` (`sent`, `served`,
`failureKinds`, `lastFailure`, the notary queue `notary`, the grant checks
`grants`, replication `replication`) and `node_info.mailboxHolder`.

## Linux verification with the portable profile and the offline toolkit

On Linux use the wrapper's explicit portable profile — the commands and
constraints are described in
[tests/build/README-linux-toolkit.md](tests/build/README-linux-toolkit.md)
and [AGENTS.md](AGENTS.md). On a networked Linux host an up-to-date checkout
and a regular build are enough: the profile uses no APFS image and no
external disks.

For isolated or offline Linux environments there is a full checksum-bound
offline toolkit. It is published in a **separate repository**
[glebkudr/chat_toolkit](https://github.com/glebkudr/chat_toolkit) (branch
`main`), not in this one — only the sources live here. The toolkit contains
Node/npm, Foundry, solc, native SDKs, vendored Rust crates, the frontend
payload and lock files compatible with the MSRV pinned in `Cargo.toml`
(Rust 1.91). The full archive is 1 308 816 276 bytes in 157 binary parts;
integrity is verified by `SHA256SUMS` and the streaming `assemble.py`, which
restores the original `toolkit.json` and never overwrites an existing
directory.

Quick start on Ubuntu 24.04 x86_64 (the publishing commit is pinned in the
toolkit repository's manifest):

```sh
git clone --depth 1 https://github.com/glebkudr/chat_toolkit.git
cd chat_toolkit
sha256sum --check --strict SHA256SUMS
python3 assemble.py --destination ../toolkit-r006
```

Unpacking, full verification and activation are described in the toolkit
repository's README. The native scenarios need working local Unix IPC
(`AF_UNIX`/`SOCK_STREAM`); before running the rig use the runtime doctor —
it checks create/bind/listen/connect/accept and reports `blocked` honestly.
In local Docker (OrbStack) Unix IPC works; sandboxes with `EPERM` on AF_UNIX
are unusable for daemon scenarios. The toolkit was built before the former
path was removed and still contains its tools; publishing the toolkit does
not by itself change the app's acceptance statuses.
