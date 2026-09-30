//! Open-read groups at the node (Docs/V1_DISCOVERY_2026_09_27.md, part 2):
//! owners and admins publish the roster of a public group once a day and
//! whenever what it says changes — a removal, a ban, a new roster, a
//! closing — not on every epoch; members read its public mailbox like a
//! group mailbox, and followers read it about once a minute from one
//! holder, four every fifth time. A channel's followers read every five
//! minutes, four holders every sixth time, and read its last 30 days once;
//! its team lays its archive parts (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! parts 4 and 10b).
use super::*;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};

/// Sends of rosters, beside the outbox's messages.
pub(super) const ROSTER_JOB: &str = "roster:";
/// Sends of a channel's archive parts, beside the outbox's messages.
pub(super) const ARCHIVE_JOB: &str = "archive:";
/// Sends of a closed channel's key updates, beside the outbox's messages.
pub(super) const KEYS_JOB: &str = "keys:";
/// Sends of a closed channel's subscriber's own key.
pub(super) const HELLO_JOB: &str = "hello:";
/// How often key updates to publish are looked for.
const KEYS_CHECK: Duration = Duration::from_secs(60);
/// How often a channel's parts to lay are looked for.
const ARCHIVE_CHECK: Duration = Duration::from_secs(5 * 60);
/// How often a channel's follower reads.
const CHANNEL_READ_EVERY: Duration = Duration::from_secs(5 * 60);
/// Every this many channel reads go to four holders: every 30 minutes.
const CHANNEL_WIDE_EVERY: u64 = 6;
/// Days back a channel's follower or newcomer reads once.
const CHANNEL_HISTORY: u64 = 30;
/// Rounds of reading those days, from four holders, before they are left.
const HISTORY_ROUNDS: u8 = 3;
const HISTORY_EVERY: Duration = Duration::from_secs(60);
/// Conversations of public mailboxes in the reader, beside the others.
pub(super) const PUBLIC: &str = "public:";
/// How often a roster to publish is looked for.
const ROSTER_CHECK: Duration = Duration::from_secs(60);
/// How often a follower reads.
pub(super) const FOLLOW_READ_EVERY: Duration = Duration::from_secs(60);
/// Every this many follower reads go to four holders instead of one: any
/// four meet the seven that stored an entry.
const WIDE_EVERY: u64 = 5;
const WIDE: usize = 4;

/// When a public mailbox is read this poll.
pub(super) enum Read {
    /// A member's: every poll, from every holder.
    Every,
    /// A follower's, due: from this many holders.
    Few(usize),
    /// A follower's, not due yet.
    Later,
}

#[derive(Default)]
pub(super) struct Public {
    roster_due: Option<Instant>,
    /// Rosters stored at a quorum today: their jobs.
    published: BTreeSet<String>,
    pub(super) follow_due: Option<Instant>,
    follow_reads: u64,
    pub(super) channel_due: Option<Instant>,
    channel_reads: u64,
    /// Channels whose last days are read, by conversation: the rounds done.
    history: BTreeMap<String, u8>,
    history_due: Option<Instant>,
    archive_due: Option<Instant>,
    keys_due: Option<Instant>,
    pub(super) key_updates: u64,
    rekeyed: u64,
    /// Archive parts stored at a quorum today: their jobs.
    laid: BTreeSet<String>,
    pub(super) posts: u64,
    pub(super) rosters: u64,
    archives: u64,
    archives_laid: u64,
    /// Subscribers' own keys this owner took.
    subscriber_keys: u64,
    /// Reseeds onto subscribers' own keys this owner made.
    hard_reseeds: u64,
    /// This subscriber's own keys stored at a quorum.
    pub(super) hellos: u64,
}

impl Public {
    pub(super) fn info(&self) -> Value {
        json!({
            "published": self.published.len(),
            "followReads": self.follow_reads + self.channel_reads,
            "posts": self.posts,
            "rosters": self.rosters,
            "archives": self.archives,
            "archivesLaid": self.archives_laid,
            "keyUpdates": self.key_updates,
            "rekeyed": self.rekeyed,
            "subscriberKeys": self.subscriber_keys,
            "hardReseeds": self.hard_reseeds,
            "hellos": self.hellos,
        })
    }
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.roster_due
            .into_iter()
            .chain(self.follow_due)
            .chain(self.channel_due)
            .chain(self.history_due)
            .chain(self.archive_due)
            .chain(self.keys_due)
            .min()
    }
    pub(super) fn stored(&mut self, job: &str) {
        if job.starts_with(ARCHIVE_JOB) {
            if self.laid.insert(job.to_owned()) {
                self.archives_laid += 1;
            }
        } else {
            self.published.insert(job.to_owned());
        }
    }
}

impl Runtime {
    /// Publish the roster of each public group this profile owns or
    /// administers, once a day and when what it says changes, and a
    /// closing one.
    pub(super) fn maintain_public_rosters(&mut self, now: u64, instant: Instant) {
        let public = &mut self.mailbox_client.public;
        if public.roster_due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        public.roster_due = Some(instant + ROSTER_CHECK);
        let current = period(now);
        let today = format!(":{current}:");
        public.published.retain(|job| job.contains(&today));
        let Ok(groups) = self.core.groups() else {
            return;
        };
        for group in groups {
            if group.role == "member" {
                continue;
            }
            let Ok(Some(key)) = self.core.public_roster_key(&group.id) else {
                continue;
            };
            let job = format!("{ROSTER_JOB}{}:{current}:{key}", group.id);
            let client = &self.mailbox_client;
            if client.public.published.contains(&job) || client.sending(&job) {
                continue;
            }
            let Ok(Some(mut delivery)) = self.core.public_roster(&group.id, now) else {
                continue;
            };
            self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
            delivery.message_id = job.clone();
            self.mailbox_client.start_send(job, delivery);
        }
    }

    /// Publish the key updates of each closed channel whose commit this
    /// profile made, once a minute until each is stored at a quorum.
    pub(super) fn maintain_channel_keys(&mut self, now: u64, instant: Instant) {
        let public = &mut self.mailbox_client.public;
        if public.keys_due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        public.keys_due = Some(instant + KEYS_CHECK);
        let Ok(groups) = self.core.groups() else {
            return;
        };
        // This profile's own keys for the closed channels it follows.
        if let Ok(hellos) = self.core.channel_hellos(now) {
            for mut delivery in hellos {
                let hash = hex::encode(sha2::Sha256::digest(&delivery.envelope));
                let job = format!(
                    "{HELLO_JOB}{}:{hash}:{}",
                    delivery.conversation_id, delivery.period
                );
                if self.mailbox_client.sending(&job) {
                    continue;
                }
                self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
                delivery.message_id = job.clone();
                self.mailbox_client.start_send(job, delivery);
            }
        }
        for group in groups
            .iter()
            .filter(|g| g.kind == "channel" && g.access != "public" && g.role != "member")
        {
            // A team member out: the owner moves the channel to new keys
            // onto its subscribers' own; those without one get keys again.
            if group.role == "owner" {
                if let Ok(Some(_)) = self.core.channel_hard_reseed(&group.id, now) {
                    self.mailbox_client.public.hard_reseeds += 1;
                }
                // One by one: a card not found yet holds up nobody else.
                for subscriber in self.core.channel_reissues(&group.id).unwrap_or_default() {
                    let _ = self.channel_subscribe(
                        &group.id,
                        std::slice::from_ref(&subscriber),
                        &format!("again:{}", group.epoch),
                        now,
                    );
                }
            }
            // Banned subscribers whose keys a note of the team told of
            // after the ban: their removal is due.
            if let Ok(due) = self.core.channel_removals_due(&group.id)
                && !due.is_empty()
            {
                let _ = self.core.change_group(
                    &group.id,
                    agentic_core::GroupChange {
                        unsubscribe: due,
                        ..agentic_core::GroupChange::default()
                    },
                    &format!("banned:{}:{}", group.id, group.epoch),
                    now,
                );
            }
            let Ok(docs) = self.core.channel_key_docs(&group.id, now) else {
                continue;
            };
            for mut delivery in docs {
                let hash = hex::encode(sha2::Sha256::digest(&delivery.envelope));
                let job = format!("{KEYS_JOB}{}:{hash}:{}", group.id, delivery.period);
                if self.mailbox_client.sending(&job) {
                    continue;
                }
                self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
                delivery.message_id = job.clone();
                self.mailbox_client.start_send(job, delivery);
            }
        }
    }

    /// Lay the archive parts of each public channel this profile is in the
    /// team of, every few minutes: until a copy is read back, the same
    /// part under the same stamp.
    pub(super) fn maintain_channel_archives(&mut self, now: u64, instant: Instant) {
        let public = &mut self.mailbox_client.public;
        if public.archive_due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        public.archive_due = Some(instant + ARCHIVE_CHECK);
        let today = format!(":{}", period(now));
        public.laid.retain(|job| job.ends_with(&today));
        let Ok(groups) = self.core.groups() else {
            return;
        };
        for group in groups
            .iter()
            .filter(|g| g.kind == "channel" && g.role != "member")
        {
            let Ok(parts) = self.core.channel_archive(&group.id, now) else {
                continue;
            };
            for mut delivery in parts {
                let hash = hex::encode(sha2::Sha256::digest(&delivery.envelope));
                let job = format!("{ARCHIVE_JOB}{}:{hash}:{}", group.id, delivery.period);
                let client = &self.mailbox_client;
                if client.public.laid.contains(&job) || client.sending(&job) {
                    continue;
                }
                self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
                delivery.message_id = job.clone();
                self.mailbox_client.start_send(job, delivery);
            }
        }
    }

    /// Public mailboxes to read this poll: the public groups this profile
    /// is in, every poll; the groups it follows, when their read is due.
    pub(super) fn public_reads(
        &mut self,
        now: u64,
        instant: Instant,
    ) -> Vec<([u8; 32], String, u64, Read)> {
        let current = period(now);
        let mut reads = vec![];
        let mut channels = vec![];
        if let Ok(groups) = self.core.groups() {
            // Open groups, and channels open or closed: a closed one's
            // key's mailbox and the one before.
            for group in groups
                .iter()
                .filter(|g| g.access == "public" || g.kind == "channel")
            {
                for p in current.saturating_sub(1)..=current {
                    for mailbox in self
                        .core
                        .public_mailboxes(&group.id, p * PERIOD_SECONDS)
                        .unwrap_or_default()
                    {
                        reads.push((mailbox, format!("{PUBLIC}{}", group.id), p, Read::Every));
                    }
                }
                // A closed channel has no history to read back.
                if group.kind == "channel" && group.access == "public" {
                    channels.push(group.id.clone());
                }
            }
        }
        let follows: Vec<_> = self
            .core
            .follows()
            .unwrap_or_default()
            .into_iter()
            .filter(|f| !f.closed)
            .collect();
        let (channel_follows, group_follows): (Vec<_>, Vec<_>) =
            follows.into_iter().partition(|f| f.kind == "channel");
        let public = &mut self.mailbox_client.public;
        // Groups: one round a minute reads every open follow once.
        let group_read = if group_follows.is_empty() {
            public.follow_due = None;
            None
        } else {
            let due = public.follow_due.is_none_or(|due| due <= instant);
            if due {
                public.follow_due = Some(instant + FOLLOW_READ_EVERY);
                public.follow_reads += 1;
            }
            Some(if !due {
                Read::Later
            } else if public.follow_reads.is_multiple_of(WIDE_EVERY) {
                Read::Few(WIDE)
            } else {
                Read::Few(1)
            })
        };
        // Channels: one round every five minutes.
        let channel_read = if channel_follows.is_empty() {
            public.channel_due = None;
            None
        } else {
            let due = public.channel_due.is_none_or(|due| due <= instant);
            if due {
                public.channel_due = Some(instant + CHANNEL_READ_EVERY);
                public.channel_reads += 1;
            }
            Some(if !due {
                Read::Later
            } else if public.channel_reads.is_multiple_of(CHANNEL_WIDE_EVERY) {
                Read::Few(WIDE)
            } else {
                Read::Few(1)
            })
        };
        for (list, read) in [
            (&group_follows, group_read),
            (&channel_follows, channel_read),
        ] {
            let Some(read) = read else { continue };
            for follow in list.iter() {
                for p in current.saturating_sub(1)..=current {
                    for mailbox in self
                        .core
                        .public_mailboxes(&follow.id, p * PERIOD_SECONDS)
                        .unwrap_or_default()
                    {
                        let read = match read {
                            Read::Few(n) => Read::Few(n),
                            _ => Read::Later,
                        };
                        reads.push((mailbox, format!("{PUBLIC}{}", follow.id), p, read));
                    }
                }
            }
        }
        // A channel's last 30 days, read once from four holders: by its
        // followers and its newcomers alike.
        channels.extend(
            channel_follows
                .iter()
                .filter(|f| !f.sealed)
                .map(|f| f.id.clone()),
        );
        let public = &mut self.mailbox_client.public;
        public.history.retain(|id, _| channels.contains(id));
        let round = public.history_due.is_none_or(|due| due <= instant);
        if round {
            public.history_due = Some(instant + HISTORY_EVERY);
        }
        if channels.iter().all(|id| {
            public
                .history
                .get(id)
                .is_some_and(|done| *done >= HISTORY_ROUNDS)
        }) {
            public.history_due = None;
        }
        for id in &channels {
            let done = public.history.entry(id.clone()).or_default();
            if *done >= HISTORY_ROUNDS {
                continue;
            }
            let due = round;
            if due {
                *done += 1;
            }
            for p in current.saturating_sub(CHANNEL_HISTORY)..current.saturating_sub(1) {
                if let Ok(Some(mailbox)) = self.core.public_mailbox(id, p * PERIOD_SECONDS) {
                    reads.push((
                        mailbox,
                        format!("{PUBLIC}{id}"),
                        p,
                        if due { Read::Few(WIDE) } else { Read::Later },
                    ));
                }
            }
        }
        reads
    }

    /// An entry read from a public mailbox.
    pub(super) fn public_import(&mut self, conversation: &str, envelope: &[u8], stored_at: u64) {
        let Some(id) = conversation.strip_prefix(PUBLIC) else {
            return;
        };
        let Ok(now) = now() else { return };
        match self.core.receive_public_entry(id, envelope, stored_at, now) {
            Ok(agentic_core::PublicEntry::Post) => self.mailbox_client.public.posts += 1,
            Ok(agentic_core::PublicEntry::Roster) => self.mailbox_client.public.rosters += 1,
            Ok(agentic_core::PublicEntry::Archive) => self.mailbox_client.public.archives += 1,
            Ok(agentic_core::PublicEntry::Rekeyed) => self.mailbox_client.public.rekeyed += 1,
            Ok(agentic_core::PublicEntry::SubscriberKey) => {
                self.mailbox_client.public.subscriber_keys += 1;
            }
            _ => {}
        }
    }
}
