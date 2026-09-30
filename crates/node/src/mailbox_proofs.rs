//! Proof propagation of the mailbox swarm (Docs/V1_STORAGE_REDESIGN_2026_09_24.md):
//! every node — holder, sender or reader — asks one member of its directory
//! each round for the proofs it learned since last time, keeps those that
//! verify here and so blocks the book, holder or grant issuer they prove
//! against. Proofs spread from node to node; nothing trusts a complaint.
use super::mailbox_client::{Purpose, Slots};
use super::mailbox_holder::{MAX_PROOFS, Proofs, Request, Response};
use super::*;
use agentic_mailbox_swarm::Account;
use std::collections::BTreeMap;

/// How often a node asks a member for proofs.
const PROOF_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct Gossip {
    due: Option<Instant>,
    in_flight: bool,
    /// Per member, how many of its proofs this node has read.
    cursors: BTreeMap<[u8; 32], u64>,
    /// Receipt accounts of holders proven to equivocate.
    pub(super) blocked: BTreeSet<Account>,
    pub(super) learned: u64,
}

impl Gossip {
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.due
    }
}

impl Runtime {
    pub(super) fn maintain_proofs(&mut self, instant: Instant) {
        let client = &self.mailbox_client;
        if client.gossip.in_flight || client.gossip.due.is_some_and(|due| due > instant) {
            return;
        }
        if client.gossip.due.is_none()
            && let Ok(blocked) = self.mailbox_holder.blocked_holders()
        {
            // Proofs kept before a restart block from the start.
            self.mailbox_client.gossip.blocked = blocked;
        }
        let own = self.mailbox_holder.unit();
        let client = &self.mailbox_client;
        let members: Vec<[u8; 32]> = client
            .directory
            .keys()
            .copied()
            .filter(|unit| Some(*unit) != own && client.usable(unit))
            .collect();
        if members.is_empty() {
            return;
        }
        let mut pick = [0; 8];
        if random::fill(&mut pick).is_err() {
            return;
        }
        let unit = members[(u64::from_be_bytes(pick) % members.len() as u64) as usize];
        let mut slots: Slots = self.mailbox_client.slots();
        if !slots.take(&unit) {
            return;
        }
        let client = &self.mailbox_client;
        let Some(holder) = client.directory.get(&unit).cloned() else {
            return;
        };
        let after = client.gossip.cursors.get(&unit).copied().unwrap_or(0);
        let gossip = &mut self.mailbox_client.gossip;
        gossip.due = Some(instant + PROOF_INTERVAL);
        gossip.in_flight = true;
        self.send_mailbox(
            holder,
            Request::Proofs { after },
            Purpose::Proofs { unit, after },
        );
    }

    pub(super) fn proofs_answer(
        &mut self,
        unit: [u8; 32],
        after: u64,
        outcome: std::result::Result<Response, String>,
    ) {
        self.mailbox_client.gossip.in_flight = false;
        let Ok(Response::Proofs { proofs, next }) = outcome else {
            return;
        };
        let Ok(proofs) = Proofs::try_from(&proofs) else {
            return;
        };
        let count = proofs.senders.len() + proofs.holders.len() + proofs.grants.len();
        if let Ok(kept) = self.mailbox_holder.accept_proofs(&proofs) {
            self.mailbox_client.gossip.learned +=
                (kept.senders.len() + kept.holders.len() + kept.grants.len()) as u64;
        }
        if let Ok(blocked) = self.mailbox_holder.blocked_holders() {
            self.mailbox_client.gossip.blocked = blocked;
        }
        let gossip = &mut self.mailbox_client.gossip;
        gossip
            .cursors
            .insert(unit, next.unwrap_or(after + count as u64));
        // A full page: the member has more; ask again right away.
        if next.is_some() && count == MAX_PROOFS {
            gossip.due = None;
        }
    }
}
