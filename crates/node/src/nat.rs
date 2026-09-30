//! Reachability is a short-lived routing fact, never an application authorization.
use super::*;
use libp2p::{autonat, core::ConnectedPoint};
#[path = "nat_behaviour.rs"]
mod behaviour;
#[path = "nat_callbacks.rs"]
mod callbacks;
pub(super) use behaviour::GuardedAutonat;
pub(super) use callbacks::CallbackLeases;

struct Probe<P> {
    id: P,
    server: PeerId,
    endpoint: Multiaddr,
    deadline: Instant,
    witness: Option<(ConnectionId, Multiaddr)>,
    response: Option<Multiaddr>,
}
pub(super) struct Reachability<P> {
    local_peer: PeerId,
    lease: Duration,
    timeout: Duration,
    pending: Option<Probe<P>>,
    public: Option<(Multiaddr, Instant)>,
    status: &'static str,
}
fn direct(mut address: Multiaddr, peer: PeerId) -> Option<Multiaddr> {
    if let Some(Protocol::P2p(id)) = address.iter().last() {
        if id != peer {
            return None;
        }
        address.pop();
    }
    supported_endpoint(&address, true).ok()?;
    Some(address)
}
fn transport(address: &Multiaddr) -> Option<(bool, u16)> {
    address.iter().find_map(|p| match p {
        Protocol::Tcp(port) => Some((false, port)),
        Protocol::Udp(port) => Some((true, port)),
        _ => None,
    })
}
impl<P: Copy + Eq> Reachability<P> {
    pub(super) fn new(local_peer: PeerId, lease: Duration, timeout: Duration) -> Self {
        Self {
            local_peer,
            lease,
            timeout,
            pending: None,
            public: None,
            status: "unknown",
        }
    }
    pub(super) fn status(&self) -> &'static str {
        self.status
    }
    pub(super) fn public_address(&self) -> Option<&Multiaddr> {
        self.public.as_ref().map(|(address, _)| address)
    }
    pub(super) fn begin(&mut self, id: P, server: PeerId, endpoint: Multiaddr, now: Instant) {
        self.pending = Some(Probe {
            id,
            server,
            endpoint,
            deadline: now + self.timeout,
            witness: None,
            response: None,
        });
    }
    pub(super) fn inbound(
        &mut self,
        server: PeerId,
        connection: ConnectionId,
        local: Multiaddr,
        remote: Multiaddr,
        now: Instant,
    ) {
        let Some(probe) = self.pending.as_mut() else {
            return;
        };
        let (Some(local), Some(remote)) = (direct(local, self.local_peer), direct(remote, server))
        else {
            return;
        };
        if probe.server == server
            && now < probe.deadline
            && transport(&remote) != transport(&probe.endpoint)
        {
            probe.witness = Some((connection, local));
        }
    }
    fn witness(&self, id: P, server: PeerId) -> Option<ConnectionId> {
        let probe = self.pending.as_ref()?;
        (probe.id == id && probe.server == server).then_some(probe.witness.as_ref()?.0)
    }
    pub(super) fn succeeded(
        &mut self,
        id: P,
        server: PeerId,
        address: Multiaddr,
        now: Instant,
    ) -> Option<ConnectionId> {
        let probe = self.pending.as_mut()?;
        if probe.id != id || probe.server != server || now >= probe.deadline {
            return None;
        }
        probe.response = Some(direct(address, self.local_peer)?);
        self.finish(now)
    }
    pub(super) fn finish(&mut self, now: Instant) -> Option<ConnectionId> {
        let probe = self.pending.as_ref()?;
        let (connection, local) = probe.witness.as_ref()?;
        let address = probe.response.as_ref()?;
        if now >= probe.deadline || transport(local) != transport(address) {
            return None;
        }
        let connection = *connection;
        self.public = Some((address.clone(), now + self.lease));
        self.status = "public";
        self.pending = None;
        Some(connection)
    }
    pub(super) fn failed(&mut self, id: P, server: PeerId, private: bool) -> bool {
        if self
            .pending
            .as_ref()
            .is_none_or(|p| p.id != id || p.server != server)
        {
            return false;
        }
        self.pending = None;
        self.public = None;
        self.status = if private { "private" } else { "unknown" };
        true
    }
    pub(super) fn expire(&mut self, now: Instant) -> Option<ConnectionId> {
        let mut close = None;
        if self.pending.as_ref().is_some_and(|p| p.deadline <= now) {
            close = self
                .pending
                .take()
                .and_then(|p| p.witness.map(|(id, _)| id));
            self.public = None;
            self.status = "unknown";
        }
        if self
            .public
            .as_ref()
            .is_some_and(|(_, deadline)| *deadline <= now)
        {
            self.public = None;
            self.status = "unknown";
        }
        close
    }
}
pub(super) struct NatStatus {
    pub(super) peers: Vec<(PeerId, Multiaddr)>,
    reach: Reachability<autonat::ProbeId>,
    published: Option<Multiaddr>,
    server: bool,
    allow_local: bool,
    interval: Duration,
    successful: u64,
    failed: u64,
    refused: u64,
    inbound: u64,
    denied: u64,
}
impl NatStatus {
    pub(super) fn new(peer: PeerId, config: &NodeConfig) -> Result<Self> {
        if !(10..=900).contains(&config.autonat_probe_seconds)
            || (config.autonat_allow_local && !config.autonat_server)
        {
            return Err(
                "AutoNAT interval must be10..900seconds; allow-local requires server mode".into(),
            );
        }
        let interval = Duration::from_secs(config.autonat_probe_seconds.into());
        Ok(Self {
            peers: provider_routes(&config.autonat_peers)?,
            reach: Reachability::new(peer, interval * 3, Duration::from_secs(8)),
            published: None,
            server: config.autonat_server,
            allow_local: config.autonat_allow_local,
            interval,
            successful: 0,
            failed: 0,
            refused: 0,
            inbound: 0,
            denied: 0,
        })
    }
    pub(super) fn server_enabled(&self) -> bool {
        self.server
    }
    pub(super) fn reconfigured(&self, peer: PeerId, peers: &[String]) -> Result<Self> {
        Ok(Self {
            peers: provider_routes(peers)?,
            reach: Reachability::new(peer, self.interval * 3, Duration::from_secs(8)),
            published: None,
            server: self.server,
            allow_local: self.allow_local,
            interval: self.interval,
            successful: 0,
            failed: 0,
            refused: 0,
            inbound: 0,
            denied: 0,
        })
    }
    pub(super) fn configuration(&self) -> autonat::Config {
        autonat::Config {
            timeout: Duration::from_secs(8),
            boot_delay: Duration::from_secs(1),
            refresh_interval: self.interval,
            retry_interval: self.interval,
            throttle_server_period: self.interval - Duration::from_secs(1),
            use_connected: false,
            confidence_max: 2,
            max_peer_addresses: 8,
            throttle_clients_global_max: if self.server { 64 } else { 0 },
            throttle_clients_peer_max: if self.server { 8 } else { 0 },
            throttle_clients_period: Duration::from_secs(60),
            only_global_ips: !self.allow_local,
        }
    }
    pub(super) fn public_route(&self, peer: PeerId) -> Option<String> {
        self.reach
            .public_address()
            .map(|a| a.clone().with(Protocol::P2p(peer)).to_string())
    }
    pub(super) fn info(&self, peer: PeerId) -> Value {
        json!({"status":self.reach.status(),"publicAddress":self.public_route(peer),
            "serverEnabled":self.server,"probeIntervalSeconds":self.interval.as_secs(),
            "configuredPeers":self.peers.iter().map(|(p, _)| p.to_string()).collect::<Vec<_>>(),
            "successfulProbes":self.successful,"failedProbes":self.failed,"refusedProbes":self.refused,
            "inboundProbes":self.inbound,"deniedProbes":self.denied})
    }
}
impl Runtime {
    pub(super) fn maintain_nat(&mut self) {
        let now = clock::instant();
        self.swarm.behaviour_mut().autonat.expire_callbacks(now);
        if self
            .nat
            .reach
            .pending
            .as_ref()
            .is_some_and(|p| p.deadline <= now)
        {
            self.nat.failed = self.nat.failed.saturating_add(1);
        }
        if let Some(id) = self.nat.reach.expire(now) {
            self.swarm.close_connection(id);
        }
        // Either the authenticated callback or its success response may arrive first.
        if let Some(id) = self.nat.reach.finish(now) {
            self.nat.successful = self.nat.successful.saturating_add(1);
            self.swarm.close_connection(id);
        }
        let current = self.nat.reach.public_address().cloned();
        if current != self.nat.published {
            if let Some(old) = self.nat.published.take() {
                self.swarm.remove_external_address(&old);
            }
            if let Some(address) = &current {
                self.swarm.add_external_address(address.clone());
            }
            self.nat.published = current;
        }
    }
    pub(super) fn nat_event(&mut self, event: &SwarmEvent<NetworkEvent>) {
        use autonat::{
            Event, InboundProbeError, InboundProbeEvent, OutboundProbeError, OutboundProbeEvent,
            ResponseError,
        };
        let now = clock::instant();
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint:
                    ConnectedPoint::Listener {
                        local_addr,
                        send_back_addr,
                    },
                ..
            } if !relay_support::is_circuit(local_addr)
                && !relay_support::is_circuit(send_back_addr) =>
            {
                self.nat.reach.inbound(
                    *peer_id,
                    *connection_id,
                    local_addr.clone(),
                    send_back_addr.clone(),
                    now,
                );
            }
            SwarmEvent::Behaviour(NetworkEvent::Identify(identify::Event::Received {
                peer_id,
                connection_id,
                info,
                ..
            })) if self.nat.peers.iter().any(|(p, _)| p == peer_id)
                && self
                    .connections
                    .get(connection_id)
                    .is_some_and(|p| !p.relayed) =>
            {
                self.swarm
                    .behaviour_mut()
                    .autonat
                    .observe(info.observed_addr.clone());
            }
            SwarmEvent::Behaviour(NetworkEvent::Autonat(Event::OutboundProbe(
                OutboundProbeEvent::Request { probe_id, peer },
            ))) => {
                if let Some((_, address)) = self.nat.peers.iter().find(|(p, _)| p == peer) {
                    self.nat.reach.begin(*probe_id, *peer, address.clone(), now);
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::Autonat(Event::OutboundProbe(
                OutboundProbeEvent::Response {
                    probe_id,
                    peer,
                    address,
                },
            ))) => {
                if let Some(id) = self
                    .nat
                    .reach
                    .succeeded(*probe_id, *peer, address.clone(), now)
                {
                    self.nat.successful = self.nat.successful.saturating_add(1);
                    self.swarm.close_connection(id);
                }
                // An unmatched success waits within the original probe deadline. It cannot
                // publish an address unless a fresh authenticated callback completes it.
            }
            SwarmEvent::Behaviour(NetworkEvent::Autonat(Event::OutboundProbe(
                OutboundProbeEvent::Error {
                    probe_id,
                    peer: Some(peer),
                    error,
                },
            ))) => {
                let witness = self.nat.reach.witness(*probe_id, *peer);
                if self.nat.reach.failed(
                    *probe_id,
                    *peer,
                    matches!(
                        error,
                        OutboundProbeError::Response(ResponseError::DialError)
                    ),
                ) {
                    self.nat.failed = self.nat.failed.saturating_add(1);
                    if matches!(
                        error,
                        OutboundProbeError::Response(ResponseError::DialRefused)
                    ) {
                        self.nat.refused = self.nat.refused.saturating_add(1);
                    }
                    if let Some(id) = witness {
                        self.swarm.close_connection(id);
                    }
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::Autonat(Event::InboundProbe(
                InboundProbeEvent::Request { .. },
            ))) => {
                self.nat.inbound = self.nat.inbound.saturating_add(1);
            }
            SwarmEvent::Behaviour(NetworkEvent::Autonat(Event::InboundProbe(
                InboundProbeEvent::Error {
                    error: InboundProbeError::Response(ResponseError::DialRefused),
                    ..
                },
            ))) => {
                self.nat.denied = self.nat.denied.saturating_add(1);
            }
            _ => {}
        }
        self.maintain_nat();
    }
}
