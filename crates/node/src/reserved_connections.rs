//! Finite selected capacity alongside the owner's bounded ordinary class. No vote authority.
use super::*;
use libp2p::{
    core::{Endpoint, transport::PortUse},
    swarm::{
        CloseConnection, ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    },
};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

const ORDINARY: usize = 64;
const DHT_SERVER_ORDINARY: usize = 128;
const RESERVED: usize = 64;
const ESTABLISHED: u32 = 192;
const PENDING_ORDINARY: usize = 16;
const PENDING: u32 = 48;

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "selected peers return with the phase 2 directory: holders reserve capacity for their swarms"
    )
)]
#[derive(Clone)]
pub(super) struct Reservation {
    peer: PeerId,
    permit: Arc<AtomicBool>,
    request: Option<Arc<AtomicBool>>,
    not_before: u64,
    expires: u64,
}
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "selected peers return with the phase 2 directory: holders reserve capacity for their swarms"
    )
)]
impl Reservation {
    pub fn new(
        peer: PeerId,
        permit: Arc<AtomicBool>,
        not_before: u64,
        expires: u64,
    ) -> Result<Self> {
        if expires <= not_before || expires - not_before > 60 {
            return Err("invalid selected connection lease".into());
        }
        Ok(Self {
            peer,
            permit,
            request: None,
            not_before,
            expires,
        })
    }
    pub fn client(
        peer: PeerId,
        authority: Arc<AtomicBool>,
        request: Arc<AtomicBool>,
        not_before: u64,
        expires: u64,
    ) -> Result<Self> {
        let mut grant = Self::new(peer, authority, not_before, expires)?;
        grant.request = Some(request);
        Ok(grant)
    }
    pub(super) fn live(&self, time: u64) -> bool {
        time >= self.not_before
            && time < self.expires
            && self.permit.load(Ordering::Acquire)
            && self
                .request
                .as_ref()
                .is_none_or(|r| r.load(Ordering::Acquire))
    }
}

struct LiveConnection {
    id: ConnectionId,
    peer: PeerId,
    reserved: bool,
    ordinary_order: u64,
}
pub(super) type Catalog = Arc<RwLock<BTreeMap<PeerId, Vec<Reservation>>>>;
pub(super) struct Limits {
    inner: connection_limits::Behaviour,
    ordinary_limit: usize,
    grants: Catalog,
    processing: processing::Budget,
    // Order of entering the ordinary class, including demotion. A client admitted
    // while selected sockets were reserved must survive their later revocation.
    established: Vec<LiveConnection>,
    order: u64,
    pending: HashMap<ConnectionId, Option<PeerId>>,
    closing: HashSet<ConnectionId>,
}
impl Limits {
    pub fn new(per_peer: u32) -> Self {
        let grants = Catalog::default();
        Self {
            inner: connection_limits::Behaviour::new(
                connection_limits::ConnectionLimits::default()
                    .with_max_pending_incoming(Some(32))
                    .with_max_pending_outgoing(Some(PENDING))
                    .with_max_established(Some(ESTABLISHED))
                    .with_max_established_per_peer(Some(per_peer)),
            ),
            ordinary_limit: ORDINARY,
            processing: processing::Budget::new(grants.clone()),
            grants,
            established: Vec::new(),
            order: 0,
            pending: HashMap::new(),
            closing: HashSet::new(),
        }
    }
    /// This is a local owner choice, never a remote peer's advertised protocol role.
    pub fn with_dht_server(mut self, enabled: bool) -> Self {
        self.ordinary_limit = if enabled {
            DHT_SERVER_ORDINARY
        } else {
            ORDINARY
        };
        self
    }
    pub fn processing(&self) -> processing::Budget {
        self.processing.clone()
    }
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "selected peers return with the phase 2 directory: holders reserve capacity for their swarms"
        )
    )]
    pub fn replace(&mut self, reservations: Vec<Reservation>) -> Result<()> {
        if reservations.len() > 12 * RESERVED {
            return Err("too many selected connection grants".into());
        }
        let mut next = BTreeMap::<PeerId, Vec<Reservation>>::new();
        for grant in reservations {
            next.entry(grant.peer).or_default().push(grant);
        }
        if next.len() > RESERVED {
            return Err("too many selected connection peers".into());
        }
        *self.grants.write().unwrap_or_else(|e| e.into_inner()) = next;
        self.refresh_classes();
        Ok(())
    }
    fn reserved(&self) -> BTreeSet<PeerId> {
        let Ok(time) = now() else {
            return BTreeSet::new();
        };
        self.grants
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(_, grants)| grants.iter().any(|g| g.live(time)))
            .map(|(p, _)| *p)
            .collect()
    }
    fn ordinary(&self, reserved: &BTreeSet<PeerId>) -> usize {
        self.established
            .iter()
            .filter(|c| !reserved.contains(&c.peer))
            .count()
    }
    fn refresh_classes(&mut self) -> BTreeSet<PeerId> {
        let reserved = self.reserved();
        for c in &mut self.established {
            let current = reserved.contains(&c.peer);
            if c.reserved && !current {
                self.order = self.order.saturating_add(1);
                c.ordinary_order = self.order;
            }
            c.reserved = current;
        }
        reserved
    }
    fn admit(&mut self, peer: PeerId) -> std::result::Result<(), ConnectionDenied> {
        let reserved = self.refresh_classes();
        if !reserved.contains(&peer) && self.ordinary(&reserved) >= self.ordinary_limit {
            return Err(ConnectionDenied::new(std::io::Error::other(
                "ordinary connection limit reached",
            )));
        }
        Ok(())
    }
    pub fn info(&self) -> Value {
        let reserved = self.reserved();
        let ordinary = self.ordinary(&reserved);
        json!({"established":self.established.len(),"ordinaryEstablished":ordinary,
            "reservedEstablished":self.established.len()-ordinary,
            "reservedPeers":reserved.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "ordinaryLimit":self.ordinary_limit,"establishedLimit":ESTABLISHED,"reservedPeerLimit":RESERVED,
            "pendingOrdinaryLimit":PENDING_ORDINARY,"pendingLimit":PENDING})
    }
}
impl NetworkBehaviour for Limits {
    type ConnectionHandler = libp2p::swarm::dummy::ConnectionHandler;
    type ToSwarm = Infallible;
    fn handle_pending_inbound_connection(
        &mut self,
        id: ConnectionId,
        local: &Multiaddr,
        remote: &Multiaddr,
    ) -> std::result::Result<(), ConnectionDenied> {
        self.inner
            .handle_pending_inbound_connection(id, local, remote)
    }
    fn handle_established_inbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        local: &Multiaddr,
        remote: &Multiaddr,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        let handler = self
            .inner
            .handle_established_inbound_connection(id, peer, local, remote)?;
        self.admit(peer)?;
        Ok(handler)
    }
    fn handle_pending_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: Option<PeerId>,
        addresses: &[Multiaddr],
        role: Endpoint,
    ) -> std::result::Result<Vec<Multiaddr>, ConnectionDenied> {
        let reserved = self.reserved();
        if !peer.is_some_and(|p| reserved.contains(&p))
            && self
                .pending
                .values()
                .filter(|p| !p.is_some_and(|p| reserved.contains(&p)))
                .count()
                >= PENDING_ORDINARY
        {
            return Err(ConnectionDenied::new(std::io::Error::other(
                "ordinary pending connection limit reached",
            )));
        }
        let addresses = self
            .inner
            .handle_pending_outbound_connection(id, peer, addresses, role)?;
        self.pending.insert(id, peer);
        Ok(addresses)
    }
    fn handle_established_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        address: &Multiaddr,
        role: Endpoint,
        port: PortUse,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        self.pending.remove(&id);
        let handler = self
            .inner
            .handle_established_outbound_connection(id, peer, address, role, port)?;
        self.admit(peer)?;
        Ok(handler)
    }
    fn on_swarm_event(&mut self, event: FromSwarm) {
        self.inner.on_swarm_event(event);
        match event {
            FromSwarm::ConnectionEstablished(e) => {
                self.pending.remove(&e.connection_id);
                self.order = self.order.saturating_add(1);
                self.established.push(LiveConnection {
                    id: e.connection_id,
                    peer: e.peer_id,
                    reserved: self.reserved().contains(&e.peer_id),
                    ordinary_order: self.order,
                });
            }
            FromSwarm::ConnectionClosed(e) => {
                self.established.retain(|c| c.id != e.connection_id);
                self.closing.remove(&e.connection_id);
            }
            FromSwarm::DialFailure(e) => {
                self.pending.remove(&e.connection_id);
            }
            _ => {}
        }
    }
    fn on_connection_handler_event(
        &mut self,
        peer: PeerId,
        id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        self.inner.on_connection_handler_event(peer, id, event);
    }
    fn poll(&mut self, _: &mut Context<'_>) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        // The runtime pumps every100ms. Re-read shared fences and wall time even
        // without incoming frames. A close stays occupied until the actual event.
        let reserved = self.refresh_classes();
        let excess = self.ordinary(&reserved).saturating_sub(self.ordinary_limit);
        let mut ordinary = self
            .established
            .iter()
            .filter(|c| !reserved.contains(&c.peer))
            .collect::<Vec<_>>();
        ordinary.sort_by_key(|c| std::cmp::Reverse(c.ordinary_order));
        let next = if excess > 0 {
            ordinary
                .iter()
                .take(excess)
                .find(|c| !self.closing.contains(&c.id))
                .map(|c| (c.id, c.peer))
        } else if ordinary.len() == self.ordinary_limit {
            // Cold reconnect may establish both directions to one peer. At the
            // ordinary ceiling those siblings must not permanently exclude
            // distinct peers. Keep the oldest non-closing socket of every peer;
            // selected sockets and physical accounting remain unchanged.
            let mut surviving = BTreeSet::new();
            ordinary
                .iter()
                .rev()
                .filter(|c| !self.closing.contains(&c.id))
                .find(|c| !surviving.insert(c.peer))
                .map(|c| (c.id, c.peer))
        } else {
            None
        };
        if let Some((id, peer_id)) = next {
            self.closing.insert(id);
            return Poll::Ready(ToSwarm::CloseConnection {
                peer_id,
                connection: CloseConnection::One(id),
            });
        }
        Poll::Pending
    }
}
