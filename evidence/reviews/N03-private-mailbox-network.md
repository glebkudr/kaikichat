# N03 — bounded private pointer publication and retrieval

Incremental network integration of the existing private MLS mailbox/Core contract. This implements
actual ephemeral DHT pointer caching and owner publication/lookup. It does not implement retained
indexes, ciphertext custody, independent operators, paid admission or autonomous R=10 repair.
No N03/D05/E01–E26 task is marked complete on the strength of this slice.

## Observable behavior

Owner IPC adds three strictly parsed commands; scoped MCP/agent authority is unchanged:

- `publish_mailbox`: conversationId, indexId (32-byte hex), 1–4 validated IP multiaddresses,
  expiresAt and expectedSequence. Core derives and durably saves the exact finite signed batch
  before network work. Exact active retries coalesce; changed active intent is rejected. Restart
  retries use the previously saved bytes. Expired daily records are omitted from retries.
- `lookup_mailbox`: conversationId. The recipient derives only previous/current/next keys and
  performs actual bounded GET_VALUE queries. Public signature/key/lease validation precedes the
  existing Core AEAD/epoch/checkpoint verifier. A locator is resolved only after successful SQLCipher
  persistence. Forged candidates do not create contacts, history or checkpoints.
- `mailbox_head`: conversationId. Returns the accepted current-epoch, unexpired indexId/endpoints,
  sequence, epoch and expiresAt, or null. It is an owner API, not a public record inventory.

`node_info.mailboxes` reports cache count/bytes and bounded publication/lookup rows. A publication
is `published` only after every currently publishable record receives matching acknowledgements
from at least two distinct remote PeerIDs. This means two ephemeral cache acknowledgements, not
independent operators, durable storage or message delivery. An unreachable publication is failed
with zero acknowledged records. Message phase/replicas/outbox remain governed by actual ciphertext
receipts; a published locator leaves an offline message queued with zero replicas.

## Bounds and privacy

- The shared authenticated peer routing table remains bounded at128 peers and4 routes per peer.
  Iterative referrals remain hints, never contacts or the trusted operator registry.
- Pointer and peer searches share two active routing slots. Each pointer query has32 initiated
  request attempts,128 referral candidates and a20second deadline. GET reads at most three daily
  keys (at most96 attempts); failed dials consume work too.
- Four active owner jobs, at most31 records per publication and16 retained status rows. The fifth
  active job is refused before Core allocates a sequence or writes a publication. Completed rows
  are evicted; replacing the network marks active work failed and releases its query bookkeeping.
- Ephemeral cache:128 records,4096 bytes per wire, at most512KiB retained wire payload plus bounded
  metadata. Both signed wall expiry and monotonic deadline apply; exact replays do not renew a
  lease. Sequence rollback and equal-sequence wire forks are refused while the record is retained.
  Eviction/expiry discards ephemeral replay memory; durable recipient checkpoint remains in Core.
- Incoming GET/PUT admission reuses the existing limiter with128 global/32 per-peer requests per
  minute, sufficient for31 daily pointers. Existing bootstrap/FIND_NODE policy remains64/8. The
  outer frame/key/size bounds run before cryptographic verification. Generic unsigned records and
  provider inventories are refused. A signature does not buy storage or defeat Sybil attacks.
- PUT admission validates and inserts the finite cache record before permitting an ACK. Upstream
  FilterBoth prevents a second unconditional store. Incoming publisher fields are removed.
- Publication explicitly owns FIND_NODE then selective PUT_VALUE under one logical job/budget.
  Unlike libp2p's convenience put_record path, it never constructs a record carrying the local
  publisher PeerID, including requests preloaded into new connection handlers. Explicit outgoing
  notifications also clear that field. The endpoint still sees its network peer and traffic timing;
  this does not claim anonymity against that peer or a global observer.
- Relay-only policy blocks this direct Kad implementation before new Core publication or search.
  Routing private records over circuits, automatic outbox-to-index integration and UI mailbox
  controls remain future integration work.

## Test-first review and independent observations

All new/changed backend tests received FINAL ACCEPT from the separately launched, context-free
`/root/node_test_critic` before their associated production implementation. Initial cache/API tests
were RED on absent APIs; five actual-process tests were RED on unknown owner methods.

The first network review required four stronger oracles before ACCEPT: deleting only the saved
read head before the second-cache lookup, capturing exact outgoing retry wires and absent publisher
fields at independent servers, SQL INSERT/UPDATE failures before resolved status, and proving that
real pending requests hold four jobs while a fifth leaves no Core publication row.

Eight unit tests cover the actual RecordStore interface and signed cache admission; malformed,
wrong-domain/key/signature and expired records; rollback/forks;128-record global capacity and
replacement/expiry; provider refusal; wall/monotonic expiry; shared routing slots; unchanged default
bootstrap limits and the128/32 record policy. A regression test runs200 deterministic100ms cache
maintenance ticks during wall-clock rollback: the record is live at10seconds, pruned at20seconds,
and its exact replay remains refused. This exposed fractional elapsed time being lost by repeatedly
resetting a rounded clock anchor. The implementation now preserves that anchor until a newer wall
observation actually advances the floor. RED and corrected test received independent review.

Five actual-process tests use real production daemons, MLS profiles and SQLCipher, plus raw,
independent libp2p Kad/bootstrap swarms whose connection-handler events capture the actual wire:

1. Sender publishes to two real cache nodes, then stops. Recipient resolves from one, stops, removes
   only its disposable saved read head, loses that cache and resolves again from the surviving
   cache. No sender/local cache/old connection survives. A restarted publisher then uses two fresh
   capture servers: each sees every originally persisted key+wire with no publisher PeerID. Later
   direct message/reply receipt resumes exactly once, preserving the queued operation.
2. A raw server returns forged records under the correct keys; actual GETs occur but no head is
   accepted. Replacing them with valid records while SQL writes are deliberately rejected produces
   failed lookup and zero read rows. Removing the triggers and redoing network GETs resolves and
   persists the correct checkpoint across restart.
3. Forty independent nodes form a strictly closer referral chain for the current private key.
   Server-side GET counters observe exactly32 target-key requests and at most96 in all windows;
   no locator appears and query slots become reusable. The fixture explicitly removes only Bob's
   stopped discovery cache so a stale Alice dial cannot consume one attempt outside that counter.
   All MLS/application state is preserved. This isolation correction received separate ACCEPT.
4. Wrong owner proof, unknown/extra fields, malformed index, unsupported DNS endpoint and expired
   lease are refused. No reachable cache produces failed/zero-ACK state. Reopening Core confirms
   rejected inputs allocated no extra sequences. Relay-only refusals leave the prior batch intact.
5. Independent servers capture actual unanswered PUTs while four jobs remain pending. Exact retry
   coalesces; the fifth returns busy before any SQL publication. After server recovery, subsequent
   jobs publish at sequence1; eighteen conversations leave at most16 status rows. The raw fixture
   drops held requests, so initially held jobs may fail; it does not claim to resume those frames.

The chain test initially exposed a real limit bypass: libp2p preloads pending requests directly
into new connection handlers, bypassing NotifyHandler interception. Query statistics now bound
both paths; normal exhaustion does not cancel unrelated queued work. No production logic is used
by the independent fixture's GET counters or PUT wire capture.

Mechanical Rust const-parameter inference and Clippy test edits also received ACCEPT without
changing inputs or assertions. Existing workspace libraries were reused; the only dependency
change is the direct local path to agentic-crypto. No package/version was installed.

## Regression evidence

Node Clippy and all79 node tests (26 unit,53 process) passed before the complete gates.
Both final full runners exited0 on2026-09-05:242 Rust tests,30 frontend tests, strict formatting/
Clippy, TypeScript/Vite, four actual hidden WKWebView flows and a normally packaged macOS release
with deep/strict ad-hoc signature verification and no WebDriver in the normal dependency graph.
Own vision compared the newly packaged restored-network screenshot with the preceding verified
image: layout, scrollable controls and relay state remain intact. No native fixture processes remain.
Apple notarization is not configured; other release platforms and full V1 acceptance remain open.

The first Linux run (`ain-nat-c228cc6c`) passed six outcomes then failed because its test controller
read `/proc/<pid>/cmdline` as that owned process exited. All owned resources were cleaned up.
`fixture.py` now treats only FileNotFoundError/ProcessLookupError as process disappearance, including
the equivalent exit-between-ownership-check-and-kill case. The executable ownership check, timeout
and all product assertions remain. This narrow test-helper correction received FINAL ACCEPT.
Failure log: `N03-mailbox-dht-network-fixture-failure.log`.

The final current-source Linux run passed all7 outcomes: sourceHash
`c4c2b0797a8392e01092b23bedeee21f3b4abd4bbeb3d309eb231906156629df`, runId`ain-nat-fed57e57`, cleanupErrors`[]`.
Exact run-label queries found no remaining containers or networks for either run. Logs:
`N03-mailbox-dht-native-green.log`, `N03-mailbox-dht-network-green.log`,
`N03-mailbox-dht-node-green.log`; clock regression RED:`N03-mailbox-clock-red.log`.
Current network report:`output/network-e2e/result.json`. The fixture-only correction did not change
Rust/desktop sources after the native gate. These checks do not close the full V1 goal.
