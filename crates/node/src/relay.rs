//! Bounded Circuit Relay v2 lifecycle. Application messages still use the shared outbox reducer.
use super::*;
use libp2p::core::transport::ListenerId;
use std::sync::Mutex;

pub(super) fn is_circuit(address: &Multiaddr) -> bool {
    address.iter().any(|p| matches!(p, Protocol::P2pCircuit))
}
pub(super) struct RelayReservation {
    address: Multiaddr,
    peer: PeerId,
    listener: Option<ListenerId>,
    confirmed: bool,
    attempt: u32,
    due: Instant,
}
impl RelayReservation {
    pub(super) fn validate(values: &[String], relay_only: bool) -> Result<Vec<Self>> {
        if values.len() > 4 || (relay_only && values.is_empty()) {
            return Err("provide1..4 relays for relay-only mode".into());
        }
        let mut reservations = vec![];
        for (peer, address) in provider_routes(values)? {
            reservations.push(Self {
                address,
                peer,
                listener: None,
                confirmed: false,
                attempt: 0,
                due: clock::instant(),
            });
        }
        Ok(reservations)
    }
    fn failed(&mut self) {
        self.listener = None;
        self.confirmed = false;
        let delay = 500u64.saturating_mul(1 << self.attempt.min(6)).min(30000);
        self.attempt = self.attempt.saturating_add(1);
        self.due = clock::instant() + Duration::from_millis(delay);
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PeerConnection {
    pub(super) peer_id: String,
    remote_address: String,
    local_address: Option<String>,
    pub(super) relayed: bool,
}
pub(super) struct RelayServerStatus {
    pub(super) enabled: bool,
    capacity: u8,
    reservations: HashMap<PeerId, u64>,
    admissions: Arc<Mutex<HashSet<PeerId>>>,
    reservation_seconds: u16,
    circuit_seconds: u16,
    expired_reservations: u64,
    timed_out_circuits: u64,
    byte_limited_circuits: u64,
    denied_reservations: u64,
    accepted_circuits: u64,
    active_circuits: u64,
}
impl RelayServerStatus {
    pub(super) fn new(
        enabled: bool,
        capacity: u8,
        reservation_seconds: u16,
        circuit_seconds: u16,
    ) -> Self {
        Self {
            enabled,
            capacity,
            reservations: HashMap::new(),
            admissions: Arc::new(Mutex::new(HashSet::new())),
            reservation_seconds,
            circuit_seconds,
            expired_reservations: 0,
            timed_out_circuits: 0,
            byte_limited_circuits: 0,
            denied_reservations: 0,
            accepted_circuits: 0,
            active_circuits: 0,
        }
    }
    pub(super) fn recreated(&self) -> Self {
        Self::new(
            self.enabled,
            self.capacity,
            self.reservation_seconds,
            self.circuit_seconds,
        )
    }
    pub(super) fn configuration(&self) -> relay::Config {
        let capacity = usize::from(self.capacity);
        let admissions = self.admissions.clone();
        let mut config = relay::Config {
            // Upstream0.21.1 checks its ceiling for renewals too. The admission hook
            // enforces the actual capacity (including pending accepts), leaving renewals room.
            max_reservations: capacity + 1,
            max_reservations_per_peer: 1,
            reservation_duration: Duration::from_secs(self.reservation_seconds.into()),
            max_circuits: 32,
            max_circuits_per_peer: 4,
            max_circuit_duration: Duration::from_secs(self.circuit_seconds.into()),
            max_circuit_bytes: 1024 * 1024,
            ..Default::default()
        };
        // Last: upstream rate denial must not create a pending admission.
        config.reservation_rate_limiters.push(Box::new(
            move |peer: PeerId, _: &Multiaddr, _: Instant| {
                let Ok(mut peers) = admissions.lock() else {
                    return false;
                };
                if peers.contains(&peer) {
                    return true;
                }
                if peers.len() >= capacity {
                    return false;
                }
                peers.insert(peer);
                true
            },
        ));
        config
    }
    fn release_admission(&self, peer: &PeerId) {
        if let Ok(mut peers) = self.admissions.lock() {
            peers.remove(peer);
        }
    }
    pub(super) fn info(&self) -> Value {
        let peers: Vec<_> = self
            .reservations
            .iter()
            .map(|(peer, renewals)| json!({"peerId":peer.to_string(),"renewals":renewals}))
            .collect();
        json!({"enabled":self.enabled,"capacity":self.capacity,
            "activeReservations":self.reservations.len(),"deniedReservations":self.denied_reservations,
            "acceptedCircuits":self.accepted_circuits,"activeCircuits":self.active_circuits,
            "reservationSeconds":self.reservation_seconds,"circuitSeconds":self.circuit_seconds,"circuitBytes":1024*1024,"maxCircuits":32,
            "reservationPeers":peers,"expiredReservations":self.expired_reservations,"timedOutCircuits":self.timed_out_circuits,"byteLimitedCircuits":self.byte_limited_circuits})
    }
    #[allow(deprecated)] // Upstream still emits ReservationReqAcceptFailed; release pending capacity.
    fn event(&mut self, event: &relay::Event) {
        match event {
            relay::Event::ReservationReqAccepted {
                src_peer_id,
                renewed,
            } => {
                let renewals = self.reservations.entry(*src_peer_id).or_default();
                if *renewed {
                    *renewals = renewals.saturating_add(1);
                }
            }
            relay::Event::ReservationClosed { src_peer_id }
            | relay::Event::ReservationTimedOut { src_peer_id } => {
                self.reservations.remove(src_peer_id);
                self.release_admission(src_peer_id);
                if matches!(event, relay::Event::ReservationTimedOut { .. }) {
                    self.expired_reservations = self.expired_reservations.saturating_add(1);
                }
            }
            relay::Event::ReservationReqDenied { .. } => {
                self.denied_reservations = self.denied_reservations.saturating_add(1);
            }
            relay::Event::ReservationReqAcceptFailed { src_peer_id, .. } => {
                if !self.reservations.contains_key(src_peer_id) {
                    self.release_admission(src_peer_id);
                }
            }
            relay::Event::CircuitReqAccepted { .. } => {
                self.accepted_circuits = self.accepted_circuits.saturating_add(1);
                self.active_circuits = self.active_circuits.saturating_add(1);
            }
            relay::Event::CircuitClosed { error, .. } => {
                self.active_circuits = self.active_circuits.saturating_sub(1);
                if let Some(error) = error {
                    if error.kind() == std::io::ErrorKind::TimedOut {
                        self.timed_out_circuits = self.timed_out_circuits.saturating_add(1);
                    } else if error.to_string() == "Max circuit bytes reached." {
                        // Pinned libp2p-relay0.21.1 emits this dedicated CopyFuture quota error.
                        self.byte_limited_circuits = self.byte_limited_circuits.saturating_add(1);
                    }
                }
            }
            _ => {}
        }
    }
}
impl Runtime {
    pub(super) fn relay_routes(&self) -> Vec<String> {
        self.relays
            .iter()
            .filter(|r| r.confirmed)
            .map(|r| {
                r.address
                    .clone()
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(*self.swarm.local_peer_id()))
                    .to_string()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub(super) fn maintain_relays(&mut self) {
        for reservation in &mut self.relays {
            if reservation.due > clock::instant() || reservation.confirmed {
                continue;
            }
            if let Some(listener) = reservation.listener {
                // A stalled provider cannot retain an unbounded pending reservation.
                self.swarm.remove_listener(listener);
                continue;
            }
            if !provider_connected(
                &mut self.swarm,
                reservation.peer,
                reservation.address.clone(),
            ) {
                reservation.due = clock::instant() + Duration::from_millis(500);
                continue;
            }
            let address = reservation.address.clone().with(Protocol::P2pCircuit);
            match self.swarm.listen_on(address) {
                Ok(listener) => {
                    reservation.listener = Some(listener);
                    reservation.due = clock::instant() + Duration::from_secs(15);
                }
                Err(_) => {
                    reservation.failed();
                    self.failed_reservations = self.failed_reservations.saturating_add(1);
                }
            }
        }
    }
    pub(super) fn relay_event(&mut self, event: &SwarmEvent<NetworkEvent>) {
        match event {
            SwarmEvent::Behaviour(NetworkEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
            )) => {
                if let Some(reservation) = self
                    .relays
                    .iter_mut()
                    .find(|r| r.peer == *relay_peer_id && r.listener.is_some())
                {
                    reservation.confirmed = true;
                    reservation.attempt = 0;
                }
            }
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                ..
            } => {
                for address in addresses {
                    self.listeners
                        .remove(&format!("{address}/p2p/{}", self.swarm.local_peer_id()));
                    self.swarm.remove_external_address(address);
                }
                if let Some(reservation) = self
                    .relays
                    .iter_mut()
                    .find(|r| r.listener == Some(*listener_id))
                {
                    reservation.failed();
                    self.failed_reservations = self.failed_reservations.saturating_add(1);
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            } => {
                self.connections.insert(
                    *connection_id,
                    PeerConnection {
                        peer_id: peer_id.to_string(),
                        remote_address: endpoint.get_remote_address().to_string(),
                        local_address: match endpoint {
                            libp2p::core::ConnectedPoint::Listener { local_addr, .. } => {
                                Some(local_addr.to_string())
                            }
                            libp2p::core::ConnectedPoint::Dialer { .. } => None,
                        },
                        relayed: endpoint.is_relayed(),
                    },
                );
            }
            SwarmEvent::ConnectionClosed {
                connection_id,
                peer_id,
                endpoint,
                ..
            } => {
                self.connections.remove(connection_id);
                if !endpoint.is_relayed()
                    && !self
                        .connections
                        .values()
                        .any(|p| p.peer_id == peer_id.to_string() && !p.relayed)
                {
                    // A combined AutoNAT server also opens temporary callbacks. Only the last
                    // direct connection's closure can release an otherwise pending admission.
                    self.relay_server.release_admission(peer_id);
                    self.relay_server.reservations.remove(peer_id);
                }
            }
            SwarmEvent::Behaviour(NetworkEvent::RelayServer(event)) => {
                self.relay_server.event(event)
            }
            _ => {}
        }
    }
}
