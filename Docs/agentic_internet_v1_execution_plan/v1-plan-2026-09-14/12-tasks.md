# 12. Network and operator mode

A clean client joins an independent network, communicates behind NAT, and respects resource limits under attacks.

**Status:** planned; execution stopped per user instruction. This chapter does not confirm acceptance of the implementation.

[Overall order](README.md) · [Execution rules and commands](RUNBOOK.md) · [Full map](COVERAGE.md)

Review references: AR-R04, AR-R07, AR-R12, AR-R17.

The paths below are existing entry points and the responsibility boundary. New files are created only when necessary within these modules; this is not a requirement to create a new crate per task. The exact name of a new test/symbol is fixed at the tests-first stage.

## Algorithmic reference points

Full analysis and verified bibliography: [V1_ALGORITHM_RESEARCH_2026_09_19.md](../../V1_ALGORITHM_RESEARCH_2026_09_19.md). Below is the minimal reading list per task; tasks without an entry are plumbing without algorithmic novelty.

- **V1-N01** (private rendezvous rotation): Tor rend-spec-v3 / prop224; Coral DSHT; Signal sealed sender + limits (NDSS 2024); Pond; IPNS. Own construction — see research §6.
- **V1-N02** (surviving holders after losses): Kademlia (Maymounkov–Mazières, IPTPS 2002); the distinction between "not found" vs "unavailable" — honest-incomplete lookup semantics, research §6; Dynamo hinted handoff.
- **V1-N03** (bootstrap): Bitcoin peer discovery + Heilman eclipse attacks (USENIX Security 2015); BEP 5; IPFS bootstrap docs; TOFU (Perspectives).
- **V1-N04** (NAT matrix): Ford et al., "Peer-to-Peer Communication Across Network Address Translators" (USENIX ATC 2005); Guha–Francis STUNT (IMC 2005); RFC 8445/5389/8656 (ICE/STUN/TURN); libp2p hole-punching paper + DCUtR spec + Circuit Relay v2 + AutoNAT v2; Tailscale NAT traversal post.
- **V1-N05** (hostile peers, resource limits): SEDA (SOSP 2001); go-libp2p resource manager; WFQ; Google SRE overload chapter; eclipse-attack defenses — table in research §6.

<a id="v1-n01"></a>
## V1-N01. Close the loop on durable private rendezvous rotation

**Type:** backend. **Source cards:** N03, I04, D05. **Position in dependency order:** 80.
**After:** [V1-W04](04-tasks.md#v1-w04), [V1-R03](05-tasks.md#v1-r03).

**Change boundary / entry points:** `crates/node/src/bootstrap.rs`, `crates/node/src/bootstrap_schedule.rs`, `crates/node/src/routing.rs`, `crates/node/src/nat.rs`, `crates/node/src/relay.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/network_preferences.rs`, `tests/network`, `scripts/check-network.mjs`, `apps/desktop/src/NetworkPanel.tsx`.

**Implementation plan:**

1. Reuse Kademlia/private pointers for bounded mailbox rotation and up-to-date contact hints.
2. Ensure retained data of old periods can be read without unbounded enumeration of slots.
3. Update pointers after repair/epoch change with signature/domain/expiry checks.

**Verifiable scenarios:**

- After rotation, an offline recipient discovers the live history without the sender.
- Public enumeration, a stale pointer, and an unverified NodeRecord do not disclose contacts or the new route.

**Checks:** NETWORK CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The rendezvous lifecycle is tied to first contact, established history, and repair, not just DHT unit tests.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-n02"></a>
## V1-N02. Find admissible surviving holders after losses

**Type:** backend. **Source cards:** N02, N03, N05, D04. **Position in dependency order:** 81.
**After:** [V1-N01](12-tasks.md#v1-n01), [V1-ECO02](10-tasks.md#v1-eco02).

**Change boundary / entry points:** `crates/node/src/bootstrap.rs`, `crates/node/src/bootstrap_schedule.rs`, `crates/node/src/routing.rs`, `crates/node/src/nat.rs`, `crates/node/src/relay.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/network_preferences.rs`, `tests/network`, `scripts/check-network.mjs`, `apps/desktop/src/NetworkPanel.tsx`.

**Implementation plan:**

1. Check the current advertised-route limit: Full130 picks a survivor among four; that is not all nine losses out of ten.
2. Wire up the existing authenticated placement/registry discovery for the remaining admissible survivors without unbounded scanning.
3. When no live copy can be found, show the exact availability boundary; do not substitute Full130's conditions.

**Verifiable scenarios:**

- A surviving holder outside the four initial hints is discovered via a verifiable route.
- Someone else's placement proof and an unreachable network do not lead to a false empty/complete.

**Checks:** NETWORK CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The R10 discoverability region is defined and verified separately from the historical Full130 fixture.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-n03"></a>
## V1-N03. Close clean company-off bootstrap

**Type:** backend. **Source cards:** N02, N05, L06. **Position in dependency order:** 82.
**After:** [V1-N02](12-tasks.md#v1-n02), [V1-A01](03-tasks.md#v1-a01), [V1-ECO02](10-tasks.md#v1-eco02).

**Change boundary / entry points:** `crates/node/src/bootstrap.rs`, `crates/node/src/bootstrap_schedule.rs`, `crates/node/src/routing.rs`, `crates/node/src/nat.rs`, `crates/node/src/relay.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/network_preferences.rs`, `tests/network`, `scripts/check-network.mjs`, `apps/desktop/src/NetworkPanel.tsx`.

**Implementation plan:**

1. Gather several independent hints: invitation, cached peers, LAN where available, replaceable bootstrap/registry sources.
2. A new empty client must verify network/genesis/trust without relying on a single vendor endpoint.
3. Record the operators and failure domains of the external rig separately from local processes.

**Verifiable scenarios:**

- With company DNS/API/bootstrap turned off, the client joins via an independent invitation.
- A false registry/root/peer record is not accepted even though the endpoint is reachable.

**Checks:** NETWORK CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is a fresh-join transcript in an independent testnet; a single laptop does not count as independent operators.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

**External prerequisite:** Reachable nodes of independent operators and an approved testnet trust manifest.

<a id="v1-n04"></a>
## V1-N04. Accept the NAT/hole-punch/relay matrix for ordinary clients

**Type:** verification. **Source cards:** N01, N04, M06. **Position in dependency order:** 83.
**After:** [V1-C02](00-tasks.md#v1-c02), [V1-M04](09-tasks.md#v1-m04).

**Change boundary / entry points:** `crates/node/src/bootstrap.rs`, `crates/node/src/bootstrap_schedule.rs`, `crates/node/src/routing.rs`, `crates/node/src/nat.rs`, `crates/node/src/relay.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/network_preferences.rs`, `tests/network`, `scripts/check-network.mjs`, `apps/desktop/src/NetworkPanel.tsx`.

**Implementation plan:**

1. Reuse the isolated Linux/OrbStack topology; pin down NAT types and the unavailable direct path.
2. Check relay replacement, a possible direct upgrade, and reconnect for the packaged CLI and the desktop flow.
3. Preserve peer authentication/E2EE, limits, and cleanup without changing the host firewall.

**Verifiable scenarios:**

- Messages and ACKs arrive via another relay after the first is lost; a cold reconnect repeats the admissible path.
- The relay sees no plaintext and does not substitute the peer; a localhost-only exchange is not passed off as NAT.

**Checks:** NETWORK CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** There is real NAT/relay evidence for ordinary adapters on supported profiles.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.

<a id="v1-n05"></a>
## V1-N05. Verify hostile peers and operator limits

**Type:** verification. **Source cards:** N01, N06, F05. **Position in dependency order:** 84.
**After:** [V1-R04](05-tasks.md#v1-r04), [V1-N04](12-tasks.md#v1-n04), [V1-S04](02-tasks.md#v1-s04).

**Change boundary / entry points:** `crates/node/src/bootstrap.rs`, `crates/node/src/bootstrap_schedule.rs`, `crates/node/src/routing.rs`, `crates/node/src/nat.rs`, `crates/node/src/relay.rs`, `crates/node/src/operator_network.rs`, `crates/core/src/network_preferences.rs`, `tests/network`, `scripts/check-network.mjs`, `apps/desktop/src/NetworkPanel.tsx`.

**Implementation plan:**

1. Assemble oversize/slow-reader/forged-record/proof cases on real TCP/Noise/QUIC peers.
2. Measure memory, CPU, queue, and fairness for an honest peer under the current limits.
3. Node mode is enabled explicitly with a disk/network/CPU/price/TTL budget; disabling preserves the obligation or an explicit drain policy.

**Verifiable scenarios:**

- An honest chat keeps working under malicious peer load; resources stay bounded.
- The default client does not start storing others' data without consent; quota exhaustion does not issue a receipt.

**Checks:** NETWORK CUSTODY FRONTEND FORMAT profiles from [RUNBOOK](RUNBOOK.md). Backend tests → RED → independent final ACCEPT → production → targeted GREEN; for tests-only tasks production does not change. For verification tasks, real producer/adapters are used rather than manual setup of internal state.

**Done when:** The operator UX/daemon policy and the actual limits agree; the current revision passes the entire targeted hostile suite.

**Evidence:** task ID, test symbols/number of checks run, commands and exit codes, critic verdict for new backend tests, source/config/oracle/binary hashes, the outcome of each scenario, saved failures. Currently `not_run`.
