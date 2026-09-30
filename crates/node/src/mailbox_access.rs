//! Access by book (Docs/V1_DISCOVERY_2026_09_27.md, part 1): the mailbox
//! protocol takes requests only from registry units and from peers that
//! showed a pass of an active book. This node lets others in as a server and
//! shows its own pass or unit record as a client.
use super::mailbox_client::{Holder, Purpose};
use super::mailbox_holder::{
    ACCESS_GRACE, CredentialWire, Refusal, Request, Response, UnitRecordWire,
};
use super::*;
use agentic_mailbox_swarm::access::AccessPass;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period};
use agentic_mailbox_swarm::directory::UnitRecord;
use agentic_mailbox_swarm::stamp::BookTerms;
use std::collections::{BTreeMap, BTreeSet};

/// A request that waits for a peer to let this node in.
type Waiting = (Holder, Request, Purpose);

/// The client side: whom this node showed what.
#[derive(Default)]
pub(super) struct Access {
    /// Peers that let this node in, and the day they did it for.
    accepted: BTreeMap<PeerId, u64>,
    /// Peers that did not know this unit, and the day they said so.
    strangers: BTreeMap<PeerId, u64>,
    /// Requests waiting for their peer to let this node in.
    waiting: BTreeMap<PeerId, Vec<Waiting>>,
    /// Peers looking at this node's credential now.
    showing: BTreeSet<PeerId>,
    /// When the credential may be shown to a peer again: after a refusal,
    /// or a moment after a peer forgot it.
    due: BTreeMap<PeerId, Instant>,
    /// What this node could show last time it looked.
    credential: Option<&'static str>,
    shown: u64,
    refused: BTreeMap<String, u64>,
}

impl Access {
    pub(super) fn info(&self) -> Value {
        json!({
            "credential": self.credential.unwrap_or("none"),
            "accepted": self.accepted.len(),
            "waiting": self.waiting.values().map(Vec::len).sum::<usize>(),
            "waitingPeers": self.waiting.len(),
            "showing": self.showing.len(),
            "shown": self.shown,
            "refused": self.refused,
        })
    }
    /// Requests counted as in flight while they wait for their peer.
    pub(super) fn waiting(&self) -> impl Iterator<Item = &Purpose> {
        self.waiting
            .values()
            .flatten()
            .map(|(_, _, purpose)| purpose)
    }
    /// When a credential waits to be shown again.
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.waiting
            .keys()
            .filter(|peer| !self.showing.contains(*peer))
            .filter_map(|peer| self.due.get(peer))
            .min()
            .copied()
    }
}

/// How long to wait before showing a peer the credential again after it
/// refused it: soon while it reads a book or rules, long after a refusal
/// that penalized this node.
fn retry_after(refusal: &str) -> Duration {
    Duration::from_secs(match refusal {
        "unknown_book" | "grant_pending" => 5,
        "bad_pass" | "grant" | "malformed" => 300,
        _ => 60,
    })
}

/// A request failed because its peer refused this node's credential only
/// while it reads the book or the grant rules: shown again in seconds.
pub(super) fn let_in_soon(error: &str) -> bool {
    matches!(error, "access: unknown_book" | "access: grant_pending")
}

/// A peer that forgot this node's credential is shown it after the
/// requests already on their way were refused, in a fresh second of its
/// small path.
const SHOW_AGAIN_AFTER: Duration = Duration::from_secs(1);

/// When a pass or introduction of `day` stops letting its peer in.
fn day_end(day: u64) -> u64 {
    day.saturating_add(1)
        .saturating_mul(PERIOD_SECONDS)
        .saturating_add(ACCESS_GRACE)
}

impl Runtime {
    /// Send `request` to `holder`. A peer that has not let this node in
    /// today is shown its credential first while the request waits.
    pub(super) fn send_mailbox(&mut self, holder: Holder, request: Request, purpose: Purpose) {
        let now = clock::wall();
        let (true, false, Ok(now)) = (
            self.chain_configured(),
            matches!(request, Request::Access { .. }),
            now,
        ) else {
            self.send_raw(holder, request, purpose);
            return;
        };
        let day = period(now);
        let peer = holder.peer;
        let access = &mut self.mailbox_client.access;
        access.accepted.retain(|_, d| *d == day);
        access.strangers.retain(|_, d| *d == day);
        if !access.waiting.contains_key(&peer)
            && (access.accepted.contains_key(&peer)
                // A unit goes straight to peers: they know it from the
                // directory.
                || (self.mailbox_holder.unit().is_some() && !access.strangers.contains_key(&peer)))
        {
            return self.send_gated(holder, request, purpose);
        }
        access
            .waiting
            .entry(peer)
            .or_default()
            .push((holder, request, purpose));
        self.show_access(peer, now);
    }

    /// Send a request that a peer may refuse for want of a credential; a
    /// copy is kept to send again behind the credential.
    fn send_gated(&mut self, holder: Holder, request: Request, purpose: Purpose) {
        let copy = (holder.clone(), request.clone());
        let id = self.send_raw(holder, request, purpose);
        self.mailbox_client.access_copies.insert(id, copy);
    }

    /// Show `peer` this node's credential when it is due; with nothing to
    /// show, the requests waiting for it fail.
    fn show_access(&mut self, peer: PeerId, now: u64) {
        let instant = clock::instant();
        let access = &mut self.mailbox_client.access;
        if access.showing.contains(&peer) || access.due.get(&peer).is_some_and(|due| *due > instant)
        {
            return;
        }
        access.due.remove(&peer);
        let Some((holder, _, purpose)) = access.waiting.get(&peer).and_then(|w| w.first()) else {
            return;
        };
        let (holder, unit) = (holder.clone(), purpose.unit());
        let Some(credential) = self.access_credential(now) else {
            for (_, _, purpose) in self
                .mailbox_client
                .access
                .waiting
                .remove(&peer)
                .unwrap_or_default()
            {
                self.mailbox_client
                    .local
                    .push((purpose, Err("access: book_required".into())));
            }
            return;
        };
        let access = &mut self.mailbox_client.access;
        access.showing.insert(peer);
        access.shown += 1;
        self.send_raw(
            holder,
            Request::Access { credential },
            Purpose::Access { peer, unit },
        );
    }

    /// Show credentials that came due.
    pub(super) fn maintain_access(&mut self) {
        let Ok(now) = clock::wall() else { return };
        let peers: Vec<PeerId> = self.mailbox_client.access.waiting.keys().copied().collect();
        for peer in peers {
            self.show_access(peer, now);
        }
    }

    /// What this node shows: its record while it is an active unit, else a
    /// pass of its active book that lasts longest.
    fn access_credential(&mut self, now: u64) -> Option<CredentialWire> {
        if let Some(record) = self.own_unit_record() {
            self.mailbox_client.access.credential = Some("unit");
            return Some(CredentialWire::Unit {
                record: UnitRecordWire::from(&record),
            });
        }
        let access = self
            .core
            .mailbox_access(self.own_transport_key(), period(now), now)
            .ok()
            .flatten();
        self.mailbox_client.access.credential =
            Some(if access.is_some() { "book" } else { "none" });
        access.map(|access| CredentialWire::Pass {
            pass: (&access.pass).into(),
            grant: access.grant,
        })
    }

    /// A peer answered this node's credential.
    pub(super) fn access_answer(
        &mut self,
        peer: PeerId,
        outcome: std::result::Result<Response, String>,
    ) {
        let access = &mut self.mailbox_client.access;
        access.showing.remove(&peer);
        match outcome {
            Ok(Response::Access { .. }) => {
                let day = clock::wall().map(period).unwrap_or_default();
                access.accepted.insert(peer, day);
                access.strangers.remove(&peer);
                for (holder, request, purpose) in access.waiting.remove(&peer).unwrap_or_default() {
                    self.send_gated(holder, request, purpose);
                }
            }
            Ok(Response::Refused { code }) => {
                *access.refused.entry(code.clone()).or_default() += 1;
                access
                    .due
                    .insert(peer, clock::instant() + retry_after(&code));
                // Not let in (yet): the lanes fail these as they would a
                // transport failure, so they hold no request slot, and retry
                // on their own schedules.
                self.fail_waiting(peer, &format!("access: {code}"));
            }
            Ok(_) => self.fail_waiting(peer, "access: unexpected answer"),
            Err(error) => self.fail_waiting(peer, &format!("access: {error}")),
        }
    }

    fn fail_waiting(&mut self, peer: PeerId, error: &str) {
        let waiting = self
            .mailbox_client
            .access
            .waiting
            .remove(&peer)
            .unwrap_or_default();
        for (_, _, purpose) in waiting {
            self.purpose_outcome(purpose, Err(error.to_owned()));
        }
    }

    /// `peer` refused a request for want of a credential (it restarted, or
    /// never knew this unit): the request waits and goes again behind it.
    pub(super) fn access_lost(
        &mut self,
        peer: PeerId,
        sent: Option<(Holder, Request)>,
        purpose: Purpose,
    ) {
        let day = clock::wall().map(period).unwrap_or_default();
        let unit = self.mailbox_holder.unit().is_some();
        let access = &mut self.mailbox_client.access;
        access.accepted.remove(&peer);
        if unit {
            access.strangers.insert(peer, day);
        }
        let Some((holder, request)) = sent else {
            return self.purpose_outcome(purpose, Err("access: access_required".into()));
        };
        if !access.showing.contains(&peer) && !access.waiting.contains_key(&peer) {
            access.due.insert(peer, clock::instant() + SHOW_AGAIN_AFTER);
        }
        access
            .waiting
            .entry(peer)
            .or_default()
            .push((holder, request, purpose));
    }

    /// Let `peer` in, or say why not. A bad credential makes it and its
    /// address wait.
    pub(super) fn serve_access(&mut self, peer: PeerId, credential: CredentialWire) -> Response {
        let refused = |refusal: Refusal| Response::Refused {
            code: refusal.code().into(),
        };
        let Ok(now) = clock::wall() else {
            return refused(Refusal::Storage);
        };
        if !self.access_gate.enabled() {
            return Response::Access {
                until: day_end(period(now)),
            };
        }
        let outcome = match credential {
            CredentialWire::Pass { pass, grant } => self.accept_pass(peer, &pass, grant, now),
            CredentialWire::Unit { record } => self.accept_unit(peer, &record, now),
        };
        match outcome {
            Ok(until) => Response::Access { until },
            Err(refusal) => {
                if matches!(refusal, Refusal::Pass | Refusal::Grant | Refusal::Malformed) {
                    self.access_gate.penalize(&peer);
                }
                refused(refusal)
            }
        }
    }

    fn accept_pass(
        &mut self,
        peer: PeerId,
        wire: &super::mailbox_holder::AccessPassWire,
        grant: Option<agentic_grant_book::GrantBook>,
        now: u64,
    ) -> std::result::Result<u64, Refusal> {
        let pass = AccessPass::try_from(wire)?;
        if mailbox_directory::peer_of(&pass.peer) != Some(peer) {
            return Err(Refusal::Pass);
        }
        let terms = match (self.mailbox_holder.book_terms(&pass.book), grant) {
            (Some(terms), _) => terms,
            // A granted book is let in on its issuer's rules before its
            // notaries vouch for it; it pays for nothing until then.
            (None, Some(grant)) if grant.id() == pass.book => self.grant_terms(&grant, now)?,
            (None, Some(_)) => return Err(Refusal::Pass),
            (None, None) => {
                if self.access_gate.may_read_book(&peer) {
                    self.book_wanted(pass.book, false);
                }
                return Err(Refusal::UnknownBook);
            }
        };
        let until = self
            .mailbox_holder
            .check_access(&pass, &pass.peer, &terms, now)?;
        self.access_gate.accept_book(peer, pass.book, until);
        Ok(until)
    }

    fn grant_terms(
        &mut self,
        grant: &agentic_grant_book::GrantBook,
        now: u64,
    ) -> std::result::Result<BookTerms, Refusal> {
        if !self.grant_rules_current(grant.server, grant.day) {
            return Err(Refusal::GrantPending);
        }
        match self.mailbox_holder.check_grant(grant, now) {
            Ok(()) => Ok(BookTerms {
                key: grant.book,
                count: grant.count,
                valid_until: grant.expiry,
            }),
            Err(Refusal::GrantPending) if !self.chain_configured() => Err(Refusal::Grant),
            Err(refusal) => Err(refusal),
        }
    }

    fn accept_unit(
        &mut self,
        peer: PeerId,
        wire: &UnitRecordWire,
        now: u64,
    ) -> std::result::Result<u64, Refusal> {
        let record = UnitRecord::try_from(wire)?;
        let unit = record.verify(&NETWORK_DOMAIN).map_err(|_| Refusal::Pass)?;
        if mailbox_directory::peer_of(&record.transport_key) != Some(peer) {
            return Err(Refusal::Pass);
        }
        if !self.directory_lists(&unit) {
            return Err(Refusal::UnknownUnit);
        }
        let until = day_end(period(now));
        self.access_gate.accept_unit(peer, until);
        // Its record is as good as one pulled from a peer.
        self.directory_page(vec![wire.clone()]);
        Ok(until)
    }
}
