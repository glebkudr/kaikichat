//! Notary lane of the mailbox swarm (Docs/V1_STORAGE_REDESIGN_2026_09_24.md):
//! a spent book slot, or an issuer's grant serial, is put on record with the
//! ten units nearest to its key, where the first writer wins. The sender
//! records each new stamp and each grant it receives; every holder records
//! each entry it newly stores and, when a notary answers with another first
//! statement for the key, keeps the two as proof — even when the two spends
//! went to swarms that never meet.
use super::mailbox_client::{Purpose, Slots};
use super::mailbox_holder::{AnswerWire, MAX_NOTARIZE, Proofs, Request, Response, Statement};
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Statements waiting for their notaries; older ones are dropped beyond it.
const MAX_QUEUED: usize = 1_024;
/// Notary requests one node keeps in flight: the lane stays in the
/// background, and what piles up meanwhile goes out in one batch per notary.
pub(super) const IN_FLIGHT: usize = 2;
/// At most one batch per notary this often, so the notary lane never takes
/// a notary's request rate from stores and reads.
pub(super) const PACE: Duration = Duration::from_secs(1);

/// One statement still to put on record with some of its notaries.
struct Pending {
    statement: Statement,
    units: Vec<[u8; 32]>,
}

#[derive(Default)]
pub(super) struct Lane {
    queue: VecDeque<Pending>,
    /// When the next batch may go to each notary.
    due: BTreeMap<[u8; 32], Instant>,
    /// Turns the notaries over from batch to batch, from a start of this
    /// node's own, so nodes do not all ask the same notary first.
    turn: usize,
    pub(super) notarized: u64,
    pub(super) proofs: u64,
    pub(super) batches: u64,
}

impl Lane {
    pub(super) fn waiting(&self) -> usize {
        self.queue.len()
    }
    /// When a notary with statements waiting may next be sent a batch.
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.queue
            .iter()
            .flat_map(|pending| pending.units.iter())
            .filter_map(|unit| self.due.get(unit))
            .min()
            .copied()
    }
}

impl Runtime {
    /// Queue a statement for its notaries.
    pub(super) fn notarize_later(&mut self, statement: Statement) {
        let units = self.notaries_of(&statement.key(&NETWORK_DOMAIN));
        let lane = &mut self.mailbox_client.notary;
        if lane.queue.len() >= MAX_QUEUED {
            lane.queue.pop_front();
        }
        lane.queue.push_back(Pending { statement, units });
    }

    pub(super) fn maintain_notary(&mut self) {
        for stamp in self.mailbox_holder.take_fresh() {
            self.notarize_later(Statement::Ticket(stamp));
        }
        let Ok(now) = now() else { return };
        // This node is one of some keys' notaries itself.
        if let Some(own) = self.mailbox_holder.unit() {
            let mut local = Vec::new();
            for pending in &mut self.mailbox_client.notary.queue {
                if let Some(at) = pending.units.iter().position(|unit| *unit == own) {
                    pending.units.remove(at);
                    local.push(pending.statement.clone());
                }
            }
            for statement in local {
                let noted = self.mailbox_holder.notarize(&statement, now);
                self.mailbox_client.notary.notarized += 1;
                if let (Statement::Commit(claim), Ok(noted)) = (&statement, noted)
                    && let Statement::Commit(first) = &noted.first
                {
                    self.group_answer(own, claim, first);
                }
            }
        }
        let busy: BTreeSet<[u8; 32]> = self
            .mailbox_client
            .requests
            .values()
            .filter_map(|purpose| match purpose {
                Purpose::Notarize { unit, .. } => Some(*unit),
                _ => None,
            })
            .collect();
        let mut room = IN_FLIGHT.saturating_sub(busy.len());
        let mut slots: Slots = self.mailbox_client.slots();
        let start = usize::from(
            self.swarm
                .local_peer_id()
                .to_bytes()
                .last()
                .copied()
                .unwrap_or(0),
        );
        let instant = clock::instant();
        let client = &mut self.mailbox_client;
        let lane = &mut client.notary;
        lane.due.retain(|_, due| *due > instant);
        let units: Vec<[u8; 32]> = lane
            .queue
            .iter()
            .flat_map(|pending| pending.units.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|unit| !busy.contains(unit) && !lane.due.contains_key(unit))
            .collect();
        let mut outgoing = Vec::new();
        for n in 0..units.len() {
            if room == 0 {
                break;
            }
            let unit = units[(start + lane.turn + n) % units.len()];
            let Some(holder) = client
                .directory
                .get(&unit)
                .filter(|holder| !client.gossip.blocked.contains(&holder.account))
            else {
                // Neither listed nor usable: nothing more goes to it.
                for pending in &mut lane.queue {
                    pending.units.retain(|u| *u != unit);
                }
                continue;
            };
            if !slots.take(&unit) {
                continue;
            }
            let mut statements = Vec::new();
            for pending in &mut lane.queue {
                if statements.len() == MAX_NOTARIZE {
                    break;
                }
                if let Some(at) = pending.units.iter().position(|u| *u == unit) {
                    pending.units.remove(at);
                    statements.push(pending.statement.clone());
                }
            }
            room -= 1;
            lane.batches += 1;
            lane.due.insert(unit, instant + PACE);
            outgoing.push((
                holder.clone(),
                Request::Notarize {
                    statements: statements.iter().map(Into::into).collect(),
                },
                Purpose::Notarize { statements, unit },
            ));
        }
        lane.turn = lane.turn.wrapping_add(outgoing.len());
        lane.queue.retain(|pending| !pending.units.is_empty());
        for (holder, request, purpose) in outgoing {
            self.send_mailbox(holder, request, purpose);
        }
    }

    /// A notary answered a batch: another first statement for one of our
    /// keys is a double spend, or a serial granted twice, that we can prove.
    pub(super) fn notary_answer(
        &mut self,
        unit: [u8; 32],
        submitted: Vec<Statement>,
        outcome: std::result::Result<Response, String>,
    ) {
        let Ok(Response::Notarized { answers }) = outcome else {
            // Not let in yet (access by book): the next batch to this notary
            // carries them again.
            if matches!(&outcome, Err(error) if error.starts_with("access")) {
                let lane = &mut self.mailbox_client.notary;
                for statement in submitted {
                    if lane.queue.len() >= MAX_QUEUED {
                        lane.queue.pop_front();
                    }
                    lane.queue.push_back(Pending {
                        statement,
                        units: vec![unit],
                    });
                }
            }
            return;
        };
        // Answers match statements by position only when every one came.
        if answers.len() != submitted.len() {
            return;
        }
        for (submitted, answer) in submitted.into_iter().zip(answers) {
            let AnswerWire::Noted { first, .. } = answer else {
                continue;
            };
            self.mailbox_client.notary.notarized += 1;
            if let (Ok(Statement::Commit(first)), Statement::Commit(claim)) =
                (Statement::try_from(&first), &submitted)
            {
                self.group_answer(unit, claim, &first);
                continue;
            }
            let proven = match (Statement::try_from(&first), submitted) {
                (Ok(Statement::Ticket(first)), Statement::Ticket(submitted)) => {
                    first.operation != submitted.operation
                        && first.ticket_id(&NETWORK_DOMAIN) == submitted.ticket_id(&NETWORK_DOMAIN)
                        && self
                            .mailbox_holder
                            .keep_sender_proof(&first, &submitted)
                            .unwrap_or(false)
                }
                (Ok(Statement::Grant(first)), Statement::Grant(submitted)) => {
                    first != submitted
                        && first.id() == submitted.id()
                        && self
                            .mailbox_holder
                            .accept_proofs(&Proofs {
                                grants: vec![agentic_grant_book::GrantEquivocation {
                                    first,
                                    second: submitted,
                                }],
                                ..Proofs::default()
                            })
                            .is_ok_and(|kept| !kept.grants.is_empty())
                }
                _ => false,
            };
            if proven {
                self.mailbox_client.notary.proofs += 1;
            }
        }
    }
}
