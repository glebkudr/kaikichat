//! Bounded, private-to-the-owner peer lookup. Kad referrals are transport hints, never contacts.
use super::*;
use libp2p::{
    core::{Endpoint, transport::PortUse},
    kad,
    swarm::{ConnectionDenied, FromSwarm, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm},
};
use std::{
    collections::{BTreeMap, VecDeque},
    num::NonZeroUsize,
    task::{Context, Poll},
};
type Kad = kad::Behaviour<kad::store::MemoryStore>;
type KadIn = THandlerInEvent<Kad>;
type KadOut = THandlerOutEvent<Kad>;
const MAX_REQUESTS: u32 = 32;
const MAX_CANDIDATES: usize = 128;
const OUTBOUND_CAP: usize = 16;
const HOLD_MAX: usize = 512;
const KAD_PROTOCOL: &str = "/agentic-internet/kad/1";
pub(super) type DhtLookupAdmission = bootstrap_support::Admission<1024, 32>;
struct Entry {
    addresses: Vec<Multiaddr>,
    expires: Instant,
}
struct Lookup {
    state: &'static str,
    requests: u32,
    root: Option<String>,
    addresses: Vec<Multiaddr>,
    seen: HashSet<PeerId>,
    deadline: Instant,
    auth_sent: bool,
    automatic_after: Instant,
}
impl Lookup {
    fn active(&self) -> bool {
        matches!(self.state, "searching" | "authenticating")
    }
    fn info(&self, peer: PeerId) -> Value {
        json!({"peerId":peer.to_string(),"state":self.state,"requests":self.requests,"rootId":self.root})
    }
}
pub(super) struct GuardedRouting {
    own: PeerId,
    automatic_due: Instant,
    inner: Kad,
    known: HashMap<PeerId, Entry>,
    serving: HashMap<PeerId, bool>,
    queries: HashMap<kad::QueryId, PeerId>,
    lookups: BTreeMap<PeerId, Lookup>,
    owned_dials: HashSet<ConnectionId>,
    replies: VecDeque<ToSwarm<(), KadIn>>,
    outbound_hold: VecDeque<ToSwarm<(), KadIn>>,
    admission: DhtLookupAdmission,
    lookup_rejections: BTreeMap<&'static str, u64>,
    kad_event_counts: HashMap<&'static str, u64>,
    qerr_kinds: BTreeMap<String, u64>,
    qerr_peers: HashMap<PeerId, u64>,
    replies_dropped: u64,
    admit_dropped: HashMap<&'static str, u64>,
    outbound_held_total: u64,
    held_dropped_dead: u64,
    held_dropped_overflow: u64,
}
impl GuardedRouting {
    pub(super) fn new(own: PeerId, server: bool) -> Self {
        let mut config = kad::Config::new(StreamProtocol::new(KAD_PROTOCOL));
        config
            .set_kbucket_inserts(kad::BucketInserts::Manual)
            .set_query_timeout(Duration::from_secs(15))
            .set_parallelism(NonZeroUsize::MIN)
            .set_max_packet_size(8192)
            .set_substreams_timeout(Duration::from_secs(3))
            .set_caching(kad::Caching::Disabled)
            .set_record_filtering(kad::StoreInserts::FilterBoth)
            .set_periodic_bootstrap_interval(None)
            .set_publication_interval(None)
            .set_replication_interval(None)
            .set_provider_publication_interval(None);
        // Records are neither served nor stored: the DHT only routes to peers.
        let store = kad::store::MemoryStore::new(own);
        let mut inner = Kad::with_config(own, store, config);
        // Explicit modes prevent external-address confirmation from opting a client into service.
        inner.set_mode(Some(if server {
            kad::Mode::Server
        } else {
            kad::Mode::Client
        }));
        Self {
            own,
            automatic_due: clock::instant(),
            inner,
            known: HashMap::new(),
            serving: HashMap::new(),
            queries: HashMap::new(),
            lookups: BTreeMap::new(),
            owned_dials: HashSet::new(),
            replies: VecDeque::new(),
            outbound_hold: VecDeque::new(),
            admission: DhtLookupAdmission::new(clock::instant()),
            lookup_rejections: BTreeMap::new(),
            kad_event_counts: HashMap::new(),
            qerr_kinds: BTreeMap::new(),
            qerr_peers: HashMap::new(),
            replies_dropped: 0,
            admit_dropped: HashMap::new(),
            outbound_held_total: 0,
            held_dropped_dead: 0,
            held_dropped_overflow: 0,
        }
    }
    fn reply(&mut self, peer: PeerId, id: ConnectionId, event: KadIn) {
        if self.replies.len() >= 64 {
            self.replies_dropped += 1;
            return;
        }
        self.replies.push_back(ToSwarm::NotifyHandler {
            peer_id: peer,
            handler: libp2p::swarm::NotifyHandler::One(id),
            event,
        });
    }
    fn filtered(peer: PeerId, addresses: Vec<Multiaddr>) -> Vec<Multiaddr> {
        let mut selected = vec![];
        for mut address in addresses.into_iter().take(20) {
            if !matches!(address.iter().last(), Some(Protocol::P2p(_))) {
                address.push(Protocol::P2p(peer));
            }
            if relay_support::is_circuit(&address)
                || routes(&[address.to_string()], Some(peer), false).is_err()
            {
                continue;
            }
            if matches!(address.iter().next(),Some(Protocol::Ip4(ip)) if ip.is_multicast() || ip.is_broadcast())
                || matches!(address.iter().next(),Some(Protocol::Ip6(ip)) if ip.is_multicast())
            {
                continue;
            }
            if !selected.contains(&address) {
                selected.push(address);
            }
            if selected.len() == 4 {
                break;
            }
        }
        selected
    }
    /// Called only after the existing root/PeerID/session verifier or from its encrypted cache.
    pub(super) fn remember(&mut self, peer: PeerId, addresses: Vec<Multiaddr>) {
        let addresses = Self::filtered(peer, addresses);
        if peer == self.own
            || addresses.is_empty()
            || (self.known.len() >= 128 && !self.known.contains_key(&peer))
        {
            return;
        }
        let changed = self
            .known
            .get(&peer)
            .is_none_or(|e| e.addresses != addresses);
        self.known.insert(
            peer,
            Entry {
                addresses,
                expires: clock::instant() + Duration::from_secs(60),
            },
        );
        if changed {
            self.refresh_peer(peer);
        }
    }
    fn can_serve(&self, peer: &PeerId) -> bool {
        // Only a live Identify-confirmed KAD server may enter the routing table.
        self.serving.get(peer) == Some(&true)
    }
    fn refresh_peer(&mut self, peer: PeerId) {
        self.inner.remove_peer(&peer);
        if self.can_serve(&peer)
            && let Some(entry) = self
                .known
                .get(&peer)
                .filter(|e| e.expires > clock::instant())
        {
            for address in &entry.addresses {
                self.inner.add_address(&peer, address.clone());
            }
        }
    }
    /// Capability metadata from a live Identify exchange. It grants no address,
    /// root identity, committee membership or custody authority.
    pub(super) fn observe_protocols(&mut self, peer: PeerId, protocols: &[StreamProtocol]) {
        if peer == self.own || (self.serving.len() >= 128 && !self.serving.contains_key(&peer)) {
            return;
        }
        let serving = protocols.iter().any(|p| p.as_ref() == KAD_PROTOCOL);
        if self.serving.insert(peer, serving) != Some(serving) {
            self.refresh_peer(peer);
        }
    }
    #[cfg(test)]
    pub(super) fn addresses(&self, peer: PeerId) -> Vec<Multiaddr> {
        self.known
            .get(&peer)
            .map(|e| e.addresses.clone())
            .unwrap_or_default()
    }
    pub(super) fn start(&mut self, peer: PeerId) -> Result<Value> {
        if peer == self.own {
            return Err("lookup requires a remote PeerID".into());
        }
        if let Some(l) = self.lookups.get(&peer).filter(|l| l.active()) {
            return Ok(l.info(peer));
        }
        if self.active_count() >= 2 {
            return Err("at most two lookups may be active".into());
        }
        if self.lookups.len() >= 16
            && !self.lookups.contains_key(&peer)
            && let Some(old) = self
                .lookups
                .iter()
                .find(|(_, l)| !l.active())
                .map(|(p, _)| *p)
        {
            self.lookups.remove(&old);
        }
        let query = self.inner.get_closest_peers(peer);
        self.queries.insert(query, peer);
        let l = Lookup {
            state: "searching",
            requests: 0,
            root: None,
            addresses: vec![],
            seen: HashSet::new(),
            deadline: clock::instant() + Duration::from_secs(20),
            auth_sent: false,
            automatic_after: clock::instant() + Duration::from_secs(60),
        };
        let info = l.info(peer);
        self.lookups.insert(peer, l);
        Ok(info)
    }
    /// Local outbox failures share a global throttle and a per-target retry window.
    pub(super) fn start_automatically(&mut self, peer: PeerId, now: Instant) -> bool {
        if now < self.automatic_due
            || self
                .lookups
                .get(&peer)
                .is_some_and(|l| l.active() || now < l.automatic_after)
        {
            return false;
        }
        if self.start(peer).is_err() {
            return false;
        }
        self.automatic_due = now + Duration::from_secs(5);
        if let Some(l) = self.lookups.get_mut(&peer) {
            l.automatic_after = now + Duration::from_secs(60);
        }
        true
    }
    fn in_flight(&self) -> usize {
        self.queries
            .keys()
            .filter_map(|id| {
                self.inner
                    .query(id)
                    .map(|query| query.stats().num_pending())
            })
            .map(|pending| pending as usize)
            .sum()
    }
    fn record_query_error(
        &mut self,
        peer: PeerId,
        query_id: kad::QueryId,
        source: Option<&(dyn std::error::Error + 'static)>,
    ) {
        let live = self.queries.contains_key(&query_id);
        *self
            .kad_event_counts
            .entry(if live {
                "queryErrorLive"
            } else {
                "queryErrorOrphan"
            })
            .or_insert(0) += 1;
        let kind = source
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .map(|error| format!("{:?}", error.kind()))
            .unwrap_or_else(|| "unexpectedMessage".to_owned());
        *self.qerr_kinds.entry(kind).or_insert(0) += 1;
        *self.qerr_peers.entry(peer).or_insert(0) += 1;
    }
    pub(super) fn info(&self) -> Value {
        let lookup_admission = self.admission.snapshot();
        let lookup_top = self.admission.top_peers(5);
        let mut qerr_top = self
            .qerr_peers
            .iter()
            .map(|(peer, count)| (*peer, *count))
            .collect::<Vec<_>>();
        qerr_top.sort_by(|(a_peer, a_count), (b_peer, b_count)| {
            b_count
                .cmp(a_count)
                .then_with(|| a_peer.to_bytes().cmp(&b_peer.to_bytes()))
        });
        qerr_top.truncate(8);
        let kad_event = |kind| self.kad_event_counts.get(kind).copied().unwrap_or(0);
        let admit_dropped = |kind| self.admit_dropped.get(kind).copied().unwrap_or(0);
        json!({"enabled":true,"mode":self.inner.mode().to_string(),"blockedByPolicy":false,
            "peers":self.known.len(),"addresses":self.known.values().map(|e|e.addresses.len()).sum::<usize>(),
            "knownPeers":self.known.keys().map(PeerId::to_string).collect::<Vec<_>>(),
            "servingPeers":self.serving.iter().filter(|(_, serving)| **serving).map(|(peer, _)| peer.to_string()).collect::<Vec<_>>(),
            "inFlight":self.active_count(),"lookups":self.lookups.iter().map(|(p,l)|l.info(*p)).collect::<Vec<_>>(),
            "ownedDials":self.owned_dials.len(),"lookupRejections":self.lookup_rejections,
            "repliesDropped":self.replies_dropped,"outboundHeld":self.outbound_hold.len(),
            "outboundHeldTotal":self.outbound_held_total,"inFlightReq":self.in_flight(),
            "heldDropped":{"dead":self.held_dropped_dead,"overflow":self.held_dropped_overflow},
            "kadEv":{
                "queryError":kad_event("queryError"),
                "queryErrorLive":kad_event("queryErrorLive"),"queryErrorOrphan":kad_event("queryErrorOrphan"),
                "qErrKinds":self.qerr_kinds,
                "qErrTopPeers":qerr_top.into_iter().map(|(peer,count)|json!({"peer":peer.to_string(),"count":count})).collect::<Vec<_>>(),
                "findNodeRes":kad_event("findNodeRes"),
                "putRecordReq":kad_event("putRecordReq"),"getRecordReq":kad_event("getRecordReq"),
                "findNodeReq":kad_event("findNodeReq"),"other":kad_event("other")},
            "admitDropped":{"findNode":admit_dropped("findNode")},
            "dhtAdmission":{
                "lookup":{"total":lookup_admission.0,"peers":lookup_admission.1,
                    "exhausted":self.admission.total_exhausted()},
                "topPeers":{
                    "lookup":lookup_top.into_iter().map(|(peer,count)|json!({"peer":peer.to_string(),"count":count})).collect::<Vec<_>>()}}})
    }
    pub(super) fn auth_ready(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.lookups
            .iter()
            .filter(|(_, l)| l.state == "authenticating" && !l.auth_sent)
            .map(|(p, l)| (*p, l.addresses.clone()))
            .collect()
    }
    pub(super) fn auth_sent(&mut self, peer: PeerId) {
        if let Some(l) = self.lookups.get_mut(&peer) {
            l.auth_sent = true;
        }
    }
    pub(super) fn auth_finished(&mut self, peer: PeerId, root: Option<String>) {
        if let Some(l) = self
            .lookups
            .get_mut(&peer)
            .filter(|l| l.state == "authenticating")
        {
            l.state = if root.is_some() {
                "verified"
            } else {
                "authentication-failed"
            };
            l.root = root;
            l.addresses.clear();
            l.seen.clear();
        }
    }
    fn guard_queries(&mut self) -> bool {
        let mut stopped = false;
        for mut query in self.inner.iter_queries_mut() {
            let id = query.id();
            let Some(peer) = self.queries.get(&id) else {
                query.finish();
                stopped = true;
                continue;
            };
            let Some(l) = self.lookups.get_mut(peer) else {
                query.finish();
                stopped = true;
                continue;
            };
            l.requests = query.stats().num_requests().min(MAX_REQUESTS);
            if query.stats().num_requests() > MAX_REQUESTS || clock::instant() >= l.deadline {
                l.state = if query.stats().num_requests() > MAX_REQUESTS {
                    "budget-exhausted"
                } else {
                    "not-found"
                };
                query.finish();
                stopped = true;
            }
        }
        stopped
    }
    fn completed(
        &mut self,
        id: kad::QueryId,
        result: kad::GetClosestPeersResult,
        stats: kad::QueryStats,
    ) {
        let Some(peer) = self.queries.remove(&id) else {
            return;
        };
        let Some(l) = self.lookups.get_mut(&peer) else {
            return;
        };
        l.requests = stats.num_requests().min(MAX_REQUESTS);
        if !l.active() {
            l.seen.clear();
            return;
        }
        let peers = match result {
            Ok(r) => r.peers,
            Err(kad::GetClosestPeersError::Timeout { peers, .. }) => peers,
        };
        let fallback = self
            .known
            .get(&peer)
            .filter(|e| e.expires > clock::instant())
            .map(|e| e.addresses.clone())
            .unwrap_or_default();
        let found = peers
            .into_iter()
            .find(|p| p.peer_id == peer)
            .map(|p| {
                Self::filtered(
                    peer,
                    if p.addrs.is_empty() {
                        fallback.clone()
                    } else {
                        p.addrs
                    },
                )
            })
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| {
                if l.addresses.is_empty() {
                    fallback
                } else {
                    l.addresses.clone()
                }
            });
        if found.is_empty() {
            l.state = "not-found";
        } else {
            l.state = "authenticating";
            l.addresses = found;
        }
        l.seen.clear();
    }
    fn trim_table(&mut self) {
        let mut remove = vec![];
        for bucket in self.inner.kbuckets() {
            for entry in bucket.iter() {
                let peer = *entry.node.key.preimage();
                for address in entry.node.value.iter() {
                    if self.serving.get(&peer) == Some(&false)
                        || !self
                            .known
                            .get(&peer)
                            .is_some_and(|k| k.addresses.contains(address))
                    {
                        remove.push((peer, address.clone()));
                    }
                }
            }
        }
        for (peer, address) in remove {
            self.inner.remove_address(&peer, &address);
        }
    }
}
impl NetworkBehaviour for GuardedRouting {
    type ConnectionHandler = THandler<Kad>;
    type ToSwarm = ();
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
    fn handle_established_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        address: &Multiaddr,
        role: Endpoint,
        port: PortUse,
    ) -> std::result::Result<THandler<Self>, ConnectionDenied> {
        self.owned_dials.remove(&id);
        self.inner
            .handle_established_outbound_connection(id, peer, address, role, port)
    }
    fn handle_pending_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: Option<PeerId>,
        addresses: &[Multiaddr],
        role: Endpoint,
    ) -> std::result::Result<Vec<Multiaddr>, ConnectionDenied> {
        if !self.owned_dials.contains(&id) {
            return Ok(vec![]);
        }
        let mut hints = self
            .inner
            .handle_pending_outbound_connection(id, peer, addresses, role)?;
        if let Some(peer) = peer
            && let Some(entry) = self
                .known
                .get(&peer)
                .filter(|entry| entry.expires > clock::instant())
        {
            hints.extend(entry.addresses.iter().cloned());
        }
        Ok(peer.map(|p| Self::filtered(p, hints)).unwrap_or_default())
    }
    fn on_swarm_event(&mut self, event: FromSwarm) {
        if let FromSwarm::ConnectionClosed(e) = event
            && e.remaining_established == 0
        {
            self.serving.remove(&e.peer_id);
            self.inner.remove_peer(&e.peer_id);
        }
        if let FromSwarm::DialFailure(e) = event {
            self.owned_dials.remove(&e.connection_id);
        }
        // Ignore unsolicited swarm address publications. Only authenticated records populate Kad.
        if matches!(event, FromSwarm::NewExternalAddrOfPeer(_)) {
            return;
        }
        self.inner.on_swarm_event(event);
        self.trim_table();
    }
    fn on_connection_handler_event(&mut self, peer: PeerId, id: ConnectionId, event: KadOut) {
        let event_kind = match &event {
            KadOut::QueryError { error, query_id } => {
                self.record_query_error(peer, *query_id, std::error::Error::source(error));
                "queryError"
            }
            KadOut::FindNodeRes { .. } => "findNodeRes",
            KadOut::PutRecord { .. } => "putRecordReq",
            KadOut::GetRecord { .. } => "getRecordReq",
            KadOut::FindNodeReq { .. } => "findNodeReq",
            _ => "other",
        };
        *self.kad_event_counts.entry(event_kind).or_insert(0) += 1;
        let event = match event {
            KadOut::FindNodeRes {
                closer_peers,
                query_id,
            } => {
                let selected = if let Some(target) = self.queries.get(&query_id)
                    && let Some(l) = self.lookups.get_mut(target).filter(|l| l.active())
                {
                    let selected =
                        Self::bounded_referrals(self.own, &mut l.seen, closer_peers, |p| {
                            (p.node_id, &mut p.multiaddrs)
                        });
                    if let Some(p) = selected.iter().find(|p| p.node_id == *target) {
                        l.addresses = p.multiaddrs.clone();
                    }
                    selected
                } else {
                    closer_peers
                };
                KadOut::FindNodeRes {
                    closer_peers: selected,
                    query_id,
                }
            }
            KadOut::FindNodeReq { key, request_id } => {
                let rejection = if key.len() > 64 {
                    Some("findNode.keySize")
                } else if !self.admission.allow(peer, clock::instant()) {
                    Some("findNode.admission")
                } else {
                    None
                };
                if let Some(reason) = rejection {
                    *self.lookup_rejections.entry(reason).or_insert(0) += 1;
                    self.reply(peer, id, KadIn::Reset(request_id));
                    return;
                }
                // Exact peer lookup can name an ordinary client. Return only its
                // trusted current endpoint; keep clients out of replica referrals.
                if let Ok(target) = PeerId::from_bytes(&key)
                    && let Some(entry) = self
                        .known
                        .get(&target)
                        .filter(|e| e.expires > clock::instant())
                {
                    let closer_peers = vec![kad::KadPeer {
                        node_id: target,
                        multiaddrs: entry.addresses.clone(),
                        connection_ty: kad::ConnectionType::CanConnect,
                    }];
                    self.reply(
                        peer,
                        id,
                        KadIn::FindNodeRes {
                            closer_peers,
                            request_id,
                        },
                    );
                    return;
                }
                KadOut::FindNodeReq { key, request_id }
            }
            // Records are neither served nor stored here.
            KadOut::GetRecord { request_id, .. } | KadOut::PutRecord { request_id, .. } => {
                self.reply(peer, id, KadIn::Reset(request_id));
                return;
            }
            KadOut::GetProvidersReq { request_id, .. } => {
                self.reply(peer, id, KadIn::Reset(request_id));
                return;
            }
            KadOut::AddProvider { .. } => return,
            event => event,
        };
        let found_query = match &event {
            KadOut::FindNodeRes { query_id, .. }
                if self
                    .queries
                    .get(query_id)
                    .and_then(|p| self.lookups.get(p))
                    .is_some_and(|l| !l.addresses.is_empty()) =>
            {
                Some(*query_id)
            }
            _ => None,
        };
        self.inner.on_connection_handler_event(peer, id, event);
        // The exact-target referral is sufficient to begin authentication even when an old
        // address already failed in this iterative query. It is never a verified result.
        if let Some(id) = found_query
            && let Some(mut query) = self.inner.query_mut(&id)
        {
            query.finish();
        }
        self.trim_table();
    }
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ToSwarm<(), KadIn>> {
        let now = clock::instant();
        self.known.retain(|peer, entry| {
            if entry.expires <= now {
                self.inner.remove_peer(peer);
                false
            } else {
                true
            }
        });
        for l in self
            .lookups
            .values_mut()
            .filter(|l| l.active() && l.deadline <= now)
        {
            l.state = "not-found";
            l.seen.clear();
            l.addresses.clear();
        }
        if let Some(reply) = self.replies.pop_front() {
            return Poll::Ready(reply);
        }
        for _ in 0..32 {
            while self.in_flight() < OUTBOUND_CAP
                && let Some(action) = self.outbound_hold.pop_front()
            {
                let ToSwarm::NotifyHandler { event, .. } = &action else {
                    unreachable!("outbound hold contains only handler notifications")
                };
                let live = Self::outbound_request(event)
                    .is_none_or(|(query_id, _)| self.queries.contains_key(&query_id));
                if !live {
                    self.held_dropped_dead += 1;
                    continue;
                }
                return Poll::Ready(action);
            }
            self.guard_queries();
            let action = self.inner.poll(cx);
            let stopped = self.guard_queries();
            match action {
                Poll::Ready(ToSwarm::GenerateEvent(kad::Event::OutboundQueryProgressed {
                    id,
                    result: kad::QueryResult::GetClosestPeers(result),
                    stats,
                    step,
                })) if step.last && self.queries.contains_key(&id) => {
                    self.completed(id, result, stats);
                    return Poll::Ready(ToSwarm::GenerateEvent(()));
                }
                Poll::Ready(ToSwarm::GenerateEvent(_) | ToSwarm::NewExternalAddrOfPeer { .. }) => {}
                Poll::Ready(ToSwarm::Dial { opts }) => {
                    if stopped || self.queries.is_empty() || self.owned_dials.len() >= 16 {
                        continue;
                    }
                    self.owned_dials.insert(opts.connection_id());
                    return Poll::Ready(ToSwarm::Dial { opts });
                }
                Poll::Ready(ToSwarm::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                }) => {
                    if !self.admit_request(&event) {
                        continue;
                    }
                    if let Some((_, kind)) = Self::outbound_request(&event)
                        && self.in_flight() >= OUTBOUND_CAP
                    {
                        if self.outbound_hold.len() >= HOLD_MAX {
                            self.held_dropped_overflow += 1;
                            *self.admit_dropped.entry(kind).or_insert(0) += 1;
                        } else {
                            self.outbound_hold.push_back(ToSwarm::NotifyHandler {
                                peer_id,
                                handler,
                                event,
                            });
                            self.outbound_held_total += 1;
                        }
                        continue;
                    }
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler,
                        event,
                    });
                }
                Poll::Ready(action) => return Poll::Ready(action.map_out(|_| ())),
                Poll::Pending => return Poll::Pending,
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
impl GuardedRouting {
    pub(super) fn active_count(&self) -> usize {
        self.lookups.values().filter(|l| l.active()).count()
    }
    fn outbound_request(event: &KadIn) -> Option<(kad::QueryId, &'static str)> {
        match event {
            KadIn::FindNodeReq { query_id, .. } => Some((*query_id, "findNode")),
            KadIn::GetProvidersReq { query_id, .. } => Some((*query_id, "getProviders")),
            KadIn::AddProvider { query_id, .. } => Some((*query_id, "addProvider")),
            KadIn::GetRecord { query_id, .. } => Some((*query_id, "getRecord")),
            KadIn::PutRecord { query_id, .. } => Some((*query_id, "putRecord")),
            _ => None,
        }
    }
    /// Only the peer lookups still searching send requests.
    fn admit_request(&mut self, event: &KadIn) -> bool {
        let (id, kind) = match event {
            KadIn::FindNodeReq { query_id, .. } => (*query_id, "findNode"),
            KadIn::GetRecord { query_id, .. } => (*query_id, "getRecord"),
            KadIn::PutRecord { query_id, .. } => (*query_id, "putRecord"),
            _ => return true,
        };
        let admitted = kind == "findNode"
            && self
                .queries
                .get(&id)
                .and_then(|p| self.lookups.get(p))
                .is_some_and(|l| l.state == "searching");
        if !admitted {
            *self.admit_dropped.entry(kind).or_insert(0) += 1;
        }
        admitted
    }
    fn bounded_referrals<P>(
        own: PeerId,
        seen: &mut HashSet<PeerId>,
        peers: Vec<P>,
        mut fields: impl FnMut(&mut P) -> (PeerId, &mut Vec<Multiaddr>),
    ) -> Vec<P> {
        peers
            .into_iter()
            .take(20)
            .filter_map(|mut p| {
                let (peer, addresses) = fields(&mut p);
                if peer == own || (seen.len() >= MAX_CANDIDATES && !seen.contains(&peer)) {
                    return None;
                }
                *addresses = Self::filtered(peer, std::mem::take(addresses));
                if addresses.is_empty() {
                    return None;
                }
                seen.insert(peer);
                Some(p)
            })
            .collect()
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LookupRequest {
    peer_id: String,
}
impl Runtime {
    pub(super) fn routing_info(&self) -> Value {
        self.swarm
            .behaviour()
            .routing
            .as_ref()
            .map(GuardedRouting::info)
            .unwrap_or_else(|| {
                json!({"enabled":false,"mode":"disabled","blockedByPolicy":self.relay_only,
                    "peers":0,"addresses":0,"inFlight":0,"lookups":[]})
            })
    }
    pub(super) fn lookup_peer(
        &mut self,
        input: LookupRequest,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let peer: PeerId = input
            .peer_id
            .parse()
            .map_err(|_| ("invalid_request", "Invalid PeerID".into()))?;
        if peer == *self.swarm.local_peer_id() {
            return Err(("invalid_request", "Lookup requires a remote PeerID".into()));
        }
        let cached = self
            .core
            .cached_node_records(now().map_err(|e| ("unavailable", e.to_string()))?)
            .map_err(|e| ("unavailable", e.to_string()))?;
        let routing = self.swarm.behaviour_mut().routing.as_mut().ok_or((
            "policy_blocked",
            "Peer lookup currently requires direct networking".into(),
        ))?;
        for record in cached {
            if let Ok(remote) = record.peer_id.parse() {
                routing.remember(
                    remote,
                    record
                        .addresses
                        .iter()
                        .filter_map(|s| s.parse().ok())
                        .collect(),
                );
            }
        }
        routing.start(peer).map_err(|e| ("busy", e.to_string()))
    }
    pub(super) fn maintain_routing(&mut self) {
        let ready = self
            .swarm
            .behaviour()
            .routing
            .as_ref()
            .map(GuardedRouting::auth_ready)
            .unwrap_or_default();
        for (peer, addresses) in ready {
            if self.discovery.pending.len() >= 4 {
                break;
            }
            // A stale endpoint may still own a stalled handshake for this same PeerID.
            // Permit one independent dial to the newly discovered route for this auth attempt.
            if !self.swarm.is_connected(&peer) {
                let _ = self.swarm.dial(
                    DialOpts::peer_id(peer)
                        .condition(libp2p::swarm::dial_opts::PeerCondition::Disconnected)
                        .allocate_new_port()
                        .addresses(addresses.clone())
                        .build(),
                );
            }
            let Ok(Some(addresses)) = self.prepare_peer_routes(peer, addresses) else {
                continue;
            };
            let Ok(time) = now() else {
                continue;
            };
            let Ok(wire) = self.binding(time) else {
                continue;
            };
            let Ok(cached) = self.core.cached_node_records(time) else {
                continue;
            };
            let root = cached
                .into_iter()
                .find(|r| r.peer_id == peer.to_string())
                .map(|r| r.author);
            let id = self
                .swarm
                .behaviour_mut()
                .bootstrap
                .send_request_with_addresses(
                    &peer,
                    Exchange { node_record: wire },
                    addresses.clone(),
                );
            self.discovery.pending.insert(
                id,
                bootstrap_support::Hint {
                    peer,
                    addresses,
                    root,
                },
            );
            self.discovery.routing_pending.insert(id);
            if let Some(r) = self.swarm.behaviour_mut().routing.as_mut() {
                r.auth_sent(peer);
            }
        }
    }
}
