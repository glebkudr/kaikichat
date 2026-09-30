//! Bind codecs to the authenticated handler peer and retain one shared lease for
//! the entire libp2p request worker, including its wait for an application response.
use super::*;
use futures::io::{AsyncRead, AsyncWrite};
use libp2p::{
    core::{Endpoint, transport::PortUse},
    swarm::{ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm},
};
use std::{
    io,
    net::IpAddr,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};
#[path = "processing_budget.rs"]
mod budget;
pub(super) use budget::Budget;
use budget::Lease;
#[path = "processing_gate.rs"]
mod gate;
pub(super) use gate::{Admission, BOOK_RATE, Gate, Principal};
#[cfg(test)]
#[path = "processing_gate_tests.rs"]
mod gate_tests;

pub(super) type Cbor<Req, Resp> = Behaviour<cbor::codec::Codec<Req, Resp>>;
pub(super) fn cbor<Req, Resp>(
    budget: Budget,
    protocol: &'static str,
    request: u64,
    response: u64,
    config: request_response::Config,
) -> Cbor<Req, Resp>
where
    Req: Send + Serialize + DeserializeOwned + 'static,
    Resp: Send + Serialize + DeserializeOwned + 'static,
{
    gated_cbor(budget, None, protocol, request, response, config)
}

/// A protocol whose inbound requests pass `gate` before their bytes are read.
pub(super) fn gated_cbor<Req, Resp>(
    budget: Budget,
    gate: Option<Gate>,
    protocol: &'static str,
    request: u64,
    response: u64,
    config: request_response::Config,
) -> Cbor<Req, Resp>
where
    Req: Send + Serialize + DeserializeOwned + 'static,
    Resp: Send + Serialize + DeserializeOwned + 'static,
{
    Behaviour::with_codec(
        budget,
        gate,
        request.max(response) as usize,
        cbor::codec::Codec::default()
            .set_request_size_maximum(request)
            .set_response_size_maximum(response),
        [(StreamProtocol::new(protocol), ProtocolSupport::Full)],
        config,
    )
}

/// The authenticated peer of a connection and the IP it connects from.
type Factory = Arc<Mutex<Option<(PeerId, Option<IpAddr>)>>>;
struct Binding(Factory);
impl Binding {
    fn new(factory: Factory, peer: PeerId, remote: &Multiaddr) -> Self {
        *factory.lock().unwrap_or_else(|e| e.into_inner()) = Some((peer, ip_of(remote)));
        Self(factory)
    }
}

/// The first IP of an address: a relayed peer counts as its relay's.
fn ip_of(address: &Multiaddr) -> Option<IpAddr> {
    address.iter().find_map(|protocol| match protocol {
        libp2p::multiaddr::Protocol::Ip4(ip) => Some(IpAddr::V4(ip)),
        libp2p::multiaddr::Protocol::Ip6(ip) => Some(IpAddr::V6(ip)),
        _ => None,
    })
}

/// What a request stream holds while it is processed.
enum Held {
    Shared(Lease),
    /// A peer without a principal showing its pass: outside the shared
    /// budget, under the gate's small path.
    Probation,
}
impl Held {
    fn is_live(&self) -> bool {
        match self {
            Self::Shared(lease) => lease.is_live(),
            Self::Probation => true,
        }
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

pub(super) struct Codec<C> {
    inner: C,
    factory: Factory,
    peer: Option<(PeerId, Option<IpAddr>)>,
    budget: Budget,
    gate: Option<Gate>,
    bytes: usize,
    lease: Option<Held>,
}
impl<C: Clone> Clone for Codec<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            factory: self.factory.clone(),
            peer: self
                .peer
                .or_else(|| *self.factory.lock().unwrap_or_else(|e| e.into_inner())),
            budget: self.budget.clone(),
            gate: self.gate.clone(),
            bytes: self.bytes,
            lease: None,
        }
    }
}
impl<C> Codec<C> {
    fn acquire(&mut self) -> io::Result<()> {
        if self.lease.is_none() {
            let (peer, _) = self.peer.ok_or(io::ErrorKind::PermissionDenied)?;
            self.lease = Some(Held::Shared(self.budget.acquire(peer, self.bytes).inspect_err(
                |error| {
                    tracing::debug!(%peer, bytes = self.bytes, %error, "processing admission rejected");
                },
            )?));
        }
        Ok(())
    }
    /// An inbound request: the gate decides first whether it is read at all
    /// and whether it counts against the shared budget.
    fn admit(&mut self) -> io::Result<()> {
        if self.lease.is_none()
            && let Some(gate) = &self.gate
        {
            let (peer, address) = self.peer.ok_or(io::ErrorKind::PermissionDenied)?;
            match gate.admit(peer, address).inspect_err(|error| {
                tracing::debug!(%peer, %error, "mailbox admission rejected");
            })? {
                Admission::Probation => self.lease = Some(Held::Probation),
                admission => tracing::trace!(%peer, ?admission, "mailbox admission"),
            }
        }
        self.acquire()
    }
    fn operation<'a, T>(&'a mut self, inner: &'a mut T) -> io::Result<(&'a mut C, Fenced<'a, T>)> {
        self.acquire()?;
        Ok((
            &mut self.inner,
            Fenced {
                inner,
                lease: self.lease.as_ref().ok_or(io::ErrorKind::PermissionDenied)?,
            },
        ))
    }
}

struct Fenced<'a, T> {
    inner: &'a mut T,
    lease: &'a Held,
}
impl<T: AsyncRead + Unpin> AsyncRead for Fenced<'_, T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if !this.lease.is_live() {
            return Poll::Ready(Err(io::ErrorKind::PermissionDenied.into()));
        }
        Pin::new(&mut *this.inner).poll_read(cx, bytes)
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for Fenced<'_, T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if !this.lease.is_live() {
            return Poll::Ready(Err(io::ErrorKind::PermissionDenied.into()));
        }
        Pin::new(&mut *this.inner).poll_write(cx, bytes)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if !this.lease.is_live() {
            return Poll::Ready(Err(io::ErrorKind::PermissionDenied.into()));
        }
        Pin::new(&mut *this.inner).poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if !this.lease.is_live() {
            return Poll::Ready(Err(io::ErrorKind::PermissionDenied.into()));
        }
        Pin::new(&mut *this.inner).poll_close(cx)
    }
}

#[async_trait::async_trait]
impl<C: request_response::Codec<Protocol = StreamProtocol> + Send> request_response::Codec
    for Codec<C>
{
    type Protocol = StreamProtocol;
    type Request = C::Request;
    type Response = C::Response;
    async fn read_request<T: AsyncRead + Unpin + Send>(
        &mut self,
        protocol: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request> {
        self.admit()?;
        let (codec, mut io) = self.operation(io)?;
        codec.read_request(protocol, &mut io).await
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(
        &mut self,
        protocol: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response> {
        let (codec, mut io) = self.operation(io)?;
        codec.read_response(protocol, &mut io).await
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(
        &mut self,
        protocol: &StreamProtocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()> {
        let (codec, mut io) = self.operation(io)?;
        codec.write_request(protocol, &mut io, request).await
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(
        &mut self,
        protocol: &StreamProtocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()> {
        let (codec, mut io) = self.operation(io)?;
        codec.write_response(protocol, &mut io, response).await
    }
}

pub(super) struct Behaviour<
    C: request_response::Codec<Protocol = StreamProtocol> + Clone + Send + 'static,
> {
    inner: request_response::Behaviour<Codec<C>>,
    factory: Factory,
}
impl<C: request_response::Codec<Protocol = StreamProtocol> + Clone + Send + 'static> Behaviour<C> {
    pub fn with_codec(
        budget: Budget,
        gate: Option<Gate>,
        bytes: usize,
        codec: C,
        protocols: impl IntoIterator<Item = (StreamProtocol, ProtocolSupport)>,
        config: request_response::Config,
    ) -> Self {
        let factory = Arc::new(Mutex::new(None));
        Self {
            inner: request_response::Behaviour::with_codec(
                Codec {
                    inner: codec,
                    factory: factory.clone(),
                    peer: None,
                    budget,
                    gate,
                    bytes,
                    lease: None,
                },
                protocols,
                config,
            ),
            factory,
        }
    }
}
impl<C: request_response::Codec<Protocol = StreamProtocol> + Clone + Send + 'static> Deref
    for Behaviour<C>
{
    type Target = request_response::Behaviour<Codec<C>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<C: request_response::Codec<Protocol = StreamProtocol> + Clone + Send + 'static> DerefMut
    for Behaviour<C>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<C: request_response::Codec<Protocol = StreamProtocol> + Clone + Send + 'static>
    NetworkBehaviour for Behaviour<C>
{
    type ConnectionHandler =
        <request_response::Behaviour<Codec<C>> as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = request_response::Event<C::Request, C::Response>;
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
        // Locked libp2p 0.29 clones its codec synchronously here; worker clones
        // retain this immutable authenticated peer after the factory is cleared.
        let _binding = Binding::new(self.factory.clone(), peer, remote);
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
        let _binding = Binding::new(self.factory.clone(), peer, address);
        self.inner
            .handle_established_outbound_connection(id, peer, address, role, port)
    }
    fn on_swarm_event(&mut self, event: FromSwarm) {
        self.inner.on_swarm_event(event);
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
        self.inner.poll(cx)
    }
}
