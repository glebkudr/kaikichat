//! Sender and reader of the recipient-mailbox swarm
//! (Docs/V1_STORAGE_REDESIGN_2026_09_24.md). The sender stores a stamped
//! envelope at every holder of its mailbox's swarm and calls it stored at a
//! quorum of verified receipts. The reader pages this profile's incoming
//! mailboxes of the current and previous period from every holder by cursor
//! and imports each message once, holding back one that arrives before its
//! MLS predecessor.
use super::mailbox_holder::{
    EntryWire, MAX_PAGE, ReceiptWire, Refusal, Request, Response, StampWire,
};
use super::*;
use agentic_core::{EnvelopeOrder, SwarmDelivery};
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, RETENTION_PERIODS, period};
use agentic_mailbox_swarm::receipt::Receipt;
use agentic_mailbox_swarm::select::{Member, QUORUM, SWARM_SIZE, rendezvous};
use agentic_mailbox_swarm::stamp::Stamp;
use std::collections::BTreeMap;

/// How often every incoming mailbox is read.
const READ_INTERVAL: Duration = Duration::from_secs(5);
/// Readings of the tree mailbox of a big group this profile was invited to,
/// beside the conversations.
pub(super) const TREE: &str = "tree:";
/// Oldest pending messages considered per pass.
const SEND_WINDOW: usize = 16;
/// How often group messages waiting while their group moved on are sealed
/// again.
const RESEAL_INTERVAL: Duration = Duration::from_secs(30);
/// Envelopes held back per conversation until their predecessor arrives.
const MAX_BUFFERED: usize = 256;
/// Closed periods still to read, per conversation and poll, beside the
/// current and previous one: catching up after days away is spread over
/// polls instead of one burst the processing budget would refuse.
const CATCH_UP: usize = 4;
/// Concurrent sender and reader requests: half the ordinary processing
/// slots, so a holder's own inbound work and other protocols keep the rest.
const MAX_REQUESTS: usize = 8;
/// Concurrent requests to one holder, within its per-peer processing slots.
const MAX_PER_HOLDER: usize = 2;
/// Requests started per second, all together and per holder: well within
/// the node's shared processing rate (256, and 64 per peer), which counts
/// outbound streams too.
const RATE: usize = 128;
/// Within what a holder lets one book's peers start (access by book).
const RATE_PER_HOLDER: usize = processing::BOOK_RATE;
const RATE_WINDOW: Duration = Duration::from_secs(1);
/// Failed attempts after which a stored message stops trying a holder;
/// replication inside the swarm owns that copy from then on.
const FINISH_ATTEMPTS: u32 = 3;

/// Groups of up to this many members are read at every poll from every
/// holder; bigger ones less often, from fewer
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 4).
const SMALL_GROUP: usize = 100;
const BIGGEST_GROUP: usize = 2000;
/// How often the biggest group is read.
const BIGGEST_GROUP_EVERY: Duration = Duration::from_secs(20);
/// A big group is read from `WIDE_HOLDERS` this often, from one otherwise:
/// any four meet the seven that stored an entry.
const WIDE_READ_EVERY: Duration = Duration::from_secs(60);
const WIDE_HOLDERS: usize = 4;

/// How often a group of `members` is read: at every poll up to a hundred,
/// then growing to 20 s at 2000, in steps of a tenth of a second. A node
/// polls every `READ_INTERVAL`, so it reads at the first poll after this.
pub(super) fn group_read_every(members: usize) -> Duration {
    if members <= SMALL_GROUP {
        return READ_INTERVAL;
    }
    let over = (members.min(BIGGEST_GROUP) - SMALL_GROUP) as u128;
    let span = (BIGGEST_GROUP - SMALL_GROUP) as u128;
    let extra = (BIGGEST_GROUP_EVERY - READ_INTERVAL).as_millis() * over / span;
    READ_INTERVAL + Duration::from_millis((extra / 100 * 100) as u64)
}

/// When a member reads a big group next, and from how many holders.
#[derive(Clone, Copy)]
pub(super) struct GroupSchedule {
    due: Instant,
    wide: Instant,
}

impl GroupSchedule {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            due: now,
            wide: now,
        }
    }

    /// At a poll: the number of holders to read from now, or `None` when
    /// the group's read is not due.
    pub(super) fn poll(&mut self, members: usize, now: Instant) -> Option<usize> {
        if members <= SMALL_GROUP {
            return Some(SWARM_SIZE);
        }
        if self.due > now {
            return None;
        }
        self.due = now + group_read_every(members);
        if self.wide <= now {
            self.wide = now + WIDE_READ_EVERY;
            Some(WIDE_HOLDERS)
        } else {
            Some(1)
        }
    }
}

/// A holder of the swarm directory: registry unit → transport and receipt key.
#[derive(Clone, Debug)]
pub(super) struct Holder {
    pub(super) peer: PeerId,
    pub(super) addresses: Vec<Multiaddr>,
    pub(super) account: Account,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Stats {
    /// Messages stored at a quorum.
    pub(super) stored: u64,
    pub(super) receipts: u64,
    /// Receipts that do not match the request or the listed holder key.
    pub(super) rejected_receipts: u64,
    pub(super) refusals: u64,
    /// Requests that never got a response.
    pub(super) failures: u64,
    /// Messages imported from read pages.
    pub(super) received: u64,
    /// `receive_swarm_envelope` calls (decrypt attempts), held ones retried
    /// included; the one-time order inspection of a held envelope is not
    /// counted.
    pub(super) import_attempts: u64,
    /// Entries that do not open or import.
    pub(super) rejected_entries: u64,
    /// Group messages sealed again in a later epoch.
    pub(super) resealed: u64,
}

/// What a sent receipt must match.
#[derive(Clone)]
pub(super) struct Expected {
    mailbox: [u8; 32],
    operation: [u8; 32],
    ticket: [u8; 32],
    unit: [u8; 32],
    account: Account,
}

pub(super) enum Purpose {
    Store {
        message_id: String,
        expected: Expected,
    },
    Read {
        mailbox: [u8; 32],
        unit: [u8; 32],
        /// Sent after the period's writes closed.
        closed: bool,
    },
    /// Replication: our summaries to a swarm member.
    Summaries { unit: [u8; 32] },
    /// Replication: a page of a member's copy of a mailbox.
    Pull { mailbox: [u8; 32], unit: [u8; 32] },
    /// Statements put on record with one of their keys' notaries.
    Notarize {
        statements: Vec<mailbox_holder::Statement>,
        unit: [u8; 32],
    },
    /// The proofs a member learned after `after`.
    Proofs { unit: [u8; 32], after: u64 },
    /// A holder asks one of a grant's notaries when it first saw it.
    GrantCheck { digest: [u8; 32], unit: [u8; 32] },
    /// The grant of a book a holder refused as unknown, shown to it.
    OfferGrant { message_id: String, unit: [u8; 32] },
    /// This node's pass or unit record, shown to a peer before requests
    /// that wait for it.
    Access { peer: PeerId, unit: [u8; 32] },
    /// A peer's directory; it need not be a unit.
    Directory { peer: PeerId },
    /// A page of an intro mailbox read while looking for an id's card.
    CardRead {
        network_id: String,
        period: u64,
        unit: [u8; 32],
    },
}

/// Rate window key of directory pulls, which go to peers, not units.
const DIRECTORY_PULLS: [u8; 32] = [0xdd; 32];

impl Purpose {
    /// Label of the request kind in diagnostics.
    fn kind(&self) -> &'static str {
        match self {
            Self::Store { .. } => "store",
            Self::Read { .. } => "read",
            Self::Summaries { .. } => "summaries",
            Self::Pull { .. } => "pull",
            Self::Notarize { .. } => "notarize",
            Self::Proofs { .. } => "proofs",
            Self::GrantCheck { .. } => "grantCheck",
            Self::OfferGrant { .. } => "offerGrant",
            Self::Directory { .. } => "directory",
            Self::CardRead { .. } => "cardRead",
            Self::Access { .. } => "access",
        }
    }
    pub(super) fn unit(&self) -> [u8; 32] {
        match self {
            Self::Store { expected, .. } => expected.unit,
            Self::Read { unit, .. }
            | Self::Summaries { unit }
            | Self::Pull { unit, .. }
            | Self::Notarize { unit, .. }
            | Self::Proofs { unit, .. }
            | Self::GrantCheck { unit, .. }
            | Self::OfferGrant { unit, .. }
            | Self::CardRead { unit, .. }
            | Self::Access { unit, .. } => *unit,
            Self::Directory { .. } => DIRECTORY_PULLS,
        }
    }
}

/// Request slots left in this pass: in flight, and started this second.
pub(super) struct Slots {
    total: usize,
    per_holder: BTreeMap<[u8; 32], usize>,
    started: usize,
    started_per_holder: BTreeMap<[u8; 32], usize>,
}

impl Slots {
    pub(super) fn take(&mut self, unit: &[u8; 32]) -> bool {
        let held = self.per_holder.entry(*unit).or_default();
        let started = self.started_per_holder.entry(*unit).or_default();
        if self.total >= MAX_REQUESTS
            || *held >= MAX_PER_HOLDER
            || self.started >= RATE
            || *started >= RATE_PER_HOLDER
        {
            return false;
        }
        *held += 1;
        *started += 1;
        self.total += 1;
        self.started += 1;
        true
    }
}

/// Requests started in the current second.
#[derive(Default)]
struct Window {
    start: Option<Instant>,
    total: usize,
    per_holder: BTreeMap<[u8; 32], usize>,
}

impl Window {
    fn current(&mut self, instant: Instant) -> &mut Self {
        if self
            .start
            .is_none_or(|start| instant >= start + RATE_WINDOW)
        {
            *self = Self {
                start: Some(instant),
                ..Self::default()
            };
        }
        self
    }
    /// When a full window opens again.
    fn reopens(&self) -> Option<Instant> {
        let full = self.total >= RATE || self.per_holder.values().any(|n| *n >= RATE_PER_HOLDER);
        self.start.filter(|_| full).map(|start| start + RATE_WINDOW)
    }
}

struct Sending {
    delivery: SwarmDelivery,
    swarm: Vec<[u8; 32]>,
    receipts: BTreeMap<[u8; 32], Receipt>,
    /// Holders whose answer will not change: a refusal or a bad receipt.
    refused: BTreeSet<[u8; 32]>,
    in_flight: BTreeSet<[u8; 32]>,
    retry: BTreeMap<[u8; 32], Backoff>,
    /// Stored at a quorum; the rest of the swarm still gets its copy.
    stored: bool,
}

impl Sending {
    /// Every holder answered, refused or, once stored, was given up on.
    fn finished(&self) -> bool {
        self.stored
            && self.swarm.iter().all(|unit| {
                self.receipts.contains_key(unit)
                    || self.refused.contains(unit)
                    || (!self.in_flight.contains(unit)
                        && self
                            .retry
                            .get(unit)
                            .is_some_and(|b| b.attempt >= FINISH_ATTEMPTS))
            })
    }
}

struct Reading {
    conversation: String,
    period: u64,
    swarm: Vec<[u8; 32]>,
    cursors: BTreeMap<[u8; 32], u64>,
    in_flight: BTreeSet<[u8; 32]>,
    /// Holders to read next: all of them at a poll, again after a full page.
    pending: BTreeSet<[u8; 32]>,
    /// Operations imported, rejected or held back.
    seen: BTreeSet<[u8; 32]>,
    /// Holders read to the end after the period's writes closed.
    ended: BTreeSet<[u8; 32]>,
}

struct Held {
    period: u64,
    envelope: Vec<u8>,
    op: [u8; 32],
}

#[derive(Clone, Copy)]
struct Backoff {
    attempt: u32,
    due: Instant,
}

impl Backoff {
    fn after(previous: Option<Backoff>, now: Instant) -> Self {
        let attempt = previous.map_or(0, |b| b.attempt.saturating_add(1));
        let millis = 500u64.saturating_mul(1u64 << attempt.min(6)).min(30_000);
        Self {
            attempt,
            due: now + Duration::from_millis(millis),
        }
    }
}

#[derive(Default)]
pub(super) struct Client {
    /// Outcomes of raw requests not issued by the sender or reader.
    pub(super) results:
        BTreeMap<request_response::OutboundRequestId, std::result::Result<Response, String>>,
    pub(super) directory: BTreeMap<[u8; 32], Holder>,
    sends: BTreeMap<String, Sending>,
    /// Messages whose delivery cannot be prepared yet (no verified book).
    blocked: BTreeMap<String, Backoff>,
    reads: BTreeMap<[u8; 32], Reading>,
    /// By conversation, in each sender's MLS order.
    held: BTreeMap<String, BTreeMap<EnvelopeOrder, Held>>,
    /// Turns the catch-up window over closed periods from poll to poll.
    rotation: usize,
    /// Periods read completely, per conversation, not yet recorded as read
    /// through because an earlier one is still open.
    completed: BTreeMap<String, BTreeSet<u64>>,
    pub(super) requests: HashMap<request_response::OutboundRequestId, Purpose>,
    /// The peer each request went to.
    peers: HashMap<request_response::OutboundRequestId, PeerId>,
    /// Outcomes decided here without asking (no credential to show),
    /// delivered at the next pass.
    pub(super) local: Vec<(Purpose, std::result::Result<Response, String>)>,
    /// Access by book: who takes requests here, and what this node showed.
    access_gate: processing::Gate,
    pub(super) access: mailbox_access::Access,
    /// Requests a peer may refuse for want of a credential, kept to send
    /// again behind it.
    pub(super) access_copies: HashMap<request_response::OutboundRequestId, (Holder, Request)>,
    pub(super) sync: mailbox_replication::Sync,
    pub(super) notary: mailbox_notary::Lane,
    pub(super) grants: mailbox_grants::Lane,
    pub(super) gossip: mailbox_proofs::Gossip,
    poll_due: Option<Instant>,
    reseal_due: Option<Instant>,
    window: Window,
    /// Requests this node sent, and served as a holder, by kind.
    pub(super) sent: BTreeMap<&'static str, u64>,
    /// Failed requests by cause: processing capacity, request rate, dial.
    failure_kinds: BTreeMap<&'static str, u64>,
    pub(super) served: BTreeMap<&'static str, u64>,
    stats: Stats,
    /// Read requests sent, by conversation, since the node started.
    asked: BTreeMap<String, u64>,
    /// When each big group is read next.
    group_schedules: BTreeMap<String, GroupSchedule>,
    last_failure: Option<String>,
    /// Contact by ID: the card, its publication and lookups.
    pub(super) intro: mailbox_intro::Intro,
    /// Groups: claims on record and decisions.
    pub(super) groups: mailbox_groups::GroupLane,
    /// Open-read groups: rosters published, follows read.
    pub(super) public: mailbox_public::Public,
    /// Groups' doors: cards published, applications read, batches.
    pub(super) door: mailbox_door::DoorLane,
}

impl Client {
    pub(super) fn new(access_gate: processing::Gate) -> Self {
        Self {
            access_gate,
            ..Self::default()
        }
    }
    /// The directory's members; the units among them are let in without a
    /// pass.
    pub(super) fn set_directory(&mut self, members: BTreeMap<[u8; 32], Holder>) {
        self.directory = members;
        self.list_units();
    }
    fn list_units(&self) {
        self.access_gate
            .set_units(self.directory.values().map(|holder| holder.peer).collect());
    }
    /// Until phase 2's signed directory, holders are learned directly.
    #[cfg_attr(not(test), allow(dead_code, reason = "phase 2 fills the directory"))]
    pub(super) fn learn_holder(&mut self, unit: [u8; 32], holder: Holder) {
        self.directory.insert(unit, holder);
        self.list_units();
    }
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the managed-time driver waits on it")
    )]
    pub(super) fn in_flight(&self) -> usize {
        self.requests.len() + self.access.waiting().count() + self.local.len()
    }
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn stats(&self) -> Stats {
        self.stats.clone()
    }
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn buffered(&self) -> usize {
        self.held.values().map(BTreeMap::len).sum()
    }
    #[cfg_attr(not(test), allow(dead_code, reason = "phase 2 manages the directory"))]
    pub(super) fn forget_holder(&mut self, unit: &[u8; 32]) {
        self.directory.remove(unit);
        self.list_units();
    }
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn replication(&self) -> mailbox_replication::Replication {
        self.sync.stats.clone()
    }
    /// Diagnostics: directory, jobs, reads and counters.
    pub(super) fn info(&self) -> Value {
        let now = clock::instant();
        json!({
            "holders": self.directory.len(),
            "requests": self.requests.len(),
            "sends": self.sends.iter().map(|(id, s)| json!({
                "messageId": id,
                "period": s.delivery.period,
                "receipts": s.receipts.len(),
                "refused": s.refused.len(),
                "inFlight": s.in_flight.len(),
                "stored": s.stored,
            })).collect::<Vec<_>>(),
            "blocked": self.blocked.len(),
            "reads": self.reads.values().map(|r| json!({
                "conversationId": r.conversation,
                "period": r.period,
                "holders": r.swarm.len(),
                "cursors": r.cursors.values().copied().collect::<Vec<_>>(),
                "inFlight": r.in_flight.len(),
                "pending": r.pending.len(),
                "seen": r.seen.len(),
            })).collect::<Vec<_>>(),
            "asked": self.asked,
            "held": self.held.values().map(BTreeMap::len).sum::<usize>(),
            "pollInMs": self.poll_due.map(|due| due.saturating_duration_since(now).as_millis() as u64),
            "stored": self.stats.stored,
            "receipts": self.stats.receipts,
            "rejectedReceipts": self.stats.rejected_receipts,
            "refusals": self.stats.refusals,
            "failures": self.stats.failures,
            "received": self.stats.received,
            "importAttempts": self.stats.import_attempts,
            "rejectedEntries": self.stats.rejected_entries,
            "resealed": self.stats.resealed,
            "lastFailure": self.last_failure,
            "notary": {
                "waiting": self.notary.waiting(),
                "batches": self.notary.batches,
                "notarized": self.notary.notarized,
                "proofs": self.notary.proofs,
            },
            "grants": self.grants.info(),
            "intro": self.intro.info(),
            "groups": self.groups.info(),
            "sent": self.sent,
            "access": self.access.info(),
            "public": self.public.info(),
            "door": self.door.info(),
            "failureKinds": self.failure_kinds,
            "served": self.served,
            "proofs": {
                "learned": self.gossip.learned,
                "blockedHolders": self.gossip.blocked.len(),
            },
            "replication": {
                "pulled": self.sync.stats.pulled,
                "conflicts": self.sync.stats.conflicts,
                "summaries": self.sync.stats.summaries,
                "pages": self.sync.stats.pages,
                "pulls": self.sync_pulls(),
            },
        })
    }
    /// A new book arrived: messages that waited for one are prepared now.
    pub(super) fn retry_blocked(&mut self) {
        self.blocked.clear();
    }
    /// The earliest retry, blocked message or read poll.
    pub(super) fn next_due(&self) -> Option<Instant> {
        let local = (!self.local.is_empty()).then(clock::instant);
        self.sends
            .values()
            .flat_map(|s| {
                s.retry
                    .iter()
                    .filter(|(unit, _)| {
                        !s.in_flight.contains(*unit)
                            && !s.receipts.contains_key(*unit)
                            && !s.refused.contains(*unit)
                    })
                    .map(|(_, b)| b.due)
            })
            .chain(self.blocked.values().map(|b| b.due))
            .chain(self.poll_due)
            .chain(self.sync.next_due())
            .chain(self.notary.next_due())
            .chain(self.window.reopens())
            .chain(self.gossip.next_due())
            .chain(self.intro.next_due())
            .chain(self.groups.next_due())
            .chain(self.public.next_due())
            .chain(self.door.next_due())
            .chain(self.access.next_due())
            .chain(local)
            .min()
    }
    fn sync_pulls(&self) -> usize {
        self.sync.pulls()
    }
    /// Listed and not proven to equivocate.
    pub(super) fn usable(&self, unit: &[u8; 32]) -> bool {
        self.directory
            .get(unit)
            .is_some_and(|holder| !self.gossip.blocked.contains(&holder.account))
    }
    pub(super) fn slots(&mut self) -> Slots {
        let mut per_holder = BTreeMap::new();
        let mut total = 0;
        for purpose in self.requests.values().chain(self.access.waiting()) {
            *per_holder.entry(purpose.unit()).or_default() += 1;
            total += 1;
        }
        let window = self.window.current(clock::instant());
        Slots {
            total,
            per_holder,
            started: window.total,
            started_per_holder: window.per_holder.clone(),
        }
    }
    /// A holder whose answer about a message will not change.
    pub(super) fn refuse_holder(&mut self, message_id: &str, unit: [u8; 32]) {
        let Some(sending) = self.sends.get_mut(message_id) else {
            return;
        };
        sending.refused.insert(unit);
        if sending.finished() {
            self.sends.remove(message_id);
        }
    }
    pub(super) fn sending(&self, id: &str) -> bool {
        self.sends.contains_key(id)
    }
    /// Start storing `delivery` at its swarm under the job `id`.
    pub(super) fn start_send(&mut self, id: String, delivery: SwarmDelivery) {
        let swarm = self.swarm(&delivery.mailbox);
        self.sends.insert(
            id,
            Sending {
                delivery,
                swarm,
                receipts: BTreeMap::new(),
                refused: BTreeSet::new(),
                in_flight: BTreeSet::new(),
                retry: BTreeMap::new(),
                stored: false,
            },
        );
    }
    pub(super) fn swarm(&self, mailbox: &[u8; 32]) -> Vec<[u8; 32]> {
        let members: Vec<_> = self
            .directory
            .keys()
            .map(|commitment| Member {
                commitment: *commitment,
            })
            .collect();
        rendezvous(mailbox, &members, SWARM_SIZE)
            .into_iter()
            .map(|m| m.commitment)
            .collect()
    }
}

/// Refusals a holder may lift later: it learns the book or its unit, or its
/// storage recovers.
fn transient(code: &str) -> bool {
    matches!(
        code,
        "storage" | "no_unit" | "unknown_book" | "access_required" | "grant_pending"
    )
}

impl Runtime {
    /// Send one mailbox request and remember what it is for.
    /// Send without looking at access (`send_mailbox` looks first).
    pub(super) fn send_raw(
        &mut self,
        holder: Holder,
        request: Request,
        purpose: Purpose,
    ) -> request_response::OutboundRequestId {
        let peer = holder.peer;
        let id = self
            .swarm
            .behaviour_mut()
            .mailbox
            .send_request_with_addresses(&peer, request, holder.addresses);
        let client = &mut self.mailbox_client;
        client.peers.insert(id, peer);
        *client.sent.entry(purpose.kind()).or_default() += 1;
        let window = client.window.current(clock::instant());
        window.total += 1;
        *window.per_holder.entry(purpose.unit()).or_default() += 1;
        client.requests.insert(id, purpose);
        id
    }

    pub(super) fn maintain_mailbox_swarm(&mut self) {
        for (purpose, outcome) in std::mem::take(&mut self.mailbox_client.local) {
            self.purpose_outcome(purpose, outcome);
        }
        self.maintain_access();
        let Ok(now) = now() else { return };
        let instant = clock::instant();
        self.maintain_intro_card(now, instant);
        self.maintain_public_rosters(now, instant);
        self.maintain_channel_archives(now, instant);
        self.maintain_channel_keys(now, instant);
        self.maintain_doors(now, instant);
        self.maintain_rejoins(now, instant);
        self.maintain_swarm_sends(now, instant);
        self.maintain_swarm_reads(now, instant);
        self.maintain_lookups(now, instant);
        self.maintain_groups(now, instant);
        self.maintain_replication(instant);
        self.maintain_notary();
        self.maintain_grants(now, instant);
        self.maintain_proofs(instant);
    }

    fn maintain_swarm_sends(&mut self, now: u64, instant: Instant) {
        // Group messages that waited while their group moved on are sealed
        // again; what was in flight for them goes.
        if self
            .mailbox_client
            .reseal_due
            .is_none_or(|due| due <= instant)
        {
            self.mailbox_client.reseal_due = Some(instant + RESEAL_INTERVAL);
            if let Ok(resealed) = self.core.reseal_stale_group_sends() {
                for id in resealed {
                    self.mailbox_client.sends.remove(&id);
                    self.mailbox_client.stats.resealed += 1;
                }
            }
        }
        if self.mailbox_client.directory.len() < QUORUM {
            return;
        }
        let Ok(pending) = self.core.swarm_outbox(SEND_WINDOW) else {
            return;
        };
        let client = &mut self.mailbox_client;
        let ids: BTreeSet<_> = pending.iter().map(|p| p.message_id.clone()).collect();
        client.sends.retain(|id, s| {
            (s.stored
                || ids.contains(id)
                || id.starts_with(mailbox_intro::CARD_JOB)
                || id.starts_with(mailbox_public::ROSTER_JOB)
                || id.starts_with(mailbox_public::ARCHIVE_JOB)
                || id.starts_with(mailbox_public::KEYS_JOB)
                || id.starts_with(mailbox_public::HELLO_JOB)
                || id.starts_with(mailbox_door::DOOR_CARD_JOB))
                && !s.finished()
        });
        client.blocked.retain(|id, _| ids.contains(id));
        for item in pending {
            let id = item.message_id;
            // The recipient no longer reads the pinned period: re-prepare.
            if self
                .mailbox_client
                .sends
                .get(&id)
                .is_some_and(|s| period(now) > s.delivery.period + 1)
            {
                self.mailbox_client.sends.remove(&id);
            }
            if self.mailbox_client.sends.contains_key(&id)
                || self
                    .mailbox_client
                    .blocked
                    .get(&id)
                    .is_some_and(|b| b.due > instant)
            {
                continue;
            }
            match self.core.prepare_swarm_delivery(&id, now) {
                Ok(delivery) => {
                    // The spent slot goes on record with its notaries.
                    self.notarize_later(mailbox_holder::Statement::Ticket(delivery.stamp.clone()));
                    let client = &mut self.mailbox_client;
                    client.blocked.remove(&id);
                    let swarm = client.swarm(&delivery.mailbox);
                    client.sends.insert(
                        id,
                        Sending {
                            delivery,
                            swarm,
                            receipts: BTreeMap::new(),
                            refused: BTreeSet::new(),
                            in_flight: BTreeSet::new(),
                            retry: BTreeMap::new(),
                            stored: false,
                        },
                    );
                }
                Err(_) => {
                    let client = &mut self.mailbox_client;
                    let previous = client.blocked.get(&id).copied();
                    client.blocked.insert(id, Backoff::after(previous, instant));
                }
            }
        }
        let client = &mut self.mailbox_client;
        let mut slots = client.slots();
        let mut outgoing = Vec::new();
        for (id, sending) in &mut client.sends {
            let d = &sending.delivery;
            for unit in &sending.swarm {
                if sending.receipts.contains_key(unit)
                    || sending.refused.contains(unit)
                    || sending.in_flight.contains(unit)
                    || sending.retry.get(unit).is_some_and(|b| {
                        b.due > instant || (sending.stored && b.attempt >= FINISH_ATTEMPTS)
                    })
                {
                    continue;
                }
                let Some(holder) = client.directory.get(unit) else {
                    continue;
                };
                if client.gossip.blocked.contains(&holder.account) {
                    sending.refused.insert(*unit);
                    continue;
                }
                if !slots.take(unit) {
                    continue;
                }
                sending.in_flight.insert(*unit);
                outgoing.push((
                    holder.clone(),
                    Request::Store {
                        mailbox: d.mailbox.to_vec(),
                        period: d.period,
                        envelope: d.envelope.clone(),
                        stamp: StampWire::from(&d.stamp),
                    },
                    Purpose::Store {
                        message_id: id.clone(),
                        expected: Expected {
                            mailbox: d.mailbox,
                            operation: d.stamp.operation,
                            ticket: d.stamp.ticket_id(&NETWORK_DOMAIN),
                            unit: *unit,
                            account: holder.account,
                        },
                    },
                ));
            }
        }
        for (holder, request, purpose) in outgoing {
            self.send_mailbox(holder, request, purpose);
        }
    }

    fn maintain_swarm_reads(&mut self, now: u64, instant: Instant) {
        if self.mailbox_client.directory.is_empty() {
            return;
        }
        let poll = self
            .mailbox_client
            .poll_due
            .is_none_or(|due| due <= instant);
        if poll {
            self.mailbox_client.poll_due = Some(instant + READ_INTERVAL);
            let current = period(now);
            let mut wanted = BTreeMap::new();
            let mut active = BTreeSet::new();
            // Mailboxes read from a few random holders only.
            let mut subsets = BTreeMap::new();
            let rotation = self.mailbox_client.rotation;
            self.mailbox_client.rotation = rotation.wrapping_add(CATCH_UP);
            let mut after: Option<String> = None;
            while let Ok(ids) = self.core.conversation_ids(after.as_deref(), 32) {
                let Some(last) = ids.last().cloned() else {
                    break;
                };
                for id in ids {
                    let Some(from) = self.read_from(&id, current) else {
                        continue;
                    };
                    // The two writable periods every poll; closed ones not
                    // yet read completely a few at a time, in turn.
                    let done = self.mailbox_client.completed.get(&id);
                    let closed: Vec<u64> = (from..current.saturating_sub(1))
                        .filter(|p| done.is_none_or(|done| !done.contains(p)))
                        .collect();
                    let turn: BTreeSet<u64> = if closed.len() <= CATCH_UP {
                        closed.iter().copied().collect()
                    } else {
                        (0..CATCH_UP)
                            .map(|i| closed[(rotation + i) % closed.len()])
                            .collect()
                    };
                    for p in from..=current {
                        if let Ok(mailbox) = self.core.swarm_mailbox(&id, true, p * PERIOD_SECONDS)
                        {
                            wanted.insert(mailbox, (id.clone(), p));
                            if p + 1 >= current || turn.contains(&p) {
                                active.insert(mailbox);
                            }
                        }
                    }
                }
                after = Some(last);
            }
            // This profile's own intro mailboxes, read the same way, less often.
            let intro_due = self
                .mailbox_client
                .intro
                .read_due
                .is_none_or(|due| due <= instant);
            if intro_due {
                self.mailbox_client.intro.read_due =
                    Some(instant + mailbox_intro::INTRO_READ_EVERY);
            }
            if let Some(from) = self.intro_read_from(current) {
                let done = self.mailbox_client.completed.get(mailbox_intro::INTRO);
                let closed: Vec<u64> = (from..current.saturating_sub(1))
                    .filter(|p| done.is_none_or(|done| !done.contains(p)))
                    .collect();
                let turn: BTreeSet<u64> = if closed.len() <= CATCH_UP {
                    closed.iter().copied().collect()
                } else {
                    (0..CATCH_UP)
                        .map(|i| closed[(rotation + i) % closed.len()])
                        .collect()
                };
                for p in from..=current {
                    if let Ok(mailbox) = self.core.own_intro_mailbox(p * PERIOD_SECONDS) {
                        wanted.insert(mailbox, (mailbox_intro::INTRO.to_owned(), p));
                        if intro_due && (p + 1 >= current || turn.contains(&p)) {
                            active.insert(mailbox);
                        }
                    }
                }
            }
            // Group mailboxes: the newest two epochs', read the same way.
            if let Ok(groups) = self.core.groups() {
                for info in groups {
                    let Some(from) = self.read_from(&info.id, current) else {
                        continue;
                    };
                    let done = self.mailbox_client.completed.get(&info.id);
                    let closed: Vec<u64> = (from..current.saturating_sub(1))
                        .filter(|p| done.is_none_or(|done| !done.contains(p)))
                        .collect();
                    let turn: BTreeSet<u64> = if closed.len() <= CATCH_UP {
                        closed.iter().copied().collect()
                    } else {
                        (0..CATCH_UP)
                            .map(|i| closed[(rotation + i) % closed.len()])
                            .collect()
                    };
                    // A big group is read less often, from fewer holders.
                    let holders = self
                        .mailbox_client
                        .group_schedules
                        .entry(info.id.clone())
                        .or_insert_with(|| GroupSchedule::new(instant))
                        .poll(info.members.len(), instant);
                    for p in from..=current {
                        let Ok(mailboxes) = self.core.group_mailboxes(&info.id, p * PERIOD_SECONDS)
                        else {
                            continue;
                        };
                        for mailbox in mailboxes {
                            wanted.insert(mailbox, (info.id.clone(), p));
                            let Some(holders) = holders else {
                                continue;
                            };
                            if p + 1 >= current || turn.contains(&p) {
                                active.insert(mailbox);
                                if holders < SWARM_SIZE {
                                    subsets.insert(mailbox, holders);
                                }
                            }
                        }
                    }
                }
            }
            // Trees of big groups this profile was let into: it joins once
            // one is whole.
            for p in current.saturating_sub(1)..=current {
                if let Ok(trees) = self.core.group_tree_mailboxes(p * PERIOD_SECONDS) {
                    for (group, mailbox) in trees {
                        wanted.insert(mailbox, (format!("{TREE}{group}"), p));
                        active.insert(mailbox);
                    }
                }
            }
            // The doors this profile opens, when their read is due.
            for (mailbox, conversation, p, due) in self.door_reads(now, instant) {
                wanted.insert(mailbox, (conversation, p));
                if due {
                    active.insert(mailbox);
                }
            }
            // Open groups' public mailboxes: a member's every poll, a
            // follower's when its read is due, from a few holders.
            for (mailbox, conversation, p, read) in self.public_reads(now, instant) {
                wanted.insert(mailbox, (conversation, p));
                match read {
                    mailbox_public::Read::Every => {
                        active.insert(mailbox);
                    }
                    mailbox_public::Read::Few(holders) => {
                        active.insert(mailbox);
                        subsets.insert(mailbox, holders);
                    }
                    mailbox_public::Read::Later => {}
                }
            }
            let client = &mut self.mailbox_client;
            client
                .reads
                .retain(|mailbox, _| wanted.contains_key(mailbox));
            for (mailbox, (conversation, p)) in wanted {
                let swarm = client.swarm(&mailbox);
                client
                    .reads
                    .entry(mailbox)
                    .or_insert_with(|| Reading {
                        conversation,
                        period: p,
                        swarm: vec![],
                        cursors: BTreeMap::new(),
                        in_flight: BTreeSet::new(),
                        pending: BTreeSet::new(),
                        seen: BTreeSet::new(),
                        ended: BTreeSet::new(),
                    })
                    .swarm = swarm;
            }
            for (mailbox, reading) in client.reads.iter_mut() {
                if !active.contains(mailbox) {
                    continue;
                }
                let mut idle: Vec<[u8; 32]> = reading
                    .swarm
                    .iter()
                    .filter(|unit| !reading.in_flight.contains(*unit))
                    .copied()
                    .collect();
                // A few random holders of the swarm: any four meet the seven
                // that stored an entry.
                if let Some(holders) = subsets.get(mailbox) {
                    let mut pick = [0; 8];
                    let _ = random::fill(&mut pick);
                    let start = u64::from_be_bytes(pick) as usize;
                    let len = idle.len();
                    if len > 0 {
                        idle.rotate_left(start % len);
                    }
                    idle.truncate(*holders);
                }
                reading.pending = idle.into_iter().collect();
            }
        }
        let client = &mut self.mailbox_client;
        let mut slots = client.slots();
        let mut outgoing = Vec::new();
        for (mailbox, reading) in &mut client.reads {
            let pending: Vec<_> = reading.pending.iter().copied().collect();
            for unit in &pending {
                if reading.in_flight.contains(unit) {
                    continue;
                }
                let Some(holder) = client
                    .directory
                    .get(unit)
                    .filter(|holder| !client.gossip.blocked.contains(&holder.account))
                else {
                    reading.pending.remove(unit);
                    continue;
                };
                if !slots.take(unit) {
                    continue;
                }
                reading.in_flight.insert(*unit);
                *client
                    .asked
                    .entry(reading.conversation.clone())
                    .or_default() += 1;
                reading.pending.remove(unit);
                outgoing.push((
                    holder.clone(),
                    Request::Read {
                        mailbox: mailbox.to_vec(),
                        after: reading.cursors.get(unit).copied().unwrap_or(0),
                        limit: MAX_PAGE as u16,
                    },
                    Purpose::Read {
                        mailbox: *mailbox,
                        unit: *unit,
                        closed: reading.period + 2 <= period(now),
                    },
                ));
            }
        }
        for (holder, request, purpose) in outgoing {
            self.send_mailbox(holder, request, purpose);
        }
    }

    /// Route the outcome of a mailbox request to the sender or reader that
    /// issued it; a raw request's outcome is kept for its caller.
    pub(super) fn mailbox_outcome(
        &mut self,
        request_id: request_response::OutboundRequestId,
        outcome: std::result::Result<Response, String>,
    ) {
        let peer = self.mailbox_client.peers.remove(&request_id);
        let Some(purpose) = self.mailbox_client.requests.remove(&request_id) else {
            self.mailbox_client.results.insert(request_id, outcome);
            return;
        };
        let copy = self.mailbox_client.access_copies.remove(&request_id);
        match (peer, outcome) {
            // A peer that no longer lets this node in: the request waits for
            // the credential and goes again, unseen by its lane.
            (Some(peer), Ok(Response::Refused { code }))
                if code == "access_required" && !matches!(purpose, Purpose::Access { .. }) =>
            {
                self.access_lost(peer, copy, purpose);
            }
            (_, outcome) => self.purpose_outcome(purpose, outcome),
        }
    }

    /// Deliver an outcome to what asked for it.
    pub(super) fn purpose_outcome(
        &mut self,
        purpose: Purpose,
        outcome: std::result::Result<Response, String>,
    ) {
        if let Err(error) = &outcome {
            self.mailbox_client.stats.failures += 1;
            let kind = if error.starts_with("access") {
                "access"
            } else if error.contains("capacity") {
                "capacity"
            } else if error.contains("rate") {
                "rate"
            } else if error.contains("dial") {
                "dial"
            } else {
                "other"
            };
            *self.mailbox_client.failure_kinds.entry(kind).or_default() += 1;
            self.mailbox_client.last_failure =
                Some(format!("{}: {error}", hex::encode(&purpose.unit()[..4])));
        }
        match purpose {
            Purpose::Store {
                message_id,
                expected,
            } => self.swarm_stored(message_id, expected, outcome),
            Purpose::Read {
                mailbox,
                unit,
                closed,
            } => self.swarm_page(mailbox, unit, closed, outcome),
            Purpose::Summaries { unit } => self.replication_summaries(unit, outcome),
            Purpose::Pull { mailbox, unit } => self.replication_page(mailbox, unit, outcome),
            Purpose::Notarize { statements, unit } => self.notary_answer(unit, statements, outcome),
            Purpose::Proofs { unit, after } => self.proofs_answer(unit, after, outcome),
            Purpose::GrantCheck { digest, unit } => self.grant_answer(digest, unit, outcome),
            Purpose::Directory { peer } => self.directory_answer(peer, outcome),
            Purpose::OfferGrant { message_id, unit } => {
                self.grant_offered(message_id, unit, outcome)
            }
            Purpose::CardRead {
                network_id,
                period,
                unit,
            } => self.card_page(network_id, period, unit, outcome),
            Purpose::Access { peer, .. } => self.access_answer(peer, outcome),
        }
    }

    fn swarm_stored(
        &mut self,
        message_id: String,
        expected: Expected,
        outcome: std::result::Result<Response, String>,
    ) {
        let client = &mut self.mailbox_client;
        // Checked even after the quorum, so a bad holder is always counted.
        let verdict = match &outcome {
            Ok(Response::Stored { receipt }) => {
                let receipt = verified_receipt(receipt, &expected);
                if receipt.is_some() {
                    client.stats.receipts += 1;
                } else {
                    client.stats.rejected_receipts += 1;
                }
                Some(receipt)
            }
            Ok(Response::Refused { .. }) => {
                client.stats.refusals += 1;
                None
            }
            Ok(
                Response::Page { .. }
                | Response::Summaries { .. }
                | Response::Notarized { .. }
                | Response::Proofs { .. }
                | Response::Learned
                | Response::Directory { .. }
                | Response::Access { .. },
            ) => {
                client.stats.rejected_receipts += 1;
                Some(None)
            }
            Err(_) => None,
        };
        // The job may have moved to another period since the request.
        let Some(sending) = client.sends.get_mut(&message_id).filter(|s| {
            (s.delivery.mailbox, s.delivery.stamp.operation)
                == (expected.mailbox, expected.operation)
        }) else {
            return;
        };
        let unit = expected.unit;
        sending.in_flight.remove(&unit);
        // A holder that does not know a granted book is shown its grant.
        if matches!(&outcome, Ok(Response::Refused { code }) if code == Refusal::UnknownBook.code())
            && let Ok(Some(grant)) = self.core.mailbox_book_grant(&sending.delivery.stamp.book)
            && let Some(holder) = client.directory.get(&unit)
        {
            let id = self
                .swarm
                .behaviour_mut()
                .mailbox
                .send_request_with_addresses(
                    &holder.peer,
                    Request::LearnGrant { grant },
                    holder.addresses.clone(),
                );
            *client.sent.entry("offerGrant").or_default() += 1;
            let window = client.window.current(clock::instant());
            window.total += 1;
            *window.per_holder.entry(unit).or_default() += 1;
            client.requests.insert(
                id,
                Purpose::OfferGrant {
                    message_id: message_id.clone(),
                    unit,
                },
            );
        }
        match (verdict, &outcome) {
            (Some(Some(receipt)), _) => {
                sending.receipts.insert(unit, receipt);
            }
            (Some(None), _) => {
                sending.refused.insert(unit);
            }
            (None, Ok(Response::Refused { code })) if !transient(code) => {
                sending.refused.insert(unit);
            }
            _ => {
                let previous = sending.retry.get(&unit).copied();
                sending
                    .retry
                    .insert(unit, Backoff::after(previous, clock::instant()));
            }
        }
        if sending.finished() {
            client.sends.remove(&message_id);
            return;
        }
        if sending.stored || sending.receipts.len() < QUORUM {
            return;
        }
        let swarm: Vec<Account> = sending
            .swarm
            .iter()
            .filter_map(|unit| client.directory.get(unit).map(|h| h.account))
            .collect();
        let receipts: Vec<Receipt> = sending.receipts.values().cloned().collect();
        let card = message_id.starts_with(mailbox_intro::CARD_JOB);
        let roster = message_id.starts_with(mailbox_public::ROSTER_JOB)
            || message_id.starts_with(mailbox_public::ARCHIVE_JOB)
            || message_id.starts_with(mailbox_public::KEYS_JOB)
            || message_id.starts_with(mailbox_public::HELLO_JOB)
            || message_id.starts_with(mailbox_door::DOOR_CARD_JOB);
        if message_id.starts_with(mailbox_door::DOOR_CARD_JOB) {
            client.door.stored(&message_id);
        } else if roster {
            client.public.stored(&message_id);
        }
        // A key update stored at a quorum is published.
        if let Some(job) = message_id.strip_prefix(mailbox_public::KEYS_JOB) {
            let parts: Vec<&str> = job.split(':').collect();
            if let [group, hash, _] = parts.as_slice()
                && self.core.channel_key_doc_stored(group, hash).is_ok()
            {
                client.public.key_updates += 1;
            }
        }
        // A subscriber's own key stored at a quorum is published.
        if let Some(job) = message_id.strip_prefix(mailbox_public::HELLO_JOB) {
            let parts: Vec<&str> = job.split(':').collect();
            if let [conversation, hash, _] = parts.as_slice()
                && self.core.channel_hello_stored(conversation, hash).is_ok()
            {
                client.public.hellos += 1;
            }
        }
        // An archive part stored at a quorum is laid that day.
        if let Some(job) = message_id.strip_prefix(mailbox_public::ARCHIVE_JOB) {
            let parts: Vec<&str> = job.split(':').collect();
            if let [group, hash, day] = parts.as_slice()
                && let Ok(day) = day.parse()
            {
                let _ = self.core.channel_part_stored(group, hash, day);
            }
        }
        if card {
            client.intro.stored(&message_id);
        }
        if (card
            || roster
            || self
                .core
                .complete_swarm_delivery(&message_id, &swarm, &receipts)
                .is_ok())
            && let Some(sending) = self.mailbox_client.sends.get_mut(&message_id)
        {
            sending.stored = true;
            self.mailbox_client.stats.stored += 1;
            if sending.finished() {
                self.mailbox_client.sends.remove(&message_id);
            }
        }
    }

    fn swarm_page(
        &mut self,
        mailbox: [u8; 32],
        unit: [u8; 32],
        closed: bool,
        outcome: std::result::Result<Response, String>,
    ) {
        let client = &mut self.mailbox_client;
        let Some(reading) = client.reads.get_mut(&mailbox) else {
            return;
        };
        reading.in_flight.remove(&unit);
        let Ok(Response::Page { entries, next, .. }) = outcome else {
            return;
        };
        let cursor = reading.cursors.entry(unit).or_insert(0);
        if next <= *cursor && !entries.is_empty() {
            // A holder that pages backwards is not read further this poll.
            client.stats.rejected_entries += 1;
            return;
        }
        *cursor = next.max(*cursor);
        if entries.len() >= MAX_PAGE {
            reading.pending.insert(unit);
        } else if closed {
            // Read to the end by a request sent after no one could write
            // this period any more.
            reading.ended.insert(unit);
        }
        let complete = reading.ended.len() >= QUORUM.min(reading.swarm.len().max(1));
        let (conversation, period) = (reading.conversation.clone(), reading.period);
        let mut fresh = Vec::new();
        for entry in entries {
            let stored_at = entry.stored_at;
            match fresh_entry(&mailbox, entry) {
                Some((op, envelope)) if reading.seen.insert(op) => {
                    fresh.push((op, envelope, stored_at));
                }
                Some(_) => {}
                None => client.stats.rejected_entries += 1,
            }
        }
        if conversation.starts_with(mailbox_public::PUBLIC) {
            // Only the current and previous day of a public mailbox are read.
            for (_, envelope, stored_at) in fresh {
                self.public_import(&conversation, &envelope, stored_at);
            }
            return;
        }
        if conversation.starts_with(mailbox_door::DOOR) {
            for (_, envelope, _) in fresh {
                self.door_import(&conversation, period, &envelope);
            }
            return;
        }
        if let Some(group) = conversation.strip_prefix(TREE) {
            let Ok(now) = now() else { return };
            for (_, envelope, _) in fresh {
                match self
                    .core
                    .receive_group_tree_envelope(group, period, &envelope, now)
                {
                    Ok(Some(_)) => self.mailbox_client.stats.received += 1,
                    Ok(None) => {}
                    Err(_) => self.mailbox_client.stats.rejected_entries += 1,
                }
            }
            return;
        }
        for (op, envelope, _) in fresh {
            self.swarm_import(&conversation, period, op, envelope);
        }
        if complete {
            self.read_completely(&conversation, period);
        }
    }

    fn swarm_import(&mut self, conversation: &str, period: u64, op: [u8; 32], envelope: Vec<u8>) {
        let Ok(now) = now() else { return };
        self.mailbox_client.stats.import_attempts += 1;
        if conversation == mailbox_intro::INTRO {
            self.intro_import(&envelope, now);
            return;
        }
        match self
            .core
            .receive_swarm_envelope(conversation, period, &envelope, now)
        {
            Ok(_) => {
                self.mailbox_client.stats.received += 1;
                self.release_held(conversation, now);
            }
            Err(CoreError::Crypto(agentic_crypto::CryptoError::ReceiveGap)) => {
                // Held in its sender's order; its predecessor lets it go.
                match self
                    .core
                    .swarm_envelope_order(conversation, period, &envelope, now)
                {
                    Ok(order) => self.hold(
                        conversation,
                        order,
                        Held {
                            period,
                            envelope,
                            op,
                        },
                    ),
                    // Too far ahead to place: read again later.
                    Err(_) => self.forget_seen(conversation, period, &op),
                }
            }
            Err(_) => self.mailbox_client.stats.rejected_entries += 1,
        }
    }

    /// Record a completely read period; move the conversation's read-through
    /// period over every contiguous one.
    /// The first period of a conversation's incoming mailboxes still to read:
    /// after its read-through period, else from the period before the
    /// conversation began (nothing predates it), within retention. A start
    /// ahead of this clock still reads the current and previous period.
    fn read_from(&self, conversation: &str, current: u64) -> Option<u64> {
        if conversation == mailbox_intro::INTRO {
            return self.intro_read_from(current);
        }
        let floor = current.saturating_sub(RETENTION_PERIODS);
        let from = match self.core.swarm_read_through(conversation).ok()? {
            Some(through) => through + 1,
            None => self
                .core
                .conversation_started_at(conversation)
                .map_or(floor, |at| period(at).saturating_sub(1)),
        };
        Some(from.max(floor).min(current.saturating_sub(1).max(floor)))
    }

    fn read_completely(&mut self, conversation: &str, completed: u64) {
        let Ok(now) = now() else { return };
        let Some(from) = self.read_from(conversation, period(now)) else {
            return;
        };
        let done = self
            .mailbox_client
            .completed
            .entry(conversation.into())
            .or_default();
        done.insert(completed);
        let mut next = from;
        let mut last = None;
        while done.contains(&next) {
            last = Some(next);
            next += 1;
        }
        let Some(last) = last else { return };
        let recorded = if conversation == mailbox_intro::INTRO {
            self.core.set_intro_read_through(last)
        } else {
            self.core.set_swarm_read_through(conversation, last, now)
        };
        if recorded.is_ok() {
            done.retain(|p| *p > last);
        }
    }

    /// Hold an envelope that arrived before its predecessor. When the buffer
    /// is full the last one in order goes, and is read again from a holder.
    fn hold(&mut self, conversation: &str, order: EnvelopeOrder, item: Held) {
        let held = self
            .mailbox_client
            .held
            .entry(conversation.into())
            .or_default();
        held.insert(order, item);
        if held.len() > MAX_BUFFERED
            && let Some((_, evicted)) = held.pop_last()
        {
            self.forget_seen(conversation, evicted.period, &evicted.op);
        }
    }

    /// Let another holder's copy of `op` be imported later.
    fn forget_seen(&mut self, conversation: &str, period: u64, op: &[u8; 32]) {
        if let Some(reading) = self
            .mailbox_client
            .reads
            .values_mut()
            .find(|r| r.conversation == conversation && r.period == period)
        {
            reading.seen.remove(op);
        }
    }

    /// Import held envelopes of a conversation in each sender's order; a
    /// sender whose next predecessor is still missing waits, the others go.
    fn release_held(&mut self, conversation: &str, now: u64) {
        let Some(orders) = self
            .mailbox_client
            .held
            .get(conversation)
            .map(|held| held.keys().cloned().collect::<Vec<_>>())
        else {
            return;
        };
        let mut waiting: Option<(u64, Vec<u8>)> = None;
        for order in orders {
            if waiting
                .as_ref()
                .is_some_and(|(epoch, sender)| (*epoch, sender) == (order.epoch, &order.sender))
            {
                continue;
            }
            let Some(item) = self
                .mailbox_client
                .held
                .get_mut(conversation)
                .and_then(|held| held.remove(&order))
            else {
                continue;
            };
            self.mailbox_client.stats.import_attempts += 1;
            match self
                .core
                .receive_swarm_envelope(conversation, item.period, &item.envelope, now)
            {
                Ok(_) => self.mailbox_client.stats.received += 1,
                Err(CoreError::Crypto(agentic_crypto::CryptoError::ReceiveGap)) => {
                    waiting = Some((order.epoch, order.sender.clone()));
                    if let Some(held) = self.mailbox_client.held.get_mut(conversation) {
                        held.insert(order, item);
                    }
                }
                Err(_) => self.mailbox_client.stats.rejected_entries += 1,
            }
        }
    }
}

/// A receipt for exactly the expected store, signed by the listed holder key.
fn verified_receipt(wire: &ReceiptWire, expected: &Expected) -> Option<Receipt> {
    let receipt = Receipt::try_from(wire).ok()?;
    ((
        receipt.mailbox,
        receipt.operation,
        receipt.ticket,
        receipt.holder,
    ) == (
        expected.mailbox,
        expected.operation,
        expected.ticket,
        expected.unit,
    ) && receipt.signer(&NETWORK_DOMAIN).ok() == Some(expected.account))
    .then_some(receipt)
}

/// An entry whose stamp pays for its envelope in this mailbox.
pub(super) fn fresh_entry(mailbox: &[u8; 32], entry: EntryWire) -> Option<([u8; 32], Vec<u8>)> {
    let stamp = Stamp::try_from(&entry.stamp).ok()?;
    stamp
        .pays_for(mailbox, entry.period, &entry.envelope)
        .then_some((stamp.operation, entry.envelope))
}

#[cfg(test)]
mod group_schedule_tests {
    //! How often members read a group's mailbox
    //! (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 4).
    use super::*;

    const POLL: Duration = READ_INTERVAL;

    /// Reads of a group of `members` over `span` of polls: how many, and
    /// requests to holders in all.
    fn reads(members: usize, span: Duration) -> (usize, usize) {
        let start = clock::instant();
        let mut schedule = GroupSchedule::new(start);
        let (mut reads, mut requests) = (0, 0);
        let mut at = Duration::ZERO;
        while at < span {
            if let Some(holders) = schedule.poll(members, start + at) {
                reads += 1;
                requests += holders;
            }
            at += POLL;
        }
        (reads, requests)
    }

    #[test]
    fn a_group_of_up_to_a_hundred_is_read_at_every_poll_from_every_holder() {
        assert_eq!(reads(100, Duration::from_secs(120)), (24, 24 * SWARM_SIZE));
    }

    #[test]
    fn a_bigger_group_is_read_less_often_the_bigger_it_is_from_one_holder_and_four_a_minute() {
        assert_eq!(group_read_every(101), POLL);
        assert!(group_read_every(283) > Duration::from_secs(6));
        assert!(group_read_every(283) < Duration::from_secs(7));
        assert_eq!(group_read_every(2000), Duration::from_secs(20));
        assert!(group_read_every(1000) < group_read_every(1500));
        // Past a hundred: one holder a read, four once a minute.
        assert_eq!(reads(101, Duration::from_secs(120)), (24, 2 * 4 + 22));
        let (read, asked) = reads(283, Duration::from_secs(120));
        assert_eq!(asked - read, 2 * (4 - 1));
        // A group of 2000 over two minutes: every 20 s, four holders at the
        // start of each minute and one otherwise.
        assert_eq!(reads(2000, Duration::from_secs(120)), (6, 2 * 4 + 4));
    }
}
