//! What a node reads from the chain (phase 1b of
//! Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md). Reads run as tasks; their
//! answers are applied on the next pump.
//! - Holders look unknown books up themselves: one read per book at a time,
//!   a missing or failed one is not read again for a minute. Today's grant
//!   rules are read again when a check needs them ten minutes later.
//! - A buyer's node quotes the shop's terms (`coins_buy`) and reads its own
//!   unpaid request until the chain has it.
//! - A holder that knows the identity server reports the grants it saw
//!   spent twice and reads the grants the server revoked
//!   (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md).
use super::chain::{BookRecord, Chain, ChainError, GrantDay, ShopTerms};
use super::identity_server::{
    ClaimOpened, ClaimStatus, IdentityError, IdentityServer, Reported, RevocationPage,
};
use super::*;
use agentic_grant_book::GrantRevocation;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::stamp::BookTerms;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How long a missing or failed read is answered from memory.
const ABSENT_FOR: Duration = Duration::from_secs(60);
/// How long an absent book's entries hold a replication cursor: a lagging
/// RPC catches up, a bad holder's entries are passed over.
const HOLD_FOR: Duration = Duration::from_secs(600);
/// How long today's grant rules are used before they are read again.
const TODAY_FOR: Duration = Duration::from_secs(600);
/// Reads running at once; a book wanted beyond them waits for a later store
/// or round.
const MAX_RUNNING: usize = 16;
/// How long an absent book is remembered at all.
const FORGET_AFTER: Duration = Duration::from_secs(3_600);
/// An own purchase request is read this often during its first hour...
const PURCHASE_FIRST: Duration = Duration::from_secs(30);
const FIRST_HOUR: u64 = 3_600;
/// ...and this often after that, until it is paid.
const PURCHASE_LATER: Duration = Duration::from_secs(600);
/// An open claim is read this often until the identity server decides.
const CLAIM_READ: Duration = Duration::from_secs(5);
/// A claim request the server did not answer is posted again after this.
const CLAIM_POST_AGAIN: Duration = Duration::from_secs(30);
/// An ETH quote is handed out for at most this long after its read.
const QUOTE_FOR: Duration = Duration::from_secs(60);
/// A holder reads the identity server's revocations this often...
const REVOCATIONS_EVERY: Duration = Duration::from_secs(600);
/// ...and sooner after a failed read; an unanswered report goes again then.
const IDENTITY_RETRY: Duration = Duration::from_secs(60);
/// Most revocation pages read in one round.
const REVOCATION_PAGES: usize = 16;
/// Reports sent at once: they share the reads' running count.
const MAX_REPORTING: usize = 2;

type Read<T> = std::result::Result<T, ChainError>;

enum Answer {
    Book([u8; 32], Read<Option<BookRecord>>),
    Purchase([u8; 32], Read<Option<BookRecord>>),
    Shop(Read<ShopTerms>),
    GrantDay(Account, u64, Read<GrantDay>),
    ClaimPosted(std::result::Result<ClaimOpened, IdentityError>),
    Units(Read<Vec<[u8; 32]>>),
    ClaimRead(std::result::Result<ClaimStatus, IdentityError>),
    /// Revocations read after the cursor, the cursor after them, and how
    /// the round ended.
    Revocations(Vec<GrantRevocation>, u64, RoundEnd),
    Reported([u8; 32], std::result::Result<Reported, IdentityError>),
}

/// How a round of revocation reads ended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RoundEnd {
    /// The server listed no more.
    Done,
    /// The round's pages ran out first: read on at once.
    More,
    Failed,
}

/// A holder's side of the identity server's penalties.
#[derive(Default)]
struct PenaltyLane {
    /// Revocations read so far; each run reads them all from the start.
    after: u64,
    reading: bool,
    /// When revocations are read next; `None` before the first read.
    read_due: Option<Instant>,
    /// A report was answered during a read: read again right after it.
    read_soon: bool,
    /// Books whose report is under way.
    reporting: std::collections::BTreeSet<[u8; 32]>,
    report_due: Option<Instant>,
    /// Not holding (or not started): nothing due until this node holds.
    idle: bool,
    reported: u64,
    revoked: u64,
}

/// The claim at the identity server as the node drives it; the claim itself
/// lives in core.
#[derive(Default)]
struct ClaimLane {
    /// Loaded from core on the first pump.
    loaded: bool,
    running: bool,
    post_due: Option<Instant>,
    read_due: Option<Instant>,
    /// How the last claim ended.
    last: Option<Value>,
}

enum BookRead {
    /// Under way; `first` is when the book was first found absent.
    Reading { first: Option<Instant> },
    /// Not on the chain yet, or the read failed.
    Absent { first: Instant, retry_at: Instant },
}

enum DayRead {
    Reading,
    /// Answered at `read_at`; days before the chain's today never change.
    Known {
        read_at: Instant,
        settled: bool,
    },
    Failed {
        retry_at: Instant,
    },
}

enum ShopRead {
    Unknown,
    Reading,
    /// The terms and when they were read.
    Known(ShopTerms, Instant),
}

/// What a holder may do with an entry of a book it does not know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BookState {
    /// A read is under way or was just started: wait for it.
    Reading,
    /// Found absent lately, within the hold: wait and read again later.
    Absent,
    /// Absent for longer than the hold, or nothing to read from.
    PassOver,
}

pub(super) struct Lane {
    pub(super) source: Option<Arc<dyn Chain>>,
    sender: mpsc::UnboundedSender<Answer>,
    answers: mpsc::UnboundedReceiver<Answer>,
    running: Arc<AtomicUsize>,
    books: BTreeMap<[u8; 32], BookRead>,
    days: BTreeMap<(Account, u64), DayRead>,
    shop: ShopRead,
    /// Own unpaid purchase requests and when each is read next (`None`
    /// while a read runs). Loaded from core on the first pump.
    purchases: Option<BTreeMap<[u8; 32], Option<Instant>>>,
    identity: Option<Arc<dyn IdentityServer>>,
    claim: ClaimLane,
    penalties: PenaltyLane,
    reads: u64,
    learned: u64,
    failures: u64,
}

impl Default for Lane {
    fn default() -> Self {
        let (sender, answers) = mpsc::unbounded_channel();
        Self {
            source: None,
            sender,
            answers,
            running: Arc::new(AtomicUsize::new(0)),
            books: BTreeMap::new(),
            days: BTreeMap::new(),
            shop: ShopRead::Unknown,
            purchases: None,
            identity: None,
            claim: ClaimLane::default(),
            penalties: PenaltyLane {
                idle: true,
                ..PenaltyLane::default()
            },
            reads: 0,
            learned: 0,
            failures: 0,
        }
    }
}

impl Lane {
    pub(super) fn info(&self) -> Value {
        json!({
            "configured": self.source.is_some(),
            "running": self.running.load(Ordering::SeqCst),
            "absentBooks": self.books.values().filter(|b| matches!(b, BookRead::Absent { .. })).count(),
            "unpaidPurchases": self.purchases.as_ref().map_or(0, BTreeMap::len),
            "reads": self.reads,
            "learned": self.learned,
            "failures": self.failures,
            "penalties": {
                "revocationsRead": self.penalties.after,
                "revoked": self.penalties.revoked,
                "reported": self.penalties.reported,
                "reporting": self.penalties.reporting.len(),
            },
        })
    }

    /// The earliest own purchase or claim step due.
    pub(super) fn next_due(&self) -> Option<Instant> {
        let claim = [self.claim.post_due, self.claim.read_due]
            .into_iter()
            .flatten()
            .filter(|_| !self.claim.running);
        let penalties = [
            self.penalties.read_due.filter(|_| !self.penalties.reading),
            self.penalties.report_due,
        ];
        self.purchases
            .iter()
            .flat_map(|purchases| purchases.values().filter_map(|due| *due))
            .chain(claim)
            .chain(penalties.into_iter().flatten())
            .min()
    }

    fn spawn(&mut self, read: impl std::future::Future<Output = Answer> + Send + 'static) {
        self.reads += 1;
        self.running.fetch_add(1, Ordering::SeqCst);
        let (sender, running) = (self.sender.clone(), self.running.clone());
        tokio::spawn(async move {
            let answer = read.await;
            // Queue the answer before the read stops counting as running.
            let _ = sender.send(answer);
            running.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Where a book stands, starting a read when none is running or
    /// remembered. A replicated entry of a book absent past the hold is
    /// passed over without reading; a store reads again after a minute.
    fn want_book(&mut self, book: [u8; 32], replica: bool, instant: Instant) -> BookState {
        let Some(source) = self.source.clone() else {
            return BookState::PassOver;
        };
        let first = match self.books.get(&book) {
            Some(BookRead::Reading { .. }) => return BookState::Reading,
            Some(BookRead::Absent { first, retry_at }) => {
                if replica && instant >= *first + HOLD_FOR {
                    return BookState::PassOver;
                }
                if instant < *retry_at {
                    return BookState::Absent;
                }
                Some(*first)
            }
            None => None,
        };
        if self.running.load(Ordering::SeqCst) >= MAX_RUNNING {
            return BookState::Absent;
        }
        self.books.insert(book, BookRead::Reading { first });
        self.spawn(async move { Answer::Book(book, source.book(book).await) });
        BookState::Reading
    }

    fn book_absent(&mut self, book: [u8; 32], instant: Instant) {
        let first = match self.books.get(&book) {
            Some(BookRead::Reading { first: Some(first) } | BookRead::Absent { first, .. }) => {
                *first
            }
            _ => instant,
        };
        self.books.insert(
            book,
            BookRead::Absent {
                first,
                retry_at: instant + ABSENT_FOR,
            },
        );
    }
}

/// When an unpaid request made at `created_at` is read next.
fn next_purchase_read(created_at: u64, now: u64, instant: Instant) -> Instant {
    if now.saturating_sub(created_at) < FIRST_HOUR {
        instant + PURCHASE_FIRST
    } else {
        instant + PURCHASE_LATER
    }
}

fn hex0x(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes))
}

impl Runtime {
    /// Read books and grant rules from `source`.
    /// A node that reads the chain takes payment, and asks for passes.
    pub(super) fn set_chain(&mut self, source: Arc<dyn Chain>) {
        self.chain.source = Some(source);
        self.access_gate.set_enabled(true);
    }

    /// Read the registry's active units; the directory applies the answer.
    /// False without a chain.
    pub(super) fn read_units(&mut self) -> bool {
        let Some(source) = self.chain.source.clone() else {
            return false;
        };
        self.chain
            .spawn(async move { Answer::Units(source.units().await) });
        true
    }

    /// Claim grants at `server` (`coins_claim`).
    pub(super) fn set_identity(&mut self, server: Arc<dyn IdentityServer>) {
        self.chain.identity = Some(server);
    }

    pub(super) fn chain_configured(&self) -> bool {
        self.chain.source.is_some()
    }

    /// Chain reads started and not answered yet.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the managed-time rig waits for them")
    )]
    pub(super) fn chain_in_flight(&self) -> usize {
        self.chain.running.load(Ordering::SeqCst)
    }

    /// Answers of finished reads not applied yet: the next pump applies them.
    #[cfg(test)]
    pub(super) fn chain_answers_waiting(&self) -> bool {
        !self.chain.answers.is_empty()
    }

    /// A stamp named `book`, which this holder does not know: read it.
    /// `replica` for an entry pulled from another holder.
    pub(super) fn book_wanted(&mut self, book: [u8; 32], replica: bool) -> BookState {
        self.chain.want_book(book, replica, clock::instant())
    }

    /// Apply the answers of finished reads and read own purchases due.
    pub(super) fn maintain_chain(&mut self) {
        let instant = clock::instant();
        self.chain.books.retain(|_, read| match read {
            BookRead::Absent { first, .. } => instant < *first + FORGET_AFTER,
            BookRead::Reading { .. } => true,
        });
        while let Ok(answer) = self.chain.answers.try_recv() {
            match answer {
                Answer::Book(book, Ok(Some(record))) => {
                    let terms = BookTerms {
                        key: record.key,
                        count: record.count,
                        valid_until: record.valid_until,
                    };
                    if self.mailbox_holder.learn_book(book, terms).is_ok() {
                        self.chain.books.remove(&book);
                        self.chain.learned += 1;
                    } else {
                        self.chain.book_absent(book, instant);
                    }
                }
                Answer::Book(book, outcome) => {
                    if outcome.is_err() {
                        self.chain.failures += 1;
                    }
                    self.chain.book_absent(book, instant);
                }
                Answer::Purchase(book, outcome) => self.purchase_answer(book, outcome, instant),
                claim @ (Answer::ClaimPosted(_) | Answer::ClaimRead(_)) => {
                    self.claim_answer(claim, instant);
                }
                Answer::Revocations(revocations, last, end) => {
                    self.revocations_read(&revocations, last, end, instant);
                }
                Answer::Reported(book, outcome) => self.report_answer(book, outcome, instant),
                Answer::Units(outcome) => {
                    if outcome.is_err() {
                        self.chain.failures += 1;
                    }
                    self.units_read(outcome.ok(), instant);
                }
                Answer::Shop(Ok(terms)) => self.chain.shop = ShopRead::Known(terms, instant),
                Answer::Shop(Err(_)) => {
                    self.chain.failures += 1;
                    self.chain.shop = ShopRead::Unknown;
                }
                Answer::GrantDay(server, day, Ok(rules)) => {
                    self.mailbox_holder.learn_grant_day(server, day, rules);
                    self.chain.days.insert(
                        (server, day),
                        DayRead::Known {
                            read_at: instant,
                            settled: day < rules.today,
                        },
                    );
                }
                Answer::GrantDay(server, day, Err(_)) => {
                    self.chain.failures += 1;
                    self.chain.days.insert(
                        (server, day),
                        DayRead::Failed {
                            retry_at: instant + ABSENT_FOR,
                        },
                    );
                }
            }
        }
        self.read_purchases(instant);
        self.drive_claim(instant);
        self.drive_penalties(instant);
    }

    /// A holder reports the grants it saw spent twice and reads the
    /// revocations when due.
    fn drive_penalties(&mut self, instant: Instant) {
        let Some(server) = self.chain.identity.clone() else {
            return;
        };
        let lane = &mut self.chain.penalties;
        if self.mailbox_holder.unit().is_none() {
            lane.read_due = None;
            lane.report_due = None;
            lane.idle = true;
            return;
        }
        if std::mem::take(&mut lane.idle) {
            // Reports kept from before are sent once this node holds.
            lane.report_due = Some(instant);
        }
        if !lane.reading && lane.read_due.is_none_or(|due| due <= instant) {
            lane.reading = true;
            let (after, server) = (lane.after, server.clone());
            self.chain.spawn(async move {
                let mut revocations = Vec::new();
                let mut last = after;
                for _ in 0..REVOCATION_PAGES {
                    match server.revocations(last).await {
                        // A page that does not move the cursor ends the round.
                        Ok(RevocationPage {
                            revocations: page,
                            last: next,
                        }) if !page.is_empty() && next > last => {
                            revocations.extend(page);
                            last = next;
                        }
                        Ok(_) => return Answer::Revocations(revocations, last, RoundEnd::Done),
                        Err(_) => return Answer::Revocations(revocations, last, RoundEnd::Failed),
                    }
                }
                Answer::Revocations(revocations, last, RoundEnd::More)
            });
        }
        if self.mailbox_holder.take_new_reports() {
            self.chain.penalties.report_due = Some(instant);
        }
        if self
            .chain
            .penalties
            .report_due
            .is_none_or(|due| due > instant)
        {
            return;
        }
        self.chain.penalties.report_due = None;
        for report in self.mailbox_holder.reports().unwrap_or_default() {
            let book = report.grant.id();
            let lane = &mut self.chain.penalties;
            if lane.reporting.contains(&book) {
                continue;
            }
            if lane.reporting.len() >= MAX_REPORTING {
                // The rest go as these are answered.
                lane.report_due = Some(instant + IDENTITY_RETRY);
                break;
            }
            lane.reporting.insert(book);
            let server = server.clone();
            self.chain.spawn(async move {
                let outcome = server
                    .report(&report.grant, &report.first, &report.second)
                    .await;
                Answer::Reported(book, outcome)
            });
        }
    }

    fn revocations_read(
        &mut self,
        revocations: &[GrantRevocation],
        last: u64,
        end: RoundEnd,
        instant: Instant,
    ) {
        let lane = &mut self.chain.penalties;
        lane.reading = false;
        let Ok(now) = clock::wall() else {
            lane.read_due = Some(instant + IDENTITY_RETRY);
            return;
        };
        let mut kept = true;
        for revocation in revocations {
            match self.mailbox_holder.learn_revocation(revocation, now) {
                Ok(true) => self.chain.penalties.revoked += 1,
                Ok(false) => {}
                Err(_) => kept = false,
            }
        }
        let lane = &mut self.chain.penalties;
        if !kept {
            // Read them again rather than skip one this node failed to keep.
            lane.read_due = Some(instant + IDENTITY_RETRY);
            return;
        }
        lane.after = last;
        let soon = std::mem::take(&mut lane.read_soon);
        lane.read_due = Some(match end {
            RoundEnd::More => instant,
            _ if soon => instant,
            RoundEnd::Failed => instant + IDENTITY_RETRY,
            RoundEnd::Done => instant + REVOCATIONS_EVERY,
        });
        if end == RoundEnd::Failed {
            self.chain.failures += 1;
        }
    }

    /// The server's answer is final; without one the report goes again.
    fn report_answer(
        &mut self,
        book: [u8; 32],
        outcome: std::result::Result<Reported, IdentityError>,
        instant: Instant,
    ) {
        self.chain.penalties.reporting.remove(&book);
        if matches!(outcome, Ok(_) | Err(IdentityError::Refused(_))) {
            // Answered: any report left waiting for a slot goes now.
            let lane = &mut self.chain.penalties;
            lane.report_due = Some(lane.report_due.map_or(instant, |due| due.min(instant)));
        }
        match outcome {
            Ok(answer) => {
                if self.mailbox_holder.reported(&book).is_ok() {
                    self.chain.penalties.reported += 1;
                }
                // The server may have just revoked the identity's grants.
                if answer.banned || answer.revoked > 0 {
                    let lane = &mut self.chain.penalties;
                    if lane.reading {
                        lane.read_soon = true;
                    } else {
                        lane.read_due = Some(instant);
                    }
                }
            }
            Err(IdentityError::Refused(_)) => {
                if self.mailbox_holder.reported(&book).is_ok() {
                    self.chain.penalties.reported += 1;
                }
            }
            Err(_) => {
                self.chain.failures += 1;
                let again = instant + IDENTITY_RETRY;
                let lane = &mut self.chain.penalties;
                lane.report_due = Some(lane.report_due.map_or(again, |due| due.min(again)));
            }
        }
    }

    /// Post the claim request or read the open claim when due; after a
    /// restart the claim in core is picked up again.
    fn drive_claim(&mut self, instant: Instant) {
        let Some(server) = self.chain.identity.clone() else {
            return;
        };
        if !self.chain.claim.loaded {
            self.chain.claim.loaded = true;
            match self.core.mailbox_claim() {
                Ok(Some(claim)) if claim.claim_id.is_some() => {
                    self.chain.claim.read_due = Some(instant);
                }
                Ok(Some(_)) => self.chain.claim.post_due = Some(instant),
                _ => {}
            }
        }
        if self.chain.claim.running {
            return;
        }
        let Ok(Some(claim)) = self.core.mailbox_claim() else {
            self.chain.claim.post_due = None;
            self.chain.claim.read_due = None;
            return;
        };
        match (
            &claim.claim_id,
            self.chain.claim.post_due,
            self.chain.claim.read_due,
        ) {
            (None, Some(due), _) if due <= instant => {
                self.chain.claim.running = true;
                self.chain.claim.post_due = None;
                let request = claim.request;
                self.chain
                    .spawn(async move { Answer::ClaimPosted(server.create(&request).await) });
            }
            (Some(id), _, Some(due)) if due <= instant => {
                self.chain.claim.running = true;
                self.chain.claim.read_due = None;
                let id = id.clone();
                self.chain
                    .spawn(async move { Answer::ClaimRead(server.status(&id).await) });
            }
            _ => {}
        }
    }

    fn claim_ended(&mut self, outcome: Value) {
        let _ = self.core.finish_mailbox_claim();
        self.chain.claim.last = Some(outcome);
        self.chain.claim.post_due = None;
        self.chain.claim.read_due = None;
    }

    fn claim_answer(&mut self, answer: Answer, instant: Instant) {
        self.chain.claim.running = false;
        match answer {
            Answer::ClaimPosted(Ok(opened)) => {
                if self
                    .core
                    .open_mailbox_claim(&opened.claim_id, &opened.login_url, opened.expires_at)
                    .is_ok()
                {
                    self.chain.claim.read_due = Some(instant + CLAIM_READ);
                }
            }
            // The server's decision, such as a request gone stale while it
            // was down: this claim is over, the next one is signed afresh.
            Answer::ClaimPosted(Err(IdentityError::Refused(code))) => {
                self.claim_ended(json!({"status": "denied", "reason": code}));
            }
            Answer::ClaimPosted(Err(_)) => {
                self.chain.failures += 1;
                self.chain.claim.post_due = Some(instant + CLAIM_POST_AGAIN);
            }
            Answer::ClaimRead(Ok(ClaimStatus::Granted(grant))) => {
                if self.add_mailbox_grant(&grant).is_ok() {
                    self.mailbox_client.retry_blocked();
                    self.claim_ended(json!({"status": "granted", "book": hex0x(grant.id())}));
                } else {
                    self.claim_ended(json!({"status": "denied", "reason": "invalid_grant"}));
                }
            }
            Answer::ClaimRead(Ok(ClaimStatus::Denied(reason))) => {
                self.claim_ended(json!({"status": "denied", "reason": reason}));
            }
            Answer::ClaimRead(Err(IdentityError::Refused(code))) => {
                self.claim_ended(json!({"status": "denied", "reason": code}));
            }
            Answer::ClaimRead(outcome) => {
                if outcome.is_err() {
                    self.chain.failures += 1;
                }
                // Pending, or no answer: ask again, also past the claim's
                // end (the server expires it).
                self.chain.claim.read_due = Some(instant + CLAIM_READ);
            }
            _ => {}
        }
    }

    /// `coins_claim`: the login link of the claim for a grant, once the
    /// identity server opened it.
    pub(super) fn coins_claim(
        &mut self,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        if self.chain.identity.is_none() {
            return Err((
                "identity_not_configured",
                "This node knows no identity server: start it with --identity-server".into(),
            ));
        }
        let claim = self
            .core
            .start_mailbox_claim(now)
            .map_err(|error| ("invalid_request", error.to_string()))?;
        if let (Some(id), Some(url), Some(expires_at)) =
            (&claim.claim_id, &claim.login_url, claim.expires_at)
        {
            return Ok(json!({
                "status": "open",
                "claimId": id,
                "loginUrl": url,
                "expiresAt": expires_at,
            }));
        }
        self.chain.claim.loaded = true;
        if !self.chain.claim.running {
            self.chain.claim.post_due = Some(clock::instant());
            self.drive_claim(clock::instant());
        }
        Err((
            "claim_pending",
            "Asking the identity server; ask again".into(),
        ))
    }

    /// Start reads of own unpaid requests that are due; after a restart
    /// every request is read once right away.
    fn read_purchases(&mut self, instant: Instant) {
        let Some(source) = self.chain.source.clone() else {
            return;
        };
        if self.chain.purchases.is_none() {
            let Ok(pending) = self.core.mailbox_purchases() else {
                return;
            };
            self.chain.purchases = Some(
                pending
                    .into_iter()
                    .map(|p| (p.book, Some(instant)))
                    .collect(),
            );
        }
        let due: Vec<[u8; 32]> = self
            .chain
            .purchases
            .iter()
            .flatten()
            .filter(|(_, due)| due.is_some_and(|due| due <= instant))
            .map(|(book, _)| *book)
            .collect();
        for book in due {
            if let Some(purchases) = self.chain.purchases.as_mut() {
                purchases.insert(book, None);
            }
            let source = source.clone();
            self.chain
                .spawn(async move { Answer::Purchase(book, source.book(book).await) });
        }
    }

    fn purchase_answer(
        &mut self,
        book: [u8; 32],
        outcome: Read<Option<BookRecord>>,
        instant: Instant,
    ) {
        let Ok(now) = now() else { return };
        let request = self
            .core
            .mailbox_purchases()
            .ok()
            .and_then(|pending| pending.into_iter().find(|p| p.book == book));
        let Some(request) = request else {
            if let Some(purchases) = self.chain.purchases.as_mut() {
                purchases.remove(&book);
            }
            return;
        };
        match outcome {
            Ok(Some(record)) if record.key == request.key => {
                if self
                    .core
                    .confirm_mailbox_purchase(book, record.key, record.count, record.valid_until)
                    .is_ok()
                {
                    if let Some(purchases) = self.chain.purchases.as_mut() {
                        purchases.remove(&book);
                    }
                    // Messages waiting for a book go now.
                    self.mailbox_client.retry_blocked();
                    return;
                }
            }
            Ok(_) => {}
            Err(_) => self.chain.failures += 1,
        }
        if let Some(purchases) = self.chain.purchases.as_mut() {
            purchases.insert(
                book,
                Some(next_purchase_read(request.created_at, now, instant)),
            );
        }
    }

    /// `coins_buy`: a payment for a book of the profile's key.
    pub(super) fn coins_buy(
        &mut self,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let Some(source) = self.chain.source.clone() else {
            return Err((
                "chain_not_configured",
                "This node reads no chain: start it with the chain flags".into(),
            ));
        };
        let instant = clock::instant();
        let terms = match self.chain.shop {
            // The rate moves: a quote is read again after a minute.
            ShopRead::Known(terms, read) if read + QUOTE_FOR > instant => terms,
            ShopRead::Reading => {
                return Err((
                    "chain_pending",
                    "Reading the shop's terms; ask again".into(),
                ));
            }
            ShopRead::Known(..) | ShopRead::Unknown => {
                self.chain.shop = ShopRead::Reading;
                self.chain
                    .spawn(async move { Answer::Shop(source.shop().await) });
                return Err((
                    "chain_pending",
                    "Reading the shop's terms; ask again".into(),
                ));
            }
        };
        let request = self
            .core
            .start_mailbox_purchase(now)
            .map_err(|error| ("invalid_request", error.to_string()))?;
        self.chain
            .purchases
            .get_or_insert_with(BTreeMap::new)
            .entry(request.book)
            .or_insert(Some(instant + PURCHASE_FIRST));
        let (shop, key, salt, token) = (
            hex0x(terms.address),
            hex0x(request.key),
            hex0x(request.salt),
            hex0x(terms.usdc),
        );
        let chain_id = terms.chain_id;
        // One percent more than the quote, for the rate to move before the
        // payment lands; the shop gives back what is over.
        let eth = terms.quote.map(|quote| {
            let value = quote.saturating_add(quote.div_ceil(100));
            json!({
                "to": shop,
                "quote": quote.to_string(),
                "value": value.to_string(),
                "calldata": hex0x(chain::buy_calldata(&request.key, &request.salt)),
                "uri": format!("ethereum:{shop}@{chain_id}/buy?address={key}&bytes32={salt}&value={value}"),
            })
        });
        let price = terms.price_usdc;
        Ok(json!({
            "book": hex0x(request.book),
            "key": key,
            "salt": salt,
            "shop": shop,
            "chainId": chain_id,
            "count": terms.book_size,
            "validSeconds": terms.validity,
            "priceUsdc": price.to_string(),
            "eth": eth,
            "usdc": {
                "token": token,
                "amount": price.to_string(),
                "approve": {
                    "to": token,
                    "calldata": hex0x(chain::approve_calldata(&terms.address, price)),
                    "uri": format!("ethereum:{token}@{chain_id}/approve?address={shop}&uint256={price}"),
                },
                "buy": {
                    "to": shop,
                    "calldata": hex0x(chain::buy_with_usdc_calldata(&request.key, &request.salt)),
                    "uri": format!("ethereum:{shop}@{chain_id}/buyWithUsdc?address={key}&bytes32={salt}"),
                },
            },
            "createdAt": request.created_at,
        }))
    }

    /// `coins_balance`: the profile's books, its unpaid request and the
    /// stamps it can still spend.
    pub(super) fn coins_balance(
        &self,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let failed = |error: agentic_core::CoreError| ("invalid_request", error.to_string());
        let mut books = Vec::new();
        let mut remaining = 0u64;
        for book in self.core.mailbox_books().map_err(failed)? {
            let granted = self.core.mailbox_book_grant(&book.book).map_err(failed)?;
            if book.valid_until > now {
                remaining += u64::from(book.count.saturating_sub(book.used));
            }
            books.push(json!({
                "book": hex0x(book.book),
                "kind": if granted.is_some() { "granted" } else { "bought" },
                "count": book.count,
                "used": book.used,
                "validUntil": book.valid_until,
            }));
        }
        let pending: Vec<Value> = self
            .core
            .mailbox_purchases()
            .map_err(failed)?
            .into_iter()
            .map(|p| {
                json!({
                    "book": hex0x(p.book),
                    "key": hex0x(p.key),
                    "salt": hex0x(p.salt),
                    "createdAt": p.created_at,
                })
            })
            .collect();
        let claim = self.core.mailbox_claim().map_err(failed)?.map(|claim| {
            json!({
                "status": if claim.login_url.is_some() { "open" } else { "starting" },
                "claimId": claim.claim_id,
                "loginUrl": claim.login_url,
                "expiresAt": claim.expires_at,
            })
        });
        Ok(json!({
            "books": books,
            "pending": pending,
            "remaining": remaining,
            "claim": claim,
            "lastClaim": self.chain.claim.last,
        }))
    }

    /// Whether the holder has current rules for `server` on `day`; if not,
    /// a read is started (or under way) and the check waits for it. Without
    /// a chain the holder uses whatever rules it has.
    pub(super) fn grant_rules_current(&mut self, server: Account, day: u64) -> bool {
        let instant = clock::instant();
        let Some(source) = self.chain.source.clone() else {
            return true;
        };
        match self.chain.days.get(&(server, day)) {
            Some(DayRead::Reading) => return false,
            Some(DayRead::Known { read_at, settled }) => {
                if *settled || instant < *read_at + TODAY_FOR {
                    return true;
                }
            }
            Some(DayRead::Failed { retry_at }) if instant < *retry_at => return false,
            _ => {}
        }
        self.chain.days.insert((server, day), DayRead::Reading);
        self.chain.spawn(async move {
            Answer::GrantDay(server, day, source.grant_day(server, day).await)
        });
        false
    }
}
