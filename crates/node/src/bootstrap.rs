//! Self-record exchange only. Discovery hints confer neither contact trust nor registry authority.
use super::*;
#[path = "bootstrap_schedule.rs"]
mod schedule;
pub(super) use schedule::{Admission, Hint, Schedule};
#[cfg(test)]
#[path = "bootstrap_route_tests.rs"]
mod route_tests;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Exchange {
    #[serde(with = "serde_bytes")]
    pub(super) node_record: Vec<u8>,
}
pub(super) fn behaviour(budget: processing::Budget) -> processing::Cbor<Exchange, Exchange> {
    processing::cbor(
        budget,
        "/agentic-internet/bootstrap/1",
        8192,
        8192,
        request_response::Config::default()
            .with_request_timeout(Duration::from_secs(5))
            .with_max_concurrent_streams(8),
    )
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedPeer {
    peer_id: String,
    root_id: String,
    #[serde(skip)]
    expires_at: u64,
}
pub(super) struct Discovery {
    schedule: Schedule,
    pub(super) pending: HashMap<request_response::OutboundRequestId, Hint>,
    pub(super) routing_pending: HashSet<request_response::OutboundRequestId>,
    verified: HashMap<PeerId, VerifiedPeer>,
    admission: Admission,
    refresh_due: Instant,
    /// Advertised routes carried by the last record this node sent.
    announced: Option<Vec<String>>,
    cached: usize,
    failed: u64,
    rejected: u64,
    limited: u64,
}
impl Discovery {
    pub(super) fn new(values: &[String], own: PeerId) -> Result<Self> {
        if values.len() > 4 {
            return Err("configure at most4 bootstrap hints".into());
        }
        let mut explicit = Vec::new();
        let mut peers = HashSet::new();
        for value in values {
            let Some((peer, addresses)) = routes(std::slice::from_ref(value), None, false)? else {
                continue;
            };
            if peer == own || !peers.insert(peer) {
                return Err("bootstrap hints require distinct remote PeerIDs".into());
            }
            explicit.push(Hint {
                peer,
                addresses,
                root: None,
            });
        }
        let now = clock::instant();
        Ok(Self {
            schedule: Schedule::new(explicit, now),
            pending: HashMap::new(),
            routing_pending: HashSet::new(),
            verified: HashMap::new(),
            admission: Admission::new(now),
            refresh_due: now,
            announced: None,
            cached: 0,
            failed: 0,
            rejected: 0,
            limited: 0,
        })
    }
}
impl Runtime {
    pub(super) fn bootstrap_info(&self) -> Value {
        let now = now().unwrap_or(u64::MAX);
        let peers: Vec<_> = self
            .discovery
            .verified
            .iter()
            .filter(|(peer, record)| record.expires_at > now && self.swarm.is_connected(peer))
            .map(|(_, record)| record)
            .collect();
        // The routes in use: the owner's, else the network's.
        let routes = self
            .preferences
            .bootstrap_peers
            .as_ref()
            .unwrap_or(&self.network_routes);
        json!({"state":if peers.is_empty() {"bootstrap-needed"} else {"connected"},
            "routes":routes,
            "action":if peers.is_empty() {"Provide a reachable independent peer with --bootstrap <multiaddr>, or import a current contact invitation; unavailable hints retry automatically."} else {""},
            "networkDomain":hex::encode(NETWORK_DOMAIN),"verifiedPeers":peers,
            "candidateHints":self.discovery.schedule.len(),"cachedHints":self.discovery.cached,
            "policyBlockedHints":self.discovery.schedule.blocked(self.relay_only),
            "inFlight":self.discovery.pending.len(),"failedAttempts":self.discovery.failed,
            "rejectedRecords":self.discovery.rejected,"rateLimitedRequests":self.discovery.limited})
    }
    fn admit_bootstrap(
        &mut self,
        wire: &[u8],
        peer: PeerId,
        connection: ConnectionId,
        root: Option<[u8; 32]>,
    ) -> Result<()> {
        if self.relay_only && !self.connections.get(&connection).is_some_and(|c| c.relayed) {
            return Err("bootstrap connection violates relay-only policy".into());
        }
        let now = now()?;
        let record = self.core.verify_node_record(wire, &peer.to_string(), now)?;
        routes(&record.addresses, Some(peer), true)?;
        if root.is_some_and(|root| root != record.author) {
            return Err("bootstrap cache root does not match peer response".into());
        }
        self.core
            .remember_node_record(wire, &peer.to_string(), now)?;
        if let Some(routing) = self.swarm.behaviour_mut().routing.as_mut() {
            let addresses = record
                .addresses
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            routing.remember(peer, addresses);
        }
        // Verification is retained only for live connections; bounded by transport admission.
        self.discovery.verified.insert(
            peer,
            VerifiedPeer {
                peer_id: peer.to_string(),
                root_id: network_id(&record.author),
                expires_at: record.expires_at,
            },
        );
        Ok(())
    }
    pub(super) fn bootstrap_event(&mut self, event: &SwarmEvent<NetworkEvent>) {
        if let SwarmEvent::Behaviour(NetworkEvent::Identify(libp2p::identify::Event::Received {
            peer_id,
            connection_id,
            info,
            ..
        })) = event
            && self
                .connections
                .get(connection_id)
                .is_some_and(|c| c.peer_id == peer_id.to_string())
            && let Some(routing) = self.swarm.behaviour_mut().routing.as_mut()
        {
            routing.observe_protocols(*peer_id, &info.protocols);
        }
        if let SwarmEvent::ConnectionClosed {
            peer_id,
            num_established: 0,
            ..
        } = event
            && self.discovery.verified.remove(peer_id).is_some()
        {
            self.discovery
                .schedule
                .disconnected(*peer_id, clock::instant());
        }
    }
    pub(super) fn bootstrap_message(&mut self, event: request_response::Event<Exchange, Exchange>) {
        match event {
            request_response::Event::Message {
                peer,
                connection_id,
                message,
            } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    let allowed = self.discovery.admission.allow(peer, clock::instant());
                    let node_record = if !allowed {
                        self.discovery.limited = self.discovery.limited.saturating_add(1);
                        vec![]
                    } else if self
                        .admit_bootstrap(&request.node_record, peer, connection_id, None)
                        .is_ok()
                    {
                        self.discovery.announced = Some(self.advertised());
                        now().and_then(|now| self.binding(now)).unwrap_or_default()
                    } else {
                        self.discovery.rejected = self.discovery.rejected.saturating_add(1);
                        vec![]
                    };
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .bootstrap
                        .send_response(channel, Exchange { node_record });
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    if let Some(hint) = self.discovery.pending.remove(&request_id) {
                        let success = peer == hint.peer
                            && self
                                .admit_bootstrap(
                                    &response.node_record,
                                    peer,
                                    connection_id,
                                    hint.root,
                                )
                                .is_ok();
                        if self.discovery.routing_pending.remove(&request_id) {
                            let root = success
                                .then(|| {
                                    self.discovery
                                        .verified
                                        .get(&peer)
                                        .map(|v| v.root_id.clone())
                                })
                                .flatten();
                            if let Some(r) = self.swarm.behaviour_mut().routing.as_mut() {
                                r.auth_finished(peer, root);
                            }
                        }
                        if !success {
                            self.discovery.rejected = self.discovery.rejected.saturating_add(1);
                        }
                        self.discovery
                            .schedule
                            .finished(hint.peer, success, clock::instant());
                    }
                }
            },
            request_response::Event::OutboundFailure { request_id, .. } => {
                if let Some(hint) = self.discovery.pending.remove(&request_id) {
                    if self.discovery.routing_pending.remove(&request_id)
                        && let Some(r) = self.swarm.behaviour_mut().routing.as_mut()
                    {
                        r.auth_finished(hint.peer, None);
                    }
                    self.discovery.failed = self.discovery.failed.saturating_add(1);
                    self.discovery
                        .schedule
                        .finished(hint.peer, false, clock::instant());
                }
            }
            _ => {}
        }
    }
    pub(super) fn maintain_bootstrap(&mut self) {
        let instant = clock::instant();
        if self.discovery.refresh_due <= instant {
            if let Ok(records) = now().and_then(|now| Ok(self.core.cached_node_records(now)?)) {
                let own = *self.swarm.local_peer_id();
                let hints: Vec<_> = records
                    .into_iter()
                    .filter_map(|record| {
                        let peer: PeerId = record.peer_id.parse().ok()?;
                        if peer == own {
                            return None;
                        }
                        let (_, addresses) = routes(&record.addresses, Some(peer), true).ok()??;
                        Some(Hint {
                            peer,
                            addresses,
                            root: Some(record.author),
                        })
                    })
                    .collect();
                if let Some(r) = self.swarm.behaviour_mut().routing.as_mut() {
                    for hint in &hints {
                        r.remember(hint.peer, hint.addresses.clone());
                    }
                }
                self.discovery.cached = hints.len();
                self.discovery.schedule.replace_sources(
                    hints,
                    self.swarm.behaviour().lan.hints(instant),
                    instant,
                );
            }
            self.discovery.refresh_due = instant + Duration::from_secs(5);
        }
        // A verified peer holding an older signed route (for example one sent
        // before the listeners bound after a network change) cannot route to
        // this node until the ordinary refresh. Announce the change now.
        let advertised = self.advertised();
        if self
            .discovery
            .announced
            .as_ref()
            .is_some_and(|sent| *sent != advertised)
        {
            let verified: Vec<_> = self.discovery.verified.keys().copied().collect();
            self.discovery.schedule.announce(&verified, instant);
            self.discovery.announced = Some(advertised);
        }
        let ready = self.discovery.schedule.ready(instant, self.relay_only);
        if ready.is_empty() {
            return;
        }
        let binding = now().and_then(|now| self.binding(now));
        if binding.is_ok() {
            self.discovery.announced = Some(self.advertised());
        }
        for hint in ready {
            // Explicit/cache work and routed target authentication share the same four slots.
            if self.discovery.pending.len() >= 4 {
                self.discovery.schedule.finished(hint.peer, false, instant);
                continue;
            }
            let addresses = self.prepare_peer_routes(hint.peer, hint.addresses.clone());
            if let (Ok(binding), Ok(Some(addresses))) = (&binding, addresses) {
                let id = self
                    .swarm
                    .behaviour_mut()
                    .bootstrap
                    .send_request_with_addresses(
                        &hint.peer,
                        Exchange {
                            node_record: binding.clone(),
                        },
                        addresses,
                    );
                self.discovery.pending.insert(id, hint);
            } else {
                self.discovery.failed = self.discovery.failed.saturating_add(1);
                self.discovery.schedule.finished(hint.peer, false, instant);
            }
        }
    }
}
