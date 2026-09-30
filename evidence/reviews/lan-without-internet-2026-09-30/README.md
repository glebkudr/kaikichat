# Kaiki without the Internet: one LAN, both online (2026-09-30)

The question comes from the README table "How it compares", column "Works
without Internet". Briar, Berty and Jami deliver without the Internet only
while both devices are near each other and online; until then a message
stays on the sender's device. Only LXMF/Reticulum keeps messages for an
offline recipient inside a local mesh. So the bar for Kaiki is direct
delivery on one LAN while both contacts are online.

**Answer: partly.** Direct one-to-one messages between contacts go both
ways on a LAN with the chain, the holders and the bootstrap routes
unreachable, within about half a second, once local discovery is on at both
ends and the peers have found each other. But only when the recipient
already knows the sender's current **bought** book: a stamp from a free
grant is never accepted directly, and groups need the holders. Discovery
was not reliable on hosts with bridges (5 of 9 runs passed); the two
defects found on the way are fixed (below), and repeated native rounds on
this Mac now verify every peer every time.

## Tests

Native (real `agentic-node serve` processes, secrets over stdin into
temporary profiles, no Keychain), in `crates/node/tests/support/swarm_native.rs`,
ignored by default:

```sh
# The scenario: anvil with the contracts, ten bonded holders, three profiles.
python3 scripts/build-storage.py --worktree .local/verification-workspaces/lan-offline \
  run cargo test --locked -p agentic-node --test processes -- --ignored --exact \
  swarm_native::native_lan_without_internet --nocapture
# Local discovery only, repeated restarts; AIN_LAN_DEAD=10 first meets ten
# peers that are gone afterwards, as the holders are offline.
AIN_LAN_ROUNDS=4 AIN_LAN_DEAD=10 python3 scripts/build-storage.py \
  --worktree .local/verification-workspaces/lan-offline run cargo test --locked \
  -p agentic-node --test processes -- --ignored --exact swarm_native::lan_discovery_rounds --nocapture
```

Unit: `cargo test --locked -p agentic-node --lib lan_tests::` runs
`a_fresh_lan_address_reaches_a_moved_peer_whose_signed_record_is_full` and
the route selection tests below (`crates/node/src/lan_tests.rs`). Rig (managed time): the characterization
in [granted-direct.patch](granted-direct.patch), applied to
`crates/node/src/reproducer_mailbox_swarm_tests.rs` and run with
`cargo test --locked -p agentic-node --lib -- reproducer_tests::mailbox_swarm::a_granted_stamp_sent_directly_is_refused --nocapture`.

### `native_lan_without_internet`

1. **Online.** Alice, Bob and Carol buy a book each (BookShop) and load the
   holder directory. Alice and Bob, and Carol and Bob, become contacts by
   invitation. Alice and Bob write each other, Bob writes Carol; Carol never
   writes Bob, so Bob never reads her book. Each recipient of a direct copy
   reads the sender's book from the chain (`chain.learned` 1 at all three).
2. **The Internet goes.** No system time is faked and nothing leaves the
   machine:
   - the ten holders stop;
   - the chain RPC is a local TCP port that accepts connections and never
     answers (a request hangs until the node's 10 s timeout);
   - the four bootstrap routes are local UDP ports that drop every packet, as
     a router without an uplink does (QUIC handshakes time out).

   The three daemons restart with these flags, listening on `0.0.0.0` (TCP
   and QUIC) on new ports.
3. **Local discovery.** Off by default. Each owner turns it on the way the
   settings panel does (`network_settings`, then `configure_network` with
   every preference sent back and `lanDiscovery: true`).
4. **Both online on the LAN.** Alice and Bob write each other; again after
   75 s of silence.
5. **A book the recipient never read.** Carol writes Bob, Bob writes Carol.
6. **The Internet comes back for Bob only** (the holders stay gone): Bob
   restarts with the real chain RPC.

## Results (debug build, one Mac)

Nine runs of `native_lan_without_internet`: five passed, four stopped at
discovery (below). The passing runs, in seconds:

| Step | Run 1 | Run 3 | Chain 1 | Chain 2 | Chain 3 |
|---|---|---|---|---|---|
| Daemon start, RPC hanging, holders and bootstrap unreachable | 0.03–0.04 | 0.04–0.06 | 0.05–0.08 | 0.03–0.05 | 0.04 |
| Every pair verified by signed records after LAN turned on | 5.6 | 5.3 | 5.4 | 5.3 | 5.2 |
| Alice ↔ Bob read both ways | 0.51 | 0.51 | 0.53 | 0.51 | 0.50 |
| Both "Delivered" at the senders, after that | 0.002 | 0.002 | 0.003 | 0.001 | 0.002 |
| Both ways again after 75 s of silence | 0.51 | 0.50 | 0.51 | 0.51 | 0.52 |
| Bob → Carol (Carol knows Bob's book) | 0.51 | 0.51 | 0.53 | 0.51 | 0.51 |
| Carol → Bob (Bob never read her book) | queued | queued | queued | queued | queued |
| Carol → Bob once Bob reads the chain again | 67.2 | 52.4 | 52.4 | 52.2 | 52.5 |

Run 1 is before the fix below, the others after it; "Chain" runs had mDNS
tracing on. Reports: [native.json](native.json).

- mDNS found every peer (`lanDiscovery.peers`); the connections ran over the
  Mac's LAN address `192.168.10.41`, over TCP and QUIC.
- While Carol's message waited, Bob refused 6–7 direct frames, his chain lane
  showed `absentBooks: 1` (her book could not be read); Carol's outbox held
  the message (`pendingOutbox: 1`, phase `queued`).
- The daemon's IPC answered at once while the chain reads hung.

The four runs that stopped at discovery:

| Run | Fix | What the nodes showed |
|---|---|---|
| 2 | no | not verified in 120 s (no diagnostics then) |
| 4, 5 | yes | no mDNS peer at all at any node; see "A window without mDNS" |
| Chain 4 | yes | Alice and Bob verified Carol but not each other: routes on bridge addresses (below) |

## Found and fixed: a full signed record hid the LAN address

Looking for the cause of run 2, `lan_discovery_rounds` with
`AIN_LAN_DEAD=10` failed every time in another way: the first start after
meeting verifies in 5 s; every later restart, when the three profiles keep
each other's signed records, never did in 120 s, though mDNS listed every
peer (`failedAttempts` 57, no connection). (Run 2 itself left no
diagnostics; its online records listed one route, so it more likely hit one
of the two failures below.)

A record lists at most 8 routes, and a host with bridges (docker, VPN, this
Mac) lists 8. `Schedule::replace_sources` appended LAN addresses only while
a hint had fewer than 8, so the fresh address a moved or restarted peer
answers mDNS from was never dialed; only its stale routes were. The fix puts
the LAN addresses (at most 4) first and keeps the signed routes after them,
up to 8, and the signed root still decides the exchange
(`crates/node/src/bootstrap_schedule.rs`). The same fix was made earlier
for explicit bootstrap routes. Routes are dialed two at a time in order
(`dial_concurrency_factor(2)`), so the order matters for speed.

| `lan_discovery_rounds`, `AIN_LAN_DEAD=10` | Round 0 | Round 1 | Round 2 | Round 3 |
|---|---|---|---|---|
| Before | 5.2 s | not in 120 s | not in 120 s | not in 120 s |
| After | 5.2 s | 10.2 s | 5.2 s | 10.1 s |

Without dead peers (fresh profiles) five rounds took 5.4 s each before the
fix: the four silent bootstrap routes hold the four exchange slots for the
5 s QUIC handshake timeout first.

## Found and fixed: the first four mDNS routes could all be unusable

Chain 4 was traced. Alice's first four mDNS routes of Bob were
`172.31.250.0/tcp`, `172.31.250.0/quic`, `172.16.42.0/tcp` and
`192.168.215.0/tcp`: addresses of this Mac's OrbStack bridges that end in
`.0` and refuse connections (`EADDRNOTAVAIL`). Carol's first route at Alice
was `192.168.10.41`, and she connected. `LanHints` kept the first four
routes of a peer and refreshed them every 10 s, so a peer whose first four
were unusable was never reached (`crates/node/src/lan.rs`, `observe`); the
fixed LAN-first merge could not help. The traced loop of
`lan_discovery_rounds` hit the same once in three rounds.

Why the routes carry bridge addresses: libp2p-mdns sends a response from a
socket bound to each interface's address and replaces the IP of every
advertised route with the packet's source IP
(`libp2p-mdns 0.48`, `iface.rs` and `query.rs`). On one host the other
process answers from every bridge. On two Macs the multicast route sends
them all through the LAN interface, still with the bridge source addresses,
so a Mac with Docker, OrbStack or VPN bridges likely feeds its peers the
same unusable routes, and OrbStack uses the same subnets on every Mac (from
the code; not tested on two machines).

The fix (`crates/node/src/lan.rs`): the node reads its own interface
subnets (`if-watch`, the watcher libp2p already uses) and

- never keeps a route on a subnet's network or broadcast address (the `.0`
  bridges; a /31 or /32 has none);
- keeps up to 8 routes per peer, as many as a hint carries, ordered: another
  host on one of this host's subnets first, then this host's own addresses
  (another process here, or a bridge address another Mac shares), then
  routes outside these subnets; when a peer's routes are full, a better
  route replaces the worst kept one;
- puts a route that refused a dial behind the peer's untried routes of the
  same rank, and lets a route heard again replace it, so a peer is never
  locked onto the routes heard first, also when the interfaces cannot be
  read.

The bootstrap schedule still dials a contact's LAN routes first (at most 4
next to a full signed record), two at a time; they are now the best four.
Unit tests: `mdns_routes_on_a_host_with_bridges_keep_the_usable_ones_first`,
`a_peer_heard_from_many_foreign_bridges_is_dialed_on_the_lan_first`,
`routes_that_refused_a_dial_give_way_to_untried_ones`
(`crates/node/src/lan_tests.rs`).

Native results on this Mac (debug build; "before" is the same build with
the old `lan.rs`):

| Run | Rounds or runs | Every peer verified | Not in 120 s |
|---|---|---|---|
| `lan_discovery_rounds`, before | 20 + 12 | 24 (23 in 5.2–5.4 s, one in 10.4 s) | 8 |
| `lan_discovery_rounds`, after | 12 + 30 | 42, all in 5.2–5.4 s | 0 |
| `lan_discovery_rounds`, `AIN_LAN_DEAD=10`, after | 8 | 8, all in 5.2 s (5.2–10.2 s after the first fix only) | 0 |
| `native_lan_without_internet`, after | 3 | 3, in 5.2–5.3 s after LAN turned on | 0 |

The rounds that failed before looked like chain 4: every node listed the
others as mDNS peers, one node verified none of them after 40–48 failed
attempts and held no connection.

To see the kept routes, a verification worktree printed each node's LAN
hints at every bootstrap refresh (a temporary `eprintln!`, not committed),
and the round listed each node's last hints of each other node:

| Node-to-peer route lists | Rounds | Lists | Only `.0` routes | First pair on `.0` | Any `.0` route |
|---|---|---|---|---|---|
| Before | 12 | 72 | 20 | 28 | 71 |
| After | 30 | 180 | 0 | 0 | 0 |

Before, exactly the four failed rounds had a pair whose nodes both held
only `.0` routes of each other; a pair with one usable direction still
verified, which is why most rounds passed. After, every list held the
`192.168.10.41` and `192.168.139.3` routes, which connect. In
`native_lan_without_internet` the messages again went both ways in 0.51 s,
and Carol's message arrived 52 s after Bob read the chain.

## A window without mDNS

From about 19:07 to 19:19 (runs 4 and 5) no test process received mDNS at
all: every node listed no LAN peer while each held its five sockets on UDP
5353; two plain `lan_discovery_rounds` failed; a multicast sniffer started
from the same shell saw nothing. Right after, the same binary discovered in
5.4 s, and a multicast loopback self-test worked. The cause was not found
(another session's network changes, or the macOS local network permission
of the app the processes ran under, are guesses).

## A book from a grant is never accepted directly

Rig characterization ([granted-direct.patch](granted-direct.patch)), the
mirror of `a_direct_message_pays_with_the_stamp_of_its_swarm_copy`: Alice
pays with a granted book, Bob is reachable only directly and reads the
chain. In 600 managed seconds Bob received nothing; his node asked the shop
for the book 10 times (`absentBooks: 1`); Alice's message stayed `queued`.
With a bought book the same setup delivers at once.

Cause: the direct path's `book_wanted` reads only `BookShop`
(`crates/node/src/chain.rs`, `Chain::book`); a grant is shown only to holders
(`LearnGrant`, `crates/node/src/mailbox_client.rs`) and `StampedDelivery`
carries none. New users are funded by the identity server's grants, so for
them direct delivery never works, online (the holders carry the message
instead) or offline (nothing arrives).

## What the user sees when the book is unknown

The sender's message shows **"Queued"** with no error or explanation; the
recipient sees nothing. The sender's node retries directly every 30 s at
most. The message arrives by itself once the **recipient** reads the chain
(the sender getting the Internet does not help: the recipient checks the
stamp). For a granted book it never arrives directly.

## Limits

1. **Local discovery is off by default.** The desktop app turns it on
   (Settings → Network, "Find nodes on the local network"); the setting is
   kept in the profile and overrides the daemon's flags, so it also holds
   for a daemon `kaiki` starts. `agentic-node serve --lan-discovery` exists,
   but `kaiki` has neither a command nor a daemon flag for it (`DaemonFlags`,
   `crates/node/src/host.rs`). Without it, a contact is reached offline only
   through its stored signed record: the owner's daemon keeps its ports
   (`listen-addresses.json`), so that works while the contact's LAN address
   is unchanged (not tested); after a move or a new DHCP lease only local
   discovery finds it.
2. **The recipient must know the sender's current bought book.** A client
   node learns a book only from a direct stamped delivery it received while
   it could read the chain (`check_direct_stamp`, `crates/node/src/runtime.rs`;
   the other `book_wanted` callers are holder paths). Two contacts who
   usually talk through holders from different NATed networks cannot reach
   each other directly, so they do not know each other's books. Every new
   book starts over. Contacts made offline (an invitation passed by hand)
   cannot write each other until each recipient reads the chain.
3. **Granted books never work directly** (above).
4. **One-to-one only.** Group and channel messages go only through the
   holders; only invitations go directly (`AppCore::outbox`,
   `crates/core/src/lib.rs`). From the code; not run.
5. **The CLI and the window** try the preset first: up to 5 s
   (`network_preset.rs`, `TIMEOUT`) before the kept preset is used.
6. **An IP network with multicast.** No Bluetooth or Wi-Fi Direct.
7. **Hosts with bridges** (Docker, OrbStack, VPN) announce routes on
   every bridge. Local discovery now keeps the usable ones (above), but a
   node's signed record still lists its bridge addresses, `.0` ones
   included, capped at 8 routes (`Runtime::advertised`), so a contact that
   reaches it only through that record may try unusable routes first.

## Not checked here

- Two physical machines. All nodes ran on one Mac; mDNS went through the
  host's multicast loopback, the connections through its interfaces.
- macOS local network privacy (macOS 15 and later asks before an app talks
  to the LAN). The app bundle declares neither `NSLocalNetworkUsageDescription`
  nor `NSBonjourServices`; the window's daemon under that prompt needs a
  check on real Macs.
- Networks that isolate clients (guest Wi-Fi) or filter multicast.
- The owner's daemon with its saved ports and local discovery off.

## Possible fixes (not made)

- Limit 1: a `kaiki` command or daemon flag for local discovery; whether it
  is on by default is the owner's call (it announces the Peer ID on the LAN).
- Limit 3: carry the grant in `StampedDelivery` and check it like a holder
  does; offline that still needs the issuer's rules and the notaries, so an
  offline recipient would check the grant's signature and defer the rest.
- Limit 2, small: when a node reads a stamped entry from a one-to-one
  contact's mailbox (entries carry their stamps, `mailbox_client.rs`), ask
  for its book if unknown, so reading online caches every correspondent's
  current book: one chain read per new book.
- Limits 2 and 3, a policy change for the owner: take a direct delivery from
  an existing contact whose book cannot be checked now, and check the stamp
  when the chain and the notaries answer.
