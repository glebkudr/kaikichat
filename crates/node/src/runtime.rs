use crate::{Bootstrap, NETWORK_DOMAIN, Result, ipc};
use agentic_core::{AppCore, CoreError, DirectPayment, NetworkPreferences, SwarmDelivery};
use agentic_protocol::network_id;
use agentic_store::{ProfileStore, StateChange};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder, connection_limits,
    core::transport::ListenerId,
    dcutr, identify, identity,
    multiaddr::Protocol,
    noise, relay,
    request_response::{self, ProtocolSupport, cbor},
    swarm::{
        ConnectionId, NetworkBehaviour, SwarmEvent, behaviour::toggle::Toggle, dial_opts::DialOpts,
    },
    yamux,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroU8,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use zeroize::Zeroizing;
const TRANSPORT_STATE: &str = "transport/identity";
const MAX_FRAME: u64 = 131072;
const MAX_IN_FLIGHT: usize = 16;
const OUTBOX_WINDOW: usize = 1000;
#[path = "clock.rs"]
pub(crate) mod clock;
#[path = "random.rs"]
pub(crate) mod random;
#[path = "relay.rs"]
mod relay_support;
use relay_support::{PeerConnection, RelayReservation, RelayServerStatus};
#[path = "nat.rs"]
mod nat_support;
use nat_support::{GuardedAutonat, NatStatus};
#[path = "network_settings.rs"]
mod network_settings;
#[path = "processing.rs"]
mod processing;
#[path = "reserved_connections.rs"]
mod reserved_connections;
use network_settings::{ConfigureNetwork, build_network};
#[path = "bootstrap.rs"]
mod bootstrap_support;
use bootstrap_support::{Discovery, Exchange};
#[path = "lan.rs"]
mod lan_support;
use lan_support::GuardedLan;
#[path = "interfaces.rs"]
mod interfaces_support;
use interfaces_support::Interfaces;
#[path = "routing.rs"]
mod routing_support;
use routing_support::GuardedRouting;
#[path = "chain.rs"]
mod chain;
#[path = "identity.rs"]
mod identity_server;
#[path = "mailbox_access.rs"]
mod mailbox_access;
#[path = "mailbox_chain.rs"]
mod mailbox_chain;
#[path = "mailbox_client.rs"]
mod mailbox_client;
#[path = "mailbox_directory.rs"]
mod mailbox_directory;
#[path = "mailbox_discover.rs"]
mod mailbox_discover;
#[path = "mailbox_door.rs"]
mod mailbox_door;
#[path = "mailbox_grants.rs"]
mod mailbox_grants;
#[path = "mailbox_groups.rs"]
mod mailbox_groups;
#[path = "mailbox_holder.rs"]
mod mailbox_holder;
#[path = "mailbox_intro.rs"]
mod mailbox_intro;
#[path = "mailbox_notary.rs"]
mod mailbox_notary;
#[path = "mailbox_payouts.rs"]
mod mailbox_payouts;
#[path = "mailbox_proofs.rs"]
mod mailbox_proofs;
#[path = "mailbox_public.rs"]
mod mailbox_public;
#[path = "mailbox_replication.rs"]
mod mailbox_replication;
#[cfg(test)]
#[path = "reserved_connection_tests.rs"]
mod reserved_connection_tests;

pub struct NodeConfig {
    pub profile: PathBuf,
    pub ipc: PathBuf,
    pub listen: Vec<String>,
    /// Where other nodes reach this one (at most 4 IP addresses), told them
    /// instead of every address the listeners bind: a holder's public IP.
    pub public_addresses: Vec<String>,
    pub bootstrap: Vec<String>,
    pub lan_discovery: bool,
    pub dht_server: bool,
    pub relays: Vec<String>,
    pub relay_only: bool,
    pub relay_server: bool,
    pub relay_capacity: u8,
    pub relay_reservation_seconds: u16,
    pub relay_circuit_seconds: u16,
    pub autonat_peers: Vec<String>,
    pub autonat_server: bool,
    pub autonat_allow_local: bool,
    pub autonat_probe_seconds: u16,
    /// JSON-RPC endpoint books and grant rules are read from; the chain
    /// flags are given together or not at all.
    pub chain_rpc: Option<String>,
    pub chain_id: Option<u64>,
    /// `BookShop` and `GrantIssuer` addresses, 0x-hex.
    pub book_shop: Option<String>,
    pub grant_issuer: Option<String>,
    /// `NodeRegistry` address, 0x-hex.
    pub registry: Option<String>,
    pub chain_confirmations: u64,
    /// `OperatorPool` address, 0x-hex, with the chain flags.
    pub operator_pool: Option<String>,
    /// Identity server `coins_claim` asks for grants.
    pub identity_server: Option<String>,
    /// The discovery service, and the key it signs with, for the owner's
    /// CLI and window (`discover_config`).
    pub directory: Option<String>,
    pub directory_key: Option<String>,
    pub secrets: Bootstrap,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Delivery {
    #[serde(with = "serde_bytes")]
    node_record: Vec<u8>,
    /// A control message (a Welcome, receipts), or any message where no
    /// payment is taken; empty when `stamped` carries the message.
    #[serde(with = "serde_bytes")]
    envelope: Vec<u8>,
    /// An application message paid like a swarm store.
    #[serde(default)]
    stamped: Option<StampedDelivery>,
}

/// The sealed envelope a sender stores in the recipient's incoming mailbox,
/// with its stamp, delivered directly: one slot pays for both paths.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StampedDelivery {
    conversation: String,
    #[serde(with = "serde_bytes")]
    mailbox: Vec<u8>,
    period: u64,
    #[serde(with = "serde_bytes")]
    envelope: Vec<u8>,
    stamp: mailbox_holder::StampWire,
    /// The grant funding the stamp's book, if granted: the recipient checks
    /// it as a holder does. None goes on the wire as before grants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grant: Option<agentic_grant_book::GrantBook>,
}

impl From<&SwarmDelivery> for StampedDelivery {
    fn from(delivery: &SwarmDelivery) -> Self {
        Self {
            conversation: delivery.conversation_id.clone(),
            mailbox: delivery.mailbox.to_vec(),
            period: delivery.period,
            envelope: delivery.envelope.clone(),
            stamp: mailbox_holder::StampWire::from(&delivery.stamp),
            grant: None,
        }
    }
}
impl std::fmt::Debug for Delivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delivery")
            .field("record_bytes", &self.node_record.len())
            .field("envelope_bytes", &self.envelope.len())
            .finish()
    }
}
#[derive(NetworkBehaviour)]
struct Network {
    // Admission must run before stateful protocols register a connection handler. A later
    // denial has no ConnectionClosed event; request-response would retain a phantom handler.
    limits: reserved_connections::Limits,
    delivery: processing::Cbor<Delivery, Delivery>,
    bootstrap: processing::Cbor<Exchange, Exchange>,
    mailbox: processing::Cbor<mailbox_holder::Request, mailbox_holder::Response>,
    lan: GuardedLan,
    routing: Toggle<GuardedRouting>,
    relay_client: relay::client::Behaviour,
    identify: identify::Behaviour,
    dcutr: Toggle<dcutr::Behaviour>,
    relay_server: Toggle<relay::Behaviour>,
    autonat: GuardedAutonat,
}
struct Pending {
    message_id: String,
    destination: String,
    peer: PeerId,
}
struct Retry {
    attempt: u32,
    due: Instant,
}
struct Runtime {
    core: AppCore,
    profile_directory: PathBuf,
    ipc_path: PathBuf,
    swarm: Swarm<Network>,
    transport_key: identity::Keypair,
    preferences: NetworkPreferences,
    network_revision: u64,
    /// The `--bootstrap` flags: the network's routes, used while the owner
    /// names none in `preferences`.
    network_routes: Vec<String>,
    listen_specs: Vec<Multiaddr>,
    listen_ids: HashMap<ListenerId, usize>,
    listen_retry: Instant,
    listeners: BTreeSet<String>,
    /// This host's subnets and default route: they rank the listeners' routes.
    interfaces: Interfaces,
    /// The operator's public addresses as routes: told instead of the
    /// listeners.
    public_routes: Vec<String>,
    transports: BTreeSet<&'static str>,
    pending: HashMap<request_response::OutboundRequestId, Pending>,
    retries: HashMap<String, Retry>,
    rejected: u64,
    failures: u64,
    relays: Vec<RelayReservation>,
    relay_only: bool,
    relay_server: RelayServerStatus,
    failed_reservations: u64,
    hole_punch_successes: u64,
    hole_punch_failures: u64,
    connections: HashMap<ConnectionId, PeerConnection>,
    nat: NatStatus,
    discovery: Discovery,
    mailbox_holder: mailbox_holder::Service,
    mailbox_client: mailbox_client::Client,
    /// Who the mailbox protocol takes requests from (access by book).
    access_gate: processing::Gate,
    chain: mailbox_chain::Lane,
    /// An operator's prizes (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md).
    payouts: mailbox_payouts::Lane,
    /// The units core last named holders among.
    swarm_units: Vec<[u8; 32]>,
    directory: mailbox_directory::Directory,
    /// Cards for the discovery service made lately, by body, with when: a
    /// retry makes the same card, paid with the same stamps.
    discover_cards: BTreeMap<Vec<u8>, (Vec<u8>, u64)>,
    /// The discovery service the owner's CLI and window use.
    discovery_service: Option<crate::discover::DirectoryConfig>,
    /// The owner asked the daemon to stop: it exits shortly after answering.
    stopping: Option<std::time::Instant>,
}
fn now() -> Result<u64> {
    clock::wall()
}
fn keypair(store: &mut ProfileStore) -> Result<identity::Keypair> {
    if let Some(state) = store.state(TRANSPORT_STATE)? {
        return Ok(identity::Keypair::from_protobuf_encoding(&state.bytes)?);
    }
    let key = identity::Keypair::generate_ed25519();
    let bytes = Zeroizing::new(key.to_protobuf_encoding()?);
    store.commit_states(vec![StateChange {
        namespace: TRANSPORT_STATE.into(),
        expected_revision: 0,
        bytes: bytes.to_vec(),
    }])?;
    Ok(key)
}
pub async fn run(mut config: NodeConfig) -> Result<()> {
    if config.listen.is_empty() || config.listen.len() > 8 {
        return Err("provide 1..8 listen addresses".into());
    }
    if !(1..=16).contains(&config.relay_capacity) {
        return Err("relay capacity must be1..16".into());
    }
    if !(4..=3600).contains(&config.relay_reservation_seconds)
        || !(2..=120).contains(&config.relay_circuit_seconds)
    {
        return Err("relay reservation must be4..3600seconds and circuit2..120seconds".into());
    }
    let chain = chain::chain_config(&chain::ChainFlags {
        rpc: config.chain_rpc.as_deref(),
        chain_id: config.chain_id,
        book_shop: config.book_shop.as_deref(),
        grant_issuer: config.grant_issuer.as_deref(),
        registry: config.registry.as_deref(),
        operator_pool: config.operator_pool.as_deref(),
        confirmations: config.chain_confirmations,
    })?
    .map(chain::Rpc::new)
    .transpose()
    .map_err(|error| format!("chain reader: {error:?}"))?;
    let mut store = ProfileStore::open(&config.profile, &config.secrets.master_key)?;
    let key = keypair(&mut store)?;
    let mailbox_holder = mailbox_holder::Service::open(
        &config.profile,
        &config.secrets.master_key,
        &key,
        NETWORK_DOMAIN,
    )?;
    let core = AppCore::new(store, NETWORK_DOMAIN)?;
    let saved = core.network_preferences()?;
    let network_revision = saved.as_ref().map_or(0, |saved| saved.revision);
    if let Some(saved) = &saved {
        config.relays = saved.preferences.relays.clone();
        config.relay_only = saved.preferences.relay_only;
        config.autonat_peers = saved.preferences.auto_nat_peers.clone();
        config.lan_discovery = saved.preferences.lan_discovery;
        config.dht_server = saved.preferences.dht_server;
    }
    let preferences = NetworkPreferences {
        relays: config.relays.clone(),
        relay_only: config.relay_only,
        auto_nat_peers: config.autonat_peers.clone(),
        bootstrap_peers: saved.and_then(|saved| saved.preferences.bootstrap_peers),
        lan_discovery: config.lan_discovery,
        dht_server: config.dht_server,
    };
    // The `--bootstrap` flags are the network's routes: they apply while the
    // owner names none.
    let network_routes = std::mem::take(&mut config.bootstrap);
    let discovery = Discovery::new(
        preferences
            .bootstrap_peers
            .as_deref()
            .unwrap_or(&network_routes),
        key.public().to_peer_id(),
    )?;
    let public_routes = public_routes(&config.public_addresses, key.public().to_peer_id())?;
    let relays = RelayReservation::validate(&config.relays, config.relay_only)?;
    let nat = NatStatus::new(key.public().to_peer_id(), &config)?;
    drop(config.secrets.master_key);
    let relay_server = RelayServerStatus::new(
        config.relay_server,
        config.relay_capacity,
        config.relay_reservation_seconds,
        config.relay_circuit_seconds,
    );
    let access_gate = processing::Gate::new();
    let mut swarm = build_network(
        key.clone(),
        config.relay_only,
        config.lan_discovery,
        config.dht_server,
        &relay_server,
        &nat,
        &access_gate,
    )?;
    let mut listen_specs = Vec::new();
    let mut listen_ids = HashMap::new();
    for address in config.listen {
        let address: Multiaddr = address.parse()?;
        supported_endpoint(&address, false)?;
        listen_ids.insert(swarm.listen_on(address.clone())?, listen_specs.len());
        listen_specs.push(address);
    }
    // Own the SQLCipher profile lock before removing a stale socket from a crashed process.
    let (listener, _guard) = ipc::bind(&config.ipc)?;
    let (sender, mut commands) = mpsc::channel(64);
    let server = tokio::spawn(ipc::serve(
        listener,
        Arc::new(config.secrets.owner_token),
        sender,
    ));
    let mut runtime = Runtime {
        core,
        profile_directory: std::fs::canonicalize(&config.profile)?
            .parent()
            .ok_or("profile directory missing")?
            .into(),
        ipc_path: std::fs::canonicalize(&config.ipc)?,
        swarm,
        transport_key: key,
        preferences,
        network_revision,
        network_routes,
        listen_specs,
        listen_ids,
        listen_retry: clock::instant(),
        listeners: BTreeSet::new(),
        interfaces: Interfaces::default(),
        public_routes,
        transports: BTreeSet::new(),
        pending: HashMap::new(),
        retries: HashMap::new(),
        rejected: 0,
        failures: 0,
        relays,
        relay_only: config.relay_only,
        relay_server,
        failed_reservations: 0,
        hole_punch_successes: 0,
        hole_punch_failures: 0,
        connections: HashMap::new(),
        nat,
        discovery,
        mailbox_holder,
        mailbox_client: mailbox_client::Client::new(access_gate.clone()),
        access_gate,
        chain: mailbox_chain::Lane::default(),
        payouts: mailbox_payouts::Lane::default(),
        swarm_units: Vec::new(),
        directory: mailbox_directory::Directory::default(),
        discover_cards: BTreeMap::new(),
        discovery_service: config
            .directory
            .as_deref()
            .map(|url| crate::discover::DirectoryConfig::new(url, config.directory_key.as_deref()))
            .transpose()
            .map_err(|error| format!("directory: {error}"))?,
        stopping: None,
    };
    if let Some(chain) = chain {
        runtime.set_chain(Arc::new(chain));
    }
    if let Some(url) = &config.identity_server {
        let client = identity_server::IdentityClient::new(url)
            .map_err(|error| format!("identity server: {error:?}"))?;
        runtime.set_identity(Arc::new(client));
    }
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Real-execution pacing for the event-driven wakeup below; this is the
    // real-time domain, deliberately not the managed clock seam.
    let mut pumped = std::time::Instant::now();
    let mut serviced: Option<Instant> = None;
    const WAKEUP_FLOOR: Duration = Duration::from_millis(10);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interfaces = interfaces_support::Watcher::new();
    runtime.interfaces.primary = interfaces_support::default_route();
    loop {
        tokio::select! {
            Some(command)=commands.recv()=>{
                command.execute(|kind| match kind {
                    ipc::CommandKind::Owner { method, request } => runtime.command(&method, request),
                    ipc::CommandKind::Agent { proof } => runtime.agent_command(&proof),
                });
            }
            event=runtime.swarm.select_next_some()=>runtime.event(event),
            event=interfaces.next()=>{
                if runtime.interfaces.change(&event) {
                    runtime.interfaces.primary = interfaces_support::default_route();
                }
            }
            _=tick.tick()=>{
                runtime.pump();
                pumped=std::time::Instant::now();
            }
            _=tokio::signal::ctrl_c()=>break,
            _=terminate.recv()=>break,
        }
        // V1-C05 event-driven wakeup: an event or command may have made
        // scheduler work due already (a response completes a request, a
        // command queues ready work). Service it without waiting out the
        // remaining tick. A deadline only earns one extra pump: lanes can
        // legitimately hold a lapsed instant while their work is in flight
        // or blocked, so refiring for the same instant would pump after
        // every event and starve the loop (A04 freeze). A newly armed
        // deadline compares unequal and still fires promptly; anything that
        // stays blocked keeps the ordinary 100 ms tick cadence.
        // Real time: the answer to `shutdown` is written before the loop ends.
        if runtime
            .stopping
            .is_some_and(|asked| asked.elapsed() >= Duration::from_millis(200))
        {
            break;
        }
        let due = runtime.next_deadline();
        if pumped.elapsed() >= WAKEUP_FLOOR && event_wakeup_due(due, serviced, clock::instant()) {
            runtime.pump();
            pumped = std::time::Instant::now();
            serviced = due;
        }
    }
    server.abort();
    runtime.close_network().await;
    Ok(())
}
/// Whether a lapsed scheduler deadline earns an out-of-tick pump. The same
/// already-serviced instant never refires; a newly armed deadline compares
/// unequal and fires promptly once it lapses.
fn event_wakeup_due(
    next_deadline: Option<Instant>,
    serviced: Option<Instant>,
    now: Instant,
) -> bool {
    next_deadline.is_some_and(|due| due <= now && serviced != Some(due))
}
impl Runtime {
    fn desktop_network_status(&self) -> agentic_core::NetworkStatus {
        let connected_peers = self.swarm.connected_peers().count() as u32;
        agentic_core::NetworkStatus {
            connected_peers,
            state: if connected_peers > 0 {
                "online"
            } else {
                "offline"
            }
            .into(),
        }
    }
    async fn close_network(&mut self) {
        for listener in self.listen_ids.keys().copied().collect::<Vec<_>>() {
            self.swarm.remove_listener(listener);
        }
        for peer in self.swarm.connected_peers().copied().collect::<Vec<_>>() {
            let _ = self.swarm.disconnect_peer_id(peer);
        }
        // Dropping the Tokio runtime immediately can discard QUIC close packets.
        // Keep transport drivers alive for a bounded best-effort drain. Do not
        // dispatch application requests or restart listeners while shutting down.
        let _ = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
                    self.swarm.select_next_some().await
                {
                    let _ = self.swarm.disconnect_peer_id(peer_id);
                }
            }
        })
        .await;
    }

    fn agent_command(&mut self, proof: &[u8]) -> Value {
        let Ok(time) = now() else {
            return ipc::agent_failure(agentic_core::AgentFailure::unavailable());
        };
        match self.core.agent_call(proof, time) {
            Ok(result) => json!({"result":result}),
            Err(error) => ipc::agent_failure(error.agent_failure()),
        }
    }
    fn advertised(&self) -> Vec<String> {
        let mut addresses = self.relay_routes();
        if !self.relay_only {
            if !self.public_routes.is_empty() {
                addresses.extend(self.public_routes.iter().cloned());
            } else if self.nat.peers.is_empty() {
                addresses.extend(self.interfaces.by_reach(&self.listeners));
            } else if let Some(address) = self.nat.public_route(*self.swarm.local_peer_id()) {
                addresses.push(address);
            }
        }
        addresses.truncate(8);
        addresses
    }
    /// Tell others `addresses` instead of what the listeners bind.
    #[cfg(test)]
    fn set_public_addresses(&mut self, addresses: &[String]) -> Result<()> {
        self.public_routes = public_routes(addresses, *self.swarm.local_peer_id())?;
        Ok(())
    }
    fn binding(&mut self, now: u64) -> Result<Vec<u8>> {
        Ok(self.core.publish_node_record(
            &self.swarm.local_peer_id().to_string(),
            self.advertised(),
            now,
        )?)
    }
    fn receive(
        &mut self,
        delivery: &Delivery,
        peer: PeerId,
        expected_root: Option<&str>,
        now: u64,
    ) -> Result<Option<Vec<u8>>> {
        if delivery.envelope.is_empty() && delivery.stamped.is_none() {
            return Err("empty delivery".into());
        }
        let binding =
            self.core
                .verify_node_record(&delivery.node_record, &peer.to_string(), now)?;
        routes(&binding.addresses, Some(peer), true)?;
        if expected_root.is_some_and(|root| root != network_id(&binding.author)) {
            return Err("unexpected recipient root".into());
        }
        let peer = peer.to_string();
        if let Some(stamped) = &delivery.stamped {
            let at = stamped
                .period
                .saturating_mul(agentic_mailbox_swarm::address::PERIOD_SECONDS);
            // A request by id, paid for this profile's intro mailbox.
            let intro = self.core.own_intro_mailbox(at)?;
            if stamped.mailbox.as_slice() == intro.as_slice() {
                // A stranger's request is taken only with a checked stamp.
                if self.chain_configured() {
                    self.check_direct_stamp(stamped, intro, false, now)?;
                }
                let (outcome, received) = self.core.receive_intro_from(
                    &stamped.envelope,
                    &delivery.node_record,
                    &peer,
                    now,
                )?;
                self.mailbox_client.intro.count_direct(&outcome);
                return Ok(received.reply);
            }
            // A node that reads the chain checks payment like a holder; one
            // without chain flags takes messages unpaid.
            let payment = if self.chain_configured() {
                let own = self.core.swarm_mailbox(&stamped.conversation, true, at)?;
                self.check_direct_stamp(stamped, own, true, now)?
            } else {
                DirectPayment::Checked
            };
            // Core opens it only for a one-to-one contact: groups never
            // arrive this way.
            return Ok(self
                .core
                .receive_stamped_from(
                    &stamped.conversation,
                    stamped.period,
                    &stamped.envelope,
                    &delivery.node_record,
                    &peer,
                    payment,
                    now,
                )?
                .reply);
        }
        let outcome = if self.chain_configured() {
            self.core
                .receive_control_from(&delivery.envelope, &delivery.node_record, &peer, now)?
        } else {
            self.core
                .receive_from(&delivery.envelope, &delivery.node_record, &peer, now)?
        };
        Ok(outcome.reply)
    }

    /// A stamped direct delivery pays like a swarm store: for `own`, this
    /// node's incoming mailbox it is meant for, over those exact bytes, with a
    /// slot of a known book taken once. A granted book is learned as holders
    /// learn it. A `contact`'s stamp whose book cannot be read now (the
    /// chain did not answer the last read of the book, or of its grant's
    /// issuer rules) is taken unchecked: low trust.
    fn check_direct_stamp(
        &mut self,
        stamped: &StampedDelivery,
        own: [u8; 32],
        contact: bool,
        now: u64,
    ) -> Result<DirectPayment> {
        use agentic_mailbox_swarm::stamp::Stamp;
        let mailbox: [u8; 32] = stamped
            .mailbox
            .as_slice()
            .try_into()
            .map_err(|_| "malformed mailbox")?;
        if mailbox != own {
            return Err("not this node's mailbox".into());
        }
        let stamp = Stamp::try_from(&stamped.stamp).map_err(|refusal| refusal.code())?;
        if !stamp.pays_for(&mailbox, stamped.period, &stamped.envelope) {
            return Err("stamp for another operation".into());
        }
        let Some(terms) = self.mailbox_holder.book_terms(&stamp.book) else {
            let (refusal, unreadable) = match stamped
                .grant
                .as_ref()
                .filter(|grant| grant.id() == stamp.book)
            {
                Some(grant) => {
                    let pending = mailbox_holder::Refusal::GrantPending.code();
                    if let mailbox_holder::Response::Refused { code } =
                        self.offer_grant(grant.clone())
                        && code != pending
                    {
                        return Err(code.into());
                    }
                    (pending, self.grant_day_unreadable(grant.server, grant.day))
                }
                None => {
                    self.book_wanted(stamp.book, false);
                    (
                        mailbox_holder::Refusal::UnknownBook.code(),
                        self.book_unreadable(&stamp.book),
                    )
                }
            };
            if contact && unreadable {
                return Ok(DirectPayment::Unchecked);
            }
            return Err(refusal.into());
        };
        stamp
            .verify(&NETWORK_DOMAIN, &terms, now)
            .map_err(|error| error.to_string())?;
        // Take the ticket as a notary does: another operation on it is a
        // double spend, proven here and blocking the book.
        let statement = mailbox_holder::Statement::Ticket(stamp.clone());
        let noted = self
            .mailbox_holder
            .notarize(&statement, now)
            .map_err(|refusal| refusal.code())?;
        if noted.first != statement {
            if let mailbox_holder::Statement::Ticket(first) = noted.first {
                let _ = self.mailbox_holder.accept_proofs(&mailbox_holder::Proofs {
                    senders: vec![agentic_mailbox_swarm::proof::SenderEquivocation {
                        first,
                        second: stamp,
                    }],
                    ..mailbox_holder::Proofs::default()
                });
            }
            return Err("conflict: this slot paid for another message".into());
        }
        // On record with the ticket's notaries: a reuse elsewhere is proven.
        self.notarize_later(statement);
        Ok(DirectPayment::Checked)
    }
    fn event(&mut self, event: SwarmEvent<NetworkEvent>) {
        self.relay_event(&event);
        self.nat_event(&event);
        self.bootstrap_event(&event);
        match event {
            SwarmEvent::Behaviour(NetworkEvent::Bootstrap(event)) => self.bootstrap_message(event),
            SwarmEvent::Behaviour(NetworkEvent::Mailbox(event)) => self.mailbox_message(event),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
                ..
            } => {
                // Retain the assigned port while preserving a wildcard bind address.
                if let Some(index) = self.listen_ids.get(&listener_id)
                    && let Some(transport) = address.iter().nth(1)
                    && let Some(resolved) =
                        self.listen_specs[*index].replace(1, |_| Some(transport))
                {
                    self.listen_specs[*index] = resolved;
                }
                if relay_support::is_circuit(&address) {
                    return;
                }
                if self.relay_server.enabled
                    && self.nat.peers.is_empty()
                    && supported_endpoint(&address, true).is_ok()
                {
                    self.swarm.add_external_address(address.clone());
                }
                self.listeners
                    .insert(format!("{address}/p2p/{}", self.swarm.local_peer_id()));
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.swarm.remove_external_address(&address);
                self.listeners
                    .remove(&format!("{address}/p2p/{}", self.swarm.local_peer_id()));
            }
            SwarmEvent::ListenerClosed { listener_id, .. } => {
                self.listen_ids.remove(&listener_id);
                self.listen_retry = clock::instant() + Duration::from_secs(1);
            }
            SwarmEvent::ConnectionEstablished { endpoint, .. } => {
                let address = endpoint.get_remote_address();
                if endpoint.is_relayed() {
                    self.transports.insert("relay");
                } else if address.iter().any(|p| matches!(p, Protocol::QuicV1)) {
                    self.transports.insert("quic");
                } else if address.iter().any(|p| matches!(p, Protocol::Tcp(_))) {
                    self.transports.insert("tcp");
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::Dcutr(event)) => {
                if event.result.is_ok() {
                    self.hole_punch_successes = self.hole_punch_successes.saturating_add(1);
                } else {
                    self.hole_punch_failures = self.hole_punch_failures.saturating_add(1);
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::Delivery(request_response::Event::Message {
                peer,
                message,
                ..
            })) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    let result = now().and_then(|now| self.receive(&request, peer, None, now));
                    let envelope = match result {
                        Ok(reply) => reply.unwrap_or_default(),
                        Err(_) => {
                            self.rejected = self.rejected.saturating_add(1);
                            vec![]
                        }
                    };
                    let node_record = now().and_then(|now| self.binding(now)).unwrap_or_default();
                    // Failure leaves the sender's durable item pending. A duplicate regenerates its receipt.
                    let _ = self.swarm.behaviour_mut().delivery.send_response(
                        channel,
                        Delivery {
                            node_record,
                            envelope,
                            stamped: None,
                        },
                    );
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    if let Some(pending) = self.pending.remove(&request_id) {
                        let accepted = peer == pending.peer
                            && now()
                                .and_then(|now| {
                                    self.receive(&response, peer, Some(&pending.destination), now)
                                })
                                .is_ok();
                        if !accepted {
                            self.rejected = self.rejected.saturating_add(1);
                        }
                        self.retry_later(&pending.message_id);
                    }
                }
            },
            SwarmEvent::Behaviour(NetworkEvent::Delivery(
                request_response::Event::OutboundFailure { request_id, .. },
            )) => {
                if let Some(pending) = self.pending.remove(&request_id) {
                    self.retry_later(&pending.message_id);
                    self.failures = self.failures.saturating_add(1);
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::Delivery(
                request_response::Event::InboundFailure { .. },
            )) => {
                self.rejected = self.rejected.saturating_add(1);
            }
            _ => {}
        }
    }
    fn retry_later(&mut self, id: &str) {
        let attempt = self
            .retries
            .get(id)
            .map_or(0, |retry| retry.attempt.saturating_add(1));
        let millis = 500u64.saturating_mul(1u64 << attempt.min(6)).min(30000);
        self.retries.insert(
            id.into(),
            Retry {
                attempt,
                due: clock::instant() + Duration::from_millis(millis),
            },
        );
    }
    /// Scheduler deadlines of every lane: outbox retries, listener retry and
    /// the mailbox swarm's retries, polls and rounds. Read-only.
    fn deadlines(&self) -> impl Iterator<Item = Instant> + '_ {
        self.retries
            .values()
            .map(|r| Some(r.due))
            .chain([
                self.listener_missing().then_some(self.listen_retry),
                self.mailbox_client.next_due(),
                self.chain.next_due(),
                self.payouts.next_due(),
                self.directory.next_due(),
            ])
            .flatten()
    }
    /// Earliest scheduler deadline; `None` means no lane has any scheduled
    /// deadline at all.
    fn next_deadline(&self) -> Option<Instant> {
        self.deadlines().min()
    }
    /// Earliest deadline strictly after `after`: what the production tick
    /// reaches once a due deadline was serviced without effect.
    #[cfg(test)]
    fn next_deadline_after(&self, after: Instant) -> Option<Instant> {
        self.deadlines().filter(|due| *due > after).min()
    }
    /// A managed-time driver (V1-C05) advances the shared controller to this
    /// instant; with nothing scheduled it services the runtime immediately.
    #[cfg(test)]
    fn next_due(&self) -> Instant {
        self.next_deadline().unwrap_or_else(clock::instant)
    }
    fn pump(&mut self) {
        self.maintain_chain();
        self.maintain_directory();
        self.sync_swarm_units();
        self.maintain_payouts();
        self.maintain_listeners();
        self.maintain_relays();
        self.maintain_nat();
        self.maintain_bootstrap();
        self.maintain_routing();
        self.maintain_mailbox_swarm();
        let Ok(items) = self.core.outbox(OUTBOX_WINDOW) else {
            return;
        };
        let queued: HashSet<_> = items.iter().map(|item| item.message_id.as_str()).collect();
        self.retries.retain(|id, _| queued.contains(id.as_str()));
        let Ok(now) = now() else {
            return;
        };
        let Ok(binding) = self.binding(now) else {
            return;
        };
        for item in items {
            if self.pending.len() >= MAX_IN_FLIGHT {
                break;
            }
            if self
                .pending
                .values()
                .any(|p| p.message_id == item.message_id)
                || self
                    .retries
                    .get(&item.message_id)
                    .is_some_and(|r| r.due > clock::instant())
            {
                continue;
            }
            let Ok(Some((peer, addresses))) = routes(&item.addresses, None, false) else {
                continue;
            };
            if self
                .retries
                .get(&item.message_id)
                .is_some_and(|r| r.attempt > 0)
                && let Some(routing) = self.swarm.behaviour_mut().routing.as_mut()
            {
                routing.start_automatically(peer, clock::instant());
            }
            // Keep Welcome before application traffic and one queued RPC per peer.
            if self.pending.values().any(|p| p.peer == peer) {
                continue;
            }
            let addresses = match self.prepare_peer_routes(peer, addresses) {
                Ok(Some(addresses)) => addresses,
                _ => {
                    self.failures = self.failures.saturating_add(1);
                    self.retry_later(&item.message_id);
                    continue;
                }
            };
            let request_id = self
                .swarm
                .behaviour_mut()
                .delivery
                .send_request_with_addresses(
                    &peer,
                    match self.core.prepare_swarm_delivery(&item.message_id, now) {
                        // The swarm copy's stamped envelope: one slot pays
                        // for both paths.
                        Ok(paid) => Delivery {
                            node_record: binding.clone(),
                            envelope: vec![],
                            stamped: Some(StampedDelivery {
                                grant: self
                                    .core
                                    .mailbox_book_grant(&paid.stamp.book)
                                    .ok()
                                    .flatten(),
                                ..StampedDelivery::from(&paid)
                            }),
                        },
                        // A Welcome, or no book: unstamped, which only a node
                        // without payment takes for a message.
                        Err(_) => Delivery {
                            node_record: binding.clone(),
                            envelope: item.wire,
                            stamped: None,
                        },
                    },
                    addresses,
                );
            self.pending.insert(
                request_id,
                Pending {
                    message_id: item.message_id,
                    destination: item.destination,
                    peer,
                },
            );
        }
    }
    /// Shared route policy and restart-safe dialing for delivery and discovery.
    fn prepare_peer_routes(
        &mut self,
        peer: PeerId,
        mut addresses: Vec<Multiaddr>,
    ) -> Result<Option<Vec<Multiaddr>>> {
        if self.relay_only {
            addresses.retain(relay_support::is_circuit);
            let mut relayed = false;
            let mut closing_direct = false;
            let peer_id = peer.to_string();
            for (id, connection) in &self.connections {
                if connection.peer_id == peer_id {
                    if connection.relayed {
                        relayed = true;
                    } else {
                        self.swarm.close_connection(*id);
                        closing_direct = true;
                    }
                }
            }
            if closing_direct || (addresses.is_empty() && !relayed) {
                return Ok(None);
            }
        }
        if !self.swarm.is_connected(&peer) {
            addresses.retain(|address| {
                if !relay_support::is_circuit(address) {
                    return true;
                }
                let hop: Multiaddr = address
                    .iter()
                    .take_while(|p| !matches!(p, Protocol::P2pCircuit))
                    .collect();
                let Some(Protocol::P2p(provider)) = hop.iter().last() else {
                    return false;
                };
                provider_connected(&mut self.swarm, provider, hop)
            });
            if addresses.is_empty() {
                return Ok(None);
            }
            let dial = DialOpts::peer_id(peer)
                .allocate_new_port()
                .addresses(addresses.clone())
                .build();
            if let Err(error) = self.swarm.dial(dial)
                && !matches!(error, libp2p::swarm::DialError::DialPeerConditionFalse(_))
            {
                return Err(error.into());
            }
        }
        Ok(Some(addresses))
    }
    fn command(&mut self, method: &str, request: Value) -> Value {
        match self.dispatch(method, request) {
            Ok(value) => json!({"result":value}),
            Err((code, message)) => ipc::error(code, &message),
        }
    }
    fn dispatch(
        &mut self,
        method: &str,
        request: Value,
    ) -> std::result::Result<Value, (&'static str, String)> {
        fn convert(error: impl std::fmt::Display) -> (&'static str, String) {
            ("invalid_request", error.to_string())
        }
        let now = now().map_err(convert)?;
        match method {
            "lookup_peer" => self.lookup_peer(parse(request)?),
            "shutdown" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.stopping.get_or_insert_with(std::time::Instant::now);
                Ok(json!({}))
            }
            "coins_buy" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.coins_buy(now)
            }
            "coins_claim" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.coins_claim(now)
            }
            "coins_balance" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.coins_balance(now)
            }
            "operator_earnings" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.operator_earnings(now)
            }
            "operator_withdraw" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.operator_withdraw(now)
            }
            "network_settings" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                Ok(self.network_settings())
            }
            "configure_network" => {
                let input: ConfigureNetwork = parse(request)?;
                self.configure_network(input)
            }
            "provision_runtime" => {
                let input: agentic_core::ProvisionRuntimeRequest = parse(request)?;
                let provisioned =
                    self.core
                        .provision_runtime(input, now)
                        .map_err(|error| match error {
                            CoreError::Store(agentic_store::StoreError::IdempotencyConflict) => {
                                ("operation_conflict", error.to_string())
                            }
                            error => convert(error),
                        })?;
                crate::mcp::materialize(&provisioned, &self.profile_directory, &self.ipc_path)
                    .map_err(convert)
            }
            "list_runtimes" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.list_runtimes(now).map_err(convert)?)
                    .map_err(convert)
            }
            "grant_runtime" => {
                let input: agentic_core::RuntimeGrantRequest = parse(request)?;
                serde_json::to_value(self.core.grant_runtime(input, now).map_err(convert)?)
                    .map_err(convert)
            }
            "revoke_runtime" => {
                let input: RevokeRequest = parse(request)?;
                self.core
                    .revoke_runtime(input.grant_id, now)
                    .map_err(|error| match error {
                        CoreError::Unauthorized => ("unknown_grant", error.to_string()),
                        error => convert(error),
                    })?;
                Ok(Value::Null)
            }
            "inbox_unread" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.owner_inbox_unread(now).map_err(convert)?)
                    .map_err(convert)
            }
            "inbox_poll" => {
                let input: InboxPollRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .owner_inbox_poll(
                            &input.conversation_id,
                            input.limit,
                            input.lease_seconds,
                            now,
                        )
                        .map_err(inbox_error)?,
                )
                .map_err(convert)
            }
            "inbox_ack" => {
                let input: InboxAckRequest = parse(request)?;
                self.core
                    .owner_inbox_ack(&input.conversation_id, &input.lease_id, now)
                    .map_err(inbox_error)?;
                Ok(json!({"conversationId": input.conversation_id, "leaseId": input.lease_id}))
            }
            "node_info" => {
                let queued = self.core.outbox(OUTBOX_WINDOW).map_err(convert)?.len();
                Ok(
                    json!({"peerId":self.swarm.local_peer_id().to_string(),"listeners":self.listeners,"transportsUsed":self.transports,
                    "relayRoutes":self.relay_routes(),"peerConnections":self.connections.values().collect::<Vec<_>>(),
                    "connectionCapacity":self.swarm.behaviour().limits.info(),
                    "processingCapacity":self.swarm.behaviour().limits.processing().info(),
                    "accessGate":self.access_gate.info(),
                    "bootstrap":self.bootstrap_info(),"lanDiscovery":self.lan_info(),"routing":self.routing_info(),"mailboxSwarm":self.mailbox_client.info(),"mailboxHolder":self.mailbox_holder.info(),"chain":self.chain.info(),"payouts":self.payouts_info(),"directory":self.directory_info(),
                    "advertisedAddresses":self.advertised(),"autoNat":self.nat.info(*self.swarm.local_peer_id()),
                    "relayServer":self.relay_server.info(),"failedReservations":self.failed_reservations,
                    "holePunch":{"enabled":self.swarm.behaviour().dcutr.is_enabled(),"succeeded":self.hole_punch_successes,"failed":self.hole_punch_failures},
                    "inFlight":self.pending.len(),"pendingOutbox":queued,"pendingOutboxTruncated":queued==OUTBOX_WINDOW,
                    "rejectedFrames":self.rejected,"failedAttempts":self.failures}),
                )
            }
            "snapshot" => {
                let mut snapshot = self.core.snapshot().map_err(convert)?;
                snapshot.network = self.desktop_network_status();
                serde_json::to_value(snapshot).map_err(convert)
            }
            "desktop_overview" => {
                let input: DesktopOverviewRequest = parse(request)?;
                let mut overview = self
                    .core
                    .desktop_overview(input.after.as_deref())
                    .map_err(convert)?;
                overview.network = self.desktop_network_status();
                serde_json::to_value(overview).map_err(convert)
            }
            "conversation_history" => {
                let input: ConversationHistoryRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .conversation_history(&input.conversation_id, input.before.as_deref())
                        .map_err(convert)?,
                )
                .map_err(convert)
            }
            "desktop_revision" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                Ok(
                    json!({"instance":std::process::id(),"revision":self.core.desktop_revision(),"network":self.desktop_network_status()}),
                )
            }
            "create_identity" => {
                let input: IdentityRequest = parse(request)?;
                let identity =
                    self.core
                        .create_profile(&input.name)
                        .map_err(|error| match error {
                            CoreError::ProfileExists => ("profile_exists", error.to_string()),
                            error => convert(error),
                        })?;
                serde_json::to_value(identity).map_err(convert)
            }
            "create_invitation" => {
                let input: InvitationRequest = parse(request)?;
                let addresses = input.addresses.unwrap_or_else(|| self.advertised());
                if addresses.is_empty() {
                    return Err(("network_unavailable", "No confirmed route is available; wait for reachability verification or a relay reservation".into()));
                }
                routes(&addresses, Some(*self.swarm.local_peer_id()), false).map_err(convert)?;
                Ok(json!(
                    self.core
                        .create_invitation(now, addresses)
                        .map_err(convert)?
                ))
            }
            "add_contact" => {
                let input: ContactRequest = parse(request)?;
                let addresses = self
                    .core
                    .invitation_addresses(&input.invitation, now)
                    .map_err(convert)?;
                routes(&addresses, None, false).map_err(convert)?;
                serde_json::to_value(
                    self.core
                        .add_contact(&input.name, &input.invitation, now)
                        .map_err(convert)?,
                )
                .map_err(convert)
            }
            "intro_policy" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.intro_policy().map_err(convert)?).map_err(convert)
            }
            "set_intro_policy" => {
                let policy: agentic_core::IntroPolicy = parse(request)?;
                self.core.set_intro_policy(policy).map_err(convert)?;
                serde_json::to_value(self.core.intro_policy().map_err(convert)?).map_err(convert)
            }
            "intro_requests" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.intro_requests().map_err(convert)?).map_err(convert)
            }
            "accept_intro_request" => {
                let input: IntroRequestId = parse(request)?;
                let conversation = self
                    .core
                    .accept_intro_request(&input.request_id, now)
                    .map_err(intro_request_error)?;
                Ok(json!({"conversationId": conversation.id, "name": conversation.title}))
            }
            "reject_intro_request" => {
                let input: IntroRequestId = parse(request)?;
                self.core
                    .reject_intro_request(&input.request_id)
                    .map_err(intro_request_error)?;
                Ok(json!({}))
            }
            "create_group" => {
                let input: CreateGroupRequest = parse(request)?;
                let channel = match input.kind.as_deref() {
                    None | Some("group") => false,
                    Some("channel") => true,
                    Some(_) => {
                        return Err(("invalid_request", "kind is group or channel".into()));
                    }
                };
                let access = access_of(input.access.as_deref())?;
                self.create_group(
                    &input.name,
                    &input.members,
                    channel,
                    access,
                    &input.operation_id,
                    now,
                )
            }
            "change_group" => {
                let input: ChangeGroupRequest = parse(request)?;
                let access = access_of(input.access.as_deref())?;
                let retention = match &input.retention {
                    None => None,
                    Some(Value::String(forever)) if forever == "forever" => {
                        Some(agentic_protocol::group::Retention::Forever)
                    }
                    Some(days) => match days.as_u64().and_then(|d| u32::try_from(d).ok()) {
                        Some(days)
                            if agentic_protocol::group::Retention::CHOICES
                                .contains(&agentic_protocol::group::Retention::Days(days)) =>
                        {
                            Some(agentic_protocol::group::Retention::Days(days))
                        }
                        _ => {
                            return Err((
                                "invalid_request",
                                "retention is 30, 90, 180, 365 or \"forever\"".into(),
                            ));
                        }
                    },
                };
                self.change_group(
                    &input.group_id,
                    &input.add,
                    agentic_core::GroupChange {
                        remove: input.remove,
                        admins: input.admins,
                        ban: input.ban,
                        unban: input.unban,
                        access,
                        retention,
                        unsubscribe: input.unsubscribe,
                        reseed: input.reseed,
                        ..agentic_core::GroupChange::default()
                    },
                    &input.operation_id,
                    now,
                )
            }
            "join_group" => {
                let input: JoinGroupRequest = parse(request)?;
                self.join_group_by_door(
                    &input.group_ref,
                    input.note.as_deref().unwrap_or_default(),
                    &input.operation_id,
                    now,
                )
            }
            "door_requests" => {
                let input: GroupRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .door_requests(&input.group_id)
                        .map_err(mailbox_groups::group_error)?,
                )
                .map_err(convert)
            }
            "door_decide" => {
                let input: DoorDecideRequest = parse(request)?;
                // One another admin answered, or none: nothing to decide.
                let waiting = self
                    .core
                    .door_requests(&input.group_id)
                    .map_err(mailbox_groups::group_error)?;
                if !waiting.iter().any(|r| r.request_id == input.request_id) {
                    return Err(("unknown_request", "no such application waits".into()));
                }
                self.core
                    .decide_door_request(&input.group_id, &input.request_id, input.accept, now)
                    .map_err(mailbox_groups::group_error)?;
                Ok(json!({}))
            }
            "follow_group" => {
                let input: FollowRequest = parse(request)?;
                let group: [u8; 32] = hex::decode(&input.group)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or(("invalid_request", "group is a 32-byte hex reference".into()))?;
                serde_json::to_value(
                    self.core
                        .follow_group(group, &input.owner, &input.name, now)
                        .map_err(|e| ("invalid_request", e.to_string()))?,
                )
                .map_err(convert)
            }
            "unfollow_group" => {
                let input: GroupRequest = parse(request)?;
                self.core
                    .unfollow_group(&input.group_id)
                    .map_err(mailbox_groups::group_error)?;
                Ok(json!({}))
            }
            "channel_subscribe" => {
                let input: SubscribeRequest = parse(request)?;
                self.channel_subscribe(&input.group_id, &input.members, &input.operation_id, now)
            }
            "channel_storage" => {
                let input: GroupRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .channel_storage(&input.group_id, now)
                        .map_err(mailbox_groups::group_error)?,
                )
                .map_err(convert)
            }
            "follows" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.follows().map_err(convert)?).map_err(convert)
            }
            "redeem_stamps" => self.redeem_stamps(parse(request)?),
            "book_status" => self.book_status(parse(request)?),
            "discover_consent" => self.discover_consent(parse(request)?, now),
            "discover_stamps" => self.discover_stamps(parse(request)?, now),
            "discover_card" => self.discover_card(parse(request)?, now),
            "discover_withdrawal" => self.discover_withdrawal(parse(request)?, now),
            "discover_pass" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                self.discover_pass(now)
            }
            "discover_config" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                let service = self.discovery_service.as_ref().ok_or((
                    "directory_not_configured",
                    "This node knows no discovery service: start it with --directory".into(),
                ))?;
                Ok(json!({"url": service.url, "key": service.key.map(hex::encode)}))
            }
            "groups" => {
                if request != json!({}) {
                    return Err(("invalid_request", "Expected empty request".into()));
                }
                serde_json::to_value(self.core.groups().map_err(mailbox_groups::group_error)?)
                    .map_err(convert)
            }
            "group" => {
                let input: GroupRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .group(&input.group_id)
                        .map_err(mailbox_groups::group_error)?,
                )
                .map_err(convert)
            }
            "request_contact" => {
                let input: ContactByIdRequest = parse(request)?;
                self.request_contact(&input.network_id, &input.name, &input.operation_id, now)
            }
            "send_message" => {
                let input: MessageRequest = parse(request)?;
                serde_json::to_value(
                    self.core
                        .send_message(
                            &input.conversation_id,
                            &input.text,
                            &input.operation_id,
                            now,
                        )
                        .map_err(convert)?,
                )
                .map_err(convert)
            }
            _ => Err(("unknown_method", "Unknown owner method".into())),
        }
    }
}
fn parse<T: DeserializeOwned>(value: Value) -> std::result::Result<T, (&'static str, String)> {
    serde_json::from_value(value).map_err(|_| ("invalid_request", "Invalid command fields".into()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateGroupRequest {
    name: String,
    members: Vec<String>,
    /// `group` (the default) or `channel`.
    #[serde(default)]
    kind: Option<String>,
    /// A channel's: `public`, `request` or `private` (the default).
    #[serde(default)]
    access: Option<String>,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JoinGroupRequest {
    /// `G` in hex.
    group_ref: String,
    #[serde(default)]
    note: Option<String>,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DoorDecideRequest {
    group_id: String,
    request_id: String,
    accept: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChangeGroupRequest {
    group_id: String,
    #[serde(default)]
    add: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
    #[serde(default)]
    admins: Option<Vec<String>>,
    /// `public`, `request` or `private`: who reads the group (the owner's
    /// only).
    #[serde(default)]
    access: Option<String>,
    #[serde(default)]
    ban: Vec<String>,
    #[serde(default)]
    unban: Vec<String>,
    /// A channel's: days (30, 90, 180, 365) or `"forever"`.
    #[serde(default)]
    retention: Option<Value>,
    /// A closed channel's subscribers to remove, by network id.
    #[serde(default)]
    unsubscribe: Vec<String>,
    /// Give a closed channel's subscribers keys of a new seed.
    #[serde(default)]
    reseed: bool,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubscribeRequest {
    group_id: String,
    /// Network ids given the channel's keys.
    members: Vec<String>,
    operation_id: String,
}

/// `access` of an IPC request.
fn access_of(
    access: Option<&str>,
) -> std::result::Result<Option<agentic_protocol::group::Access>, (&'static str, String)> {
    use agentic_protocol::group::Access;
    match access {
        None => Ok(None),
        Some("public") => Ok(Some(Access::Public)),
        Some("request") => Ok(Some(Access::Request)),
        Some("private") => Ok(Some(Access::Private)),
        Some(_) => Err((
            "invalid_request",
            "access is public, request or private".into(),
        )),
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FollowRequest {
    /// `G` in hex.
    group: String,
    /// The owner's network id.
    owner: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GroupRequest {
    group_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IntroRequestId {
    request_id: String,
}
/// A waiting request that is not there (or whose card's keys are gone).
fn intro_request_error(error: CoreError) -> (&'static str, String) {
    let code = match error {
        CoreError::InvalidInput | CoreError::InvalidInvitation => "unknown_request",
        _ => "invalid_request",
    };
    (code, error.to_string())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContactByIdRequest {
    network_id: String,
    name: String,
    operation_id: String,
}
fn inbox_error(error: CoreError) -> (&'static str, String) {
    let code = match error {
        CoreError::InboxLeaseExpired => "inbox_lease_expired",
        CoreError::UnknownConversation => "unknown_contact",
        _ => "invalid_request",
    };
    (code, error.to_string())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InboxPollRequest {
    conversation_id: String,
    limit: usize,
    lease_seconds: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InboxAckRequest {
    conversation_id: String,
    lease_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevokeRequest {
    grant_id: [u8; 32],
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DesktopOverviewRequest {
    after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConversationHistoryRequest {
    conversation_id: String,
    before: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityRequest {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationRequest {
    #[serde(default)]
    addresses: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactRequest {
    name: String,
    invitation: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MessageRequest {
    conversation_id: String,
    text: String,
    operation_id: String,
}
/// A direct endpoint, also used to validate the first hop of circuit routes.
fn supported_endpoint(address: &Multiaddr, remote: bool) -> Result<()> {
    let mut protocols = address.iter();
    match protocols.next() {
        Some(Protocol::Ip4(ip)) if !remote || !ip.is_unspecified() => {}
        Some(Protocol::Ip6(ip)) if !remote || !ip.is_unspecified() => {}
        _ => return Err("route requires an IP address".into()),
    }
    match protocols.next() {
        Some(Protocol::Tcp(port)) if !remote || port != 0 => {}
        Some(Protocol::Udp(port)) if !remote || port != 0 => {
            if !matches!(protocols.next(), Some(Protocol::QuicV1)) {
                return Err("UDP requires QUIC v1".into());
            }
        }
        _ => return Err("unsupported route transport or port".into()),
    }
    if protocols.next().is_some() {
        return Err("unexpected route component".into());
    }
    Ok(())
}
/// The routes of the public addresses an operator names: at most 4 IP
/// addresses others dial, without a peer id.
fn public_routes(addresses: &[String], own: PeerId) -> Result<Vec<String>> {
    if addresses.len() > 4 {
        return Err("name at most 4 public addresses".into());
    }
    addresses
        .iter()
        .map(|value| {
            let address: Multiaddr = value.parse()?;
            supported_endpoint(&address, true)
                .map_err(|error| format!("public address {value}: {error}"))?;
            Ok(address.with(Protocol::P2p(own)).to_string())
        })
        .collect()
}
/// Ordinary first-hop connections do not need the listener-port reuse reserved for DCUtR.
fn provider_connected(swarm: &mut Swarm<Network>, peer: PeerId, address: Multiaddr) -> bool {
    if swarm.is_connected(&peer) {
        return true;
    }
    let _ = swarm.dial(
        DialOpts::peer_id(peer)
            .allocate_new_port()
            .addresses(vec![address])
            .build(),
    );
    false
}
/// Shared validation for explicitly configured relay and reachability providers.
fn provider_routes(values: &[String]) -> Result<Vec<(PeerId, Multiaddr)>> {
    if values.len() > 4 {
        return Err("configure at most4 independent providers".into());
    }
    let mut peers = HashSet::new();
    let mut providers = Vec::new();
    for value in values {
        if value.len() > 256 {
            return Err("provider address too long".into());
        }
        let address: Multiaddr = value.parse()?;
        let mut endpoint = address.clone();
        let Some(Protocol::P2p(peer)) = endpoint.pop() else {
            return Err("provider requires a final peer identity".into());
        };
        supported_endpoint(&endpoint, true)?;
        if !peers.insert(peer) {
            return Err("configure distinct provider peers".into());
        }
        providers.push((peer, address));
    }
    Ok(providers)
}
fn routes(
    values: &[String],
    expected: Option<PeerId>,
    allow_empty: bool,
) -> Result<Option<(PeerId, Vec<Multiaddr>)>> {
    if values.len() > 8 || (!allow_empty && values.is_empty()) {
        return Err("provide 1..8 routes".into());
    }
    let mut peer = expected;
    let mut routes = vec![];
    for value in values {
        if value.len() > 256 {
            return Err(CoreError::InvalidInput.into());
        }
        let mut address: Multiaddr = value.parse()?;
        let Some(Protocol::P2p(id)) = address.pop() else {
            return Err("route requires a final /p2p/ identity".into());
        };
        if peer.is_some_and(|expected| expected != id) {
            return Err("route PeerIDs do not match".into());
        }
        peer = Some(id);
        let mut endpoint = address.clone();
        if matches!(endpoint.iter().last(), Some(Protocol::P2pCircuit)) {
            endpoint.pop();
            if !matches!(endpoint.pop(), Some(Protocol::P2p(_))) {
                return Err("circuit route requires a relay peer identity".into());
            }
        }
        supported_endpoint(&endpoint, true)?;
        routes.push(address.with(Protocol::P2p(id)));
    }
    Ok(peer.map(|peer| (peer, routes)))
}

#[cfg(test)]
#[path = "event_wakeup_tests.rs"]
mod event_wakeup_tests;
#[cfg(test)]
#[path = "reproducer_tests.rs"]
mod reproducer_tests;
#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "nat_tests.rs"]
mod nat_tests;

#[cfg(test)]
#[path = "bootstrap_limits_tests.rs"]
mod bootstrap_limits_tests;

#[cfg(test)]
#[path = "lan_tests.rs"]
mod lan_tests;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;

#[cfg(test)]
#[path = "record_route_tests.rs"]
mod record_route_tests;
