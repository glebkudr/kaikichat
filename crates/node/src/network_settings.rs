//! Owner-controlled client routing. Persist first; a fresh swarm retires old protocol handlers.
use super::*;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ConfigureNetwork {
    expected_revision: u64,
    preferences: NetworkPreferences,
}
pub(super) fn build_network(
    key: identity::Keypair,
    relay_only: bool,
    lan_discovery: bool,
    dht_server: bool,
    relay_server: &RelayServerStatus,
    nat: &NatStatus,
    access_gate: &processing::Gate,
) -> Result<Swarm<Network>> {
    let lan = GuardedLan::new(key.public().to_peer_id(), lan_discovery && !relay_only)?;
    Ok(SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            let limits = reserved_connections::Limits::new(
                if relay_server.enabled && !nat.server_enabled() {
                    1
                } else {
                    2
                },
            )
            .with_dht_server(dht_server && !relay_only);
            let budget = limits.processing();
            Network {
                relay_client,
                bootstrap: bootstrap_support::behaviour(budget.clone()),
                mailbox: mailbox_holder::behaviour(budget.clone(), access_gate.clone()),
                lan,
                routing: (!relay_only)
                    .then(|| GuardedRouting::new(key.public().to_peer_id(), dht_server))
                    .into(),
                autonat: GuardedAutonat::new(key.public().to_peer_id(), nat),
                // Identify observations feed DCUtR candidates through the swarm. They never become
                // signed application routes or confirmed external addresses merely by being observed.
                identify: identify::Behaviour::new(
                    identify::Config::new("/agentic-internet/1".into(), key.public())
                        .with_agent_version("agentic-internet/0.1".into())
                        .with_cache_size(0),
                ),
                dcutr: (!relay_only && !relay_server.enabled)
                    .then(|| dcutr::Behaviour::new(key.public().to_peer_id()))
                    .into(),
                relay_server: relay_server
                    .enabled
                    .then(|| {
                        relay::Behaviour::new(
                            key.public().to_peer_id(),
                            relay_server.configuration(),
                        )
                    })
                    .into(),
                delivery: processing::cbor(
                    budget,
                    "/agentic-internet/delivery/1",
                    MAX_FRAME,
                    MAX_FRAME,
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(5))
                        .with_max_concurrent_streams(16),
                ),
                limits,
            }
        })?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
                .with_dial_concurrency_factor(NonZeroU8::MIN.saturating_add(1))
        })
        .with_connection_timeout(Duration::from_secs(6))
        .build())
}
impl Runtime {
    pub(super) fn network_settings(&self) -> Value {
        json!({"revision":self.network_revision,"preferences":self.preferences,
            "status":{"peerId":self.swarm.local_peer_id().to_string(),
                "listeners":self.listeners,"relayRoutes":self.relay_routes(),
                "advertisedAddresses":self.advertised(),"autoNat":self.nat.info(*self.swarm.local_peer_id()),
                "connectedPeers":self.swarm.connected_peers().count(),
                "holePunchEnabled":self.swarm.behaviour().dcutr.is_enabled(),
                "bootstrap":self.bootstrap_info(),"lanDiscovery":self.lan_info(),"routing":self.routing_info(),
                "listening":!self.listeners.is_empty()}})
    }
    pub(super) fn configure_network(
        &mut self,
        input: ConfigureNetwork,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let invalid = |error: crate::NodeError| ("invalid_request", error.to_string());
        let relays =
            RelayReservation::validate(&input.preferences.relays, input.preferences.relay_only)
                .map_err(invalid)?;
        let nat = self
            .nat
            .reconfigured(
                *self.swarm.local_peer_id(),
                &input.preferences.auto_nat_peers,
            )
            .map_err(invalid)?;
        let relay_server = self.relay_server.recreated();
        let discovery = Discovery::new(
            input
                .preferences
                .bootstrap_peers
                .as_deref()
                .unwrap_or(&self.network_routes),
            *self.swarm.local_peer_id(),
        )
        .map_err(invalid)?;
        // Construction does not bind sockets or dial peers. Validation and persistence errors
        // leave all existing reservations, connections and in-flight deliveries untouched.
        let changed = input.preferences != self.preferences;
        let replacement = if changed {
            Some(
                build_network(
                    self.transport_key.clone(),
                    input.preferences.relay_only,
                    input.preferences.lan_discovery,
                    input.preferences.dht_server,
                    &relay_server,
                    &nat,
                    &self.access_gate,
                )
                .map_err(|_| ("unavailable", "Could not prepare network transport".into()))?,
            )
        } else {
            None
        };
        let saved = self
            .core
            .save_network_preferences(input.preferences, input.expected_revision)
            .map_err(|error| match error {
                CoreError::Store(agentic_store::StoreError::StateConflict) => (
                    "state_conflict",
                    "Network settings changed; reload the current settings".into(),
                ),
                CoreError::InvalidInput => {
                    ("invalid_request", "Invalid network preferences".into())
                }
                _ => ("unavailable", "Could not save network settings".into()),
            })?;
        self.network_revision = saved.revision;
        self.preferences = saved.preferences;
        if let Some(swarm) = replacement {
            // Dropping the old swarm releases sockets before rebinding their assigned ports.
            self.swarm = swarm;
            self.relays = relays;
            self.relay_only = self.preferences.relay_only;
            self.nat = nat;
            self.relay_server = relay_server;
            self.pending.clear();
            self.retries.clear();
            self.connections.clear();
            self.discovery = discovery;
            self.listeners.clear();
            self.listen_ids.clear();
            self.listen_retry = clock::instant();
            self.maintain_listeners();
        }
        Ok(self.network_settings())
    }
    /// Whether a configured listen address has no listener: only then is
    /// the retry a deadline.
    pub(super) fn listener_missing(&self) -> bool {
        (0..self.listen_specs.len())
            .any(|index| !self.listen_ids.values().any(|value| *value == index))
    }
    pub(super) fn maintain_listeners(&mut self) {
        if self.listen_retry > clock::instant() {
            return;
        }
        for (index, address) in self.listen_specs.iter().enumerate() {
            if !self.listen_ids.values().any(|value| *value == index)
                && let Ok(id) = self.swarm.listen_on(address.clone())
            {
                self.listen_ids.insert(id, index);
            }
        }
        self.listen_retry = clock::instant() + Duration::from_secs(1);
    }
}
