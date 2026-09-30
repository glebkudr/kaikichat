//! Optional LAN hints. Only the signed bootstrap exchange can authenticate a discovered peer.
use super::*;
use bootstrap_support::Hint;
use libp2p::{
    core::{Endpoint, transport::PortUse},
    mdns,
    swarm::{ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm},
};
use std::task::{Context, Poll};

pub(super) struct LanHints {
    own: PeerId,
    entries: HashMap<PeerId, HashMap<Multiaddr, Instant>>,
}
impl LanHints {
    pub(super) fn new(own: PeerId) -> Self {
        Self {
            own,
            entries: HashMap::new(),
        }
    }
    pub(super) fn observe(&mut self, peer: PeerId, address: Multiaddr, now: Instant) {
        self.entries.retain(|_, routes| {
            routes.retain(|_, expires| *expires > now);
            !routes.is_empty()
        });
        if peer == self.own || relay_support::is_circuit(&address) {
            return;
        }
        // libp2p mDNS translates the advertised IP to the packet's source IP.
        // Never turn loopback, wildcard, multicast or DNS hints into dial work.
        if !matches!(address.iter().next(), Some(Protocol::Ip4(ip)) if !ip.is_unspecified() && !ip.is_loopback() && !ip.is_multicast() && !ip.is_broadcast())
        {
            return;
        }
        if routes(&[address.to_string()], Some(peer), false).is_err() {
            return;
        }
        if self.entries.len() >= 32 && !self.entries.contains_key(&peer) {
            return;
        }
        let routes = self.entries.entry(peer).or_default();
        if routes.len() < 4 || routes.contains_key(&address) {
            routes.insert(address, now + Duration::from_secs(60));
        }
    }
    pub(super) fn hints(&self, now: Instant) -> Vec<Hint> {
        self.entries
            .iter()
            .filter_map(|(peer, routes)| {
                let addresses: Vec<_> = routes
                    .iter()
                    .filter(|(_, expires)| **expires > now)
                    .map(|(address, _)| address.clone())
                    .collect();
                (!addresses.is_empty()).then_some(Hint {
                    peer: *peer,
                    addresses,
                    root: None,
                })
            })
            .collect()
    }
}

pub(super) struct GuardedLan {
    inner: Option<mdns::tokio::Behaviour>,
    hints: LanHints,
}
impl GuardedLan {
    pub(super) fn new(peer: PeerId, enabled: bool) -> Result<Self> {
        let inner = if enabled {
            Some(mdns::tokio::Behaviour::new(
                mdns::Config {
                    ttl: Duration::from_secs(60),
                    query_interval: Duration::from_secs(10),
                    enable_ipv6: false,
                },
                peer,
            )?)
        } else {
            None
        };
        Ok(Self {
            inner,
            hints: LanHints::new(peer),
        })
    }
    pub(super) fn hints(&self, now: Instant) -> Vec<Hint> {
        self.hints.hints(now)
    }
}
impl NetworkBehaviour for GuardedLan {
    type ConnectionHandler = <mdns::tokio::Behaviour as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = ();
    fn handle_established_inbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: &Multiaddr,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        Ok(libp2p::swarm::dummy::ConnectionHandler)
    }
    fn handle_pending_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: Option<PeerId>,
        _: &[Multiaddr],
        _: Endpoint,
    ) -> std::result::Result<Vec<Multiaddr>, ConnectionDenied> {
        // Unsigned mDNS addresses must not implicitly extend delivery/relay/AutoNAT routes.
        Ok(vec![])
    }
    fn handle_established_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: Endpoint,
        _: PortUse,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        Ok(libp2p::swarm::dummy::ConnectionHandler)
    }
    fn on_swarm_event(&mut self, event: FromSwarm) {
        match event {
            FromSwarm::NewListenAddr(e) if relay_support::is_circuit(e.addr) => return,
            FromSwarm::ExpiredListenAddr(e) if relay_support::is_circuit(e.addr) => return,
            _ => {}
        }
        if let Some(inner) = &mut self.inner {
            inner.on_swarm_event(event);
        }
    }
    fn on_connection_handler_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        libp2p::core::util::unreachable(event);
    }
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        let Some(inner) = &mut self.inner else {
            return Poll::Pending;
        };
        for _ in 0..16 {
            match inner.poll(cx) {
                Poll::Ready(ToSwarm::GenerateEvent(mdns::Event::Discovered(values))) => {
                    let now = clock::instant();
                    for (peer, mut address) in values {
                        // Retain bounded application hints instead of upstream's unbounded TTL cache.
                        // This supported API is deprecated in0.48; revisit on an upstream upgrade.
                        #[allow(deprecated)]
                        inner.expire_node(&peer);
                        if !matches!(address.iter().last(), Some(Protocol::P2p(_))) {
                            address.push(Protocol::P2p(peer));
                        }
                        self.hints.observe(peer, address, now);
                    }
                    return Poll::Ready(ToSwarm::GenerateEvent(()));
                }
                // Deliberate local cache expiry does not revoke authenticated peers or application hints.
                // Never publish unverified addresses into another behaviour's address book.
                Poll::Ready(_) => {}
                Poll::Pending => return Poll::Pending,
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
impl Runtime {
    pub(super) fn lan_info(&self) -> Value {
        let mut peers: Vec<_> = self
            .swarm
            .behaviour()
            .lan
            .hints(clock::instant())
            .iter()
            .map(|h| h.peer.to_string())
            .collect();
        peers.sort();
        json!({"enabled":self.preferences.lan_discovery,"active":self.swarm.behaviour().lan.inner.is_some(),
            "blockedByPolicy":self.preferences.lan_discovery && self.relay_only,"peers":peers})
    }
}
