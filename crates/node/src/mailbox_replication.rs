//! Replication inside the mailbox swarm (Docs/V1_STORAGE_REDESIGN_2026_09_24.md):
//! every holder periodically tells each other member of a mailbox's swarm
//! what it holds of the mailboxes they share (count and set digest). Either
//! side that sees a difference pulls the other's entries by cursor and stores
//! them through the verified store path. This repairs copies the sender
//! missed and copies a holder lost, and hands a mailbox to a unit that newly
//! joined its swarm; a slot spent twice surfaces as a conflict and a proof.
use super::mailbox_client::{Purpose, Slots};
use super::mailbox_holder::{MAX_PAGE, MAX_SUMMARIES, Request, Response, Summary, SummaryWire};
use super::*;
use agentic_mailbox_swarm::stamp::Stamp;
use std::collections::{BTreeMap, BTreeSet};

/// How often a holder compares its mailboxes with the rest of each swarm.
pub(super) const SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Holder-side replication counters.
#[derive(Clone, Debug, Default)]
pub(super) struct Replication {
    /// Entries this holder newly stored from other swarm members; pulling
    /// one it already holds does not count.
    pub(super) pulled: u64,
    /// Pulled entries whose ticket this holder already spent otherwise.
    pub(super) conflicts: u64,
    /// Summary requests sent.
    pub(super) summaries: u64,
    /// Replication pages requested.
    pub(super) pages: u64,
}

/// Reading one peer's copy of one mailbox.
#[derive(Default)]
struct Pull {
    cursor: u64,
    /// Our count and the peer's when last noted: either shrinking means a
    /// store was lost, and the cursor no longer describes what we have.
    ours: u64,
    theirs: u64,
    wanted: bool,
    in_flight: bool,
}

#[derive(Default)]
pub(super) struct Sync {
    due: Option<Instant>,
    /// Summary requests of the current round not yet sent.
    queue: Vec<([u8; 32], Vec<SummaryWire>)>,
    pulls: BTreeMap<([u8; 32], [u8; 32]), Pull>,
    pub(super) stats: Replication,
}

impl Sync {
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.due
    }
    pub(super) fn pulls(&self) -> usize {
        self.pulls
            .values()
            .filter(|p| p.wanted || p.in_flight)
            .count()
    }
}

fn wire(mailbox: &[u8; 32], summary: Summary) -> SummaryWire {
    SummaryWire {
        mailbox: mailbox.to_vec(),
        count: summary.count,
        digest: summary.digest.to_vec(),
    }
}

/// A well-formed summary list, or `None`.
pub(super) fn parse(items: &[SummaryWire]) -> Option<Vec<([u8; 32], Summary)>> {
    if items.len() > MAX_SUMMARIES {
        return None;
    }
    items
        .iter()
        .map(|item| {
            Some((
                item.mailbox.as_slice().try_into().ok()?,
                Summary {
                    count: item.count,
                    digest: item.digest.as_slice().try_into().ok()?,
                },
            ))
        })
        .collect()
}

impl Runtime {
    /// The unit a transport peer holds as, per this node's directory.
    fn directory_unit(&self, peer: &PeerId) -> Option<[u8; 32]> {
        self.mailbox_client
            .directory
            .iter()
            .find(|(_, holder)| holder.peer == *peer)
            .map(|(unit, _)| *unit)
    }

    pub(super) fn maintain_replication(&mut self, instant: Instant) {
        let Some(own) = self.mailbox_holder.unit() else {
            return;
        };
        if self.mailbox_client.directory.len() < 2 {
            return;
        }
        if self
            .mailbox_client
            .sync
            .due
            .is_none_or(|due| due <= instant)
        {
            self.mailbox_client.sync.due = Some(instant + SYNC_INTERVAL);
            // Expired mailboxes are neither kept nor compared.
            if let Ok(now) = now() {
                let _ = self.mailbox_holder.collect(now);
            }
            let mut shared: BTreeMap<[u8; 32], Vec<SummaryWire>> = BTreeMap::new();
            let mut after = None;
            while let Ok(page) = self.mailbox_holder.mailboxes(after.as_ref(), MAX_SUMMARIES) {
                let Some((last, _)) = page.last().copied() else {
                    break;
                };
                for (mailbox, summary) in page {
                    for unit in self.mailbox_client.swarm(&mailbox) {
                        if unit != own && self.mailbox_client.usable(&unit) {
                            shared
                                .entry(unit)
                                .or_default()
                                .push(wire(&mailbox, summary));
                        }
                    }
                }
                after = Some(last);
            }
            let sync = &mut self.mailbox_client.sync;
            sync.queue.clear();
            for (unit, items) in shared {
                for chunk in items.chunks(MAX_SUMMARIES) {
                    sync.queue.push((unit, chunk.to_vec()));
                }
            }
        }
        let client = &mut self.mailbox_client;
        let mut slots: Slots = client.slots();
        let mut outgoing = Vec::new();
        let mut waiting = Vec::new();
        for (unit, items) in std::mem::take(&mut client.sync.queue) {
            match client.directory.get(&unit) {
                Some(holder) if slots.take(&unit) => {
                    outgoing.push((
                        holder.clone(),
                        Request::Summaries { items },
                        Purpose::Summaries { unit },
                    ));
                }
                Some(_) => waiting.push((unit, items)),
                None => {}
            }
        }
        client.sync.queue = waiting;
        for ((mailbox, unit), pull) in client.sync.pulls.iter_mut() {
            if !pull.wanted || pull.in_flight {
                continue;
            }
            let Some(holder) = client
                .directory
                .get(unit)
                .filter(|holder| !client.gossip.blocked.contains(&holder.account))
            else {
                continue;
            };
            if !slots.take(unit) {
                continue;
            }
            pull.wanted = false;
            pull.in_flight = true;
            outgoing.push((
                holder.clone(),
                Request::Read {
                    mailbox: mailbox.to_vec(),
                    after: pull.cursor,
                    limit: MAX_PAGE as u16,
                },
                Purpose::Pull {
                    mailbox: *mailbox,
                    unit: *unit,
                },
            ));
        }
        for (holder, request, purpose) in outgoing {
            let stats = &mut self.mailbox_client.sync.stats;
            match purpose {
                Purpose::Summaries { .. } => stats.summaries += 1,
                Purpose::Pull { .. } => stats.pages += 1,
                _ => {}
            }
            self.send_mailbox(holder, request, purpose);
        }
    }

    /// A difference with `unit` about `mailbox`: pull its copy.
    fn note_difference(&mut self, mailbox: [u8; 32], unit: [u8; 32], theirs: u64) {
        let Ok(ours) = self.mailbox_holder.summary(&mailbox).map(|s| s.count) else {
            return;
        };
        let pull = self
            .mailbox_client
            .sync
            .pulls
            .entry((mailbox, unit))
            .or_default();
        if ours < pull.ours || theirs < pull.theirs {
            pull.cursor = 0;
        }
        pull.ours = ours;
        pull.theirs = theirs;
        pull.wanted = true;
    }

    /// Answer a member's summaries with ours, and pull what differs for the
    /// mailboxes this node holds as a member of their swarm.
    pub(super) fn serve_summaries(&mut self, peer: PeerId, items: &[SummaryWire]) -> Response {
        let Some(theirs) = parse(items) else {
            return Response::Refused {
                code: mailbox_holder::Refusal::Malformed.code().into(),
            };
        };
        let mut answer = Vec::with_capacity(theirs.len());
        for (mailbox, summary) in &theirs {
            let Ok(ours) = self.mailbox_holder.summary(mailbox) else {
                return Response::Refused {
                    code: mailbox_holder::Refusal::Storage.code().into(),
                };
            };
            answer.push(wire(mailbox, ours));
            let member = self
                .mailbox_holder
                .unit()
                .is_some_and(|own| self.mailbox_client.swarm(mailbox).contains(&own));
            if ours != *summary
                && member
                && let Some(unit) = self.directory_unit(&peer)
                && self.mailbox_client.usable(&unit)
            {
                self.note_difference(*mailbox, unit, summary.count);
            }
        }
        Response::Summaries { items: answer }
    }

    pub(super) fn replication_summaries(
        &mut self,
        unit: [u8; 32],
        outcome: std::result::Result<Response, String>,
    ) {
        let Ok(Response::Summaries { items }) = outcome else {
            return;
        };
        let Some(theirs) = parse(&items) else {
            return;
        };
        let own = self.mailbox_holder.unit();
        for (mailbox, summary) in theirs {
            // A node that left a mailbox's swarm still offers what it holds
            // but takes nothing more.
            let member = own.is_some_and(|own| self.mailbox_client.swarm(&mailbox).contains(&own));
            if member
                && self
                    .mailbox_holder
                    .summary(&mailbox)
                    .is_ok_and(|ours| ours != summary)
            {
                self.note_difference(mailbox, unit, summary.count);
            }
        }
    }

    pub(super) fn replication_page(
        &mut self,
        mailbox: [u8; 32],
        unit: [u8; 32],
        outcome: std::result::Result<Response, String>,
    ) {
        if let Some(pull) = self.mailbox_client.sync.pulls.get_mut(&(mailbox, unit)) {
            pull.in_flight = false;
        }
        let Ok(Response::Page {
            entries,
            next,
            grants,
        }) = outcome
        else {
            return;
        };
        let Ok(now) = now() else { return };
        let full = entries.len() >= MAX_PAGE;
        // Only grants of the page's own books are checked; entries of a book
        // whose grant is still being checked wait for a later round, while
        // those of a refused grant or an unknown bought book are passed over.
        let books: BTreeSet<[u8; 32]> = entries
            .iter()
            .filter_map(|entry| <[u8; 32]>::try_from(entry.stamp.book.as_slice()).ok())
            .collect();
        let mut checking = BTreeSet::new();
        let granted: BTreeSet<[u8; 32]> = grants.iter().map(|grant| grant.id()).collect();
        for grant in grants {
            let book = grant.id();
            if books.contains(&book)
                && matches!(
                    self.offer_grant(grant),
                    Response::Refused { code } if code == mailbox_holder::Refusal::GrantPending.code()
                )
            {
                checking.insert(book);
            }
        }
        let mut cursor = next;
        let mut stopped = false;
        let mut last = None;
        for entry in entries {
            let Ok(stamp) = Stamp::try_from(&entry.stamp) else {
                last = Some(entry.seq);
                continue;
            };
            if checking.contains(&stamp.book) {
                stopped = true;
                break;
            }
            // A bought book this holder does not know yet is read first;
            // its entries wait for the read (or a lagging RPC), up to a hold.
            if !granted.contains(&stamp.book)
                && !self.mailbox_holder.knows_book(&stamp.book)
                && self.book_wanted(stamp.book, true) != mailbox_chain::BookState::PassOver
            {
                stopped = true;
                break;
            }
            let before = self.mailbox_holder.summary(&mailbox).map(|s| s.count);
            match self.mailbox_holder.store_replica(
                mailbox,
                entry.period,
                &entry.envelope,
                &stamp,
                now,
            ) {
                Ok(_) => {
                    if self.mailbox_holder.summary(&mailbox).map(|s| s.count) != before {
                        self.mailbox_client.sync.stats.pulled += 1;
                    }
                }
                Err(mailbox_holder::Refusal::Conflict) => {
                    self.mailbox_client.sync.stats.conflicts += 1;
                }
                Err(_) => {}
            }
            last = Some(entry.seq);
        }
        if stopped {
            cursor = last.unwrap_or(0);
        }
        let ours = self.mailbox_holder.summary(&mailbox).map_or(0, |s| s.count);
        if let Some(pull) = self.mailbox_client.sync.pulls.get_mut(&(mailbox, unit)) {
            pull.cursor = pull.cursor.max(cursor);
            pull.ours = ours;
            pull.wanted |= full && !stopped;
        }
    }
}
