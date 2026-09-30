//! Optional LAN hints. Only the signed bootstrap exchange can authenticate a discovered peer.
use super::*;
use bootstrap_support::Hint;
use if_watch::{IfEvent, IpNet, Ipv4Net, tokio::IfWatcher};
use libp2p::{
    core::{Endpoint, transport::PortUse},
    mdns,
    swarm::{
        ConnectionDenied, DialError, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    },
};
use std::{
    cmp::Reverse,
    task::{Context, Poll},
};

/// Routes kept per peer: as many as one bootstrap hint carries.
const ROUTES: usize = 8;

/// Where a route leads, best first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reach {
    /// Another host on one of this host's subnets.
    Neighbor,
    /// This host: another process here, or a bridge address another host shares.
    ThisHost,
    /// Outside this host's subnets, or its subnets are unknown.
    Elsewhere,
    /// A subnet's network or broadcast address is never a host's.
    Never,
}
pub(super) fn reach(networks: &[Ipv4Net], address: &Multiaddr) -> Reach {
    let Some(Protocol::Ip4(ip)) = address.iter().next() else {
        return Reach::Never;
    };
    let within: Vec<_> = networks.iter().filter(|n| n.contains(&ip)).collect();
    if within
        .iter()
        .any(|n| n.prefix_len() <= 30 && (ip == n.network() || ip == n.broadcast()))
    {
        Reach::Never
    } else if networks.iter().any(|n| n.addr() == ip) {
        Reach::ThisHost
    } else if within.is_empty() {
        Reach::Elsewhere
    } else {
        Reach::Neighbor
    }
}

struct Route {
    address: Multiaddr,
    expires: Instant,
    failures: u8,
}

pub(super) struct LanHints {
    own: PeerId,
    networks: Vec<Ipv4Net>,
    entries: HashMap<PeerId, Vec<Route>>,
}
impl LanHints {
    pub(super) fn new(own: PeerId) -> Self {
        Self {
            own,
            networks: vec![],
            entries: HashMap::new(),
        }
    }
    pub(super) fn interface(&mut self, event: &IfEvent) {
        match event {
            IfEvent::Up(IpNet::V4(network)) if !self.networks.contains(network) => {
                self.networks.push(*network)
            }
            IfEvent::Down(IpNet::V4(network)) => self.networks.retain(|n| n != network),
            _ => {}
        }
    }
    pub(super) fn observe(&mut self, peer: PeerId, address: Multiaddr, now: Instant) {
        let networks = &self.networks;
        self.entries.retain(|_, routes| {
            routes.retain(|r| r.expires > now && reach(networks, &r.address) != Reach::Never);
            !routes.is_empty()
        });
        if peer == self.own || relay_support::is_circuit(&address) {
            return;
        }
        // libp2p mDNS translates the advertised IP to the packet's source IP:
        // a host answers from each of its interfaces, bridges included.
        // Never turn loopback, wildcard, multicast or DNS hints into dial work.
        if !matches!(address.iter().next(), Some(Protocol::Ip4(ip)) if !ip.is_unspecified() && !ip.is_loopback() && !ip.is_multicast() && !ip.is_broadcast())
        {
            return;
        }
        if routes(&[address.to_string()], Some(peer), false).is_err() {
            return;
        }
        let rank = reach(networks, &address);
        if rank == Reach::Never {
            return;
        }
        if self.entries.len() >= 32 && !self.entries.contains_key(&peer) {
            return;
        }
        let routes = self.entries.entry(peer).or_default();
        let expires = now + Duration::from_secs(60);
        if let Some(route) = routes.iter_mut().find(|r| r.address == address) {
            route.expires = expires;
            return;
        }
        if routes.len() >= ROUTES {
            // A route that leads closer, or one not refused yet, replaces the
            // worst kept one: the routes heard first cannot hold the peer.
            let Some((worst, key, _)) = routes
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    (
                        i,
                        (reach(networks, &r.address), r.failures),
                        Reverse(r.expires),
                    )
                })
                .max_by_key(|(_, key, stale)| (*key, *stale))
            else {
                return;
            };
            if (rank, 0) >= key {
                return;
            }
            routes.remove(worst);
        }
        routes.push(Route {
            address,
            expires,
            failures: 0,
        });
    }
    /// A route that refused a dial goes behind the peer's untried routes of its reach.
    pub(super) fn on_swarm_event(&mut self, event: &FromSwarm) {
        let FromSwarm::DialFailure(failure) = event else {
            return;
        };
        let (Some(peer), DialError::Transport(errors)) = (failure.peer_id, failure.error) else {
            return;
        };
        let Some(routes) = self.entries.get_mut(&peer) else {
            return;
        };
        for (address, _) in errors {
            let mut address = address.clone();
            if !matches!(address.iter().last(), Some(Protocol::P2p(_))) {
                address.push(Protocol::P2p(peer));
            }
            if let Some(route) = routes.iter_mut().find(|r| r.address == address) {
                route.failures = route.failures.saturating_add(1);
            }
        }
    }
    /// Each peer's routes, the likeliest to connect first.
    pub(super) fn hints(&self, now: Instant) -> Vec<Hint> {
        self.entries
            .iter()
            .filter_map(|(peer, routes)| {
                let mut ranked: Vec<_> = routes
                    .iter()
                    .filter(|r| r.expires > now)
                    .map(|r| ((reach(&self.networks, &r.address), r.failures), &r.address))
                    .filter(|((rank, _), _)| *rank != Reach::Never)
                    .collect();
                ranked.sort_by_key(|(key, _)| *key);
                let addresses: Vec<_> = ranked.into_iter().map(|(_, a)| a.clone()).collect();
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
    /// This host's subnets rank the routes; without them, failures still rotate routes.
    interfaces: Option<IfWatcher>,
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
            interfaces: inner.as_ref().and_then(|_| IfWatcher::new().ok()),
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
        self.hints.on_swarm_event(&event);
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
        // Interface changes first: they rank the routes discovered below.
        while let Some(interfaces) = &mut self.interfaces {
            match interfaces.poll_if_event(cx) {
                Poll::Ready(Ok(event)) => self.hints.interface(&event),
                Poll::Ready(Err(_)) => self.interfaces = None,
                Poll::Pending => break,
            }
        }
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
