//! Contain upstream AutoNAT candidate growth and require application-verified publication.
use super::*;
use libp2p::{
    core::{Endpoint, transport::PortUse},
    swarm::{
        CloseConnection, ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    },
};
use std::task::{Context, Poll};
pub(in crate::runtime) struct GuardedAutonat {
    inner: autonat::Behaviour,
    candidates: HashSet<Multiaddr>,
    requested_callbacks: HashSet<PeerId>,
    callbacks: CallbackLeases,
    closing: std::collections::VecDeque<(PeerId, ConnectionId)>,
}
impl GuardedAutonat {
    pub(in crate::runtime) fn new(peer: PeerId, config: &NatStatus) -> Self {
        let configuration = config.configuration();
        let callbacks = CallbackLeases::new(configuration.timeout);
        let mut inner = autonat::Behaviour::new(peer, configuration);
        for (server, address) in &config.peers {
            inner.add_server(*server, Some(address.clone()));
        }
        Self {
            inner,
            candidates: HashSet::new(),
            requested_callbacks: HashSet::new(),
            callbacks,
            closing: std::collections::VecDeque::new(),
        }
    }
    pub(in crate::runtime) fn observe(&mut self, address: Multiaddr) {
        // Only the runtime's explicitly configured verifier observations reach this method.
        // Upstream0.15.0 cannot remove other_candidates; cap distinct additions per process.
        if self.candidates.len() < 8
            && supported_endpoint(&address, true).is_ok()
            && self.candidates.insert(address.clone())
        {
            self.inner.probe_address(address);
        }
    }
    pub(in crate::runtime) fn expire_callbacks(&mut self, now: Instant) {
        self.closing.extend(self.callbacks.expired(now));
    }
}
impl NetworkBehaviour for GuardedAutonat {
    type ConnectionHandler = <autonat::Behaviour as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = autonat::Event;
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
        self.inner
            .handle_established_inbound_connection(id, peer, local, remote)
    }
    fn handle_pending_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: Option<PeerId>,
        addresses: &[Multiaddr],
        role: Endpoint,
    ) -> std::result::Result<Vec<Multiaddr>, ConnectionDenied> {
        self.inner
            .handle_pending_outbound_connection(id, peer, addresses, role)
    }
    fn handle_established_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        address: &Multiaddr,
        role: Endpoint,
        port: PortUse,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        self.inner
            .handle_established_outbound_connection(id, peer, address, role, port)
    }
    fn on_swarm_event(&mut self, event: FromSwarm) {
        // A service dial can complete after its request timed out. Own precisely those
        // callback connections; client-side cleanup alone cannot handle late arrivals.
        match event {
            FromSwarm::ConnectionEstablished(e) => {
                // Our local transport may finish before the client's multistream negotiation.
                // Immediate close can turn a valid dial-back into BrokenPipe at that client.
                // The client closes after verification; bound abandoned/late callbacks by
                // the existing probe timeout without touching its control/relay connection.
                self.callbacks
                    .established(e.connection_id, clock::instant());
            }
            FromSwarm::DialFailure(e) => {
                self.callbacks.closed(e.connection_id);
            }
            FromSwarm::ConnectionClosed(e) => {
                self.callbacks.closed(e.connection_id);
            }
            _ => {}
        }
        match event {
            FromSwarm::NewExternalAddrCandidate(_) => {}
            FromSwarm::NewListenAddr(e) if relay_support::is_circuit(e.addr) => {}
            _ => self.inner.on_swarm_event(event),
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
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        loop {
            match self.inner.poll(cx) {
                Poll::Ready(ToSwarm::GenerateEvent(event)) => {
                    if let autonat::Event::InboundProbe(autonat::InboundProbeEvent::Request {
                        peer,
                        ..
                    }) = &event
                    {
                        self.requested_callbacks.insert(*peer);
                    }
                    return Poll::Ready(ToSwarm::GenerateEvent(event));
                }
                Poll::Ready(ToSwarm::Dial { opts }) => {
                    if let Some(peer) = opts
                        .get_peer_id()
                        .filter(|p| self.requested_callbacks.remove(p))
                    {
                        self.callbacks.dial_started(peer, opts.connection_id());
                    }
                    return Poll::Ready(ToSwarm::Dial { opts });
                }
                Poll::Ready(
                    ToSwarm::ExternalAddrConfirmed(_) | ToSwarm::ExternalAddrExpired(_),
                ) => {}
                Poll::Pending => {
                    return match self.closing.pop_front() {
                        Some((peer_id, id)) => Poll::Ready(ToSwarm::CloseConnection {
                            peer_id,
                            connection: CloseConnection::One(id),
                        }),
                        None => Poll::Pending,
                    };
                }
                action => return action,
            }
        }
    }
}
