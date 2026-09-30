# Algorithmically hard spots of V1 and a bibliography — September 19, 2026

A consolidated research document: across all 124 tasks of the `agentic_internet_v1_execution_plan/v1-plan-2026-09-14` plan, it identifies the places where implementation runs into known-hard algorithmic problems and collects the canonical literature for each topic. Links were verified by search; items explicitly marked `[verify]` are known inaccuracies left in deliberately.

This is a navigation document, not a work plan. It does not override task acceptance criteria and adds no scope.

## How to read

- Task IDs are from v1-plan-2026-09-14 (`V1-H01` etc.); the original cards are `F01/N03/D02/…`.
- "Status in the project" is what has already been chosen in spec/code (as of `implementation/v1`).
- "Read" lists primary sources in decreasing order of applicability.

## Map of hard spots (ranking)

| # | Area | Tasks | Essence of the difficulty | Already chosen |
|---|---|---|---|---|
| 1 | Concurrent MLS commits + BFT ordering | G03, G04, QA-E09 | total order + rebase + forbidding fork/key-reuse under a Byzantine proposer | Simplex finalizer + OpenMLS 0.9 |
| 2 | Epochs/authority transition, spent-set handover | A01, A02, A06, P04, QA-E21 | reconfiguration + double-spend across the epoch boundary, stale-checkpoint rebind | successor chains, exact-QC terminal, prefix streaming |
| 3 | Atomic spend without double-spend | A02–A06, P03, ECO04, QA-E20 | global spent-set, idempotency, crash between stages | finalizer log + (domain,nullifier) |
| 4 | Autonomous repair 10→7→10 | R01–R05, N02, D03/D04, QA-E06 | loss detection ≠ timeout, fencing on repair, storms | placement proofs, probes, leases |
| 5 | Resumable history sync | H01–H11, D05, QA-E05 | durable cursor, gap-tolerant catch-up, proof-DAG traversal | page-forest, deferred body store |
| 6 | MLS persistence and key GC | S03, G05, H08 | incremental ratchet persistence in an SQL transaction; key retention | OpenMLS storage provider |
| 7 | Distributed GC/retention | A04, B04, S02, I04, QA-E07 | shared manifest multi-recipient, crash-safe sweep, TTL | tombstones, refcount CAS |
| 8 | Private rendezvous/mailbox | W01–W03, N01, I04 | rotating pointers, anti-enumeration, bounded lookup | daily keys from the MLS exporter, Kademlia |
| 9 | Networking: NAT/relay/bootstrap/hostile | N02–N05, QA-E02–E04 | hole-punch matrix, company-off trust, admission/fairness | libp2p kad/dcutr/relay/AutoNAT |
| 10 | Sortition/randomness for the registry | ECO02, R02, N05, L02 | blockhash seed knowingly biasable; committee risk | sparse Fisher–Yates + delayed seed |
| 11 | Proof-of-storage/service claims | ECO06, ECO07, R01, D03 | what is provable at all; verifier key is centralization | challenge-response inspections |
| 12 | zk-postage circuit | A05, ECO boundaries | verifier boundary, nullifier domains, anti-fallback | RISC Zero 3.0.6 Succinct |
| 13 | Anti-rollback backup/restore | I03–I06, QA-E10/E23 | an old snapshot must not resurrect rights/stamps | checkpoint floors |
| 14 | Economy: pricing/subsidy/settlement | ECO01/03/05/07/08, P05 | conservation, reorg-safe ingest, decay schedules | contracts + pull-credits |
| 15 | Deterministic simulation/soak | C03, F03, RC-SOAK, QA-E* | seed-replay, fault manifest, wall-clock soak | commonware deterministic runtime |
| 16 | Secure updates | P05, QA-E24/E25 | anti-downgrade, atomic swap, manifest freshness | signed manifest (TUF model) |
| 17 | k-of-n attestors, onboarding | O01–O10, QA-E17–E19 | pairwise subject dedup is an open problem | multisig counting (not threshold) |

---

## 1. Concurrent MLS commits on top of BFT ordering — the hardest bundle

**Tasks:** V1-G03 (BFT ordering control log), V1-G04 (concurrent commits + deterministic reject), V1-G05 (cross-epoch catch-up), V1-G02 (DA before finality), QA-E08/E09/E10.

**Status in the project:** Commonware Simplex (P-256/Ed25519, `n=3f+1`, QC, voting WAL) — `spec/finalizer/engine-v1.md`, `crates/finalizer`; OpenMLS 0.9 / RFC 9420 — `crates/crypto`, `spec/mls-adapter-v1.md`. The choice is made: a total-order log instead of CRDT/causal merge.

**Key finding:** there is no academic model of "MLS on top of a BFT total-order log" — this is a real niche. Closest formalizations: saCGKA (the server as an active sequencer) and DCGKA (proof that causal order suffices — i.e. our total order is a deliberate overpayment for roster determinism). Justification of the price of concurrency: the Bienstock–Dodis–Rösler Ω(t) lower bound.

**Read:**

- Barnes, Beurdouche, Robert, Millican, Omara, Cohn-Gordon — **RFC 9420 "The MLS Protocol"**, IETF 2023. https://www.rfc-editor.org/rfc/rfc9420 — key point: the DS must provide "at most one Commit per epoch"; `epoch_authenticator` detects forks.
- Beurdouche et al. — **RFC 9750 "MLS Architecture"**, IETF 2025. https://datatracker.ietf.org/doc/rfc9750/ — requirements on DS ordering, key-deletion policies, and buffering of undecryptable messages (our gap-store).
- Alwen, Jost, Mularczyk — **"On the Insider Security of MLS"**, CRYPTO 2022, ePrint 2020/1327. https://eprint.iacr.org/2020/1327 — what a Byzantine insider can do with roster/keys; ITK.
- Alwen, Coretti, Dodis, Tselekounis — **"Security Analysis and Improvements for the IETF MLS Standard"**, CRYPTO 2020, ePrint 2019/1189 — formalization of CGKA; weak FS without fixes.
- Weidner, Kleppmann, Hugenroth, Beresford — **DCGKA, "Key Agreement for Decentralized Secure Group Messaging"**, CCS 2021. https://doi.org/10.1145/3460120.3484542 — a counterexample: causal-order broadcast instead of total order.
- Alwen, Auerbach, Cueto Noval, Klein, Pascual-Perez, Pietrzak, Walter — **CoCoA "Concurrent Continuous Group Key Agreement"**, EUROCRYPT 2022, ePrint 2022/251 — concurrent updates without taint penalties.
- Bienstock, Dodis, Rösler — **"On the Price of Concurrency in Group Ratcheting Protocols"**, TCC 2020, ePrint 2020/1171 — lower bound on PCS under concurrency; justification of "one commit/epoch".
- Alwen, Hartmann, Kiltz, Mularczyk — **SAIK, "Server-Aided Continuous Group Key Agreement"**, CCS 2022, ePrint 2021/1456 — saCGKA: a formal model of the DS sequencer.
- Wallez, Protzenko, Beurdouche, Bhargavan — **TreeSync**, USENIX Security 2023 — machine-checked roster consistency (parent hash, unmerged leaves); an attack in the draft was found and fixed.
- Cohn-Gordon, Cremers, Garratt, Millican, Milner — **ART**, CCS 2018, ePrint 2017/666 — ancestor of TreeKEM.
- Klein et al. — **Tainted TreeKEM**, CRYPTO 2021, ePrint 2019/1489; Chevalier et al. — **Quarantined-TreeKEM**, CCS 2024, ePrint 2023/1903 — "sleeping" leaves (our offline>3 epochs).
- Matrix **dMLS** design docs (decentralised.org doc, MSC4256, arewemlsyet.com) — the only published engineering scheme for MLS fork resolution: `resolves` lists, backfill commits, base epoch.
- draft-ietf-mls-extensions (-10, WGLC) — `self_remove`, `app_data_dictionary`, AppAck; draft-ietf-mimi-arch — a centralized ordering counterexample.

## 2. BFT finalizer, epochs, and spent-set handover

**Tasks:** V1-A01/A02/A06 (authority lifecycle), V1-P01/P04 (reusable finalizer, epoch survival), V1-G03, QA-E09/E20/E21.

**Status:** Simplex engine + ancestry proofs + quiet-tip recovery + binding floors — `spec/finalizer/*`; epoch≠1 is still forbidden; spent-prefix streaming newest-first ≤7 entries — `spec/postage/*-v1.md`.

**Read:**

- Chan, Pass — **"Simplex Consensus"**, TCC 2023 (not CRYPTO), ePrint 2023/463. https://eprint.iacr.org/2023/463 — our consensus.
- Commonware monorepo `consensus/src/simplex` + docs.rs `commonware_consensus::simplex`; blogs: `commonware.xyz/blogs/threshold-simplex`, `.../reshare` (epoch reconfiguration), `.../coding` ("notarization ≠ certification" — the seam for the custody gate `CertifiableAutomaton::certify`).
- Chan, Shi — **Streamlet**, AFT 2020, ePrint 2020/088 — ancestor of Simplex.
- Castro, Liskov — **PBFT**, OSDI 1999 — §4.3 checkpoints/state transfer: the canonical ancestor of QC-verified prefix streaming.
- Yin, Malkhi, Reiter, Gueta, Abraham — **HotStuff**, PODC 2019, DOI 10.1145/3293611.3331591 — the QC/three-chain vocabulary.
- **Reconfiguration:** Lamport-Malkhi-Zhou "Reconfiguring a State Machine" (SIGACT News 41(1), 2010) + "Stoppable Paxos" (MSR-TR-2008, [verify number]); "Vertical Paxos" (PODC 2009 brief); Ongaro "Raft" ATC'14 §6 + dissertation §4; Bessani-Sousa-Alchieri **BFT-SMaRt** DSN 2014; Alvisi et al. "Dynamic Byzantine Quorum Systems" DSN 2000; Komatovic-Lewis-Pye-Neu-Roughgarden-Tas "From Permissioned to PoS Consensus" AFT 2025, ePrint 2025/1139 — **the very paper Commonware reshare cites** (direct finalization of the last block of an epoch); Buterin-Griffith Casper FFG, arXiv 1710.09437.
- **Double-spend without total order:** Baudet-Danezis-Sonnino **FastPay**, AFT 2020, arXiv 2003.11506; Baudet-Sonnino-Kelkar-Danezis **Zef**, WPES 2023, ePrint 2022/083; Collins et al. **Astro** "Online Payments by Merely Broadcasting Messages", DSN 2020; Guerraoui et al. "The Consensus Number of a Cryptocurrency", PODC 2019 — payment = consensus number 1.
- **DA before ordering (= "custody receipts before finality"):** Danezis-Kokoris-Kogias-Sonnino-Spiegelman **Narwhal & Tusk**, EuroSys 2022, arXiv 2105.11827 — the closest analogue; Spiegelman et al. **Bullshark** CCS 2022; Yang et al. **DispersedLedger** NSDI 2022 (VID); Al-Bassam **LazyLedger** arXiv 1905.09274 + **Fraud/DA proofs** arXiv 1809.09044; Ethereum proof-of-custody ethresear.ch threads (/949, /5692, specs#568).
- **Quorums:** Malkhi-Reiter "Byzantine Quorum Systems" STOC 1997 (masking vs dissemination — asymmetry: signed data tolerates n≥2b+1); Guerraoui-Vukolić "Refined Quorum Systems"; Malkhi-Nayak-Ren "Flexible BFT" CCS 2019.
- **Checkpoint sync:** Buterin "Weak Subjectivity" (blog.ethereum.org 2014); consensus-specs `phase0/weak-subjectivity` and `altair/light-client/sync-protocol` (512 attesters, >2/3 — a direct analogue of our attester threshold).

**Novelty:** "newest-first QC-verified spent-prefix streaming of spent history" — there is no canonical paper; it is assembled from PBFT §4.3 + weak subjectivity + Narwhal certificates. Worth recording as our own construction in the spec.

## 3. Replication, loss detection, autonomous repair

**Tasks:** V1-R01–R05 (10→7→10), V1-N02/N05, V1-D03/D04 continuations, QA-E06; V1-ECO06 (what counts as proof of service).

**Status:** placement proofs + inspection (challenge-response, `spec/custody-manifest-inspection-v1.md`), bounded scheduler, fencing commitments. Erasure coding is deliberately not used — full replicas.

**Read:**

- **Repair lifecycle:** Bhagwan et al. **Total Recall**, NSDI 2004 — lazy vs eager repair; Chun et al. **Carbonite** NSDI 2006 — "repair only permanent losses, not transient ones" (~2x traffic savings) — our anti-storm principle; Blake-Rodrigues "Pick Two" HotOS 2003 — the cost of maintaining redundancy; Ford et al. OSDI 2010 — correlated Google failure domains; Haeberlen et al. **Glacier** NSDI 2005 — massive redundancy for correlated failures (holder Sybil risk).
- **Churn models:** Bhagwan-Savage-Voelker "Understanding Availability" IPTPS 2003; Stutzbach-Rejaie IMC 2006.
- **Proof of storage:** Juels-Kaliski **POR** CCS 2007; Shacham-Waters **Compact POR** ASIACRYPT 2008 (public verification, ~40-byte response — verifier without the owner); Ateniese et al. **PDP** CCS 2007; Curtmola et al. **MR-PDP** ICDCS 2008 — proof of t distinct replicas + regeneration on failure (exactly R=10); Bowers-Juels-Oprea **HAIL** CCS 2009 — inspection→shard redistribution; Armknecht et al. **Mirror** USENIX Sec 2016; Fisch **PoReps** ePrint 2018/678 — **proves the fundamental unprovability of physical independence** (our explicit non-goal); Fisch "Tight Proofs of Space and Replication" EC'19; Dziembowski et al. **Proofs of Space** CRYPTO 2015; Filecoin whitepaper + "Proof of Replication" tech report + PoRep spec; Permacoin S&P'14; **Sia whitepaper** (storage contracts = payment for periodic proof — the closest model of paid receipts); Storj v3 (audits+repair); Douceur **"The Sybil Attack"** IPTPS 2002 — justification of "independence is unprovable".
- **Swarm (our economic model):** Trón "The Book of Swarm" (DISC, push/pull-sync, postage, redistribution game, insurance); "Future-proof Storage" v4.20 (formal postage spec + Appendix B lottery randomness); "swap, swear and swindle" orange paper 2016 (proof-of-custody→payment escrow chain).
- **Failure detection:** Das-Gupta-Motivala **SWIM** DSN 2002; Hayashibara et al. **φ accrual FD** SRDS 2004 — a "suspected/dead" gradation; Dadgar et al. **Lifeguard** DSN-W 2018 — −50x false positives; Dynamo SOSP'07 — hinted handoff + Merkle anti-entropy.
- **Fencing/lease on repair:** Burrows **Chubby** OSDI 2006 (sequencer); Kleppmann "How to do distributed locking" 2016 — **fencing token**; Gray-Cheriton **Leases** SOSP 1989; Junqueira-Reed-Serafini **Zab** DSN 2011 (epoch fencing); GFS SOSP 2003 (chunk leases); "Reliable Cron" ACM Queue 2015 (single-executor takeover).
- **EC alternative (deferred option):** Dimakis et al. "Network Coding for Distributed Storage" (regenerating codes, MSR/MBR tradeoff); Huang et al. **LRC** ATC 2012 ("replicas first, codes in the background" — lazy EC); Weatherspoon-Kubiatowicz IPTPS 2002 and Rodrigues-Liskov IPTPS 2005 (EC vs replication — both sides of the argument); Rashmi et al. Hitchhiker SIGCOMM 2014.
- **DHT specifics:** Maymounkov-Mazières Kademlia IPTPS 2002 (republish/TTL); CFS/PAST SOSP 2001 (k-nearest holders); Cohen-Shenker SIGCOMM 2002 (how many copies are optimal); Rhea et al. OpenDHT-on-PlanetLab WORLDS 2005 (alive ≠ storing; maintenance traffic).

## 4. Authenticated history, sync, and durable cursors

**Tasks:** V1-H01–H11 (the whole chapter 01), V1-D05 continuations, QA-E05/E21; specs `custody-history-pages-v2` (MMR-like forest, consistency per RFC 9162 §2.1.4).

**Read:**

- **Tamper-evident logs:** Laurie-Langley-Kasper **RFC 6962**; Laurie-Messeri-Stradling **RFC 9162** (§2.1.4 SUBPROOF — our model); Crosby-Wallach "Efficient Data Structures for Tamper-Evident Logging" USENIX Sec 2009; Maniatis-Baker "Timeline Entanglement" USENIX Sec 2002; Naor-Nissim USENIX Sec 1998; Tamassia "Authenticated Data Structures" ESA 2003 (survey); Google Trillian "Verifiable Data Structures"; **MMR**: Todd writeup (opentimestamps-server doc) + Grin docs (`docs.grin.mw` mmr). Note: the MMR proof format and RFC 9162 consistency proofs differ in framing — decide in the spec which one is canonical.
- **Set reconciliation:** Meyer **"Range-Based Set Reconciliation"** SRDS 2023, arXiv 2212.13567 + **negentropy/NIP-77** (hoytech) — the model for resumable sync; Yang-Gilad-Alizadeh **"Practical Rateless Set Reconciliation"** SIGCOMM 2024, arXiv 2402.02668 (rateless IBLT); Goodrich-Mitzenmacher **IBLT** Allerton 2011; Eppstein et al. SIGCOMM 2011; Minsky-Trachtenberg-Zippel CPI (IEEE TIT 2003); Dynamo §4.7 (Merkle sync); Leitão-Pereira-Rodrigues **PlumTree** SRDS 2007; Bitcoin headers-first IBD + BIP-159 + Andresen IBLT gist.
- **Crash consistency / migrations:** Pillai et al. "All File Systems Are Not Created Equal" **OSDI 2014** (not FAST); Alagappan et al. "Protocol-Aware Recovery" FAST 2018; Rae et al. **F1 online schema change** VLDB 2013 (≤1 version of difference — our resumable migration); Rodeh "B-trees, Shadowing, Clones" ACM TOS 2008; LMDB paper (Chu); SQLite atomic-commit + WAL docs.
- **Dedup/retry:** KIP-98 + Wang et al. SIGMOD 2021 (idempotent producer (PID,seq) — portable: (sender,epoch,seq) on the wire); transactional outbox (Richardson/AWS); Brooker "Timeouts, retries, backoff with jitter" (Builders' Library) + "What is Backoff For?" 2022 (backoff does not heal first attempts); Two Generals (Akkoyunlu et al. SOSP'75) — justification of at-least-once+dedup; keyset pagination (Winand use-the-index-luke); Pugh skip lists CACM 1990.

## 5. Distributed GC and retention

**Tasks:** V1-A04 (retention of deliveries), V1-B04 (GC vs leases/repair), V1-S02 (CAS dedup proof bundles), V1-I04, QA-E07.

**Read:**

- Plainfossé-Shapiro "A Survey of Distributed Garbage Collection Techniques" IWMM 1995 — taxonomy of refcount vs tracing in distributed systems (our multi-recipient manifest).
- Gray-Cheriton Leases SOSP 1989 — the theoretical basis of paid-TTL.
- Cassandra tombstones/`gc_grace_seconds`/`only_purge_repaired_tombstones` — repair-gated purge, "zombie resurrection" = our main failure mode.
- `git gc`/`prune` + Pro Git ch.10 — reachability mark-sweep with a grace period; restic design.rst / Borg internals — CDC dedup + prune; Quinlan-Dorward **Venti** FAST 2002 — the canonical CAS; LBFS SOSP 2001; IPFS pinning (per-consumer refcount over a shared DAG — a direct analogue of the shared manifest).
- Chunking attacks: Alexeev-Percival-Zhang "Chunking Attacks on File Backup Services" — keep chunker parameters secret.

## 6. Networking: Kademlia, private rendezvous, NAT, bootstrap, admission

**Tasks:** V1-N01–N05, V1-W02, QA-E02/E03/E04; specs `service-kad-v1`, `mailbox-history-locator-v1`, `dht-roles-v1`.

**Kademlia/DHT security:**

- Maymounkov-Mazières IPTPS 2002; Baumgart-Mies **S/Kademlia** ICPADS 2007 (not ICNP); Sit-Morris "Security Considerations for P2P DHT" IPTPS 2002 (verifiable invariants); Singh et al. eclipse INFOCOM 2006 + the MSR-TR predecessor; Urdaneta-Pierre-van Steen survey ACM CSUR 2011; libp2p kad-dht spec (k=20, α=10 — watch for divergences from our behaviour); IPFS kad-dht spec (republish 22h/expire 48h); Trautwein et al. IPDS SIGCOMM 2022 (measurements of a live DHT); discv5 specs + Król et al. **DISC-NG** EuroS&P 2024 (topic ads — the closest analogue of publishing rotating pointers).

**Private rendezvous — the main design source:**

- **Tor rend-spec-v3 / prop224** — daily blinded keys + SRV-rotated HSDir + descriptor encryption: almost one-to-one our daily mailbox pointers + anti-enumeration.
- Freedman-Mazières **Coral DSHT** IPTPS 2003 — sloppy hashing: many pointers under one key without a hot spot — the structure for pointer rotation.
- Langley **Pond** (mailbox groups, BBS spam-control — the debate on replacing it with HMAC is relevant to paid admission); Lund **sealed sender** 2018 + NDSS 2024 "Improving Signal's Sealed Sender" (statistical disclosure after ~5 messages — the limit of our privacy guarantees); IPNS record spec (hash-of-pubkey mutable pointers on top of the same Kad); McLachlan et al. **Torsk** CCS 2009; Lazar-Zeldovich **Alpenhorn** OSDI 2016; van den Hooff et al. **Vuvuzela** SOSP 2015.
- libp2p RFC 0002/0003 (signed envelopes/peer records) — a ready-made primitive for pointers.

**NAT/relay:**

- Ford-Srisuresh-Kegel USENIX ATC 2005 (canonical hole punching); Guha-Francis IMC 2005 (TCP NAT characterization); RFC 8445 ICE / 8489 STUN / 8656 TURN / 4787 / 5382; Seemann-Inden-Vyzovitis "Decentralized Hole Punching" ICDCS-W DINPS 2022 (research.protocol.ai); libp2p hole-punching doc (public/private matrix) + DCUtR spec + Circuit Relay v2 (RESERVE/vouchers, limits) + AutoNAT v2 (per-address nonce — fixes the v1 QUIC 4-tuple false positive, specs#503/#536 — our GuardedAutonat); Tailscale "How NAT traversal works" (+APNIC series, birthday-paradox port guessing); arXiv 2510.27500 — 4.4M real DCUtR attempts from IPFS, ~70% success — calibrating expectations for N04.

**Bootstrap without the company:**

- Bitcoin peer discovery (DNS seeds → addr gossip → peers.dat; the warning about eclipse from seed operators); Heilman et al. "Eclipse Attacks on Bitcoin" USENIX Sec 2015 (feeler/anchor connections — a checklist); BEP 5 + bootstrap-dht repo + BitTorrent "DHT Bootstrap Update"; IPFS bootstrap list + libp2p mDNS spec; Wendlandt-Andersen-Perrig **Perspectives** USENIX ATC 2008 — multi-path notaries for TOFU genesis/trust; libp2p Noise spec (PeerID-bound auth: hint = (addr,PeerID)).

**Admission/backpressure/fairness:**

- Welsh-Culler-Brewer **SEDA** SOSP 2001; Dean-Barroso "The Tail at Scale" CACM 2013; Demers-Keshav-Shenker WFQ SIGCOMM 1989; Montazeri et al. **Homa** SIGCOMM 2018 (receiver-driven credits — the model for slow-reader protection); Kung et al. credit-based flow control SIGCOMM 1994; Turner 1986 (leaky/token bucket); Brooker backoff+jitter; SRE "Handling Overload" ch.21 + Envoy admission control filter; RFC 9000 §4 (QUIC flow control); **go-libp2p resource-manager README** — the hierarchy of system/service/protocol/peer/stream limits; rust-libp2p has no analogue — this is our design template for N05; Alvaro-Rosen-Hellerstein **LDFI/Molly** SIGMOD 2015.

## 7. Deterministic simulation and acceptance

**Tasks:** V1-C03 (fault manifest), V1-C05 (fast diagnostic loop and a reproducer with controlled time), F03, RC-SOAK, QA-E* methodology.

**Status:** commonware deterministic runtime + simulated network in finalizer tests; there is no separate `test-runtime` crate; real SQL faults (SQLite triggers), not mocks.

**Adopted decision (September 19, review at `d278f2a`):** run acceleration is done with two loops — (1) harness level: choosing the diagnostic phase, a no-progress budget, early stop with a full snapshot; (2) the "clocks + timers + events" seam with four distinct time domains (the protocol `SystemTime` derivative, the scheduler's monotonic `Instant`, chain/Anvil time, real execution time) and jumping to the nearest event. Faking system clocks (`faketime`, `CLOCK_REALTIME`) does not work: `Instant`/timers do not shift; QEMU `icount` is a separate experiment only. Native A04/H11 remain the only acceptance evidence.

**Read:**

- Will Wilson "Testing Distributed Systems w/ Deterministic Simulation" Strange Loop 2014 + FoundationDB docs "Simulation and Testing" + Zhou et al. **FDB SIGMOD 2021** (the testing section formalizes the approach: seeded RNG, BUGGIFY, virtual time).
- TigerBeetle VOPR docs + "Simulation Testing for Liveness" (2023) + "Protocol-Aware DST" (2026) — seed+git-commit replay, safety/liveness modes.
- Jepsen (jepsen.io, aphyr.com "Call me maybe") — nemesis + history-checker: the operation-verification model for our soak.
- Antithesis docs — a deterministic hypervisor as reference architecture.
- Basiri et al. "Chaos Engineering" arXiv 1702.05843 + principlesofchaos.org; Netflix Simian Army.
- Fioraldi et al. AFL++ WOOT 2020 — coverage-guided exploration as a technique for enumerating protocol states.
- Alagappan et al. FAST'18 (see §4) — protocol-aware recovery checks after a fault.

## 8. Economy, randomness, sortition

**Tasks:** V1-ECO01–ECO08, V1-C04, L01–L05 continuations, V1-R02, QA-E19–E22.

**Status:** `NodeRegistry.sol` — 32-level Merkle-sum tree + delayed blockhash seed (marked producer-biasable in code); `selection.rs` — SHA-256+CBOR→rejection sampling→sparse Fisher–Yates; `risk.py` — hypergeometric model + grinding bound.

**Read:**

- **Committee sampling/risk:** Gilad et al. **Algorand** SOSP 2017 — VRF sortition + committee-size analysis; Hafid et al. IEEE Access 2019 — hypergeometric (not binomial) bounds for sharding; Serfling 1974 tail bound; joint hypergeometric (IEEE Access 2020).
- **Randomness:** Hanke-Movahedi-Williams **Dfinity consensus** arXiv 1805.04548 (threshold-relay BLS beacon); drand docs (t-of-n BLS, evmnet on BN254); arXiv 2403.09541 — last-revealer attacks on RANDAO (a direct analogue of our seed's biasability); eth2book §2.9.3; **EIP-7998** (randao_reveal→VRF — the mitigation direction); Boneh et al. **VDF** CRYPTO 2018 + Wesolowski EC 2019 + Pietrzak ITCS 2019 + survey ePrint 2018/712 + Buterin "Minimal VDF beacon" + MinRoot ePrint 2022/1626 and its cryptanalysis ePrint 2024/873 (shows the VDF path is unstable and expensive — our bounded-bias choice is reasonable for V1).
- **EVM seed specifics:** EIP-4399 (PREVRANDAO); EIP-2935/7709 + evm.codes — **the 256-block BLOCKHASH window**: the delayed seed cannot reference further back than 256 blocks (a real correctness edge case!); OWASP SCWE-084, Quantstamp and Dedaub notes on blockhash bias.
- **Postage/pricing:** Book of Swarm §3.3 + Appendix A.4 (bucket oversubscription/collision math — our issuer model); "Future-proof Storage" Appendix A.2–A.3 (redistribution game + price oracle); Swarm PriceOracle docs (target 4 copies, per-152-block adjustments); HackMD "Price discovery and price controls in Swarm" — critique: the oracle is underdetermined under a floating rate (important for SubsidyVault decay/cap logic); Roughgarden "TFM Design for Ethereum" arXiv 2012.00854 + EC'21 (DSIC/MMIC/OCA — the frame for postage pricing); Filecoin whitepaper + "Engineering Filecoin's Economy" 2020; Arweave yellow paper (the endowment model — a third comparison point).
- **Threshold sigs (a deferred option for the attester quorum):** Komlo-Goldberg **FROST** SAC 2020 + **RFC 9591** (an implementable spec); Boldyreva threshold BLS PKC 2003; BLS ASIACRYPT 2001; BGLS aggregate EC 2003; Gennaro-Goldfeder GG18/GG20 + CGGMP21 (proactive t-of-n ECDSA for long-lived committees); MuSig2 ePrint 2020/1261.

## 9. zk-postage, attestation, backup, updates

**Tasks:** V1-A05 (verifier boundary), the O series (k-of-n, providers), I03–I06 (backup anti-rollback), V1-P05 (updates), QA-E17–E19/E23–E25.

**zk/nullifiers:**

- Ben-Sasson et al. **STARK** ePrint 2018/046 + FRI ICALP 2018; RISC Zero proof-system docs + `proof-system-in-detail.pdf` + "STARK by Hand" (journal-vs-seal semantics; notably, the receipt PRF itself is HMAC-SHA-256, like our nullifier); ethSTARK ePrint 2021/582; Szepieniec "Anatomy of a STARK".
- **Steel** (Boundless/RISC Zero) — the canonical engineering reference for "an EIP-1186 proof inside a zkVM" (host preflight + guest re-verify against a trusted block hash) — exactly our `postage-zk` path; Zeth for whole-block.
- Nullifier design: Zerocash S&P 2014; **Zcash protocol spec** (Sapling nf = PRF^nf(ρ) — a precedent for an unlinkable spend marker derived from a secret inside the circuit); Tornado Cash whitepaper v1.4; **Semaphore** (nullifier = H(identity, scope) — "one action per scope" = our (seed, ticket index)); **RLN** (Vac/Waku — slashing for double-use; [verify exact spec file]).

**Backup/anti-rollback:**

- Parno et al. **Memoir** IEEE S&P 2011 — state continuity: a stale restore is an attack regardless of confidentiality (the canonical frame for "a backup must not resurrect a revoke"); Matetic et al. **ROTE** USENIX Sec 2017 — committee-based anti-rollback without a local monotonic counter (applicable to the attester quorum); Signal SVR (blog + repo + Green's 2020 critique); WhatsApp E2EE backup whitepaper + NCC Group assessment; Apple iCloud Keychain escrow (HSM, SRP, 10 attempts); Double Ratchet spec + Alwen-Coretti-Dodis EC 2019 (skipped-message-keys retention — a 1:1 analogue of our dependency-aware key GC).

**Updates:**

- Cappos et al. "A Look in the Mirror" CCS 2008 (rollback/freeze/mix-match attacks); Samuel et al. **TUF** CCS 2010 (not NDSS); TUF spec (root/targets/snapshot/timestamp, monotonic versions); Kuppusamy et al. **Uptane** escar 2016 + Uptane Standard; Kuppusamy-Diaz-Cappos **Mercury** ATC 2017 — cheap anti-rollback for repositories (relevant if updates are delivered via decentralized storage).

**Identity/onboarding (moderate):**

- RFC 7636 (PKCE), RFC 8628 (device flow), OIDC core — the standard pitfalls are already listed in the plan (alg confusion, pairwise sub, SSRF discovery). **Open problem:** pairwise-subject dedup across providers has no canonical solution; the plan honestly reduces it to bounded-risk caps (O07/ECO04).
- Metadata/groups: Chase-Perrin-Zaverucha "Signal Private Group System" CCS 2020; Hashimoto-Katsumata-Prest "Hide Metadata in MLS-like messaging" CCS 2022; Keyhive (Ink&Switch — capability-based group encryption, a counterexample without total order); Liu-Tromer **Oblivious Message Retrieval** CRYPTO 2022; Marlinspike-Perrin X3DH + Sesame + Cremers et al. USENIX Sec 2023 (formal analysis of Sesame); Megolm + Albrecht et al. DOGM IEEE S&P 2024 (history sharing conflicts with FS/PCS — methodology for "a new member without old epochs"); Unger et al. **SoK Secure Messaging** IEEE S&P 2015; Green-Miers Puncturable Encryption S&P 2015.

---

## Real research niches (no canonical solution)

1. **MLS on top of a BFT total-order log** — no formal model exists; closest are saCGKA/DCGKA/CoCoA. Our construction (finalizer sequencing + deterministic reject + rebase) is our own; worth writing up in the spec with references to Bienstock-Dodis-Rösler (the price of concurrency) and Matrix dMLS (the only engineering precedent of fork resolution).
2. **Newest-first QC-verified spent-prefix streaming** across epochs — assembled from PBFT checkpoints + weak subjectivity; no ready-made scheme found.
3. **Private rotating mailbox pointers on top of Kademlia** — a synthesis of Tor prop224 (blinded daily keys) + Coral sloppy hashing + discv5 topic ads; no ready protocol exists.
4. **Physical independence of replicas** — provably unattainable (Douceur Sybil + Fisch PoRep impossibility); the task is only to measure and bound residual risk, not to "solve" it.
5. **Cross-provider pairwise subject dedup** (Sybil on entitlement) — an open problem; only caps + measured residual risk.
6. **Biasable randomness** — even Ethereum has not closed it (RANDAO last-revealer; EIP-7998/VDF in progress); our delayed blockhash seed is correctly marked biasable — improving it is a separate decision (drand/VRF/VDF); for V1 the measured grinding bound in `risk.py` suffices.

## What is NOT a research task (for completeness)

- CRUD/UI/wire codecs, Tauri IPC, packaging, OAuth plumbing — known patterns.
- Erasure coding, gossip/anti-entropy as protocols, hole punching implementation, VRF — **deliberately absent** from V1 (see the negative scan results); references above are given in case the decision is revisited.
- `epoch != 1` is forbidden in code — until that is lifted, A02/A06/P04 remain at the design level.
