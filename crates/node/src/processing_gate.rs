//! Admission to the mailbox protocol before a request's bytes are read
//! (Docs/V1_DISCOVERY_2026_09_27.md, part 1). Units and peers that showed an
//! accepted pass get in under their principal's limits; everyone else only
//! through a small path to show a pass. There is no anonymous quota an
//! attacker could fill to keep owners of books out.
use super::super::{PeerId, Value, clock};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Requests a second all peers of one book start together (user decision).
pub(in crate::runtime) const BOOK_RATE: usize = 30;
/// The node's ordinary processing rate: split evenly between books when
/// more of them ask than it serves.
const SHARED_RATE: usize = 256;
/// The path to show a pass: all peers, one peer, one address, a second.
const PROBATION_RATE: usize = 64;
const PROBATION_PEER_RATE: usize = 2;
const PROBATION_ADDRESS_RATE: usize = 8;
/// How long a peer that showed a bad credential, and its address, wait.
const PENALTY: Duration = Duration::from_secs(300);
/// How often one peer or address may have an unknown book read from the chain.
const BOOK_READ_EVERY: Duration = Duration::from_secs(60);
const WINDOW: Duration = Duration::from_secs(1);
/// Peers of one book let in at once: a newer pass replaces the oldest, so
/// memory stays bounded by books, which cost money.
const PEERS_PER_BOOK: usize = 1_024;
/// Addresses remembered for peers; beyond it one is forgotten.
const MAX_ADDRESSES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum Principal {
    Unit,
    Book([u8; 32]),
}

#[derive(Debug)]
pub(in crate::runtime) enum Admission {
    /// The gate is off: a node that does not read the chain takes anyone.
    Open,
    Unit,
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "diagnostics and tests read the book")
    )]
    Book([u8; 32]),
    /// Only a pass is taken from this peer.
    Probation,
}

/// Requests started in the current second of the small path.
#[derive(Default)]
struct Probation {
    start: Option<Instant>,
    total: usize,
    peers: BTreeMap<PeerId, usize>,
    addresses: BTreeMap<IpAddr, usize>,
}

/// Requests of each book in the current second, and how many books asked in
/// the second before it.
#[derive(Default)]
struct Books {
    start: Option<Instant>,
    counts: BTreeMap<[u8; 32], usize>,
    previous: usize,
}

impl Books {
    fn roll(&mut self, now: Instant) {
        if self.start.is_some_and(|start| now < start + WINDOW) {
            return;
        }
        // A second without any request between makes the node quiet.
        self.previous = match self.start {
            Some(start) if now < start + 2 * WINDOW => self.counts.len(),
            _ => 0,
        };
        self.counts.clear();
        self.start = Some(now);
    }
    fn charge(&mut self, book: [u8; 32], now: Instant) -> bool {
        self.roll(now);
        let fresh = !self.counts.contains_key(&book);
        let active = self.previous.max(self.counts.len() + usize::from(fresh));
        let allowance = if active * BOOK_RATE <= SHARED_RATE {
            BOOK_RATE
        } else {
            (SHARED_RATE / active).max(1)
        };
        // A refused book still counts as one that asked.
        let count = self.counts.entry(book).or_default();
        if *count >= allowance {
            return false;
        }
        *count += 1;
        true
    }
}

#[derive(Default)]
struct State {
    enabled: bool,
    units: BTreeSet<PeerId>,
    /// Units that showed their record, until their introduction ends.
    introduced: BTreeMap<PeerId, u64>,
    /// Accepted passes: the book and when it stops letting the peer in.
    passes: BTreeMap<PeerId, ([u8; 32], u64)>,
    addresses: BTreeMap<PeerId, IpAddr>,
    penalized_peers: BTreeMap<PeerId, Instant>,
    penalized_addresses: BTreeMap<IpAddr, Instant>,
    peer_book_reads: BTreeMap<PeerId, Instant>,
    address_book_reads: BTreeMap<IpAddr, Instant>,
    probation: Probation,
    books: Books,
    probation_refused: u64,
    book_refused: u64,
    penalties: u64,
}

impl State {
    fn principal(&self, peer: &PeerId, wall: u64) -> Option<Principal> {
        if self.units.contains(peer) || self.introduced.get(peer).is_some_and(|until| *until > wall)
        {
            return Some(Principal::Unit);
        }
        self.passes
            .get(peer)
            .filter(|(_, until)| *until > wall)
            .map(|(book, _)| Principal::Book(*book))
    }
    fn probation(&mut self, peer: PeerId, now: Instant) -> bool {
        self.penalized_peers.retain(|_, until| *until > now);
        self.penalized_addresses.retain(|_, until| *until > now);
        let address = self
            .addresses
            .get(&peer)
            .copied()
            .filter(|a| !a.is_loopback());
        if self.penalized_peers.contains_key(&peer)
            || address.is_some_and(|a| self.penalized_addresses.contains_key(&a))
        {
            return false;
        }
        let p = &mut self.probation;
        if p.start.is_none_or(|start| now >= start + WINDOW) {
            *p = Probation {
                start: Some(now),
                ..Probation::default()
            };
        }
        if p.total >= PROBATION_RATE
            || p.peers.get(&peer).copied().unwrap_or(0) >= PROBATION_PEER_RATE
            || address.is_some_and(|a| {
                p.addresses.get(&a).copied().unwrap_or(0) >= PROBATION_ADDRESS_RATE
            })
        {
            return false;
        }
        p.total += 1;
        *p.peers.entry(peer).or_default() += 1;
        if let Some(a) = address {
            *p.addresses.entry(a).or_default() += 1;
        }
        true
    }
}

/// Shared by the mailbox protocol's codecs and the runtime.
#[derive(Clone, Default)]
pub(in crate::runtime) struct Gate(Arc<Mutex<State>>);

impl Gate {
    pub fn new() -> Self {
        Self::default()
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// On for a node that reads the chain: it takes payment, so it asks for
    /// passes too.
    pub fn set_enabled(&self, enabled: bool) {
        self.state().enabled = enabled;
    }
    pub fn enabled(&self) -> bool {
        self.state().enabled
    }
    /// Whether an inbound request of `peer`, connected from `address`, is
    /// read, and under which principal.
    pub fn admit(&self, peer: PeerId, address: Option<IpAddr>) -> io::Result<Admission> {
        let mut state = self.state();
        if !state.enabled {
            return Ok(Admission::Open);
        }
        if let Some(address) = address {
            if state.addresses.len() >= MAX_ADDRESSES
                && !state.addresses.contains_key(&peer)
                && let Some(first) = state.addresses.keys().next().copied()
            {
                state.addresses.remove(&first);
            }
            state.addresses.insert(peer, address);
        }
        let now = clock::instant();
        let wall = clock::wall().unwrap_or(0);
        match state.principal(&peer, wall) {
            Some(Principal::Unit) => Ok(Admission::Unit),
            Some(Principal::Book(book)) => {
                if state.books.charge(book, now) {
                    Ok(Admission::Book(book))
                } else {
                    state.book_refused += 1;
                    Err(io::Error::other("book rate exhausted"))
                }
            }
            None => {
                if state.probation(peer, now) {
                    Ok(Admission::Probation)
                } else {
                    state.probation_refused += 1;
                    Err(io::Error::other("access rate exhausted"))
                }
            }
        }
    }
    /// Who `peer` is let in as now, if anyone.
    pub fn principal(&self, peer: &PeerId) -> Option<Principal> {
        let wall = clock::wall().unwrap_or(0);
        self.state().principal(peer, wall)
    }
    pub fn accept_book(&self, peer: PeerId, book: [u8; 32], until: u64) {
        let mut state = self.state();
        let wall = clock::wall().unwrap_or(0);
        state.passes.retain(|_, (_, end)| *end > wall);
        let mut peers: Vec<(u64, PeerId)> = state
            .passes
            .iter()
            .filter(|(p, (b, _))| *b == book && **p != peer)
            .map(|(p, (_, end))| (*end, *p))
            .collect();
        if peers.len() >= PEERS_PER_BOOK {
            peers.sort();
            for (_, old) in &peers[..=peers.len() - PEERS_PER_BOOK] {
                state.passes.remove(old);
            }
        }
        state.passes.insert(peer, (book, until));
    }
    pub fn accept_unit(&self, peer: PeerId, until: u64) {
        let mut state = self.state();
        let wall = clock::wall().unwrap_or(0);
        state.introduced.retain(|_, end| *end > wall);
        state.introduced.insert(peer, until);
    }
    /// The units the directory lists.
    pub fn set_units(&self, peers: BTreeSet<PeerId>) {
        self.state().units = peers;
    }
    /// `peer` showed a bad credential: it and its address wait.
    pub fn penalize(&self, peer: &PeerId) {
        let mut state = self.state();
        let until = clock::instant() + PENALTY;
        state.penalties += 1;
        state.penalized_peers.insert(*peer, until);
        if let Some(address) = state
            .addresses
            .get(peer)
            .copied()
            .filter(|a| !a.is_loopback())
        {
            state.penalized_addresses.insert(address, until);
        }
    }
    /// Whether an unknown book `peer` showed may be read from the chain now.
    pub fn may_read_book(&self, peer: &PeerId) -> bool {
        let mut state = self.state();
        let now = clock::instant();
        let recent = |last: Option<&Instant>| last.is_some_and(|at| now < *at + BOOK_READ_EVERY);
        let address = state
            .addresses
            .get(peer)
            .copied()
            .filter(|a| !a.is_loopback());
        if recent(state.peer_book_reads.get(peer))
            || address.is_some_and(|a| recent(state.address_book_reads.get(&a)))
        {
            return false;
        }
        state
            .peer_book_reads
            .retain(|_, at| now < *at + BOOK_READ_EVERY);
        state
            .address_book_reads
            .retain(|_, at| now < *at + BOOK_READ_EVERY);
        state.peer_book_reads.insert(*peer, now);
        if let Some(a) = address {
            state.address_book_reads.insert(a, now);
        }
        true
    }
    /// A restart: passes and introductions live in memory only.
    #[cfg(test)]
    pub fn forget_passes(&self) {
        let mut state = self.state();
        state.passes.clear();
        state.introduced.clear();
    }
    pub fn info(&self) -> Value {
        let state = self.state();
        json!({
            "enabled": state.enabled,
            "units": state.units.len(),
            "introduced": state.introduced.len(),
            "passes": state.passes.len(),
            "penalized": state.penalized_peers.len(),
            "penalties": state.penalties,
            "probationRefused": state.probation_refused,
            "bookRefused": state.book_refused,
        })
    }
}
