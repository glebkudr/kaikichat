//! Per-swarm resources, shared by every application request/response protocol.
use super::super::{
    PeerId, Value, clock, now,
    reserved_connections::{Catalog, Reservation},
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const LARGE: usize = 16 * 1024 * 1024 + 1024;
const SLOTS: [usize; 2] = [16, 48];
const PEER_SLOTS: [usize; 2] = [4, 16];
const BYTE_LIMIT: usize = 4 * LARGE;
const PEER_BYTES: usize = 2 * LARGE;
const RATE: [usize; 2] = [256, 1024];
const PEER_RATE: [usize; 2] = [64, 256];

#[derive(Default)]
struct Usage {
    active: [usize; 2],
    bytes: usize,
}
struct Class {
    active: usize,
    bytes: usize,
    peak_active: usize,
    peak_bytes: usize,
    started: u64,
    window: Instant,
    rate: usize,
    peers: BTreeMap<PeerId, usize>,
}
impl Default for Class {
    fn default() -> Self {
        Self {
            active: 0,
            bytes: 0,
            peak_active: 0,
            peak_bytes: 0,
            started: 0,
            window: clock::instant(),
            rate: 0,
            peers: BTreeMap::new(),
        }
    }
}
#[derive(Default)]
struct State {
    classes: [Class; 2],
    peers: BTreeMap<PeerId, Usage>,
    peak: usize,
    capacity_rejected: u64,
    rate_rejected: u64,
}

#[derive(Clone)]
pub(in crate::runtime) struct Budget {
    catalog: Catalog,
    state: Arc<Mutex<State>>,
}
impl Budget {
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            state: Arc::new(Mutex::new(State::default())),
        }
    }
    pub fn acquire(&self, peer: PeerId, bytes: usize) -> io::Result<Lease> {
        let time = now().map_err(|e| io::Error::other(e.to_string()))?;
        // Retain original sources, never a lookup that can be revived by a later grant.
        // Release the catalog read lock before taking the accounting lock.
        let sources = self
            .catalog
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&peer)
            .into_iter()
            .flatten()
            .filter(|g| g.live(time))
            .cloned()
            .collect::<Vec<_>>();
        let class = usize::from(!sources.is_empty());
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let c = &state.classes[class];
        let peer_full = state.peers.get(&peer).is_some_and(|p| {
            p.active[class] >= PEER_SLOTS[class]
                || p.active.iter().sum::<usize>() >= 16
                || bytes > PEER_BYTES - p.bytes
        });
        if bytes == 0
            || bytes > PEER_BYTES
            || c.active >= SLOTS[class]
            || bytes > BYTE_LIMIT - c.bytes
            || peer_full
        {
            state.capacity_rejected = state.capacity_rejected.saturating_add(1);
            return Err(io::Error::other("shared processing capacity exhausted"));
        }
        let c = &mut state.classes[class];
        if clock::instant().saturating_duration_since(c.window) >= Duration::from_secs(1) {
            c.window = clock::instant();
            c.rate = 0;
            c.peers.clear();
        }
        if c.rate >= RATE[class] || c.peers.get(&peer).copied().unwrap_or(0) >= PEER_RATE[class] {
            state.rate_rejected = state.rate_rejected.saturating_add(1);
            return Err(io::Error::other("shared processing rate exhausted"));
        }
        c.rate += 1;
        *c.peers.entry(peer).or_default() += 1;
        c.active += 1;
        c.bytes += bytes;
        c.peak_active = c.peak_active.max(c.active);
        c.peak_bytes = c.peak_bytes.max(c.bytes);
        c.started = c.started.saturating_add(1);
        state.peak = state.peak.max(state.classes.iter().map(|c| c.active).sum());
        let p = state.peers.entry(peer).or_default();
        p.active[class] += 1;
        p.bytes += bytes;
        Ok(Lease {
            state: self.state.clone(),
            peer,
            class,
            bytes,
            grant: GrantFence(sources.into()),
        })
    }
    pub fn info(&self) -> Value {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let [ordinary, selected] = &state.classes;
        json!({"active":ordinary.active+selected.active,"activeLimit":64,"peakActive":state.peak,
            "ordinaryActive":ordinary.active,"selectedActive":selected.active,
            "ordinaryBytes":ordinary.bytes,"selectedBytes":selected.bytes,
            "ordinaryLimit":SLOTS[0],"selectedLimit":SLOTS[1],
            "ordinaryPeerLimit":PEER_SLOTS[0],"selectedPeerLimit":PEER_SLOTS[1],
            "ordinaryByteLimit":BYTE_LIMIT,"selectedByteLimit":BYTE_LIMIT,"peerByteLimit":PEER_BYTES,
            "peakOrdinaryActive":ordinary.peak_active,"peakSelectedActive":selected.peak_active,
            "peakOrdinaryBytes":ordinary.peak_bytes,"peakSelectedBytes":selected.peak_bytes,
            "startedOrdinary":ordinary.started,"startedSelected":selected.started,
            "capacityRejected":state.capacity_rejected,"rateRejected":state.rate_rejected,
            "peers":state.peers.iter().map(|(peer,p)|json!({"peerId":peer.to_string(),
                "ordinaryActive":p.active[0],"selectedActive":p.active[1],"bytes":p.bytes})).collect::<Vec<_>>()})
    }
}

// Share only the original authority sources, never ownership of slots/bytes.
// A replacement catalog entry cannot revive a revoked in-flight request.
#[derive(Clone)]
pub(in crate::runtime) struct GrantFence(Arc<[Reservation]>);
impl GrantFence {
    pub fn is_live(&self) -> bool {
        self.0.is_empty() || now().is_ok_and(|time| self.0.iter().any(|g| g.live(time)))
    }
}

pub(in crate::runtime) struct Lease {
    state: Arc<Mutex<State>>,
    peer: PeerId,
    class: usize,
    bytes: usize,
    grant: GrantFence,
}
impl Lease {
    pub fn is_live(&self) -> bool {
        self.grant.is_live()
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let c = &mut state.classes[self.class];
        c.active -= 1;
        c.bytes -= self.bytes;
        if let Some(p) = state.peers.get_mut(&self.peer) {
            p.active[self.class] -= 1;
            p.bytes -= self.bytes;
            if p.active == [0, 0] {
                state.peers.remove(&self.peer);
            }
        }
        // Finishing or cancelling does not refill the one-second rate window.
    }
}
