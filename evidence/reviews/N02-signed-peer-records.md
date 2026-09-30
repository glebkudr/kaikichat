# N02/N03 signed peer records and contact route refresh

Baseline:5dd2bbd. This implements a routing foundation and fixes stale return addresses in actual daemon delivery. It does not complete bootstrap/Kademlia, secret mailbox lookup, independent replicas or full V1 acceptance.

## Test-first contract and independent review

- Eight Core tests were written before implementation. Initial RED `/tmp/ain-peer-records-core-red.log` reports absent publication/cache APIs and sequence metadata. The context-free `core_test_critic` initially returned REVISE: eviction had to be demonstrated for the actual confirmed root, and a cache failure after a successful message commit needed its own recovery test. Both were added. Fresh, cryptographically valid stale/equal-conflicting records after expiry prove sequence retention independently of TTL. The critic then returned FINAL ACCEPT before production changes.
- Core tests cover same-second sequence changes, unchanged wire reuse without SQL churn, reopening, expired record renewal, root/network/peer/signature/TTL/canonical checks, no trusted chat from signed hints, a64-entry bound under70 independent signed roots, actual eviction of the confirmed contact's cache entry, persisted downgrade protection, atomic cache/contact SQL rollback, publication INSERT/UPDATE failure, and unchanged queued message ID/ciphertext followed by real MLS delivery and receipt.
- The partial-commit case deliberately lets the actual incoming MLS/history commit succeed and then fails the cache SQL update. Reopening and retrying the same envelope/record repairs the route and returns a valid receipt with identical history and MLS bytes/revision. Subsequent bidirectional messages prove usable recovery.
- `node_test_critic` separately returned FINAL ACCEPT for the real two-process regression. RED `/tmp/ain-peer-records-node-red.log` compiled and failed at the final delivery after both restarts. Bob changes his TCP listener while preserving his profile/root/PeerID and announces the address through an actual application message. The fixture reserves his obsolete port. Both daemons stop; Alice restarts and queues one operation while Bob is down. On Bob's restart, the same operation must reach his new endpoint and its signed receipt must drain Alice's outbox. No in-memory connection or second send can satisfy the test.

## Wire and state contract

The existing signed-document envelope, network domain, root authority epoch0 and4096-byte record bound are retained. Canonical CBOR bodies are:

- legacy: `[1, peer_id, [addresses...]]`;
- sequenced: `[2, peer_id, [addresses...], sequence]`, positive u64.

The verifier accepts both; legacy records are represented internally by sequence0. Every record has mandatory expiry, at most86400seconds after issuance, at most8 addresses of256bytes and a bounded peer string. Protocol-level signature/time/canonical checks are shared. The node additionally verifies actual libp2p PeerID and supported direct/circuit multiaddrs before handing routes to Core. A routing record carries public metadata and grants no messaging, agent, genesis/checkpoint, registry or storage-operator authority.

The trusted node publisher emits v2 and commits its sequence and signed wire to encrypted `network/own-record` before returning it. Route/PeerID changes and lifetime renewal increment the sequence; an unchanged current record is reused during its first half-life. Records never reset sequence merely because the transport restarts. Existing profile data loads with an absent optional contact route version. The old v1 signing method remains available for compatibility fixtures/trusted callers. Reading old wire/profile data is supported; pre-change daemons cannot consume v2 records, so communicating application/daemon instances must be upgraded together. Rolling mixed-version interoperability is not claimed.

Encrypted `network/peer-records` has at most64 entries and a640KiB serialization bound. Each root occupies one entry. Eviction uses local acceptance order, so peer-supplied timestamps cannot pin an entry; replay does not refresh its position. Currently expired records are excluded from discovery reads. New admissions prune expired records and evict the oldest remaining entry at capacity. Original signed wire is retained for subsequent discovery work.

Confirmed contacts retain their highest accepted sequence and document ID separately from the disposable cache. Equal sequence with a different signed document, lower sequence and downgrade to legacy cannot replace a newer confirmed route, even after expiry/eviction/restart. Legacy-only routes compare issuance time until first v2 acceptance. Exact replay does not rewrite state. Changing a route updates all existing contacts with that exact root; it cannot change their identity or queued ciphertext/operation ID. Cache and contact updates share one existing CAS transaction.

`receive_from` first validates binding, envelope authority and the actual application reducer. Only success learns the record. Message/MLS and route/cache are separate durable transactions; cache failure returns an error, and the existing duplicate reducer makes retry safe. A delayed valid message may carry an old record: its application content is accepted while the newer route remains intact. Last accepted contact addresses remain dial hints after record expiry; every delivery still authenticates the transport and expected root. Expiry removes the record from discovery results rather than silently changing established contact identity.

## Verification

Core targeted GREEN: `/tmp/ain-peer-records-core-green.log`, all9 tests including the follow-up below. Node targeted GREEN: `/tmp/ain-peer-records-node-green.log`, actual restart/address-change scenario.

The first full host gate passed183 Rust/27 frontend tests. The real Linux gate then found an additional regression at the combined relay/AutoNAT restart: a prior message carrying zero currently advertised routes had erased the receiver's last dial hint. `output/network-e2e/peer-record-withdrawal-red.json` preserves that failure and successful cleanup; `/tmp/ain-peer-records-withdrawal-red.log` independently reproduces it in a new Core test. The critic returned a separate FINAL ACCEPT before the correction. The test checks actual MLS admission, current empty cache record with sequence2, reopening, rejection of old sequence1, original queued operation using the last authenticated hint and becoming delivered only after a signed receipt, then replacement by sequence3.

The correction retains last contact dial hints when a new signed record has no currently advertised addresses, while advancing the contact version and storing the actual empty current record in cache. It does not revive the old record for discovery or bypass the node's route policy/PeerID/root authentication. Repeated empty records do not cause unnecessary contact writes. Full Linux and native gates are rerun on the corrected sources.

`/tmp/ain-peer-records-native-gate.log`: the complete native gate passed184 Rust/27 frontend tests, formatting/strict Clippy/type/build, all four packaged hidden WKWebView flows, normal release creation, deep/strict signature and absence of WebDriver in the ordinary dependency graph. The restored native network settings screenshot was viewed alongside the component reference: contact/history, saved provider, confirmed relay/public diagnostics and disabled unchanged Save are intact.

`/tmp/ain-peer-records-network-traced.log` and `output/network-e2e/result.json`: all6 Linux outcomes passed on source hash `95f169239cd2272f98d8125bff44d6bf715f796f31b5d0c3426ffbabc23498ad`; cleanupErrors is empty. The preceding `/tmp/ain-peer-records-network-final.log` failed earlier during AutoNAT firewall recovery. A read-only monitor of the same-source rerun retained owner `node_info` samples in `/tmp/ain-peer-records-nat-trace.json`; it showed multiple unconfirmed probes before success. The unrelated intermittent callback issue remains active investigation, not a claimed fix in this checkpoint. No assertion, timeout or fixture was weakened to obtain the successful run.

## Remaining boundaries

The cache does not create independent bootstrap providers, mDNS, Kademlia, mailbox rotation/indexes, or R=10 storage. Signed roots alone do not prove independent operators. A peer currently announces a changed address through successful application traffic; finding a peer when both sides have lost all usable addresses still requires discovery. Recovery from an old profile backup and root/device epoch transitions remain part of identity recovery work.

Follow-up: the intermittent callback issue above was subsequently traced and corrected in `N04-autonat-callback-lifetime.md`; the complete187-test/native and six-outcome Linux gates passed with that correction.
