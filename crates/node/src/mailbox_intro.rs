//! Contact by ID at the node (spec/contact-by-id-v1.md): the owner's card
//! published in its intro mailbox once a period, requests read from there,
//! and a requester's lookup of another id's card.
use super::mailbox_client::{Purpose, fresh_entry};
use super::mailbox_holder::{MAX_PAGE, Request, Response};
use super::*;
use agentic_core::{IntroOutcome, SwarmDelivery};
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, RETENTION_PERIODS, period};
use std::collections::BTreeMap;

/// How often this profile's intro mailboxes are read: requests are not
/// urgent, and every read goes to ten holders per period.
pub(super) const INTRO_READ_EVERY: Duration = Duration::from_secs(30);
/// How often the card is checked while it is not yet published.
const CARD_CHECK: Duration = Duration::from_secs(60);
/// The whole lookup of a card ends after this.
const LOOKUP_FOR: Duration = Duration::from_secs(60);
/// Periods read at once while looking back for a card.
const LOOKUP_PERIODS: u64 = 4;
/// Pages read from one holder of one period at most.
const LOOKUP_PAGES: u32 = 16;
/// A card found stays usable for requests this long.
const FOUND_FOR: Duration = Duration::from_secs(600);
/// The key prefix of card publication jobs among the swarm sends.
pub(super) const CARD_JOB: &str = "card:";
/// The reader's key of this profile's own intro mailboxes.
pub(super) const INTRO: &str = "intro";

#[derive(Default, Clone, Copy)]
struct Counts {
    joined: u64,
    pending: u64,
    ignored: u64,
    /// Closed channels' keys taken.
    subscribed: u64,
}

impl Counts {
    fn add(&mut self, outcome: &IntroOutcome) {
        match outcome {
            // A big group's invitation is joined once its tree is read.
            IntroOutcome::Joined(_) | IntroOutcome::AwaitingTree(_) => self.joined += 1,
            IntroOutcome::Pending(_) => self.pending += 1,
            IntroOutcome::Ignored => self.ignored += 1,
            IntroOutcome::Subscribed(_) => self.subscribed += 1,
        }
    }
    fn json(self) -> Value {
        json!({
            "joined": self.joined,
            "pending": self.pending,
            "ignored": self.ignored,
            "subscribed": self.subscribed,
        })
    }
}

/// One holder's reading of one period's intro mailbox during a lookup.
#[derive(Default)]
struct HolderRead {
    cursor: u64,
    pages: u32,
    in_flight: bool,
    done: bool,
    /// Refused this node's credential for a moment: read again once let in.
    refused: bool,
}

struct PeriodRead {
    mailbox: [u8; 32],
    holders: BTreeMap<[u8; 32], HolderRead>,
}

enum LookupState {
    Reading {
        started: Instant,
        /// The oldest period still to start reading.
        next: u64,
        floor: u64,
        periods: BTreeMap<u64, PeriodRead>,
        /// Valid cards found, by period: `(expires_at, envelope)`.
        found: BTreeMap<u64, (u64, Vec<u8>)>,
    },
    Found {
        envelope: Vec<u8>,
        until: Instant,
    },
    NotFound,
    /// No card found while holders still refused this node's credential:
    /// not known to be missing.
    Refused,
}

#[derive(Default)]
pub(super) struct Intro {
    /// The card job last stored at a quorum: `card:<id>:<period>`.
    published: Option<String>,
    /// The next check of the card.
    due: Option<Instant>,
    /// Report `due` as a deadline: publication is still to do.
    wanted: bool,
    lookups: BTreeMap<String, LookupState>,
    /// The next read of this profile's intro mailboxes.
    pub(super) read_due: Option<Instant>,
    swarm: Counts,
    direct: Counts,
    rejected: u64,
    /// Scenarios about other lanes keep the owner's card unpublished, so
    /// their exact counts stay theirs.
    #[cfg(test)]
    pub(super) silent: bool,
}

impl Intro {
    pub(super) fn next_due(&self) -> Option<Instant> {
        let lookup = self
            .lookups
            .values()
            .filter_map(|lookup| match lookup {
                LookupState::Reading { started, .. } => Some(*started + LOOKUP_FOR),
                LookupState::Found { until, .. } => Some(*until),
                LookupState::NotFound | LookupState::Refused => None,
            })
            .min();
        self.due
            .filter(|_| self.wanted)
            .into_iter()
            .chain(lookup)
            .min()
    }

    pub(super) fn info(&self) -> Value {
        json!({
            "published": self.published,
            "lookups": self.lookups.len(),
            "swarm": self.swarm.json(),
            "direct": self.direct.json(),
            "rejected": self.rejected,
        })
    }

    pub(super) fn stored(&mut self, job: &str) {
        self.published = Some(job.into());
    }

    pub(super) fn count_direct(&mut self, outcome: &IntroOutcome) {
        self.direct.add(outcome);
    }
}

impl Runtime {
    /// Publish the current card in this period's intro mailbox, once.
    pub(super) fn maintain_intro_card(&mut self, now: u64, instant: Instant) {
        let intro = &mut self.mailbox_client.intro;
        #[cfg(test)]
        if intro.silent {
            return;
        }
        if intro.due.is_some_and(|due| due > instant)
            || self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM
        {
            return;
        }
        let next_period = Duration::from_secs(
            (period(now) + 1)
                .saturating_mul(PERIOD_SECONDS)
                .saturating_sub(now),
        );
        let addresses = self.advertised();
        let intro = &mut self.mailbox_client.intro;
        intro.due = Some(instant + CARD_CHECK);
        intro.wanted = false;
        let Ok(card) = self.core.intro_card(addresses, now) else {
            return;
        };
        let current = period(now);
        let job = format!("{CARD_JOB}{}:{current}", card.id);
        let client = &mut self.mailbox_client;
        if client.intro.published.as_deref() == Some(job.as_str()) {
            // Published: look again when the period turns.
            client.intro.due = Some(instant + next_period);
            client.intro.wanted = true;
            return;
        }
        client.intro.wanted = true;
        if client.sending(&job) {
            return;
        }
        let Ok(mailbox) = self.core.own_intro_mailbox(now) else {
            return;
        };
        let Ok(stamp) = self
            .core
            .stamp_mailbox(&mailbox, current, &card.envelope, now)
        else {
            return;
        };
        self.notarize_later(mailbox_holder::Statement::Ticket(stamp.clone()));
        let delivery = SwarmDelivery {
            message_id: job.clone(),
            conversation_id: String::new(),
            period: current,
            mailbox,
            envelope: card.envelope,
            stamp,
        };
        self.mailbox_client.start_send(job, delivery);
    }

    /// The first period of this profile's intro mailboxes still to read:
    /// after its read-through period, else from the period before its first
    /// card; none before a card exists.
    pub(super) fn intro_read_from(&self, current: u64) -> Option<u64> {
        let floor = current.saturating_sub(RETENTION_PERIODS);
        let from = match self.core.intro_read_through().ok()? {
            Some(through) => through + 1,
            None => period(self.core.intro_since().ok()??).saturating_sub(1),
        };
        Some(from.max(floor).min(current.saturating_sub(1).max(floor)))
    }

    /// An entry read from this profile's intro mailbox.
    pub(super) fn intro_import(&mut self, envelope: &[u8], now: u64) {
        match self.core.receive_intro_envelope(envelope, now) {
            Ok(outcome) => self.mailbox_client.intro.swarm.add(&outcome),
            Err(_) => self.mailbox_client.intro.rejected += 1,
        }
    }

    /// Owner IPC `request_contact`: the conversation, once `network_id`'s card
    /// is found.
    pub(super) fn request_contact(
        &mut self,
        network_id: &str,
        name: &str,
        operation_id: &str,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let invalid = |error: CoreError| ("invalid_request", error.to_string());
        let theirs = self.core.intro_mailbox(network_id, now).map_err(invalid)?;
        if self.core.own_intro_mailbox(now).map_err(invalid)? == theirs {
            return Err(("invalid_request", "That is this profile's own id".into()));
        }
        let answer = |conversation: agentic_core::Conversation| json!({"conversationId": conversation.id, "name": conversation.title});
        if let Some(conversation) = self.core.requested_contact(operation_id).map_err(invalid)? {
            return Ok(answer(conversation));
        }
        let instant = clock::instant();
        let lookups = &mut self.mailbox_client.intro.lookups;
        match lookups.get(network_id) {
            Some(LookupState::Found { envelope, until }) if *until > instant => {
                let envelope = envelope.clone();
                match self
                    .core
                    .request_contact(name, network_id, &envelope, operation_id, now)
                {
                    Ok(conversation) => return Ok(answer(conversation)),
                    Err(CoreError::InvalidInput) => {
                        return Err(invalid(CoreError::InvalidInput));
                    }
                    // The card went stale: look again.
                    Err(_) => {
                        self.mailbox_client.intro.lookups.remove(network_id);
                    }
                }
            }
            Some(LookupState::NotFound) => {
                lookups.remove(network_id);
                return Err((
                    "card_not_found",
                    "No valid card of this id was found in its intro mailboxes".into(),
                ));
            }
            Some(LookupState::Refused) => {
                lookups.remove(network_id);
                return Err((
                    "network_unavailable",
                    "The holders have not let this node in yet; ask again".into(),
                ));
            }
            Some(LookupState::Reading { .. }) => {
                return Err(("card_pending", "Looking for the card; ask again".into()));
            }
            _ => {
                lookups.remove(network_id);
            }
        }
        if self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM {
            return Err(self.directory_unknown());
        }
        let current = period(now);
        self.mailbox_client.intro.lookups.insert(
            network_id.into(),
            LookupState::Reading {
                started: instant,
                next: current,
                floor: current.saturating_sub(RETENTION_PERIODS),
                periods: BTreeMap::new(),
                found: BTreeMap::new(),
            },
        );
        self.maintain_lookups(now, instant);
        Err(("card_pending", "Looking for the card; ask again".into()))
    }

    /// `network_id`'s card, once found: `None` while it is looked up.
    pub(super) fn card_for(
        &mut self,
        network_id: &str,
        now: u64,
    ) -> std::result::Result<Option<Vec<u8>>, (&'static str, String)> {
        let instant = clock::instant();
        let lookups = &mut self.mailbox_client.intro.lookups;
        match lookups.get(network_id) {
            Some(LookupState::Found { envelope, until }) if *until > instant => {
                return Ok(Some(envelope.clone()));
            }
            Some(LookupState::NotFound) => {
                lookups.remove(network_id);
                return Err((
                    "card_not_found",
                    "No valid card of this id was found in its intro mailboxes".into(),
                ));
            }
            Some(LookupState::Refused) => {
                lookups.remove(network_id);
                return Err((
                    "network_unavailable",
                    "The holders have not let this node in yet; ask again".into(),
                ));
            }
            Some(LookupState::Reading { .. }) => return Ok(None),
            _ => {
                lookups.remove(network_id);
            }
        }
        if self.mailbox_client.directory.len() < agentic_mailbox_swarm::select::QUORUM {
            return Err(self.directory_unknown());
        }
        let current = period(now);
        self.mailbox_client.intro.lookups.insert(
            network_id.into(),
            LookupState::Reading {
                started: instant,
                next: current,
                floor: current.saturating_sub(RETENTION_PERIODS),
                periods: BTreeMap::new(),
                found: BTreeMap::new(),
            },
        );
        self.maintain_lookups(now, instant);
        Ok(None)
    }

    /// Asked for a card while the directory has too few holders: a node that
    /// reads the chain builds one within a minute of its start, and is asked
    /// again; one that does not never will.
    fn directory_unknown(&self) -> (&'static str, String) {
        if self.chain_configured() {
            (
                "card_pending",
                "Reading the network's directory; ask again".into(),
            )
        } else {
            (
                "network_unavailable",
                "The swarm directory is not known yet".into(),
            )
        }
    }

    /// Read the intro mailboxes of every id being looked up, newest period
    /// first, a few periods at a time.
    pub(super) fn maintain_lookups(&mut self, now: u64, instant: Instant) {
        let ids: Vec<String> = self.mailbox_client.intro.lookups.keys().cloned().collect();
        let mut outgoing = Vec::new();
        for id in ids {
            let mut mailboxes = Vec::new();
            {
                let Some(lookup) = self.mailbox_client.intro.lookups.get_mut(&id) else {
                    continue;
                };
                match lookup {
                    LookupState::Found { until, .. } if *until <= instant => {
                        self.mailbox_client.intro.lookups.remove(&id);
                        continue;
                    }
                    LookupState::Found { .. } | LookupState::NotFound | LookupState::Refused => {
                        continue;
                    }
                    LookupState::Reading { .. } => {}
                }
                let LookupState::Reading {
                    started,
                    next,
                    floor,
                    periods,
                    found,
                } = lookup
                else {
                    continue;
                };
                // The newest period with a card whose newer periods are all
                // read decides.
                let newest_open = periods.keys().next_back().copied();
                if let Some((&p, _)) = found.iter().next_back()
                    && newest_open.is_none_or(|open| open < p)
                {
                    let (_, envelope) = found.remove(&p).unwrap_or_default();
                    *lookup = LookupState::Found {
                        envelope,
                        until: instant + FOUND_FOR,
                    };
                    continue;
                }
                let exhausted = periods.is_empty() && *next < *floor;
                if exhausted || *started + LOOKUP_FOR <= instant {
                    let refused = periods
                        .values()
                        .flat_map(|read| read.holders.values())
                        .any(|holder| holder.refused && !holder.done);
                    *lookup = match found.pop_last() {
                        Some((_, (_, envelope))) => LookupState::Found {
                            envelope,
                            until: instant + FOUND_FOR,
                        },
                        None if refused => LookupState::Refused,
                        None => LookupState::NotFound,
                    };
                    continue;
                }
                while periods.len() + mailboxes.len() < LOOKUP_PERIODS as usize && *next >= *floor {
                    let p = *next;
                    *next = next.checked_sub(1).unwrap_or(0);
                    if p == 0 && *next == 0 {
                        *floor = 1;
                    }
                    let Ok(mailbox) = lookup_mailbox(&self.core, &id, p * PERIOD_SECONDS) else {
                        break;
                    };
                    mailboxes.push((p, mailbox));
                }
            }
            for (p, mailbox) in mailboxes {
                let swarm = self.mailbox_client.swarm(&mailbox);
                if let Some(LookupState::Reading { periods, .. }) =
                    self.mailbox_client.intro.lookups.get_mut(&id)
                {
                    periods.insert(
                        p,
                        PeriodRead {
                            mailbox,
                            holders: swarm
                                .into_iter()
                                .map(|unit| (unit, HolderRead::default()))
                                .collect(),
                        },
                    );
                }
            }
            let client = &mut self.mailbox_client;
            let mut slots = client.slots();
            let Some(LookupState::Reading { periods, .. }) = client.intro.lookups.get_mut(&id)
            else {
                continue;
            };
            // Newest first: the card most likely to be current.
            for (p, read) in periods.iter_mut().rev() {
                for (unit, holder_read) in read.holders.iter_mut() {
                    if holder_read.in_flight || holder_read.done {
                        continue;
                    }
                    let Some(holder) = client
                        .directory
                        .get(unit)
                        .filter(|h| !client.gossip.blocked.contains(&h.account))
                    else {
                        holder_read.done = true;
                        continue;
                    };
                    if !slots.take(unit) {
                        continue;
                    }
                    holder_read.in_flight = true;
                    outgoing.push((
                        holder.clone(),
                        Request::Read {
                            mailbox: read.mailbox.to_vec(),
                            after: holder_read.cursor,
                            limit: MAX_PAGE as u16,
                        },
                        Purpose::CardRead {
                            network_id: id.clone(),
                            period: *p,
                            unit: *unit,
                        },
                    ));
                }
            }
        }
        let _ = now;
        for (holder, request, purpose) in outgoing {
            self.send_mailbox(holder, request, purpose);
        }
    }

    /// A page of a looked-up id's intro mailbox.
    pub(super) fn card_page(
        &mut self,
        network_id: String,
        p: u64,
        unit: [u8; 32],
        outcome: std::result::Result<Response, String>,
    ) {
        let Ok(now) = now() else { return };
        let mut cards = Vec::new();
        {
            let Some(LookupState::Reading { periods, .. }) =
                self.mailbox_client.intro.lookups.get_mut(&network_id)
            else {
                return;
            };
            let Some(read) = periods.get_mut(&p) else {
                return;
            };
            let mailbox = read.mailbox;
            let Some(holder_read) = read.holders.get_mut(&unit) else {
                return;
            };
            holder_read.in_flight = false;
            match outcome {
                Ok(Response::Page { entries, next, .. }) => {
                    holder_read.refused = false;
                    holder_read.pages += 1;
                    let full = entries.len() >= MAX_PAGE;
                    let forward = next > holder_read.cursor;
                    holder_read.cursor = next.max(holder_read.cursor);
                    holder_read.done = !full || !forward || holder_read.pages >= LOOKUP_PAGES;
                    for entry in entries {
                        if let Some((_, envelope)) = fresh_entry(&mailbox, entry) {
                            cards.push(envelope);
                        }
                    }
                }
                // Sent again behind the credential shown anew.
                Err(error) if mailbox_access::let_in_soon(&error) => holder_read.refused = true,
                _ => holder_read.done = true,
            }
        }
        let mut best: Option<(u64, Vec<u8>)> = None;
        for envelope in cards {
            if let Some(rank) = lookup_card(&self.core, &network_id, &envelope, now)
                && best.as_ref().is_none_or(|(known, _)| rank > *known)
            {
                best = Some((rank, envelope));
            }
        }
        let Some(LookupState::Reading { periods, found, .. }) =
            self.mailbox_client.intro.lookups.get_mut(&network_id)
        else {
            return;
        };
        if let Some((expires, envelope)) = best
            && found.get(&p).is_none_or(|(known, _)| expires > *known)
        {
            found.insert(p, (expires, envelope));
        }
        // With a card of this period found, holders not letting this node in
        // yet are not waited for.
        let card = found.contains_key(&p);
        if periods
            .get(&p)
            .is_some_and(|read| read.holders.values().all(|h| h.done || (card && h.refused)))
        {
            periods.remove(&p);
        }
        self.maintain_lookups(now, clock::instant());
    }
}

/// Where a lookup reads: the intro mailbox of a network id, or the door
/// mailbox of a group (`door:<G>`).
fn lookup_mailbox(core: &AppCore, key: &str, at: u64) -> std::result::Result<[u8; 32], CoreError> {
    match key.strip_prefix(mailbox_door::DOOR) {
        Some(group) => {
            let group: [u8; 32] = hex::decode(group)
                .ok()
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(CoreError::InvalidInput)?;
            Ok(core.door_mailbox(&group, at))
        }
        None => core.intro_mailbox(key, at),
    }
}

/// A valid card found by a lookup, ranked: an intro card by its expiry,
/// any door card alike.
fn lookup_card(core: &AppCore, key: &str, envelope: &[u8], now: u64) -> Option<u64> {
    match key.strip_prefix(mailbox_door::DOOR) {
        Some(group) => {
            let group: [u8; 32] = hex::decode(group).ok()?.try_into().ok()?;
            core.open_door_card(&group, envelope, now).ok().map(|_| 0)
        }
        None => core
            .open_intro_card(key, envelope, now)
            .ok()
            .map(|card| card.expires_at),
    }
}
