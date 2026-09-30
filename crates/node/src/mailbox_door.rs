//! Groups' doors at the node (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! part 5): the owner's and admins' nodes publish each group's door card
//! once a period, read its door mailbox about every 20 s and add everyone
//! let in with one commit a minute; an applicant's node finds a door card by
//! the group's reference and knocks.
use super::*;
use agentic_core::DoorEntry;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period};
use std::collections::BTreeSet;

/// Sends of door cards, beside the outbox's messages.
pub(super) const DOOR_CARD_JOB: &str = "doorcard:";
/// Readings of a door mailbox, beside the conversations.
pub(super) const DOOR: &str = "door:";
/// How often door cards to publish are looked for.
const CARD_CHECK: Duration = Duration::from_secs(10);
/// How often those let in are added: one batch a minute.
const BATCH_EVERY: Duration = Duration::from_secs(60);
/// How often an owner's or admin's node reads its groups' doors.
pub(super) const DOOR_READ_EVERY: Duration = Duration::from_secs(20);
/// How often groups this profile fell behind in are looked for.
const REJOIN_CHECK: Duration = Duration::from_secs(60);

#[derive(Default)]
pub(super) struct DoorLane {
    /// Card jobs stored at a quorum this period.
    published: BTreeSet<String>,
    check_due: Option<Instant>,
    batch_due: Option<Instant>,
    pub(super) read_due: Option<Instant>,
    admitted: u64,
    waiting: u64,
    batches: u64,
    rejoin_due: Option<Instant>,
    /// Requests for a place again already made, by operation id (the group
    /// and the day): one a day while the group stays behind.
    asked: BTreeSet<String>,
    /// Groups behind without a door found: their owner is asked.
    ask_owner: BTreeSet<String>,
    rejoins: u64,
}

impl DoorLane {
    pub(super) fn info(&self) -> Value {
        json!({
            "published": self.published.len(),
            "admitted": self.admitted,
            "waiting": self.waiting,
            "batches": self.batches,
            "rejoins": self.rejoins,
        })
    }
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.check_due
            .into_iter()
            .chain(self.batch_due)
            .chain(self.read_due)
            .chain(self.rejoin_due)
            .min()
    }
    pub(super) fn stored(&mut self, job: &str) {
        self.published.insert(job.to_owned());
    }
}

impl Runtime {
    /// Publish the door card of each group this profile opens as owner or
    /// admin, once per period; once a minute, batch those let in.
    pub(super) fn maintain_doors(&mut self, now: u64, instant: Instant) {
        let lane = &mut self.mailbox_client.door;
        if lane.check_due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        lane.check_due = Some(instant + CARD_CHECK);
        let batch = lane.batch_due.is_none_or(|due| due <= instant);
        if batch {
            lane.batch_due = Some(instant + BATCH_EVERY);
        }
        let current = period(now);
        let today = format!(":{current}");
        lane.published.retain(|job| job.ends_with(&today));
        // Every group this profile keeps as owner or admin batches: those let
        // in at a door, and members back after a long absence.
        if batch && let Ok(groups) = self.core.groups() {
            for group in groups.iter().filter(|g| g.role != "member") {
                if let Ok(Some(_)) = self.core.door_batch(&group.id, now) {
                    self.mailbox_client.door.batches += 1;
                }
            }
        }
        let Ok(doors) = self.core.door_mailboxes(now) else {
            return;
        };
        for (group, _) in doors {
            let job = format!("{DOOR_CARD_JOB}{group}:{current}");
            let client = &self.mailbox_client;
            if client.door.published.contains(&job) || client.sending(&job) {
                continue;
            }
            let Ok(Some(mut delivery)) = self.core.door_card(&group, now) else {
                continue;
            };
            self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
            delivery.message_id = job.clone();
            self.mailbox_client.start_send(job, delivery);
        }
    }

    /// Once a minute: ask for a place again in each group this profile fell
    /// behind in, once a day while it stays behind — at the group's door, or
    /// from its owner when it has none.
    pub(super) fn maintain_rejoins(&mut self, now: u64, instant: Instant) {
        let lane = &mut self.mailbox_client.door;
        if lane.rejoin_due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        lane.rejoin_due = Some(instant + REJOIN_CHECK);
        let Ok(stale) = self.core.stale_groups(now) else {
            return;
        };
        let lane = &mut self.mailbox_client.door;
        // Back in: forgotten.
        lane.asked.retain(|op| {
            stale
                .iter()
                .any(|s| op.starts_with(&format!("rejoin:{}:", s.group_id)))
        });
        lane.ask_owner
            .retain(|group| stale.iter().any(|s| s.group_id == *group));
        for group in stale {
            let op = format!("rejoin:{}:{}", group.group_id, period(now));
            if self.mailbox_client.door.asked.contains(&op) {
                continue;
            }
            if !self.mailbox_client.door.ask_owner.contains(&group.group_id) {
                let Ok(reference) = hex::decode(&group.group_ref)
                    .map_err(|_| ())
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes).map_err(|_| ()))
                else {
                    continue;
                };
                match self.card_for(&format!("{DOOR}{}", group.group_ref), now) {
                    Ok(Some(card)) => {
                        if self.core.rejoin_group(&reference, &card, &op, now).is_ok() {
                            self.mailbox_client.door.asked.insert(op);
                            self.mailbox_client.door.rejoins += 1;
                        }
                        continue;
                    }
                    Err(("card_not_found", _)) => {
                        self.mailbox_client
                            .door
                            .ask_owner
                            .insert(group.group_id.clone());
                    }
                    _ => continue,
                }
            }
            if let Ok(Some(card)) = self.card_for(&group.owner, now)
                && self
                    .core
                    .rejoin_by_owner(&group.group_id, &card, &op, now)
                    .is_ok()
            {
                self.mailbox_client.door.asked.insert(op);
                self.mailbox_client.door.rejoins += 1;
            }
        }
    }

    /// The door mailboxes this profile opens, current and previous period,
    /// and whether their read is due this poll.
    pub(super) fn door_reads(
        &mut self,
        now: u64,
        instant: Instant,
    ) -> Vec<([u8; 32], String, u64, bool)> {
        let lane = &mut self.mailbox_client.door;
        let due = lane.read_due.is_none_or(|due| due <= instant);
        if due {
            lane.read_due = Some(instant + DOOR_READ_EVERY);
        }
        let current = period(now);
        let mut reads = vec![];
        for p in current.saturating_sub(1)..=current {
            if let Ok(doors) = self.core.door_mailboxes(p * PERIOD_SECONDS) {
                for (group, mailbox) in doors {
                    reads.push((mailbox, format!("{DOOR}{group}"), p, due));
                }
            }
        }
        if reads.is_empty() {
            self.mailbox_client.door.read_due = None;
        }
        reads
    }

    /// An entry read from a group's door mailbox.
    pub(super) fn door_import(&mut self, conversation: &str, period: u64, envelope: &[u8]) {
        let Some(group) = conversation.strip_prefix(DOOR) else {
            return;
        };
        let Ok(now) = now() else { return };
        match self.core.receive_door_entry(group, period, envelope, now) {
            Ok(DoorEntry::Admitted(_)) => self.mailbox_client.door.admitted += 1,
            Ok(DoorEntry::Waiting(_)) => self.mailbox_client.door.waiting += 1,
            _ => {}
        }
    }

    /// Owner IPC `join_group`: knock at the door of group `G` once its card
    /// is found.
    pub(super) fn join_group_by_door(
        &mut self,
        group_ref: &str,
        note: &str,
        operation_id: &str,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let group: [u8; 32] = hex::decode(group_ref)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or((
                "invalid_request",
                "groupRef is a 32-byte hex reference".into(),
            ))?;
        let Some(card) = self.card_for(&format!("{DOOR}{group_ref}"), now)? else {
            return Err(("card_pending", "Looking for the door; ask again".into()));
        };
        let applied = self
            .core
            .apply_to_group(&group, &card, note, operation_id, now)
            .map_err(|error| ("invalid_request", error.to_string()))?;
        serde_json::to_value(applied).map_err(|error| ("internal", error.to_string()))
    }
}
