//! Grant books at holders (Docs/V1_AGENT_FIRST_SCOPE_2026_09_25.md): a
//! sender shows the grant funding its book; the holder checks it against its
//! issuer's rules for its day and asks the grant's notaries when they first
//! saw it. A grant most of its notaries, at least four, saw first on its day
//! funds a book like a purchase; a grant shown only later never does.
use super::mailbox_client::{Purpose, Slots};
use super::mailbox_holder::{
    AnswerWire, GrantVerdict, Proofs, Refusal, Request, Response, Statement, StatementWire,
    grant_verdict,
};
use super::*;
use agentic_grant_book::{GrantBook, GrantEquivocation};
use agentic_mailbox_swarm::select::{Member, notaries};
use std::collections::BTreeMap;

/// Grants checked at once; offers beyond wait for a later one.
const MAX_CHECKING: usize = 256;
/// How long a refused grant is answered from memory.
const REFUSED_FOR: Duration = Duration::from_secs(300);
/// Pause before an inconclusive check starts again.
const RETRY_AFTER: Duration = Duration::from_secs(30);
/// Transport attempts per notary before it counts as unreachable.
const ATTEMPTS: u32 = 2;

/// One grant being put to its notaries.
struct Checking {
    grant: GrantBook,
    /// Notaries still to ask, with the attempts made so far.
    to_ask: BTreeMap<[u8; 32], u32>,
    /// Notaries asked and not answered yet.
    waiting: BTreeMap<[u8; 32], u32>,
    /// When the notaries that saw it on its day first saw it.
    in_time: Vec<u64>,
    late: usize,
    unreachable: usize,
    conflict: bool,
}

#[derive(Default)]
pub(super) struct Lane {
    /// By grant digest: two grants of one serial are checked apart.
    checking: BTreeMap<[u8; 32], Checking>,
    /// Decided grants answered without asking until then: refused, or
    /// inconclusive and waiting to be asked again.
    held: BTreeMap<[u8; 32], (Instant, bool)>,
    /// Grants whose notaries this holder started asking.
    pub(super) checked: u64,
    pub(super) learned: u64,
    pub(super) refused: u64,
}

impl Lane {
    pub(super) fn info(&self) -> Value {
        json!({
            "checking": self.checking.len(),
            "checked": self.checked,
            "learned": self.learned,
            "refused": self.refused,
        })
    }
}

fn pending() -> Response {
    Response::Refused {
        code: Refusal::GrantPending.code().into(),
    }
}

impl Runtime {
    /// Keep a grant to the profile's book key and put it on record with its
    /// notaries while it is its day.
    pub(super) fn add_mailbox_grant(
        &mut self,
        grant: &GrantBook,
    ) -> std::result::Result<agentic_core::MailboxBook, agentic_core::CoreError> {
        let added = self.core.add_mailbox_grant(grant)?;
        self.notarize_later(Statement::Grant(grant.clone()));
        Ok(added)
    }

    /// A sender shows the grant of a book this holder does not know yet.
    pub(super) fn offer_grant(&mut self, grant: GrantBook) -> Response {
        let refused = |refusal: Refusal| Response::Refused {
            code: refusal.code().into(),
        };
        let Ok(now) = now() else {
            return refused(Refusal::Storage);
        };
        match self.mailbox_holder.granted(&grant.id()) {
            Ok(Some(known)) if known == grant => return Response::Learned,
            Err(refusal) => return refused(refusal),
            _ => {}
        }
        if !self.grant_rules_current(grant.server, grant.day) {
            return pending();
        }
        match self.mailbox_holder.check_grant(&grant, now) {
            // No chain to read the rules from: nothing to wait for.
            Err(Refusal::GrantPending) if !self.chain_configured() => {
                return refused(Refusal::Grant);
            }
            Err(refusal) => return refused(refusal),
            Ok(()) => {}
        }
        let digest = grant.digest();
        let instant = clock::instant();
        let units = self.notaries_of(&grant.id());
        let lane = &mut self.mailbox_client.grants;
        lane.held.retain(|_, (until, _)| *until > instant);
        if let Some((_, refused_before)) = lane.held.get(&digest) {
            return if *refused_before {
                refused(Refusal::Grant)
            } else {
                pending()
            };
        }
        if lane.checking.contains_key(&digest) || lane.checking.len() >= MAX_CHECKING {
            return pending();
        }
        lane.checked += 1;
        lane.checking.insert(
            digest,
            Checking {
                grant,
                to_ask: units.into_iter().map(|unit| (unit, 0)).collect(),
                waiting: BTreeMap::new(),
                in_time: Vec::new(),
                late: 0,
                unreachable: 0,
                conflict: false,
            },
        );
        pending()
    }

    /// The notaries of `key` among the listed units.
    pub(super) fn notaries_of(&self, key: &[u8; 32]) -> Vec<[u8; 32]> {
        let members: Vec<_> = self
            .mailbox_client
            .directory
            .keys()
            .map(|commitment| Member {
                commitment: *commitment,
            })
            .collect();
        notaries(key, &members)
            .into_iter()
            .map(|m| m.commitment)
            .collect()
    }

    /// Ask the notaries of grants under check and decide those answered.
    pub(super) fn maintain_grants(&mut self, now: u64, instant: Instant) {
        let own = self.mailbox_holder.unit();
        let mut slots: Slots = self.mailbox_client.slots();
        let mut outgoing = Vec::new();
        let mut local = Vec::new();
        let client = &mut self.mailbox_client;
        for (digest, checking) in &mut client.grants.checking {
            let units: Vec<_> = checking.to_ask.keys().copied().collect();
            for unit in units {
                if Some(unit) == own {
                    checking.to_ask.remove(&unit);
                    local.push((*digest, checking.grant.clone()));
                    continue;
                }
                let Some(holder) = client
                    .directory
                    .get(&unit)
                    .filter(|holder| !client.gossip.blocked.contains(&holder.account))
                else {
                    checking.to_ask.remove(&unit);
                    checking.unreachable += 1;
                    continue;
                };
                if !slots.take(&unit) {
                    continue;
                }
                let attempts = checking.to_ask.remove(&unit).unwrap_or(0);
                checking.waiting.insert(unit, attempts + 1);
                outgoing.push((
                    holder.clone(),
                    Request::Notarize {
                        statements: vec![StatementWire::Grant {
                            grant: checking.grant.clone(),
                        }],
                    },
                    Purpose::GrantCheck {
                        digest: *digest,
                        unit,
                    },
                ));
            }
        }
        for (digest, grant) in local {
            // This node is one of the grant's notaries itself.
            let answer = self
                .mailbox_holder
                .notarize(&Statement::Grant(grant), now)
                .map(|noted| (noted.first, noted.first_seen));
            self.note_grant_answer(digest, answer.ok());
        }
        for (holder, request, purpose) in outgoing {
            self.send_mailbox(holder, request, purpose);
        }
        self.decide_grants(now, instant);
    }

    fn decide_grants(&mut self, now: u64, instant: Instant) {
        let decided: Vec<_> = self
            .mailbox_client
            .grants
            .checking
            .iter()
            .filter_map(|(digest, c)| {
                grant_verdict(
                    c.in_time.len(),
                    c.late,
                    c.to_ask.len() + c.waiting.len(),
                    c.unreachable,
                    c.conflict,
                )
                .map(|verdict| (*digest, verdict))
            })
            .collect();
        for (digest, verdict) in decided {
            let Some(mut checking) = self.mailbox_client.grants.checking.remove(&digest) else {
                continue;
            };
            let learned = verdict == GrantVerdict::Learn && {
                checking.in_time.sort_unstable();
                let first_seen = checking.in_time[checking.in_time.len() / 2];
                self.mailbox_holder
                    .learn_grant(&checking.grant, first_seen, now)
                    .is_ok()
            };
            let lane = &mut self.mailbox_client.grants;
            if learned {
                lane.learned += 1;
            } else if verdict == GrantVerdict::Retry {
                lane.held.insert(digest, (instant + RETRY_AFTER, false));
            } else {
                lane.refused += 1;
                lane.held.insert(digest, (instant + REFUSED_FOR, true));
            }
        }
    }

    /// A notary answered about a grant under check.
    pub(super) fn grant_answer(
        &mut self,
        digest: [u8; 32],
        unit: [u8; 32],
        outcome: std::result::Result<Response, String>,
    ) {
        let Some(checking) = self.mailbox_client.grants.checking.get_mut(&digest) else {
            return;
        };
        let Some(attempts) = checking.waiting.remove(&unit) else {
            return;
        };
        let failed = outcome.is_err();
        let answer = match outcome {
            Ok(Response::Notarized { answers }) => answers.into_iter().next(),
            _ => None,
        };
        match answer {
            Some(AnswerWire::Noted {
                first: StatementWire::Grant { grant },
                first_seen,
            }) => self.note_grant_answer(digest, Some((Statement::Grant(grant), first_seen))),
            _ if failed && attempts < ATTEMPTS => {
                checking.to_ask.insert(unit, attempts);
            }
            _ => self.note_grant_answer(digest, None),
        }
    }

    /// Count one notary's answer: the grant it saw first for the serial and
    /// when, or none from an unreachable notary.
    fn note_grant_answer(&mut self, digest: [u8; 32], answer: Option<(Statement, u64)>) {
        let Some(checking) = self.mailbox_client.grants.checking.get(&digest) else {
            return;
        };
        let grant = checking.grant.clone();
        let (in_time, late, conflict) = match answer {
            Some((Statement::Grant(first), first_seen)) if first == grant => {
                let in_time = self.mailbox_holder.grant_in_time(&grant, first_seen);
                (in_time.then_some(first_seen), !in_time, false)
            }
            Some((Statement::Grant(first), _)) if first.id() == grant.id() => {
                // Another grant was first for the serial: the issuer
                // equivocated.
                let proof = GrantEquivocation {
                    first,
                    second: grant,
                };
                let proven = proof.verify().is_ok();
                if proven {
                    let _ = self.mailbox_holder.accept_proofs(&Proofs {
                        grants: vec![proof],
                        ..Proofs::default()
                    });
                }
                (None, false, proven)
            }
            _ => (None, false, false),
        };
        let Some(checking) = self.mailbox_client.grants.checking.get_mut(&digest) else {
            return;
        };
        match (in_time, late, conflict) {
            (Some(first_seen), _, _) => checking.in_time.push(first_seen),
            (None, true, _) => checking.late += 1,
            (None, false, true) => checking.conflict = true,
            (None, false, false) => checking.unreachable += 1,
        }
    }

    /// A holder answered our grant for a message it refused as unknown.
    pub(super) fn grant_offered(
        &mut self,
        message_id: String,
        unit: [u8; 32],
        outcome: std::result::Result<Response, String>,
    ) {
        let final_refusal = matches!(
            &outcome,
            Ok(Response::Refused { code })
                if code == Refusal::Grant.code() || code == Refusal::Blocked.code()
        );
        if !final_refusal {
            return;
        }
        self.mailbox_client.refuse_holder(&message_id, unit);
    }
}
