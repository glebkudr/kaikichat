//! Real runtimes for in-process tests, and a real connection between two.
#![allow(clippy::unwrap_used)]
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::runtime) fn runtime(directory: &std::path::Path) -> Runtime {
    let profile = directory.join("profile.db");
    let key = identity::Keypair::generate_ed25519();
    let peer = key.public().to_peer_id();
    let config = NodeConfig {
        profile: profile.clone(),
        ipc: directory.join("node.sock"),
        listen: vec![],
        public_addresses: vec![],
        bootstrap: vec![],
        lan_discovery: false,
        dht_server: false,
        relays: vec![],
        relay_only: false,
        relay_server: false,
        relay_capacity: 2,
        relay_reservation_seconds: 60,
        relay_circuit_seconds: 30,
        autonat_peers: vec![],
        autonat_server: false,
        autonat_allow_local: false,
        autonat_probe_seconds: 30,
        chain_rpc: None,
        chain_id: None,
        book_shop: None,
        grant_issuer: None,
        registry: None,
        chain_confirmations: 6,
        operator_pool: None,
        identity_server: None,
        directory: None,
        directory_key: None,
        secrets: Bootstrap {
            master_key: Zeroizing::new([31; 32]),
            owner_token: Zeroizing::new([32; 32]),
        },
    };
    let nat = NatStatus::new(peer, &config).unwrap();
    let relay_server = RelayServerStatus::new(false, 2, 60, 30);
    let core = AppCore::new(
        ProfileStore::open(&profile, &[31; 32]).unwrap(),
        NETWORK_DOMAIN,
    )
    .unwrap();
    let mailbox_holder =
        mailbox_holder::Service::open(&profile, &[31; 32], &key, NETWORK_DOMAIN).unwrap();
    let access_gate = processing::Gate::new();
    Runtime {
        core,
        profile_directory: directory.into(),
        ipc_path: config.ipc,
        swarm: build_network(
            key.clone(),
            false,
            false,
            false,
            &relay_server,
            &nat,
            &access_gate,
        )
        .unwrap(),
        transport_key: key,
        preferences: NetworkPreferences {
            relays: vec![],
            relay_only: false,
            auto_nat_peers: vec![],
            bootstrap_peers: vec![],
            lan_discovery: false,
            dht_server: false,
        },
        network_revision: 0,
        listen_specs: vec![],
        listen_ids: HashMap::new(),
        listen_retry: clock::instant(),
        listeners: BTreeSet::new(),
        public_routes: vec![],
        transports: BTreeSet::new(),
        pending: HashMap::new(),
        retries: HashMap::new(),
        rejected: 0,
        failures: 0,
        relays: vec![],
        relay_only: false,
        relay_server,
        failed_reservations: 0,
        hole_punch_successes: 0,
        hole_punch_failures: 0,
        connections: HashMap::new(),
        nat,
        discovery: Discovery::new(&[], peer).unwrap(),
        mailbox_holder,
        mailbox_client: mailbox_client::Client::new(access_gate.clone()),
        access_gate,
        chain: mailbox_chain::Lane::default(),
        payouts: mailbox_payouts::Lane::default(),
        swarm_units: Vec::new(),
        directory: mailbox_directory::Directory::default(),
        discover_cards: std::collections::BTreeMap::new(),
        discovery_service: None,
        stopping: None,
    }
}

pub(in crate::runtime) async fn connect(
    local: &mut Runtime,
    remote: &mut Runtime,
    quic: bool,
) -> ConnectionId {
    remote
        .swarm
        .listen_on(
            if quic {
                "/ip4/127.0.0.1/udp/0/quic-v1"
            } else {
                "/ip4/127.0.0.1/tcp/0"
            }
            .parse()
            .unwrap(),
        )
        .unwrap();
    let address = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = remote.swarm.select_next_some().await {
            break address;
        }
    };
    local
        .swarm
        .dial(
            DialOpts::peer_id(*remote.swarm.local_peer_id())
                .condition(libp2p::swarm::dial_opts::PeerCondition::Always)
                .addresses(vec![address])
                .build(),
        )
        .unwrap();
    loop {
        tokio::select! {
            event = local.swarm.select_next_some() => {
                let established = match &event { SwarmEvent::ConnectionEstablished { connection_id, .. } => Some(*connection_id), _ => None };
                local.event(event);
                if let Some(id) = established { return id; }
            }
            event = remote.swarm.select_next_some() => remote.event(event),
        }
    }
}

/// A chain every rig node reads from: books bought (confirmed or not yet),
/// grant rules by issuer and day, an outage switch and per-node holds. It
/// counts every read by node.
#[derive(Clone, Default)]
pub(in crate::runtime) struct FakeChain {
    state: std::sync::Arc<std::sync::Mutex<FakeState>>,
    released: std::sync::Arc<tokio::sync::Notify>,
}

#[derive(Default)]
struct FakeState {
    /// Books and whether their purchase is confirmed yet.
    books: BTreeMap<[u8; 32], (chain::BookRecord, bool)>,
    grants: Option<FakeGrants>,
    shop: Option<chain::ShopTerms>,
    /// Active registry units; unset, the registry does not answer.
    units: Option<Vec<[u8; 32]>>,
    down: bool,
    held: BTreeSet<usize>,
    /// Nodes whose RPC does not show confirmed purchases yet.
    lagging: BTreeSet<usize>,
    book_reads: Vec<(usize, [u8; 32])>,
    units_reads: Vec<usize>,
    grant_reads: Vec<(usize, agentic_mailbox_swarm::Account, u64)>,
    /// The operator pool, taking calls as `OperatorPool.sol` does.
    pool: Option<chain::PoolTerms>,
    /// Per day: the block armed for (zero if never) and the seed.
    seeds: BTreeMap<u64, (u64, Option<[u8; 32]>)>,
    registry: BTreeMap<[u8; 32], chain::RegistryUnit>,
    /// ETH for gas; an account not set holds one ETH.
    gas: BTreeMap<agentic_mailbox_swarm::Account, u128>,
    /// Per slot: the operation first paid and its paid places.
    paid: BTreeMap<[u8; 32], ([u8; 32], BTreeSet<u8>)>,
    owed: BTreeMap<u32, u128>,
    /// The pool has nothing to pay with: claims are credited, not paid.
    short: bool,
    transactions: Vec<FakeTx>,
}

/// A transaction the fake pool took.
#[derive(Clone, Debug)]
pub(in crate::runtime) struct FakeTx {
    pub(in crate::runtime) from: agentic_mailbox_swarm::Account,
    pub(in crate::runtime) to: chain::Address,
    pub(in crate::runtime) data: Vec<u8>,
}

/// `GrantIssuer` as the fake chain answers it.
#[derive(Clone)]
pub(in crate::runtime) struct FakeGrants {
    /// Days each issuer may grant: `from ≤ day < until`.
    pub(in crate::runtime) issuers: BTreeMap<agentic_mailbox_swarm::Account, (u64, u64)>,
    /// Cap in coins from each effective day on.
    pub(in crate::runtime) caps: BTreeMap<u64, u64>,
    pub(in crate::runtime) book_size: u32,
    pub(in crate::runtime) max_validity_days: u64,
}

impl FakeChain {
    fn state(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.state.lock().unwrap()
    }
    /// What node `node` reads.
    pub(in crate::runtime) fn for_node(&self, node: usize) -> std::sync::Arc<dyn chain::Chain> {
        std::sync::Arc::new(FakeNode {
            chain: self.clone(),
            node,
        })
    }
    /// A confirmed purchase.
    pub(in crate::runtime) fn buy(&self, book: [u8; 32], record: chain::BookRecord) {
        self.state().books.insert(book, (record, true));
    }
    /// A purchase still below the confirmations: reads do not see it.
    pub(in crate::runtime) fn buy_unconfirmed(&self, book: [u8; 32], record: chain::BookRecord) {
        self.state().books.insert(book, (record, false));
    }
    pub(in crate::runtime) fn confirm(&self, book: &[u8; 32]) {
        if let Some((_, confirmed)) = self.state().books.get_mut(book) {
            *confirmed = true;
        }
    }
    /// The registry's active units from now on.
    pub(in crate::runtime) fn set_units(&self, units: Vec<[u8; 32]>) {
        self.state().units = Some(units);
    }
    pub(in crate::runtime) fn set_shop(&self, shop: chain::ShopTerms) {
        self.state().shop = Some(shop);
    }
    pub(in crate::runtime) fn set_grants(&self, grants: FakeGrants) {
        self.state().grants = Some(grants);
    }
    /// Every read fails while the chain is down.
    pub(in crate::runtime) fn set_down(&self, down: bool) {
        self.state().down = down;
    }
    /// Reads by `node` wait until released.
    pub(in crate::runtime) fn hold(&self, node: usize) {
        self.state().held.insert(node);
    }
    /// Whether `node`'s reads are held: the managed-time driver lets virtual
    /// time pass them instead of waiting in real time.
    pub(in crate::runtime) fn is_held(&self, node: usize) -> bool {
        self.state().held.contains(&node)
    }
    /// `node`'s RPC lags: it shows no purchase until `catch_up`.
    pub(in crate::runtime) fn lag(&self, node: usize) {
        self.state().lagging.insert(node);
    }
    pub(in crate::runtime) fn catch_up(&self, node: usize) {
        self.state().lagging.remove(&node);
    }
    pub(in crate::runtime) fn release(&self, node: usize) {
        self.state().held.remove(&node);
        self.released.notify_waiters();
    }
    pub(in crate::runtime) fn set_pool(&self, pool: chain::PoolTerms) {
        self.state().pool = Some(pool);
    }
    /// `day`'s seed, as if captured.
    pub(in crate::runtime) fn set_seed(&self, day: u64, seed: [u8; 32]) {
        self.state().seeds.insert(day, (1, Some(seed)));
    }
    pub(in crate::runtime) fn seed_state(&self, day: u64) -> chain::SeedState {
        let (target, seed) = self.state().seeds.get(&day).copied().unwrap_or((0, None));
        chain::SeedState {
            target,
            seed,
            head: fake_head(),
        }
    }
    pub(in crate::runtime) fn set_registry(&self, units: Vec<([u8; 32], chain::RegistryUnit)>) {
        self.state().registry = units.into_iter().collect();
    }
    pub(in crate::runtime) fn set_gas(&self, account: agentic_mailbox_swarm::Account, wei: u128) {
        self.state().gas.insert(account, wei);
    }
    pub(in crate::runtime) fn set_short(&self, short: bool) {
        self.state().short = short;
    }
    /// The ticket's place is paid, as by a claim sent elsewhere.
    pub(in crate::runtime) fn pay_place(&self, claim: &chain::TicketClaim) {
        let slot =
            agentic_mailbox_swarm::stamp::ticket_id(&NETWORK_DOMAIN, &claim.book, claim.index);
        let operation = claimed_operation(claim);
        self.state()
            .paid
            .entry(slot)
            .or_insert((operation, BTreeSet::new()))
            .1
            .insert(claim.position);
    }
    /// The transactions the pool took, in order.
    pub(in crate::runtime) fn transactions(&self) -> Vec<FakeTx> {
        self.state().transactions.clone()
    }
    /// Reads of `book` node `node` started.
    pub(in crate::runtime) fn book_reads(&self, node: usize, book: &[u8; 32]) -> usize {
        self.state()
            .book_reads
            .iter()
            .filter(|(n, b)| *n == node && b == book)
            .count()
    }
    /// Registry reads node `node` started.
    pub(in crate::runtime) fn units_reads(&self, node: usize) -> usize {
        self.state()
            .units_reads
            .iter()
            .filter(|n| **n == node)
            .count()
    }
    /// Reads of `server`'s rules for `day` node `node` started.
    pub(in crate::runtime) fn grant_reads(
        &self,
        node: usize,
        server: &agentic_mailbox_swarm::Account,
        day: u64,
    ) -> usize {
        self.state()
            .grant_reads
            .iter()
            .filter(|(n, s, d)| *n == node && s == server && *d == day)
            .count()
    }
}

struct FakeNode {
    chain: FakeChain,
    node: usize,
}

impl FakeNode {
    async fn wait_if_held(&self) -> std::result::Result<(), chain::ChainError> {
        loop {
            let released = self.chain.released.notified();
            {
                let state = self.chain.state();
                if !state.held.contains(&self.node) {
                    return if state.down {
                        Err(chain::ChainError::Transport("down".into()))
                    } else {
                        Ok(())
                    };
                }
            }
            released.await;
        }
    }
}

#[async_trait::async_trait]
impl chain::Chain for FakeNode {
    async fn book(
        &self,
        book: [u8; 32],
    ) -> std::result::Result<Option<chain::BookRecord>, chain::ChainError> {
        self.chain.state().book_reads.push((self.node, book));
        self.wait_if_held().await?;
        let state = self.chain.state();
        if state.lagging.contains(&self.node) {
            return Ok(None);
        }
        Ok(state
            .books
            .get(&book)
            .filter(|(_, confirmed)| *confirmed)
            .map(|(record, _)| *record))
    }
    async fn grant_day(
        &self,
        server: agentic_mailbox_swarm::Account,
        day: u64,
    ) -> std::result::Result<chain::GrantDay, chain::ChainError> {
        self.chain
            .state()
            .grant_reads
            .push((self.node, server, day));
        self.wait_if_held().await?;
        let state = self.chain.state();
        let grants = state
            .grants
            .as_ref()
            .ok_or_else(|| chain::ChainError::Transport("no GrantIssuer".into()))?;
        Ok(chain::GrantDay {
            active: grants
                .issuers
                .get(&server)
                .is_some_and(|(from, until)| *from <= day && day < *until),
            cap_coins: grants
                .caps
                .range(..=day)
                .next_back()
                .map_or(0, |(_, cap)| *cap),
            book_size: grants.book_size,
            max_validity_days: grants.max_validity_days,
            today: clock::wall().unwrap() / 86_400,
        })
    }

    async fn shop(&self) -> std::result::Result<chain::ShopTerms, chain::ChainError> {
        self.wait_if_held().await?;
        self.chain
            .state()
            .shop
            .ok_or_else(|| chain::ChainError::Transport("no BookShop".into()))
    }

    async fn units(&self) -> std::result::Result<Vec<[u8; 32]>, chain::ChainError> {
        self.chain.state().units_reads.push(self.node);
        self.wait_if_held().await?;
        self.chain
            .state()
            .units
            .clone()
            .ok_or_else(|| chain::ChainError::Transport("no NodeRegistry".into()))
    }

    fn has_pool(&self) -> bool {
        self.chain.state().pool.is_some()
    }

    async fn pool(&self) -> std::result::Result<chain::PoolTerms, chain::ChainError> {
        self.wait_if_held().await?;
        self.chain
            .state()
            .pool
            .ok_or_else(|| chain::ChainError::Transport("no OperatorPool".into()))
    }

    async fn seed_state(
        &self,
        day: u64,
    ) -> std::result::Result<chain::SeedState, chain::ChainError> {
        self.wait_if_held().await?;
        Ok(self.chain.seed_state(day))
    }

    async fn registry_unit(
        &self,
        commitment: [u8; 32],
    ) -> std::result::Result<Option<chain::RegistryUnit>, chain::ChainError> {
        self.wait_if_held().await?;
        Ok(self.chain.state().registry.get(&commitment).copied())
    }

    async fn owed(&self, unit: u32) -> std::result::Result<u128, chain::ChainError> {
        self.wait_if_held().await?;
        Ok(self.chain.state().owed.get(&unit).copied().unwrap_or(0))
    }

    async fn gas_balance(
        &self,
        account: agentic_mailbox_swarm::Account,
    ) -> std::result::Result<u128, chain::ChainError> {
        self.wait_if_held().await?;
        Ok(self
            .chain
            .state()
            .gas
            .get(&account)
            .copied()
            .unwrap_or(1_000_000_000_000_000_000))
    }

    async fn check(
        &self,
        from: agentic_mailbox_swarm::Account,
        _to: chain::Address,
        data: Vec<u8>,
    ) -> std::result::Result<(), chain::ChainError> {
        self.wait_if_held().await?;
        fake_pool_call(&mut self.chain.state(), &from, &data, false)
    }

    async fn transact(
        &self,
        key: &agentic_mailbox_swarm::receipt::HolderKey,
        to: chain::Address,
        data: Vec<u8>,
    ) -> std::result::Result<[u8; 32], chain::ChainError> {
        self.wait_if_held().await?;
        let from = key.account();
        let mut state = self.chain.state();
        fake_pool_call(&mut state, &from, &data, false)?;
        if state
            .gas
            .get(&from)
            .copied()
            .unwrap_or(1_000_000_000_000_000_000)
            == 0
        {
            return Err(chain::ChainError::NoGas);
        }
        fake_pool_call(&mut state, &from, &data, true)?;
        state.transactions.push(FakeTx {
            from,
            to,
            data: data.clone(),
        });
        let count = state.transactions.len();
        Ok(alloy_primitives::keccak256([data, count.to_be_bytes().to_vec()].concat()).0)
    }
}

/// The identity server a rig node claims grants from: it opens a claim per
/// signed request (the same one for a repeated request), and a test signs
/// the human in (a grant) or denies the claim. It counts status reads,
/// keeps every double spend holders report, refuses those of another
/// issuer's grants, lists the revocations it makes one to a page and
/// counts every report and read.
#[derive(Clone, Default)]
pub(in crate::runtime) struct FakeIdentity {
    state: std::sync::Arc<std::sync::Mutex<FakeIdentityState>>,
}

#[derive(Default)]
struct FakeIdentityState {
    requests: Vec<agentic_grant_book::ClaimRequest>,
    claims: BTreeMap<String, identity_server::ClaimStatus>,
    polls: usize,
    down: bool,
    reports: Vec<FakeReport>,
    revocations: Vec<agentic_grant_book::GrantRevocation>,
    /// The key it grants with: reports of other issuers' grants are refused.
    issuer: Option<agentic_grant_book::Account>,
    refused: Vec<agentic_grant_book::GrantBook>,
    /// Published with the first report it accepts, as the real server
    /// revokes an identity's grants when it bans it.
    on_report: Vec<agentic_grant_book::GrantRevocation>,
    report_calls: usize,
    revocation_reads: usize,
}

/// A reported double spend: the grant and its two stamps of one slot.
pub(in crate::runtime) type FakeReport = (
    agentic_grant_book::GrantBook,
    agentic_mailbox_swarm::stamp::Stamp,
    agentic_mailbox_swarm::stamp::Stamp,
);

/// The server's claim TTL: a request older than this is refused as stale.
const FAKE_CLAIM_TTL: u64 = 900;

impl FakeIdentity {
    fn state(&self) -> std::sync::MutexGuard<'_, FakeIdentityState> {
        self.state.lock().unwrap()
    }
    pub(in crate::runtime) fn server(&self) -> std::sync::Arc<dyn identity_server::IdentityServer> {
        std::sync::Arc::new(self.clone())
    }
    /// Every distinct request the server opened a claim for.
    pub(in crate::runtime) fn requests(&self) -> Vec<agentic_grant_book::ClaimRequest> {
        self.state().requests.clone()
    }
    pub(in crate::runtime) fn polls(&self) -> usize {
        self.state().polls
    }
    /// While down, every call fails as a transport error.
    pub(in crate::runtime) fn set_down(&self, down: bool) {
        self.state().down = down;
    }
    /// The human signed in and the server granted `grant`.
    pub(in crate::runtime) fn sign_in(&self, claim_id: &str, grant: agentic_grant_book::GrantBook) {
        self.state().claims.insert(
            claim_id.into(),
            identity_server::ClaimStatus::Granted(grant),
        );
    }
    /// Every double spend reported, in order.
    pub(in crate::runtime) fn reports(&self) -> Vec<FakeReport> {
        self.state().reports.clone()
    }
    /// Revocations published when the first report is accepted.
    pub(in crate::runtime) fn revoke_on_report(
        &self,
        revocations: Vec<agentic_grant_book::GrantRevocation>,
    ) {
        self.state().on_report = revocations;
    }
    pub(in crate::runtime) fn set_issuer(&self, issuer: agentic_grant_book::Account) {
        self.state().issuer = Some(issuer);
    }
    /// Grants of reports refused as another issuer's.
    pub(in crate::runtime) fn refused(&self) -> Vec<agentic_grant_book::GrantBook> {
        self.state().refused.clone()
    }
    /// Report calls, answered or not.
    pub(in crate::runtime) fn report_calls(&self) -> usize {
        self.state().report_calls
    }
    /// Revocation pages asked for, answered or not.
    pub(in crate::runtime) fn revocation_reads(&self) -> usize {
        self.state().revocation_reads
    }
    pub(in crate::runtime) fn deny(&self, claim_id: &str, reason: &str) {
        self.state().claims.insert(
            claim_id.into(),
            identity_server::ClaimStatus::Denied(reason.into()),
        );
    }
}

#[async_trait::async_trait]
impl identity_server::IdentityServer for FakeIdentity {
    async fn create(
        &self,
        request: &agentic_grant_book::ClaimRequest,
    ) -> std::result::Result<identity_server::ClaimOpened, identity_server::IdentityError> {
        if request.verify().is_err() {
            return Err(identity_server::IdentityError::Refused(
                "bad_signature".into(),
            ));
        }
        let mut state = self.state();
        if state.down {
            return Err(identity_server::IdentityError::Transport("down".into()));
        }
        if request.created_at + FAKE_CLAIM_TTL < clock::wall().unwrap() {
            return Err(identity_server::IdentityError::Refused(
                "stale_request".into(),
            ));
        }
        let index = match state.requests.iter().position(|r| r == request) {
            Some(index) => index,
            None => {
                state.requests.push(request.clone());
                state.requests.len() - 1
            }
        };
        let claim_id = format!("claim-{index}");
        state
            .claims
            .entry(claim_id.clone())
            .or_insert(identity_server::ClaimStatus::Pending);
        Ok(identity_server::ClaimOpened {
            login_url: format!("https://id.test/v1/claims/{claim_id}/login"),
            claim_id,
            expires_at: request.created_at + FAKE_CLAIM_TTL,
        })
    }
    async fn status(
        &self,
        claim_id: &str,
    ) -> std::result::Result<identity_server::ClaimStatus, identity_server::IdentityError> {
        let mut state = self.state();
        if state.down {
            return Err(identity_server::IdentityError::Transport("down".into()));
        }
        state.polls += 1;
        state
            .claims
            .get(claim_id)
            .cloned()
            .ok_or_else(|| identity_server::IdentityError::Refused("unknown_claim".into()))
    }
    async fn report(
        &self,
        grant: &agentic_grant_book::GrantBook,
        first: &agentic_mailbox_swarm::stamp::Stamp,
        second: &agentic_mailbox_swarm::stamp::Stamp,
    ) -> std::result::Result<identity_server::Reported, identity_server::IdentityError> {
        let mut state = self.state();
        state.report_calls += 1;
        if state.down {
            return Err(identity_server::IdentityError::Transport("down".into()));
        }
        if state.issuer.is_some_and(|issuer| issuer != grant.server) {
            state.refused.push(grant.clone());
            return Err(identity_server::IdentityError::Refused("not_ours".into()));
        }
        state
            .reports
            .push((grant.clone(), first.clone(), second.clone()));
        let revoked = std::mem::take(&mut state.on_report);
        let count = revoked.len() as u64;
        state.revocations.extend(revoked);
        Ok(identity_server::Reported {
            banned: true,
            revoked: count,
        })
    }
    async fn revocations(
        &self,
        after: u64,
    ) -> std::result::Result<identity_server::RevocationPage, identity_server::IdentityError> {
        let mut state = self.state();
        state.revocation_reads += 1;
        if state.down {
            return Err(identity_server::IdentityError::Transport("down".into()));
        }
        // One to a page, so a reader must follow the cursor.
        let revocations: Vec<_> = state
            .revocations
            .iter()
            .skip(usize::try_from(after).unwrap())
            .take(1)
            .cloned()
            .collect();
        Ok(identity_server::RevocationPage {
            last: after + revocations.len() as u64,
            revocations,
        })
    }
}

/// The fake chain's head: a block every two seconds of the rig's clock.
fn fake_head() -> u64 {
    clock::wall().unwrap() / 2
}

fn fake_word(data: &[u8], at: usize) -> Option<[u8; 32]> {
    data.get(4 + at * 32..4 + at * 32 + 32)?.try_into().ok()
}

fn fake_uint(word: &[u8; 32]) -> u64 {
    u64::from_be_bytes(word[24..].try_into().unwrap())
}

/// `OperatorPool.claim`'s arguments, as the pool decodes them.
pub(in crate::runtime) fn decode_claim(data: &[u8]) -> (u32, [u8; 32], Vec<chain::TicketClaim>) {
    let word = |at| fake_word(data, at).unwrap();
    let unit = u32::try_from(fake_uint(&word(0))).unwrap();
    assert_eq!(fake_uint(&word(2)), 0x60, "tickets offset");
    let count = usize::try_from(fake_uint(&word(3))).unwrap();
    assert_eq!(data.len(), 4 + 4 * 32 + count * 19 * 32, "claim length");
    let tickets = (0..count)
        .map(|n| {
            let at = |field: usize| word(4 + n * 19 + field);
            let mut holders = [[0; 32]; 10];
            for (place, holder) in holders.iter_mut().enumerate() {
                *holder = at(5 + place);
            }
            let mut signature = [0; 65];
            signature[..32].copy_from_slice(&at(17));
            signature[32..64].copy_from_slice(&at(18));
            signature[64] = u8::try_from(fake_uint(&at(16))).unwrap();
            chain::TicketClaim {
                book: at(0),
                index: u32::try_from(fake_uint(&at(1))).unwrap(),
                mailbox: at(2),
                period: fake_uint(&at(3)),
                envelope: at(4),
                holders,
                position: u8::try_from(fake_uint(&at(15))).unwrap(),
                signature,
            }
        })
        .collect();
    (unit, word(1), tickets)
}

fn claimed_operation(claim: &chain::TicketClaim) -> [u8; 32] {
    agentic_mailbox_swarm::stamp::named_operation_of(
        &claim.mailbox,
        claim.period,
        &agentic_mailbox_swarm::stamp::swarm_digest(&claim.holders),
        &claim.envelope,
    )
}

/// The registry unit at `index`, if `from` may act for it: its owner, or
/// its node by the commitment's preimage.
fn fake_authorized(
    state: &FakeState,
    index: u32,
    transport: &[u8; 32],
    from: &agentic_mailbox_swarm::Account,
) -> Option<[u8; 32]> {
    let (commitment, unit) = state
        .registry
        .iter()
        .find(|(_, unit)| unit.index == index)?;
    (unit.owner == *from
        || agentic_mailbox_swarm::directory::unit_commitment(&NETWORK_DOMAIN, transport, from)
            == *commitment)
        .then_some(*commitment)
}

/// What `OperatorPool` does with `data` from `from`; applied only when
/// `apply`. `Err` where the contract reverts.
fn fake_pool_call(
    state: &mut FakeState,
    from: &agentic_mailbox_swarm::Account,
    data: &[u8],
    apply: bool,
) -> std::result::Result<(), chain::ChainError> {
    let refused = chain::ChainError::Reverted;
    let pool = state.pool.ok_or_else(|| refused.clone())?;
    let now = clock::wall().unwrap();
    let head = fake_head();
    let day = || {
        fake_word(data, 0)
            .map(|word| fake_uint(&word))
            .ok_or_else(|| refused.clone())
    };
    match data.get(..4) {
        Some([0xc9, 0xbc, 0x7a, 0x8c]) => {
            let day = day()?;
            let (target, seed) = state.seeds.get(&day).copied().unwrap_or((0, None));
            if now < (day + 1) * 86_400 || seed.is_some() || (target != 0 && head <= target + 256) {
                return Err(refused);
            }
            if apply {
                state.seeds.insert(day, (head + 5, None));
            }
        }
        Some([0x06, 0x28, 0x82, 0x9b]) => {
            let day = day()?;
            let (target, seed) = state.seeds.get(&day).copied().unwrap_or((0, None));
            if seed.is_some() || target == 0 || head <= target || head - target > 256 {
                return Err(refused);
            }
            if apply {
                let seed =
                    alloy_primitives::keccak256([day.to_be_bytes(), target.to_be_bytes()].concat())
                        .0;
                state.seeds.insert(day, (target, Some(seed)));
            }
        }
        Some([0xe4, 0x05, 0x93, 0xb5]) => {
            let (index, transport, tickets) = decode_claim(data);
            let own =
                fake_authorized(state, index, &transport, from).ok_or_else(|| refused.clone())?;
            let validity = state
                .shop
                .map(|shop| shop.validity)
                .ok_or_else(|| refused.clone())?;
            let mut paid = state.paid.clone();
            for claim in &tickets {
                let (record, confirmed) = state
                    .books
                    .get(&claim.book)
                    .copied()
                    .ok_or_else(|| refused.clone())?;
                let place = usize::from(claim.position);
                if !confirmed
                    || place >= 10
                    || claim.holders[place] != own
                    || claim.holders.iter().filter(|unit| **unit == own).count() != 1
                {
                    return Err(refused);
                }
                let operation = claimed_operation(claim);
                let stamp = agentic_mailbox_swarm::stamp::Stamp {
                    book: claim.book,
                    index: claim.index,
                    operation,
                    signature: claim.signature,
                    holders: None,
                };
                if claim.index >= record.count
                    || stamp.signer(&NETWORK_DOMAIN).ok() != Some(record.key)
                {
                    return Err(refused);
                }
                let bought = record.valid_until - validity;
                let seed = state
                    .seeds
                    .get(&(bought / 86_400))
                    .and_then(|(_, seed)| *seed)
                    .ok_or_else(|| refused.clone())?;
                let slot = agentic_mailbox_swarm::stamp::ticket_id(
                    &NETWORK_DOMAIN,
                    &claim.book,
                    claim.index,
                );
                if now >= bought + pool.ticket_lifetime
                    || !agentic_mailbox_swarm::payout::wins(&seed, &slot, &pool.win_threshold)
                {
                    return Err(refused);
                }
                let (first, places) = paid.entry(slot).or_insert((operation, BTreeSet::new()));
                if *first != operation || !places.insert(claim.position) {
                    return Err(refused);
                }
            }
            if apply {
                state.paid = paid;
                if state.short {
                    *state.owed.entry(index).or_default() +=
                        pool.prize_usdc * u128::try_from(tickets.len()).unwrap();
                }
            }
        }
        Some([0xf6, 0xa0, 0x81, 0x39]) => {
            let index = u32::try_from(
                fake_word(data, 0)
                    .map(|w| fake_uint(&w))
                    .ok_or_else(|| refused.clone())?,
            )
            .unwrap();
            let transport = fake_word(data, 1).ok_or_else(|| refused.clone())?;
            fake_authorized(state, index, &transport, from).ok_or_else(|| refused.clone())?;
            if apply && !state.short {
                state.owed.remove(&index);
            }
        }
        _ => return Err(refused),
    }
    Ok(())
}
