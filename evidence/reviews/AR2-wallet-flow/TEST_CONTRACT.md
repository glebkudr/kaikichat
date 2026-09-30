# Ordinary wallet purchase, settings and send — active implementation

The user requires whole ordinary messaging capabilities after the accepted
AR3 disjoint-book recovery. V1 remains 67 cards /22 E2E /three platforms.
The concrete entry-point gap is that Core can prepare keys and bind externally
supplied proofs, but ordinary UI/CLI users cannot buy a resource book, see its
readiness/balance or select it for sending without internal proof/work inputs.

Implement one continuous flow using the existing issuer, Core wallet, SQLCipher,
checkpoint history and ordinary sender. Native daemon obtains untrusted RPC data;
existing Rust verification decides readiness. User selects class/count and funding
lifetime, reviews the exact chain/issuer/cost transaction and signs with an external
wallet. A transaction hash or untrusted RPC paid flag never grants tickets.
UI must distinguish funding lifetime from message retention, expose the finite
pre-beacon payment window, and handle unconfirmed/late/unavailable funding.
No company payment service, new currency or private-key export is introduced.

The current first tests cover the missing Core payment-request contract. They are
part of this whole flow, not standalone V1 acceptance. The result is a concrete
unsigned `purchase(bytes32,uint8,uint32,uint64)` transaction derived from local
pinned issuer economics and the real pre-beacon Core commitment. Purchase terms
and the private book key commit together before release. Exact/cold retry preserves
transaction and terms; changed count/class/lifetime conflicts. There is no funded
balance, reservation, sender job or spend just because a request was prepared.
Existing funded-book import remains compatible; when a purchase record exists,
actual class/count/expiry must match it before the shared bind path grants balance.
No exposed allocation is refunded or reset on re-import/restart.

Tests reuse `public_postage_wallet.rs` real-EVM fixture and SQLCipher helpers.
They independently decode standard ABI words (selector checked with existing
Foundry), exercise INSERT failure at both durable rows, invalid/changed terms,
late preparation, and genuine paid proof matching/mismatch plus cold allocation
conservation. Independent review accepted these tests before production edits; the missing-API
RED is recorded. Core purchase and daemon integration now pass the initial native gate.

Next within this same implementation: ordinary daemon purchase/read/configure
APIs with bounded asynchronous RPC reads and proof checks, a real-chain payment
and ordinary send/recipient gate, desktop/CLI/MCP read model, frontend behavior
and headless visual inspection. Those backend tests must receive independent
ACCEPT before their production implementation. Successful sender retirement must
preserve original/history evidence and cannot manufacture live durability.
A Core-only GREEN does not close the wallet flow or any whole V1 card.

The shared status/retirement extension now passes its complete native scenario:
[`status-native-candidate-3.json`](status-native-candidate-3.json). It includes
three actual SQL failures, cold shared CLI/MCP/desktop status, later publication
and sender-absent recovery after real data/index loss. The common projection is
independent of renewed wallet authority; an ordinary wallet refresh leaves the
completed projection and lifecycle rows unchanged. See
[`STATUS_LIFECYCLE_NEXT.md`](STATUS_LIFECYCLE_NEXT.md) for evidence and remaining
whole-capability work, including separate retained-evidence quotas.

## Current execution

Candidate 1 passed the real native payment/send/recovery flow; candidate 2 adds
actual independent ERC-681 decoding before signing. A constrained Tauri bridge
passes a real daemon/ACL test. The frontend renders quote/balance, separate
funding lifetime and retention, cumulative owner/agent limits; 35 targeted tests
cover state, exact values, retries, revision preservation and the payment window.
Visual inspection passed pending/ready/new purchase and 880px layouts using headless Playwright screenshots; no live wallet app has
been claimed compatible merely because the standard URI was accepted by Anvil.

The trust panel now also selects registry/finalizer profiles and committee policy.
Two real bridge tests, 46 frontend tests and a production frontend build pass;
headless visual evidence is in `network-profiles.json`. This is not native UI E2E.

The native payment candidates still use fixture setup to publish Alice's complete
committee roster and configure the postage client with issuer proof. The revised
gate forbids these commands, requires peer-acquired authority and cold recovery,
then performs the same two genuine sends and recipient loss/recovery scenario.
Core import tests require pinned current trust, complete authenticated membership,
issuer-bound policy and atomic clock/roster persistence. No public carrier may
install trust, move the head or create operator keys. This integration now passes:
`authority-native-candidate-1.json` records one real peer request/acceptance,
configuration revision 1 surviving process loss, two ordinary sends and independent
cold recipient recovery. 555 inputs stayed unchanged through the run. The first
native RED and independent review remain retained; 49 Core committee tests pass.

Public authority acquisition uses `/agentic-internet/postage-authority/1`: a
profile/checkpoint query capped at 256 bytes and an optional untrusted snapshot
capped at 2MiB. It reuses the existing processing budget, five-second timeout,
four-stream limit, one-at-a-time scheduler/backoff, inbound admission, authenticated
connected peers and relay policy. Current checked clients and configured spend
operators can supply evidence. Only wallets with saved ordinary purchase terms
initiate automatic acquisition; legacy explicit operator/client setup is retained.
Receiving a response rechecks the original head and local configuration revision.
The peer's observation timestamp never extends local live authority. This proves
acquisition with an available authenticated evidence source, not universal source
discovery through arbitrary unconfigured bootstrap nodes.

The standalone CLI and scoped MCP now reuse one credential/signing/IPC client.
The owner receives `cliConfig` alongside MCP configuration; both refer to the same
private credentials. The desktop shows the exact quoted command and the macOS
debug bundle contains the adjacent CLI. Three separate CLI tests, three Core
metadata checks, all 13 MCP regressions, seven recipient index regressions, 20
targeted frontend checks and Core/node all-target Clippy pass. The bundle's deep
strict signature verifies. Headless wide/narrow command screenshots are retained.

`cli-native-candidate-1.json` records actual separate CLI sends with no owner-send
RPC: exactly two tickets granted to the runtime, owner limit zero, idempotent
retries, refusal of a third message, R10/data/index/history verification and the
existing sender-absent loss/recovery scenario. Six QC signatures and 561 unchanged
inputs bind this run. CLI delivery remains queued while the recipient is absent,
even after all paid copies are acknowledged. It proves independence of existing
delivery, not a finished shared persisted durability/funding read model.

`cli-native-ui.json` separately records the actual hidden packaged WKWebView gate:
owner UI provisioning, execution of its displayed CLI shell command, same runtime
metadata/public identities, shared MCP/CLI send retry and delivery, and UI revoke
denying both. Existing network/trust scenarios and 1051-message paged history pass.
All 40 desktop input hashes remain unchanged. Native and component screenshots
were viewed together; the long real command remains scrollable in its read-only
field and executes intact. This does not exercise native paid wallet onboarding.

The whole flow still needs native fresh-user paid onboarding, the shipped skill
and full E11, shared UI/CLI/MCP status, and successful sender retirement. No partial
module closes this contract or full V1.
