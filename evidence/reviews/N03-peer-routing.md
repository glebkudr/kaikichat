# N03 — bounded peer routing and moved-recipient recovery

Scope: an incremental N03 delivery. Full N03, private rendezvous, registry selection and E01–E26 remain open.

## Contract

`/agentic-internet/kad/1` uses actual libp2p Kademlia FIND_NODE over the existing TCP/Noise and QUIC swarm. It routes transport PeerIDs; it publishes no application records, roots, contacts, messages, mailbox contents, providers or operator registry. It is a separate network from public/IPFS Kad.

Only already authenticated NodeRecords / their validated encrypted cache seed the retained candidate table (up to128 peers,4 direct IP routes each). Manual insertion and an event wrapper exclude unsolicited addresses from routing storage and other behaviours' dialing. Signed cache expiry eventually removes hints; the in-memory hint refresh window is60seconds. The library's own k-buckets may hold fewer entries than the128 retained candidates. Existing per-process/per-peer connection limits continue to apply.

Each search has at most32 planned-and-admitted remote requests, one outstanding step at a time, an8192byte packet bound, up to128 distinct accepted referral PeerIDs with4 routes each, and a20second overall deadline (Kad query timeout15seconds). Two searches/authentication attempts may be active;16 bounded owner result rows are retained. Incoming FIND_NODE service admits64/min globally and8/min per peer. Record/provider requests are reset; even direct access to the backing store cannot persist unsolicited records. The exact32 limit is measured by independent real server request counters, not just returned status.

The owner-only `lookup_peer {peerId}` operation returns an asynchronous result row; `node_info.routing.lookups` reports `searching`, `authenticating`, `verified`, `authentication-failed`, `not-found` or `budget-exhausted`. A successful closest-peer result for a different PeerID never counts as the target. Target referrals initiate the existing signed self-record exchange; only successful network/domain/signature/session/root/route verification and Core persistence produce `verified`. An existing contact route is replaced only through the same signed high-water reducer. No contact, agent grant, stake, independent operator identity or storage receipt is created.

After a real durable delivery failure, the outbox automatically requests this same lookup. Healthy delivery starts no search. Automatic work starts at most once per5seconds globally and once per60seconds for a given target; occupied slots and active retries coalesce. Sender ciphertext, operation ID and receipt handling are unchanged. A failed old endpoint does not prevent authenticating a newer target referral; one independent transport dial can bypass its stalled handshake. Bootstrap and routed authentication share four request slots.

Relay-only disables this first Kad adapter and rejects owner lookup with `policy_blocked`; application delivery, signed bootstrap and existing relay failover remain usable. Kademlia through relays is pending.

## Test-first review and evidence

All backend test increments went through the separate, context-free `/root/node_test_critic`, with FINAL ACCEPT before corresponding production changes.

1. Initial RED: all3 real process tests failed at the absent owner API;2 unit tests could not import the absent adapter.
2. First review REVISE required preventing reverse recipient bootstrap, the existing `delivered` receipt DTO, reachable bad-signature target, per-peer route bounds and a real32-request boundary.
3. Revised5 process tests and2 unit tests accepted. The independent fixture runs2 or40 real TCP/Noise Kad swarms; its initial automatic bootstrap settles before request counting. Forty peers form successively closer referrals to the target.
4. The first implementation exposed stale-address retry and missing-address result handling. Relay-only test setup also needed an actual provider; a separate review accepted adding that provider and confirmed reservation without relaxing policy assertions.
5. All5 process tests passed. Automatic delivery was then a separate RED increment. Review required healthy-delivery non-triggering and exact5/60second timing boundaries with a freed slot; revised tests received FINAL ACCEPT.
6. Focused GREEN:4 routing unit tests and6 actual-process scenarios. Full native/network gate evidence is recorded below.

Moved-recipient tests occupy the old TCP port, retain the original profile/root/PeerID, clear only the recipient's disposable discovery cache to prevent a reverse route, and give the sender only the first of two intermediaries. Both explicit owner lookup and autonomous outbox recovery must deliver the original ID/text exactly once, observe `delivered`, and empty the outbox. They stop both intermediaries and both owners, inspect the sender's persisted new signed address, clear the recipient's disposable cache again, and require another real message after restart.

The bad-target fixture successfully negotiates Kad and receives the actual bootstrap request, then corrupts its signature. The target cannot become verified or enter the encrypted cache; the profile/contact state is unchanged. Switching to a valid signed response enables lookup and persistence. The missing-target test rejects substituting nearby peers, and a separate invitation is still needed for chat. The budget fixture observes exactly32 received requests, then verifies a new lookup can occupy the freed slot.

## Dependencies and compatibility

Enabled the `kad` feature of existing libp2p0.56.0. Before enabling it, the official crates index reported libp2p-kad0.48.0 as the latest non-yanked stable release, published2025-06-27, MSRV1.83; it matches the existing libp2p0.56/swarm0.47/core0.43 and workspace MSRV1.91. Lockfile additionally resolves uint0.10.0 and crunchy0.2.4.

Sources: [official Kad crate](https://crates.io/crates/libp2p-kad), [official configuration API](https://docs.rs/libp2p-kad/0.48.0/libp2p_kad/struct.Config.html), [manual bucket insertion](https://docs.rs/libp2p-kad/0.48.0/libp2p_kad/enum.BucketInserts.html). Upstream0.48 exposes automatic-bootstrap throttling only privately; the wrapper finishes unowned queries and prevents their dialing/requests. Associated handler event types are accessed through the public NetworkBehaviour trait to bound/refuse referrals before Kad retains them. Recheck these integration boundaries on dependency upgrades.

No application wire, SQLCipher schema, invitation format or existing owner preferences change. The owner lookup and routing diagnostics are additive. Old peers lacking the Kad protocol remain usable for direct/bootstrap/chat; they cannot forward this lookup protocol.

## Limits still open

The routing table and bootstrap hints are not an authenticated operator universe. An eclipsed client, an exhausted search budget, loss of every reachable seed, or a disconnected network may prevent lookup. Routing does not prove independent storage operators or defeat Sybil attacks. Transport PeerIDs, addresses, query targets and timing are visible to participating routing nodes; a partial routing table can reveal network associations. This is not a private mailbox address design or global-observer anonymity.

Private rotating rendezvous, retained mailbox indexes/long-offline catchup, opt-in service records, authenticated registry/checkpoints, verified placement, R=10 custody and autonomous repair remain mandatory follow-up work. This slice verifies peer-address recovery through processes on macOS; existing current-source Linux network scenarios cover compatibility, not a new Linux Kad topology acceptance.

## Full gates

Both full scripts exited0 on2026-09-05. Rust215 tests (node unit18/process48), frontend30 tests, formatting/Clippy/TypeScript/Vite, four actual hidden WKWebView flows, normal release bundle and deep/strict ad-hoc code signing all passed. The normal dependency tree contains no WebDriver. Native logs: `N03-routing-native-green.log`; focused and RED evidence are adjacent `N03-routing-*.log` files.

All7 current-source Linux network outcomes passed; sourceHash `862871cfd35f61f4f379e5b4ffc8d1da12cc639f0f974cf3386b9e4af5986ba1`, runId `ain-nat-2360af04`, cleanupErrors `[]`. Report: `output/network-e2e/result.json`; log `N03-routing-network-green.log`. Own vision compared the current packaged `output/native-e2e/network-alice-restored.png` with the prior verified native screenshot; controls, policy status and layout remain intact. Full V1/E01–E26 acceptance is still open.
