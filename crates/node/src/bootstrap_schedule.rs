//! Finite discovery work: held slots, bounded source merging, backoff and admission.
use super::*;

#[derive(Clone)]
pub(in crate::runtime) struct Hint {
    pub(in crate::runtime) peer: PeerId,
    pub(in crate::runtime) addresses: Vec<Multiaddr>,
    pub(in crate::runtime) root: Option<[u8; 32]>,
}
struct Candidate {
    hint: Hint,
    due: Instant,
    attempt: u32,
    active: bool,
}
pub(in crate::runtime) struct Schedule {
    explicit: Vec<Hint>,
    candidates: HashMap<PeerId, Candidate>,
}
impl Schedule {
    pub(in crate::runtime) fn new(explicit: Vec<Hint>, now: Instant) -> Self {
        let mut schedule = Self {
            explicit,
            candidates: HashMap::new(),
        };
        schedule.replace_sources(vec![], vec![], now);
        schedule
    }
    #[cfg(test)]
    pub(in crate::runtime) fn replace_cached(&mut self, cached: Vec<Hint>, now: Instant) {
        self.replace_sources(cached, vec![], now);
    }
    pub(in crate::runtime) fn replace_sources(
        &mut self,
        cached: Vec<Hint>,
        lan: Vec<Hint>,
        now: Instant,
    ) {
        let mut hints: HashMap<_, _> = self
            .explicit
            .iter()
            .take(4)
            .cloned()
            .map(|h| (h.peer, h))
            .collect();
        // A cache refresh cannot forget work already holding an outbound request slot.
        for (peer, candidate) in &self.candidates {
            if candidate.active {
                hints.entry(*peer).or_insert_with(|| candidate.hint.clone());
            }
        }
        for mut hint in cached.into_iter().take(64) {
            if let Some(explicit) = self.explicit.iter().find(|h| h.peer == hint.peer) {
                // The given route first: a signed record kept for a day may
                // list only addresses its peer has left.
                let mut addresses = explicit.addresses.clone();
                for address in hint.addresses {
                    if addresses.len() < 8 && !addresses.contains(&address) {
                        addresses.push(address);
                    }
                }
                hint.addresses = addresses;
            }
            if hints.len() < 68 || hints.contains_key(&hint.peer) {
                hints.insert(hint.peer, hint);
            }
        }
        for hint in lan.into_iter().take(32) {
            if let Some(existing) = hints.get_mut(&hint.peer) {
                // LAN can repair an address, but cannot erase a signed cache's expected root.
                for address in hint.addresses.into_iter().take(4) {
                    if existing.addresses.len() < 8 && !existing.addresses.contains(&address) {
                        existing.addresses.push(address);
                    }
                }
            } else if hints.len() < 100 {
                hints.insert(hint.peer, hint);
            }
        }
        self.candidates.retain(|peer, _| hints.contains_key(peer));
        for (peer, hint) in hints {
            if let Some(candidate) = self.candidates.get_mut(&peer) {
                candidate.hint = hint;
            } else {
                self.candidates.insert(
                    peer,
                    Candidate {
                        hint,
                        due: now,
                        attempt: 0,
                        active: false,
                    },
                );
            }
        }
    }
    pub(in crate::runtime) fn len(&self) -> usize {
        self.candidates.len()
    }
    pub(in crate::runtime) fn blocked(&self, relay_only: bool) -> usize {
        self.candidates
            .values()
            .filter(|c| relay_only && !c.hint.addresses.iter().any(relay_support::is_circuit))
            .count()
    }
    pub(in crate::runtime) fn ready(&mut self, now: Instant, relay_only: bool) -> Vec<Hint> {
        let available =
            4usize.saturating_sub(self.candidates.values().filter(|c| c.active).count());
        let mut ready: Vec<_> = self
            .candidates
            .iter()
            .filter(|(_, c)| {
                !c.active
                    && c.due <= now
                    && c.hint
                        .addresses
                        .iter()
                        .any(|a| !relay_only || relay_support::is_circuit(a))
            })
            .map(|(peer, c)| (*peer, c.due, !self.explicit.iter().any(|h| h.peer == *peer)))
            .collect();
        ready.sort_by_key(|(peer, due, cached)| (*due, *cached, peer.to_bytes()));
        ready
            .into_iter()
            .take(available)
            .filter_map(|(peer, _, _)| {
                let candidate = self.candidates.get_mut(&peer)?;
                candidate.active = true;
                Some(candidate.hint.clone())
            })
            .collect()
    }
    /// A changed own route is announced to healthy candidates now; failure
    /// backoff and in-flight requests keep their schedule.
    pub(in crate::runtime) fn announce(&mut self, peers: &[PeerId], now: Instant) {
        for peer in peers {
            if let Some(candidate) = self.candidates.get_mut(peer)
                && !candidate.active
                && candidate.attempt == 0
            {
                candidate.due = candidate.due.min(now);
            }
        }
    }
    pub(in crate::runtime) fn finished(&mut self, peer: PeerId, success: bool, now: Instant) {
        if let Some(candidate) = self.candidates.get_mut(&peer) {
            candidate.active = false;
            let delay = if success {
                candidate.attempt = 0;
                30_000
            } else {
                let delay = 500u64
                    .saturating_mul(1u64 << candidate.attempt.min(6))
                    .min(30_000);
                candidate.attempt = candidate.attempt.saturating_add(1);
                delay
            };
            candidate.due = now + Duration::from_millis(delay);
        }
    }
    pub(in crate::runtime) fn disconnected(&mut self, peer: PeerId, now: Instant) {
        if let Some(candidate) = self.candidates.get_mut(&peer)
            && !candidate.active
            && candidate.attempt == 0
            && candidate.due > now
        {
            // Only successful refresh has zero failures and a future idle deadline.
            // A lost verified connection consumes the first retry; repeated closes
            // cannot reset failure backoff or release an in-flight request's slot.
            candidate.due = candidate.due.min(now + Duration::from_millis(500));
            candidate.attempt = 1;
        }
    }
}

pub(in crate::runtime) struct Admission<const TOTAL: u32 = 64, const PER_PEER: u32 = 8> {
    since: Instant,
    total: u32,
    peers: HashMap<PeerId, u32>,
}
impl<const TOTAL: u32, const PER_PEER: u32> Admission<TOTAL, PER_PEER> {
    /// Inspect only after `allow` refreshed the window and refused a request.
    pub(in crate::runtime) fn total_exhausted(&self) -> bool {
        self.total >= TOTAL
    }
    pub(in crate::runtime) fn snapshot(&self) -> (u32, u32) {
        (self.total, self.peers.values().sum())
    }
    pub(in crate::runtime) fn top_peers(&self, n: usize) -> Vec<(PeerId, u32)> {
        let mut peers = self
            .peers
            .iter()
            .map(|(peer, count)| (*peer, *count))
            .collect::<Vec<_>>();
        peers.sort_by(|(a_peer, a_count), (b_peer, b_count)| {
            b_count
                .cmp(a_count)
                .then_with(|| a_peer.to_bytes().cmp(&b_peer.to_bytes()))
        });
        peers.truncate(n);
        peers
    }
    pub(in crate::runtime) fn new(now: Instant) -> Self {
        Self {
            since: now,
            total: 0,
            peers: HashMap::new(),
        }
    }
    pub(in crate::runtime) fn allow(&mut self, peer: PeerId, now: Instant) -> bool {
        if now.saturating_duration_since(self.since) >= Duration::from_secs(60) {
            self.since = now;
            self.total = 0;
            self.peers.clear();
        }
        if self.total >= TOTAL || self.peers.get(&peer).copied().unwrap_or(0) >= PER_PEER {
            return false;
        }
        *self.peers.entry(peer).or_default() += 1;
        self.total += 1;
        true
    }
}
