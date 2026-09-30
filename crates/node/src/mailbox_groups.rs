//! Groups at the node (spec/groups-v1.md): every undecided commit claim of a
//! group's current epoch is put on record with its ten notaries, their first
//! records are counted, and seven decide the epoch; a split round is renewed
//! by its committers. Also the owner IPC of groups.
use super::mailbox_holder::{Statement, claim_key};
use super::*;
use agentic_core::{GroupChange, Invitee};
use agentic_mailbox_swarm::select::QUORUM;
use std::collections::BTreeMap;

/// A claim without a decision is asked again this often.
const ASK_AGAIN: Duration = Duration::from_secs(30);
/// A split round is renewed after a random delay of up to this.
const RENEW_WITHIN_MS: u64 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoundResult {
    /// This commit has seven first records.
    Winner([u8; 32]),
    /// No commit can reach seven any more.
    Split,
    /// Not decided yet.
    Open,
}

/// The state of a round from the first claim each answering notary recorded.
pub(super) fn round_result(firsts: &[[u8; 32]], notaries: usize) -> RoundResult {
    if notaries < QUORUM {
        return RoundResult::Open;
    }
    let mut counts: BTreeMap<[u8; 32], usize> = BTreeMap::new();
    for first in firsts {
        *counts.entry(*first).or_default() += 1;
    }
    if let Some((commit, _)) = counts.iter().find(|(_, n)| **n >= QUORUM) {
        return RoundResult::Winner(*commit);
    }
    let best = counts.values().max().copied().unwrap_or(0);
    let unanswered = notaries.saturating_sub(firsts.len());
    if best + unanswered < QUORUM {
        RoundResult::Split
    } else {
        RoundResult::Open
    }
}

struct Round {
    group: String,
    epoch: u64,
    round: u32,
    claim: Vec<u8>,
    /// This profile's own commit.
    own: bool,
    notaries: Vec<[u8; 32]>,
    answers: BTreeMap<[u8; 32], [u8; 32]>,
    asked: Instant,
    renew_at: Option<Instant>,
    renewed: bool,
}

#[derive(Default)]
pub(super) struct GroupLane {
    rounds: BTreeMap<[u8; 32], Round>,
    decided: u64,
    renewed: u64,
}

impl GroupLane {
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.rounds
            .values()
            .filter_map(|round| round.renew_at.or(Some(round.asked + ASK_AGAIN)))
            .min()
    }
    pub(super) fn info(&self) -> Value {
        json!({
            "rounds": self.rounds.len(),
            "decided": self.decided,
            "renewed": self.renewed,
        })
    }
}

fn random_delay() -> Duration {
    let mut bytes = [0; 8];
    let _ = random::fill(&mut bytes);
    Duration::from_millis(u64::from_le_bytes(bytes) % RENEW_WITHIN_MS)
}

/// The commit a claim names.
fn claimed_commit(claim: &[u8]) -> Option<[u8; 32]> {
    agentic_protocol::group::verify_claim(claim, NETWORK_DOMAIN, u64::MAX / 4)
        .ok()
        .map(|verified| verified.claim.commit)
}

impl Runtime {
    /// Put undecided claims on record, count the answers and decide.
    pub(super) fn maintain_groups(&mut self, now: u64, instant: Instant) {
        if self.mailbox_client.directory.len() < QUORUM {
            return;
        }
        let Ok(groups) = self.core.groups() else {
            return;
        };
        let current: BTreeMap<String, u64> =
            groups.iter().map(|g| (g.id.clone(), g.epoch)).collect();
        for info in &groups {
            let Ok(claims) = self.core.group_claims(&info.id) else {
                continue;
            };
            for claim in claims {
                if claim.epoch != info.epoch {
                    continue;
                }
                // A commit of this profile goes on record once stored.
                if let Some(message) = &claim.message_id
                    && self.core.swarm_receipts(message).ok().flatten().is_none()
                {
                    continue;
                }
                let Some(key) = claim_key(&claim.claim, &NETWORK_DOMAIN) else {
                    continue;
                };
                if self.mailbox_client.groups.rounds.contains_key(&key) {
                    continue;
                }
                // A later round replaces the earlier ones of its epoch.
                self.mailbox_client.groups.rounds.retain(|_, r| {
                    !(r.group == info.id && r.epoch == claim.epoch && r.round < claim.round)
                });
                let notaries = self.notaries_of(&key);
                self.mailbox_client.groups.rounds.insert(
                    key,
                    Round {
                        group: info.id.clone(),
                        epoch: claim.epoch,
                        round: claim.round,
                        claim: claim.claim.clone(),
                        own: claim.message_id.is_some(),
                        notaries,
                        answers: BTreeMap::new(),
                        asked: instant,
                        renew_at: None,
                        renewed: false,
                    },
                );
                self.notarize_later(Statement::Commit(claim.claim));
            }
        }
        // Rounds of epochs already left behind go.
        self.mailbox_client
            .groups
            .rounds
            .retain(|_, r| current.get(&r.group).is_some_and(|epoch| r.epoch >= *epoch));
        let keys: Vec<[u8; 32]> = self.mailbox_client.groups.rounds.keys().copied().collect();
        for key in keys {
            let Some(round) = self.mailbox_client.groups.rounds.get_mut(&key) else {
                continue;
            };
            let firsts: Vec<[u8; 32]> = round.answers.values().copied().collect();
            match round_result(&firsts, round.notaries.len()) {
                RoundResult::Winner(commit) => {
                    let (group, epoch) = (round.group.clone(), round.epoch);
                    self.mailbox_client
                        .groups
                        .rounds
                        .retain(|_, r| !(r.group == group && r.epoch == epoch));
                    if self
                        .core
                        .decide_group_commit(&group, epoch, &hex::encode(commit), now)
                        .is_ok()
                    {
                        self.mailbox_client.groups.decided += 1;
                    }
                }
                RoundResult::Split if round.own && !round.renewed => match round.renew_at {
                    None => round.renew_at = Some(instant + random_delay()),
                    Some(at) if at <= instant => {
                        round.renewed = true;
                        round.renew_at = None;
                        let (group, next) = (round.group.clone(), round.round + 1);
                        if self.core.renew_group_claim(&group, next).is_ok() {
                            self.mailbox_client.groups.renewed += 1;
                        }
                    }
                    Some(_) => {}
                },
                RoundResult::Split => {}
                RoundResult::Open => {
                    if round.asked + ASK_AGAIN <= instant {
                        round.asked = instant;
                        let claim = round.claim.clone();
                        self.notarize_later(Statement::Commit(claim));
                    }
                }
            }
        }
    }

    /// A notary's first claim for a round's key.
    pub(super) fn group_answer(&mut self, unit: [u8; 32], submitted: &[u8], first: &[u8]) {
        let (Some(key), Some(commit)) =
            (claim_key(submitted, &NETWORK_DOMAIN), claimed_commit(first))
        else {
            return;
        };
        if claim_key(first, &NETWORK_DOMAIN) != Some(key) {
            return;
        }
        if let Some(round) = self.mailbox_client.groups.rounds.get_mut(&key) {
            round.answers.insert(unit, commit);
        }
    }

    /// Owner IPC `create_group`.
    pub(super) fn create_group(
        &mut self,
        name: &str,
        members: &[String],
        channel: bool,
        access: Option<agentic_protocol::group::Access>,
        operation_id: &str,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        if let Some(info) = self.core.created_group(operation_id).map_err(group_error)? {
            return serde_json::to_value(info).map_err(|e| ("invalid_request", e.to_string()));
        }
        let invitees = self.invitees(members, now)?;
        let info = if channel {
            self.core.create_channel(
                name,
                &invitees,
                access.unwrap_or(agentic_protocol::group::Access::Private),
                operation_id,
                now,
            )
        } else if access.is_some_and(|a| a != agentic_protocol::group::Access::Private) {
            return Err((
                "invalid_request",
                "a group is made private; open it with change_group".into(),
            ));
        } else {
            self.core.create_group(name, &invitees, operation_id, now)
        }
        .map_err(group_error)?;
        serde_json::to_value(info).map_err(|e| ("invalid_request", e.to_string()))
    }

    /// Owner IPC `channel_subscribe`: give each of `members`, looked up by
    /// its card, a closed channel's keys. The same operation id gives the
    /// same keys.
    pub(super) fn channel_subscribe(
        &mut self,
        group: &str,
        members: &[String],
        operation_id: &str,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let invitees = self.invitees(members, now)?;
        for invitee in &invitees {
            self.core
                .channel_subscribe(
                    group,
                    invitee,
                    &format!("{operation_id}:{}", invitee.network_id),
                    now,
                )
                .map_err(group_error)?;
        }
        Ok(json!({ "subscribed": members }))
    }

    /// Owner IPC `change_group`: the members to add are looked up by their
    /// cards; the rest of `change` names ids.
    pub(super) fn change_group(
        &mut self,
        group: &str,
        add: &[String],
        change: GroupChange,
        operation_id: &str,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        if let Some(made) = self
            .core
            .group_operation(group, operation_id)
            .map_err(group_error)?
        {
            return serde_json::to_value(made).map_err(|e| ("invalid_request", e.to_string()));
        }
        let invitees = self.invitees(add, now)?;
        let made = self
            .core
            .change_group(
                group,
                GroupChange {
                    add: invitees,
                    ..change
                },
                operation_id,
                now,
            )
            .map_err(group_error)?;
        serde_json::to_value(made).map_err(|e| ("invalid_request", e.to_string()))
    }

    /// The cards of `members`, once all are read.
    fn invitees(
        &mut self,
        members: &[String],
        now: u64,
    ) -> std::result::Result<Vec<Invitee>, (&'static str, String)> {
        let own = self
            .core
            .own_intro_mailbox(now)
            .map_err(|e| ("invalid_request", e.to_string()))?;
        let mut invitees = vec![];
        let mut pending = false;
        for member in members {
            let theirs = self
                .core
                .intro_mailbox(member, now)
                .map_err(|e| ("invalid_request", e.to_string()))?;
            if theirs == own {
                return Err(("invalid_request", "That is this profile's own id".into()));
            }
            match self.card_for(member, now)? {
                Some(card) => invitees.push(Invitee {
                    network_id: member.clone(),
                    card,
                }),
                None => pending = true,
            }
        }
        if pending {
            return Err(("card_pending", "Looking for the cards; ask again".into()));
        }
        Ok(invitees)
    }
}

/// A group operation's refusal as the owner IPC names it.
pub(super) fn group_error(error: CoreError) -> (&'static str, String) {
    let code = match error {
        CoreError::GroupBusy => "group_busy",
        CoreError::Unauthorized => "not_allowed",
        CoreError::Banned => "banned",
        CoreError::Crypto(agentic_crypto::CryptoError::Outdated) => "member_outdated",
        CoreError::UnknownConversation => "unknown_group",
        _ => "invalid_request",
    };
    (code, error.to_string())
}
