//! Holder side of the recipient-mailbox swarm
//! (Docs/V1_STORAGE_REDESIGN_2026_09_24.md): stores stamped envelopes per
//! mailbox, signs one secp256k1 receipt per stored operation and pages a
//! mailbox by cursor. A holder never receipts two operations for one ticket.
//! Book terms reach it only from verified purchases.
use super::*;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::access::{AccessError, AccessPass};
use agentic_mailbox_swarm::address::{
    PERIOD_SECONDS, expires_at, live, period as address_period, writable,
};
use agentic_mailbox_swarm::proof::SenderEquivocation;
use agentic_mailbox_swarm::receipt::{HolderKey, Receipt};
use agentic_mailbox_swarm::stamp::{BookTerms, Stamp, StampError};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "mailbox_holder_tests.rs"]
mod tests;

pub(super) const PROTOCOL: &str = "/agentic-internet/mailbox/1";
/// Largest envelope a holder stores: one sealed signed document.
pub(super) const MAX_ENVELOPE: usize = agentic_crypto::mailbox::MAX_SWARM_ENVELOPE;
/// Most entries one read returns.
pub(super) const MAX_PAGE: usize = 16;
/// What a notary keeps first for its key: a stamp for a book slot, or a
/// grant for an issuer's serial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Statement {
    Ticket(Stamp),
    Grant(agentic_grant_book::GrantBook),
    /// A claim on a group epoch's commit (spec/groups-v1.md), as signed.
    Commit(Vec<u8>),
}

/// A claim's notary key, whatever the clock: a claim does not expire.
pub(super) fn claim_key(wire: &[u8], domain: &[u8; 32]) -> Option<[u8; 32]> {
    agentic_protocol::group::verify_claim(wire, *domain, u64::MAX / 4)
        .ok()
        .map(|claim| claim.key)
}

impl Statement {
    /// The notary key: a book slot's ticket, an issuer's serial, or a group
    /// epoch's round.
    pub(super) fn key(&self, domain: &[u8; 32]) -> [u8; 32] {
        match self {
            Self::Ticket(stamp) => stamp.ticket_id(domain),
            Self::Grant(grant) => grant.id(),
            Self::Commit(claim) => claim_key(claim, domain).unwrap_or([0; 32]),
        }
    }
}

/// The statement a notary saw first for a key, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Notarized {
    pub(super) first: Statement,
    pub(super) first_seen: u64,
}

/// Self-contained proofs of misbehaviour a node keeps and passes on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Proofs {
    pub(super) senders: Vec<SenderEquivocation>,
    pub(super) holders: Vec<agentic_mailbox_swarm::proof::HolderEquivocation>,
    pub(super) grants: Vec<agentic_grant_book::GrantEquivocation>,
}

/// A grant spent twice, waiting to be reported to the identity server
/// (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md): the grant and the two stamps
/// of one slot of its book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Report {
    pub(super) grant: agentic_grant_book::GrantBook,
    pub(super) first: Stamp,
    pub(super) second: Stamp,
}

/// A page of the proofs a node learned, in the order it learned them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ProofPage {
    pub(super) proofs: Proofs,
    /// Cursor for the next page, if this one was full.
    pub(super) next: Option<u64>,
}

/// Most statements in one notary request.
pub(super) const MAX_NOTARIZE: usize = 64;

/// Most proofs in one page.
pub(super) const MAX_PROOFS: usize = 64;

/// Most mailbox summaries in one replication request.
pub(super) const MAX_SUMMARIES: usize = 64;

/// What a holder holds of one mailbox: how many operations, and a digest of
/// which, independent of their arrival order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Summary {
    pub(super) count: u64,
    pub(super) digest: [u8; 32],
}

/// Clock skew allowed around a grant's day when notaries first saw it.
const GRANT_TOLERANCE: u64 = 3_600;

/// How long a pass or a unit's introduction outlives its day.
pub(super) const ACCESS_GRACE: u64 = 3_600;

/// Fewest notaries that must have first seen a grant on its day.
pub(super) const GRANT_AGREEMENT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GrantVerdict {
    Learn,
    /// Final for a while: late or conflicting whatever the rest would say.
    Refuse,
    /// Unreachable notaries could still tip it: ask again later.
    Retry,
}

/// What notaries' answers about a grant decide, or nothing yet while some
/// are outstanding.
pub(super) fn grant_verdict(
    in_time: usize,
    late: usize,
    outstanding: usize,
    unreachable: usize,
    conflict: bool,
) -> Option<GrantVerdict> {
    if conflict {
        return Some(GrantVerdict::Refuse);
    }
    if in_time >= GRANT_AGREEMENT && in_time > late + outstanding + unreachable {
        return Some(GrantVerdict::Learn);
    }
    let best = in_time + outstanding + unreachable;
    if best < GRANT_AGREEMENT || best <= late {
        return Some(GrantVerdict::Refuse);
    }
    (outstanding == 0).then_some(GrantVerdict::Retry)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// This node has no registry unit to receipt as.
    NoUnit,
    /// No verified purchase of the stamp's book is known here.
    UnknownBook,
    Stamp(StampError),
    /// The stamp pays for another envelope.
    Operation,
    TooLarge,
    /// The ticket already paid for a different operation here.
    Conflict,
    /// The period is not taken here: outside the fresh window, past
    /// retention, or another than the mailbox's own.
    Period,
    /// A grant that breaks its issuer's rules for its day, or that its
    /// notaries did not first see on that day.
    Grant,
    /// A grant waiting for its notaries' answers.
    GrantPending,
    /// The book was proven to pay twice for one slot.
    Blocked,
    /// A wire field has the wrong length.
    Malformed,
    Storage,
    /// A group commit claim without its owner's roster behind it.
    Claim,
    /// Only a pass is taken from a peer that is neither a unit nor let in
    /// by one (access by book).
    AccessRequired,
    /// A pass or unit record that is not the showing peer's own, or not
    /// signed by its book's key, or for another day.
    Pass,
    /// The pass's book has ended.
    BookExpired,
    /// A unit record of a commitment the registry does not list.
    UnknownUnit,
}

impl Refusal {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::NoUnit => "no_unit",
            Self::UnknownBook => "unknown_book",
            Self::Stamp(StampError::Signature) => "stamp_signature",
            Self::Stamp(StampError::Signer) => "stamp_signer",
            Self::Stamp(StampError::Index) => "stamp_index",
            Self::Stamp(StampError::Expired) => "stamp_expired",
            Self::Operation => "operation",
            Self::TooLarge => "too_large",
            Self::Conflict => "conflict",
            Self::Period => "period",
            Self::Grant => "grant",
            Self::GrantPending => "grant_pending",
            Self::Blocked => "blocked",
            Self::Malformed => "malformed",
            Self::Storage => "storage",
            Self::Claim => "claim",
            Self::AccessRequired => "access_required",
            Self::Pass => "bad_pass",
            Self::BookExpired => "book_expired",
            Self::UnknownUnit => "unknown_unit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) seq: u64,
    /// The period its stamp paid for.
    pub(super) period: u64,
    pub(super) envelope: Vec<u8>,
    pub(super) stamp: Stamp,
    pub(super) stored_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Page {
    pub(super) entries: Vec<Entry>,
    /// Cursor for the next read: the last returned seq, or `after`.
    pub(super) next: u64,
}

/// How an entry reaches a holder: sent fresh, or pulled as a replica
/// whose stamp paid at the start of its period.
enum Arrival {
    Fresh,
    Replica { paid_at: u64 },
}

pub(super) struct Service {
    store: ProfileStore,
    key: HolderKey,
    domain: [u8; 32],
    unit: Option<[u8; 32]>,
    books: BTreeMap<[u8; 32], BookTerms>,
    /// Stamps of entries stored since the notary lane last looked.
    fresh: Vec<Stamp>,
    /// Books proven to pay twice for a slot, and when this node learned it.
    blocked: BTreeMap<[u8; 32], u64>,
    /// Grants whose books this holder learned, by book id.
    grants: BTreeMap<[u8; 32], agentic_grant_book::GrantBook>,
    /// `GrantIssuer`'s latest answers by issuer and day.
    grant_days: BTreeMap<(Account, u64), super::chain::GrantDay>,
    /// A report was queued since the identity lane last looked.
    new_reports: bool,
}

const NEXT: &str = "mailbox/next";

fn entry_name(mailbox: &[u8; 32], seq: u64) -> String {
    format!("mailbox/entry/{}/{seq:020}", hex::encode(mailbox))
}
const TICKETS: &str = "mailbox/ticket/";
fn ticket_name(ticket: &[u8; 32]) -> String {
    format!("{TICKETS}{}", hex::encode(ticket))
}
/// Retention index: `mailbox/expire/{when}/{m|t|n|g|b|r|p}/{id}` for a
/// mailbox, a ticket record, a notary record, a learned grant, a book, a
/// revocation of a grant not learned or a report not sent, due at `when`.
const EXPIRY: &str = "mailbox/expire/";
fn expiry_name(when: u64, kind: &str, id: &[u8; 32]) -> String {
    format!("{EXPIRY}{when:020}/{kind}/{}", hex::encode(id))
}
/// Learned grant books: `mailbox/grant/{book}`.
const GRANTS: &str = "mailbox/grant/";
fn grant_name(book: &[u8; 32]) -> String {
    format!("{GRANTS}{}", hex::encode(book))
}
/// Books whose purchase this holder read from the chain: `mailbox/book/{book}`.
const BOOKS: &str = "mailbox/book/";
fn book_name(book: &[u8; 32]) -> String {
    format!("{BOOKS}{}", hex::encode(book))
}
/// A book is kept until every mailbox it could pay for has ended, so late
/// copies of its entries are still repaired.
fn book_kept_until(terms: &BookTerms) -> u64 {
    expires_at(address_period(terms.valid_until))
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredBook {
    key: Account,
    count: u32,
    valid_until: u64,
}
impl From<BookTerms> for StoredBook {
    fn from(terms: BookTerms) -> Self {
        Self {
            key: terms.key,
            count: terms.count,
            valid_until: terms.valid_until,
        }
    }
}
impl From<StoredBook> for BookTerms {
    fn from(stored: StoredBook) -> Self {
        Self {
            key: stored.key,
            count: stored.count,
            valid_until: stored.valid_until,
        }
    }
}
const SUMMARIES: &str = "mailbox/summary/";
fn summary_name(mailbox: &[u8; 32]) -> String {
    format!("{SUMMARIES}{}", hex::encode(mailbox))
}
const EQUIVOCATIONS: &str = "mailbox/equivocation/";
const GRANT_EQUIVOCATIONS: &str = "mailbox/grant-equivocation/";
/// One proof per grant issuer is enough to block it.
fn grant_equivocation_name(server: &Account) -> String {
    format!("{GRANT_EQUIVOCATIONS}{}", hex::encode(server))
}
/// Every kept proof in the order this node learned it.
/// When this node learned the first proof against a book.
const BLOCKED: &str = "mailbox/blocked/";
fn blocked_name(book: &[u8; 32]) -> String {
    format!("{BLOCKED}{}", hex::encode(book))
}
/// Grants spent twice not yet reported: `mailbox/report/{book}`.
const REPORTS: &str = "mailbox/report/";
fn report_name(book: &[u8; 32]) -> String {
    format!("{REPORTS}{}", hex::encode(book))
}
/// Revocations of grants not learned here yet: `mailbox/revocation/{book}`.
const REVOCATIONS: &str = "mailbox/revocation/";
fn revocation_name(book: &[u8; 32]) -> String {
    format!("{REVOCATIONS}{}", hex::encode(book))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedReport {
    grant: agentic_grant_book::GrantBook,
    first: StampWire,
    second: StampWire,
}
const PROOFS: &str = "mailbox/proof/";
const PROOF_NEXT: &str = "mailbox/proof-next";
fn proof_name(seq: u64) -> String {
    format!("{PROOFS}{seq:020}")
}
/// One proof per holder account is enough to block it.
const HOLDER_EQUIVOCATIONS: &str = "mailbox/holder-equivocation/";
fn holder_equivocation_name(account: &Account) -> String {
    format!("{HOLDER_EQUIVOCATIONS}{}", hex::encode(account))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
enum SavedProof {
    Sender(SenderProofWire),
    Holder(HolderProofWire),
    Grant(agentic_grant_book::GrantEquivocation),
}
const NOTARY: &str = "notary/";
fn notary_name(key: &[u8; 32]) -> String {
    format!("{NOTARY}{}", hex::encode(key))
}

/// A notary record: the first statement for a key and when it was seen.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedNotary {
    statement: StatementWire,
    first_seen: u64,
}
fn equivocation_name(ticket: &[u8; 32]) -> String {
    format!("{EQUIVOCATIONS}{}", hex::encode(ticket))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedSummary {
    /// The one period this mailbox's stamps pay for.
    period: u64,
    count: u64,
    digest: String,
}

/// What a spent slot keeps after its entry: the first stamp and receipt.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedTicket {
    stamp: StampWire,
    receipt: ReceiptWire,
    keep_until: u64,
}

fn json<T: Serialize>(value: &T) -> std::result::Result<Vec<u8>, Refusal> {
    serde_json::to_vec(value).map_err(|_| Refusal::Storage)
}

/// Two stamps of one book slot for different operations.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedEquivocation {
    first: StampWire,
    second: StampWire,
}

fn bytes32(text: &str) -> std::result::Result<[u8; 32], Refusal> {
    hex::decode(text)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(Refusal::Storage)
}

/// One stored message and the receipt it was given.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved {
    seq: u64,
    period: u64,
    envelope: String,
    stamp: StampWire,
    receipt: ReceiptWire,
}

fn load<T: DeserializeOwned>(
    store: &ProfileStore,
    name: &str,
) -> std::result::Result<Option<(T, u64)>, Refusal> {
    store
        .state(name)
        .map_err(|_| Refusal::Storage)?
        .map(|state| {
            serde_json::from_slice(&state.bytes)
                .map(|value| (value, state.revision))
                .map_err(|_| Refusal::Storage)
        })
        .transpose()
}

impl Service {
    /// Opens `<profile>.mailbox.db`; the receipt key derives from the node
    /// identity.
    pub(super) fn open(
        profile: &std::path::Path,
        master: &[u8; 32],
        identity: &identity::Keypair,
        domain: [u8; 32],
    ) -> Result<Self> {
        let mut path = profile.as_os_str().to_os_string();
        path.push(".mailbox.db");
        let secret = zeroize::Zeroizing::new(identity.clone().try_into_ed25519()?.to_bytes());
        let seed: [u8; 32] = Sha256::new()
            .chain_update(b"ain-mailbox-holder-key-v1")
            .chain_update(&secret[..32])
            .finalize()
            .into();
        let key = HolderKey::from_bytes(&seed).ok_or("holder key derivation failed")?;
        let mut service = Self {
            store: ProfileStore::open(std::path::PathBuf::from(path), master)?,
            key,
            domain,
            unit: None,
            books: BTreeMap::new(),
            fresh: Vec::new(),
            blocked: BTreeMap::new(),
            grants: BTreeMap::new(),
            grant_days: BTreeMap::new(),
            new_reports: false,
        };
        service.blocked = service.load_blocked().map_err(|refusal| refusal.code())?;
        for grant in service.load_grants().map_err(|refusal| refusal.code())? {
            service.remember_grant(grant);
        }
        for (book, terms) in service.load_books().map_err(|refusal| refusal.code())? {
            service.books.insert(book, terms);
        }
        Ok(service)
    }
    pub(super) fn account(&self) -> Account {
        self.key.account()
    }
    /// The registry unit this node receipts as.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "used once books, units and senders are wired (phases 1b-3)"
        )
    )]
    pub(super) fn set_unit(&mut self, commitment: [u8; 32]) {
        self.unit = Some(commitment);
    }
    /// The unit left the registry: this node holds as nothing.
    pub(super) fn clear_unit(&mut self) {
        self.unit = None;
    }
    /// Terms of a book whose purchase was verified.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "used once books, units and senders are wired (phases 1b-3)"
        )
    )]
    pub(super) fn learn_book(
        &mut self,
        book: [u8; 32],
        terms: BookTerms,
    ) -> std::result::Result<(), Refusal> {
        let name = book_name(&book);
        let revision = match load::<StoredBook>(&self.store, &name)? {
            Some((kept, _)) if BookTerms::from(kept) == terms => {
                self.books.insert(book, terms);
                return Ok(());
            }
            Some((_, revision)) => revision,
            None => 0,
        };
        let mut changes = vec![StateChange {
            namespace: name,
            expected_revision: revision,
            bytes: json(&StoredBook::from(terms))?,
        }];
        // Terms learned earlier may have left an index entry at this time.
        let expiry = expiry_name(book_kept_until(&terms), "b", &book);
        if self.removal(&expiry)?.is_none() {
            changes.push(StateChange {
                namespace: expiry,
                expected_revision: 0,
                bytes: vec![],
            });
        }
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        self.books.insert(book, terms);
        Ok(())
    }
    /// Whether stamps of `book` can be checked here: a learned purchase or
    /// grant.
    pub(super) fn knows_book(&self, book: &[u8; 32]) -> bool {
        self.books.contains_key(book)
    }
    /// Until when `pass`, shown by the peer with transport key `peer`, lets
    /// it in under a book of `terms`: an hour past the pass's day, to absorb
    /// clock skew, or the book's end if sooner.
    pub(super) fn check_access(
        &self,
        pass: &AccessPass,
        peer: &[u8; 32],
        terms: &BookTerms,
        now: u64,
    ) -> std::result::Result<u64, Refusal> {
        if pass.peer != *peer {
            return Err(Refusal::Pass);
        }
        if self.blocked.contains_key(&pass.book) {
            return Err(Refusal::Blocked);
        }
        match pass.verify(&self.domain, terms, now) {
            Ok(()) => {}
            Err(AccessError::Expired) => return Err(Refusal::BookExpired),
            Err(_) => return Err(Refusal::Pass),
        }
        let end = pass
            .day
            .saturating_add(1)
            .saturating_mul(agentic_mailbox_swarm::address::PERIOD_SECONDS)
            .saturating_add(ACCESS_GRACE);
        Ok(end.min(terms.valid_until))
    }
    /// Whether a proof blocked `book` here.
    pub(super) fn is_blocked(&self, book: &[u8; 32]) -> bool {
        self.blocked.contains_key(book)
    }
    /// The terms of a book this node knows.
    pub(super) fn book_terms(&self, book: &[u8; 32]) -> Option<BookTerms> {
        self.books.get(book).copied()
    }
    fn load_books(&self) -> std::result::Result<Vec<([u8; 32], BookTerms)>, Refusal> {
        let through = format!("{BOOKS}~");
        let mut after = BOOKS.to_owned();
        let mut books = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(books);
            };
            for name in &names {
                let (stored, _) = load::<StoredBook>(&self.store, name)?.ok_or(Refusal::Storage)?;
                books.push((bytes32(&name[BOOKS.len()..])?, stored.into()));
            }
            after = last;
        }
    }
    /// `GrantIssuer`'s rules for `server` on `day`, as the chain answered.
    pub(super) fn learn_grant_day(
        &mut self,
        server: Account,
        day: u64,
        rules: super::chain::GrantDay,
    ) {
        self.grant_days.insert((server, day), rules);
    }
    /// Whether `grant` keeps its issuer's rules for its day at `now`. An
    /// issuer proven to grant one serial of a day twice grants nothing
    /// after that day.
    pub(super) fn check_grant(
        &self,
        grant: &agentic_grant_book::GrantBook,
        now: u64,
    ) -> std::result::Result<(), Refusal> {
        // A proven issuer needs no rules read to be refused.
        if let Some((proof, _)) = load::<agentic_grant_book::GrantEquivocation>(
            &self.store,
            &grant_equivocation_name(&grant.server),
        )? && grant.day > proof.first.day
        {
            return Err(Refusal::Blocked);
        }
        let day = self
            .grant_days
            .get(&(grant.server, grant.day))
            .ok_or(Refusal::GrantPending)?;
        let rules = agentic_grant_book::GrantRules {
            domain: self.domain,
            issuer_active: day.active,
            cap_coins: day.cap_coins,
            book_size: day.book_size,
            max_validity_days: day.max_validity_days,
        };
        grant.check(&rules, now).map_err(|_| Refusal::Grant)
    }
    /// Whether a notary that first saw `grant` at `first_seen` saw it on
    /// its day.
    pub(super) fn grant_in_time(
        &self,
        grant: &agentic_grant_book::GrantBook,
        first_seen: u64,
    ) -> bool {
        grant.first_seen_in_time(first_seen, GRANT_TOLERANCE)
    }
    /// Learn the book of a grant its notaries first saw at `first_seen`.
    pub(super) fn learn_grant(
        &mut self,
        grant: &agentic_grant_book::GrantBook,
        first_seen: u64,
        now: u64,
    ) -> std::result::Result<(), Refusal> {
        self.check_grant(grant, now)?;
        if !self.grant_in_time(grant, first_seen) {
            return Err(Refusal::Grant);
        }
        let book = grant.id();
        match self.grants.get(&book) {
            Some(known) if known == grant => return Ok(()),
            Some(_) => return Err(Refusal::Grant),
            None => {}
        }
        let mut changes = vec![
            StateChange {
                namespace: grant_name(&book),
                expected_revision: 0,
                bytes: json(grant)?,
            },
            StateChange {
                namespace: expiry_name(grant.expiry, "g", &book),
                expected_revision: 0,
                bytes: vec![],
            },
        ];
        // Revoked before it was shown here: learned, and blocked at once.
        let pending =
            load::<agentic_grant_book::GrantRevocation>(&self.store, &revocation_name(&book))?;
        let revoked = match &pending {
            Some((revocation, _))
                if revocation.revokes(grant) && !self.blocked.contains_key(&book) =>
            {
                let at = revocation.revoked_at.min(now);
                changes.push(StateChange {
                    namespace: blocked_name(&book),
                    expected_revision: 0,
                    bytes: json(&at)?,
                });
                Some(at)
            }
            _ => None,
        };
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        self.remember_grant(grant.clone());
        if let Some(at) = revoked {
            self.blocked.insert(book, at);
        }
        if let Some((revocation, revision)) = pending {
            let index = expiry_name(revocation.expiry, "r", &book);
            let removals = [(revocation_name(&book), revision)]
                .into_iter()
                .chain(self.removal(&index)?)
                .collect();
            self.store
                .commit_state_maintenance(vec![], removals)
                .map_err(|_| Refusal::Storage)?;
        }
        Ok(())
    }
    /// Take the identity server's revocation of a grant: a known grant's
    /// book is blocked as if proven spent twice, as of the revocation; a
    /// revocation of a grant not learned yet waits for it until the grant
    /// ends. Whether it was new here.
    pub(super) fn learn_revocation(
        &mut self,
        revocation: &agentic_grant_book::GrantRevocation,
        now: u64,
    ) -> std::result::Result<bool, Refusal> {
        if revocation.domain != self.domain
            || revocation.verify().is_err()
            || now >= revocation.expiry
        {
            return Ok(false);
        }
        let book = revocation.book;
        if let Some(grant) = self.grants.get(&book) {
            if !revocation.revokes(grant) || self.blocked.contains_key(&book) {
                return Ok(false);
            }
            let at = revocation.revoked_at.min(now);
            self.store
                .commit_states(vec![StateChange {
                    namespace: blocked_name(&book),
                    expected_revision: 0,
                    bytes: json(&at)?,
                }])
                .map_err(|_| Refusal::Storage)?;
            self.blocked.insert(book, at);
            return Ok(true);
        }
        let name = revocation_name(&book);
        if self
            .store
            .state(&name)
            .map_err(|_| Refusal::Storage)?
            .is_some()
        {
            return Ok(false);
        }
        self.store
            .commit_states(vec![
                StateChange {
                    namespace: name,
                    expected_revision: 0,
                    bytes: json(revocation)?,
                },
                StateChange {
                    namespace: expiry_name(revocation.expiry, "r", &book),
                    expected_revision: 0,
                    bytes: vec![],
                },
            ])
            .map_err(|_| Refusal::Storage)?;
        Ok(true)
    }
    /// Revocations kept for grants not learned here.
    fn pending_revocations(&self) -> std::result::Result<usize, Refusal> {
        self.count(REVOCATIONS)
    }
    fn count(&self, prefix: &str) -> std::result::Result<usize, Refusal> {
        let through = format!("{prefix}~");
        let mut after = prefix.to_owned();
        let mut count = 0;
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(count);
            };
            count += names.len();
            after = last;
        }
    }
    /// Grants spent twice not yet reported, by book.
    pub(super) fn reports(&self) -> std::result::Result<Vec<Report>, Refusal> {
        let through = format!("{REPORTS}~");
        let mut after = REPORTS.to_owned();
        let mut reports = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(reports);
            };
            for name in &names {
                let (saved, _) = load::<SavedReport>(&self.store, name)?.ok_or(Refusal::Storage)?;
                reports.push(Report {
                    grant: saved.grant,
                    first: Stamp::try_from(&saved.first)?,
                    second: Stamp::try_from(&saved.second)?,
                });
            }
            after = last;
        }
    }
    /// Whether a report was queued since the last call.
    pub(super) fn take_new_reports(&mut self) -> bool {
        std::mem::take(&mut self.new_reports)
    }
    /// The identity server answered the report of `book`'s grant.
    pub(super) fn reported(&mut self, book: &[u8; 32]) -> std::result::Result<(), Refusal> {
        let name = report_name(book);
        let Some((saved, revision)) = load::<SavedReport>(&self.store, &name)? else {
            return Ok(());
        };
        let index = expiry_name(saved.grant.expiry, "p", book);
        let removals = [(name, revision)]
            .into_iter()
            .chain(self.removal(&index)?)
            .collect();
        self.store
            .commit_state_maintenance(vec![], removals)
            .map_err(|_| Refusal::Storage)
    }
    fn remember_grant(&mut self, grant: agentic_grant_book::GrantBook) {
        let book = grant.id();
        self.books.insert(
            book,
            BookTerms {
                key: grant.book,
                count: grant.count,
                valid_until: grant.expiry,
            },
        );
        self.grants.insert(book, grant);
    }
    fn forget_grant(&mut self, book: &[u8; 32]) {
        if self.grants.remove(book).is_some() {
            self.books.remove(book);
        }
    }
    fn load_grants(&self) -> std::result::Result<Vec<agentic_grant_book::GrantBook>, Refusal> {
        let through = format!("{GRANTS}~");
        let mut after = GRANTS.to_owned();
        let mut grants = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(grants);
            };
            for name in &names {
                let (grant, _) = load::<agentic_grant_book::GrantBook>(&self.store, name)?
                    .ok_or(Refusal::Storage)?;
                grants.push(grant);
            }
            after = last;
        }
    }
    /// The learned grant of a book.
    pub(super) fn granted(
        &self,
        book: &[u8; 32],
    ) -> std::result::Result<Option<agentic_grant_book::GrantBook>, Refusal> {
        Ok(self.grants.get(book).cloned())
    }
    pub(super) fn unit(&self) -> Option<[u8; 32]> {
        self.unit
    }
    /// Diagnostics: the unit and receipt account this node holds as, and
    /// how many double-spent slots it has proof of.
    pub(super) fn info(&self) -> Value {
        json!({
            "unit": self.unit.map(hex::encode),
            "account": hex::encode(self.account()),
            "equivocations": self.equivocations().map(|proofs| proofs.len()).ok(),
            "grantEquivocations": self.grant_equivocations().map(|proofs| proofs.len()).ok(),
            "blockedBooks": self.blocked_books().map(|books| books.len()).ok(),
            "blockedIssuers": self.blocked_issuers().map(|issuers| issuers.len()).ok(),
            "grants": self.grants.len(),
            "pendingRevocations": self.pending_revocations().ok(),
            "reports": self.count(REPORTS).ok(),
            "tickets": self.held_tickets().ok(),
        })
    }
    /// Store one stamped envelope sent fresh for `period`, or return the
    /// receipt of the same operation already stored. Only periods someone
    /// may still write are taken.
    pub(super) fn store(
        &mut self,
        mailbox: [u8; 32],
        period: u64,
        envelope: &[u8],
        stamp: &Stamp,
        now: u64,
    ) -> std::result::Result<Receipt, Refusal> {
        self.unit.ok_or(Refusal::NoUnit)?;
        if envelope.len() > MAX_ENVELOPE {
            return Err(Refusal::TooLarge);
        }
        if !writable(period, now) {
            return Err(Refusal::Period);
        }
        self.insert(mailbox, period, envelope, stamp, Arrival::Fresh, now)
    }
    /// Store an entry pulled from another swarm member: its mailbox must
    /// still be kept, and its stamp must have been valid when its period
    /// began, so a copy paid in time is repaired after its book ended.
    pub(super) fn store_replica(
        &mut self,
        mailbox: [u8; 32],
        period: u64,
        envelope: &[u8],
        stamp: &Stamp,
        now: u64,
    ) -> std::result::Result<Receipt, Refusal> {
        self.unit.ok_or(Refusal::NoUnit)?;
        if envelope.len() > MAX_ENVELOPE {
            return Err(Refusal::TooLarge);
        }
        if !live(period, now) {
            return Err(Refusal::Period);
        }
        let paid_at = period.saturating_mul(PERIOD_SECONDS);
        self.insert(
            mailbox,
            period,
            envelope,
            stamp,
            Arrival::Replica { paid_at },
            now,
        )
    }
    fn insert(
        &mut self,
        mailbox: [u8; 32],
        period: u64,
        envelope: &[u8],
        stamp: &Stamp,
        arrival: Arrival,
        now: u64,
    ) -> std::result::Result<Receipt, Refusal> {
        let (paid_at, fresh) = match arrival {
            Arrival::Fresh => (now, true),
            Arrival::Replica { paid_at } => (paid_at, false),
        };
        let unit = self.unit.ok_or(Refusal::NoUnit)?;
        let terms = *self.books.get(&stamp.book).ok_or(Refusal::UnknownBook)?;
        stamp
            .verify(&self.domain, &terms, paid_at)
            .map_err(Refusal::Stamp)?;
        if !stamp.pays_for(&mailbox, period, envelope) {
            return Err(Refusal::Operation);
        }
        let (summary, summary_revision) =
            match load::<SavedSummary>(&self.store, &summary_name(&mailbox))? {
                Some((saved, revision)) => {
                    if saved.period != period {
                        return Err(Refusal::Period);
                    }
                    (saved, revision)
                }
                None => (
                    SavedSummary {
                        period,
                        count: 0,
                        digest: hex::encode([0; 32]),
                    },
                    0,
                ),
            };
        let ticket = stamp.ticket_id(&self.domain);
        if let Some((saved, _)) = load::<SavedTicket>(&self.store, &ticket_name(&ticket))? {
            if saved.receipt.operation != stamp.operation {
                // Both stamps verified under the book key: keep the pair as
                // proof. A failed write still refuses the store.
                let _ = self.keep_equivocation(&ticket, &saved.stamp, stamp, now);
                return Err(Refusal::Conflict);
            }
            return Receipt::try_from(&saved.receipt);
        }
        // A proven book pays for nothing new; copies of what it paid for
        // before are still repaired.
        if let Some(at) = self.blocked.get(&stamp.book) {
            // A replica is a copy of what was paid before the proof was known
            // only if its period was written by then.
            if fresh || period > agentic_mailbox_swarm::address::period(*at) + 1 {
                return Err(Refusal::Blocked);
            }
        }
        let (next, revision) = load::<u64>(&self.store, NEXT)?.unwrap_or((0, 0));
        let seq = next + 1;
        let mut digest = bytes32(&summary.digest)?;
        for (byte, op) in digest.iter_mut().zip(stamp.operation) {
            *byte ^= op;
        }
        let receipt = Receipt::sign(
            &self.domain,
            mailbox,
            stamp.operation,
            ticket,
            unit,
            now,
            &self.key,
        );
        let entry = Saved {
            seq,
            period,
            envelope: hex::encode(envelope),
            stamp: StampWire::from(stamp),
            receipt: ReceiptWire::from(&receipt),
        };
        // A slot stays spent while its book can sign or its entry is kept.
        let keep_until = terms.valid_until.max(expires_at(period));
        let record = SavedTicket {
            stamp: StampWire::from(stamp),
            receipt: ReceiptWire::from(&receipt),
            keep_until,
        };
        let mut changes = vec![
            StateChange {
                namespace: entry_name(&mailbox, seq),
                expected_revision: 0,
                bytes: json(&entry)?,
            },
            StateChange {
                namespace: ticket_name(&ticket),
                expected_revision: 0,
                bytes: json(&record)?,
            },
            StateChange {
                namespace: expiry_name(keep_until, "t", &ticket),
                expected_revision: 0,
                bytes: vec![],
            },
            StateChange {
                namespace: NEXT.into(),
                expected_revision: revision,
                bytes: json(&seq)?,
            },
            StateChange {
                namespace: summary_name(&mailbox),
                expected_revision: summary_revision,
                bytes: json(&SavedSummary {
                    period,
                    count: summary.count + 1,
                    digest: hex::encode(digest),
                })?,
            },
        ];
        if summary_revision == 0 {
            changes.push(StateChange {
                namespace: expiry_name(expires_at(period), "m", &mailbox),
                expected_revision: 0,
                bytes: vec![],
            });
        }
        changes.extend(self.payout_changes(&mailbox, period, envelope, stamp, &terms, unit)?);
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        self.fresh.push(stamp.clone());
        Ok(receipt)
    }
    /// Drop what retention no longer keeps: mailboxes past their period's
    /// retention, and ticket records whose book and mailbox both ended.
    pub(super) fn collect(&mut self, now: u64) -> std::result::Result<(), Refusal> {
        let through = format!("{EXPIRY}{now:020}/~");
        loop {
            let due = self
                .store
                .state_namespaces_between(EXPIRY, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            if due.is_empty() {
                return Ok(());
            }
            for name in due {
                let mut parts = name[EXPIRY.len()..].splitn(3, '/').skip(1);
                let (Some(kind), Some(id)) = (parts.next(), parts.next()) else {
                    return Err(Refusal::Storage);
                };
                let id = bytes32(id)?;
                let mut removals = Vec::new();
                match kind {
                    "m" => {
                        self.remove_entries(&id)?;
                        removals.push(self.removal(&summary_name(&id))?);
                    }
                    "t" => removals.push(self.removal(&ticket_name(&id))?),
                    "n" => removals.push(self.removal(&notary_name(&id))?),
                    "g" => {
                        removals.push(self.removal(&grant_name(&id))?);
                        self.forget_grant(&id);
                    }
                    "r" => removals.push(self.removal(&revocation_name(&id))?),
                    "p" => removals.push(self.removal(&report_name(&id))?),
                    // A book learned again with a later end keeps its newer
                    // index entry.
                    "b" => match load::<StoredBook>(&self.store, &book_name(&id))? {
                        Some((stored, _)) if book_kept_until(&BookTerms::from(stored)) <= now => {
                            removals.push(self.removal(&book_name(&id))?);
                            if !self.grants.contains_key(&id) {
                                self.books.remove(&id);
                            }
                        }
                        _ => {}
                    },
                    _ => return Err(Refusal::Storage),
                }
                removals.push(self.removal(&name)?);
                self.store
                    .commit_state_maintenance(vec![], removals.into_iter().flatten().collect())
                    .map_err(|_| Refusal::Storage)?;
            }
        }
    }
    fn remove_entries(&mut self, mailbox: &[u8; 32]) -> std::result::Result<(), Refusal> {
        loop {
            let names = self
                .store
                .state_namespaces_between(
                    &entry_name(mailbox, 0),
                    &entry_name(mailbox, u64::MAX),
                    64,
                )
                .map_err(|_| Refusal::Storage)?;
            if names.is_empty() {
                return Ok(());
            }
            let removals = names
                .iter()
                .map(|name| self.removal(name))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            self.store
                .commit_state_maintenance(vec![], removals.into_iter().flatten().collect())
                .map_err(|_| Refusal::Storage)?;
        }
    }
    /// The namespace and revision to remove, if the state exists.
    fn removal(&self, name: &str) -> std::result::Result<Option<(String, u64)>, Refusal> {
        Ok(self
            .store
            .state(name)
            .map_err(|_| Refusal::Storage)?
            .map(|state| (name.to_owned(), state.revision)))
    }
    /// Ticket records this holder keeps.
    pub(super) fn held_tickets(&self) -> std::result::Result<usize, Refusal> {
        let through = format!("{TICKETS}{}", "f".repeat(64));
        let mut after = TICKETS.to_owned();
        let mut count = 0;
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(count);
            };
            count += names.len();
            after = last;
        }
    }
    /// How many operations of `mailbox` this holder stores, and the XOR of
    /// them: the same set gives the same summary in any arrival order.
    pub(super) fn summary(&self, mailbox: &[u8; 32]) -> std::result::Result<Summary, Refusal> {
        load::<SavedSummary>(&self.store, &summary_name(mailbox))?.map_or(
            Ok(Summary::default()),
            |(saved, _)| {
                Ok(Summary {
                    count: saved.count,
                    digest: bytes32(&saved.digest)?,
                })
            },
        )
    }
    /// Held mailboxes after `after` in id order, at most `limit`
    /// (≤ MAX_SUMMARIES), with their summaries.
    pub(super) fn mailboxes(
        &self,
        after: Option<&[u8; 32]>,
        limit: usize,
    ) -> std::result::Result<Vec<([u8; 32], Summary)>, Refusal> {
        let limit = limit.min(MAX_SUMMARIES);
        if limit == 0 {
            return Ok(vec![]);
        }
        let start = after.map_or_else(|| SUMMARIES.to_owned(), summary_name);
        let names = self
            .store
            .state_namespaces_between(&start, &format!("{SUMMARIES}{}", "f".repeat(64)), limit)
            .map_err(|_| Refusal::Storage)?;
        names
            .iter()
            .map(|name| {
                let mailbox = bytes32(&name[SUMMARIES.len()..])?;
                Ok((mailbox, self.summary(&mailbox)?))
            })
            .collect()
    }
    /// Keep `statement` if it is the first for its key; answer with the
    /// first.
    pub(super) fn notarize(
        &mut self,
        statement: &Statement,
        now: u64,
    ) -> std::result::Result<Notarized, Refusal> {
        // Only verifiable statements take a key: nobody can occupy another
        // book's slot or an issuer's serial first.
        let (key, keep_until) = match statement {
            Statement::Ticket(stamp) => {
                let terms = self.books.get(&stamp.book).ok_or(Refusal::UnknownBook)?;
                if stamp.signer(&self.domain).map_err(Refusal::Stamp)? != terms.key {
                    return Err(Refusal::Stamp(StampError::Signer));
                }
                if stamp.index >= terms.count {
                    return Err(Refusal::Stamp(StampError::Index));
                }
                (stamp.ticket_id(&self.domain), terms.valid_until)
            }
            Statement::Grant(grant) => {
                grant.verify_signature().map_err(|_| Refusal::Grant)?;
                (grant.id(), grant.expiry)
            }
            Statement::Commit(claim) => {
                let verified = agentic_protocol::group::verify_claim(claim, self.domain, now)
                    .map_err(|_| Refusal::Claim)?;
                // As long as the epoch's mailbox lives, and two periods more.
                let keep = agentic_mailbox_swarm::address::expires_at(
                    agentic_mailbox_swarm::address::period(now),
                )
                .saturating_add(2 * agentic_mailbox_swarm::address::PERIOD_SECONDS);
                (verified.key, keep)
            }
        };
        if let Some(noted) = self.notary_record(&key)? {
            match (&noted.first, statement) {
                (Statement::Ticket(first), Statement::Ticket(second))
                    if first.operation != second.operation =>
                {
                    let _ = self.keep_equivocation(&key, &StampWire::from(first), second, now);
                }
                (Statement::Grant(first), Statement::Grant(second))
                    if first.digest() != second.digest() =>
                {
                    let _ = self.keep_grant_equivocation(first, second);
                }
                _ => {}
            }
            return Ok(noted);
        }
        if let Statement::Ticket(stamp) = statement
            && self.blocked.contains_key(&stamp.book)
        {
            return Err(Refusal::Blocked);
        }
        let record = SavedNotary {
            statement: StatementWire::from(statement),
            first_seen: now,
        };
        self.store
            .commit_states(vec![
                StateChange {
                    namespace: notary_name(&key),
                    expected_revision: 0,
                    bytes: json(&record)?,
                },
                StateChange {
                    namespace: expiry_name(keep_until, "n", &key),
                    expected_revision: 0,
                    bytes: vec![],
                },
            ])
            .map_err(|_| Refusal::Storage)?;
        Ok(Notarized {
            first: statement.clone(),
            first_seen: now,
        })
    }
    /// The statement first notarized here for `key`, and when.
    pub(super) fn notary_record(
        &self,
        key: &[u8; 32],
    ) -> std::result::Result<Option<Notarized>, Refusal> {
        load::<SavedNotary>(&self.store, &notary_name(key))?
            .map(|(saved, _)| {
                Ok(Notarized {
                    first: Statement::try_from(&saved.statement).map_err(|_| Refusal::Storage)?,
                    first_seen: saved.first_seen,
                })
            })
            .transpose()
    }
    /// Keep two stamps of one slot as proof, if the book's key signed both.
    pub(super) fn keep_sender_proof(
        &mut self,
        first: &Stamp,
        second: &Stamp,
    ) -> std::result::Result<bool, Refusal> {
        let Some(terms) = self.books.get(&first.book) else {
            return Ok(false);
        };
        let proof = SenderEquivocation {
            first: first.clone(),
            second: second.clone(),
        };
        if proof.verify(&self.domain, &terms.key).is_err() {
            return Ok(false);
        }
        self.keep_equivocation(
            &first.ticket_id(&self.domain),
            &StampWire::from(first),
            second,
            clock::wall().unwrap_or(0),
        )?;
        Ok(true)
    }
    /// Keep a serial granted twice as proof against its issuer, and pass it
    /// on, unless a proof of the same or an earlier day is already kept: the
    /// earliest proven day is where the issuer's grants stop.
    fn keep_grant_equivocation(
        &mut self,
        first: &agentic_grant_book::GrantBook,
        second: &agentic_grant_book::GrantBook,
    ) -> std::result::Result<bool, Refusal> {
        let name = grant_equivocation_name(&first.server);
        let revision = match load::<agentic_grant_book::GrantEquivocation>(&self.store, &name)? {
            Some((kept, _)) if kept.first.day <= first.day => return Ok(false),
            Some((_, revision)) => revision,
            None => 0,
        };
        let proof = agentic_grant_book::GrantEquivocation {
            first: first.clone(),
            second: second.clone(),
        };
        let mut changes = vec![StateChange {
            namespace: name,
            expected_revision: revision,
            bytes: json(&proof)?,
        }];
        changes.extend(self.log_change(SavedProof::Grant(proof))?);
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        Ok(true)
    }
    /// Every issuer serial this holder saw granted twice, as proofs.
    pub(super) fn grant_equivocations(
        &self,
    ) -> std::result::Result<Vec<agentic_grant_book::GrantEquivocation>, Refusal> {
        let through = format!("{GRANT_EQUIVOCATIONS}{}", "f".repeat(64));
        let mut after = GRANT_EQUIVOCATIONS.to_owned();
        let mut proofs = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(proofs);
            };
            for name in &names {
                let (proof, _) = load::<agentic_grant_book::GrantEquivocation>(&self.store, name)?
                    .ok_or(Refusal::Storage)?;
                proofs.push(proof);
            }
            after = last;
        }
    }
    /// Stamps of entries newly stored since the last call, for the notary
    /// check.
    pub(super) fn take_fresh(&mut self) -> Vec<Stamp> {
        std::mem::take(&mut self.fresh)
    }
    /// Books proven to pay twice for one slot.
    pub(super) fn blocked_books(&self) -> std::result::Result<BTreeSet<[u8; 32]>, Refusal> {
        Ok(self.blocked.keys().copied().collect())
    }
    /// Holders proven to receipt two operations for one slot.
    pub(super) fn blocked_holders(&self) -> std::result::Result<BTreeSet<Account>, Refusal> {
        let through = format!("{HOLDER_EQUIVOCATIONS}~");
        let mut after = HOLDER_EQUIVOCATIONS.to_owned();
        let mut accounts = BTreeSet::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(accounts);
            };
            for name in &names {
                let account = hex::decode(&name[HOLDER_EQUIVOCATIONS.len()..])
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or(Refusal::Storage)?;
                accounts.insert(account);
            }
            after = last;
        }
    }
    /// Grant issuers proven to grant one serial twice.
    pub(super) fn blocked_issuers(&self) -> std::result::Result<BTreeSet<Account>, Refusal> {
        Ok(self
            .grant_equivocations()?
            .into_iter()
            .map(|proof| proof.first.server)
            .collect())
    }
    /// Keep every offered proof that verifies here and is new; answer with
    /// those. A sender proof verifies only against a book this node knows.
    pub(super) fn accept_proofs(
        &mut self,
        proofs: &Proofs,
    ) -> std::result::Result<Proofs, Refusal> {
        let mut kept = Proofs::default();
        for proof in &proofs.senders {
            let Some(terms) = self.books.get(&proof.first.book) else {
                continue;
            };
            if self.blocked.contains_key(&proof.first.book)
                || proof.verify(&self.domain, &terms.key).is_err()
            {
                continue;
            }
            let ticket = proof.first.ticket_id(&self.domain);
            if load::<SavedEquivocation>(&self.store, &equivocation_name(&ticket))?.is_some() {
                continue;
            }
            self.keep_equivocation(
                &ticket,
                &StampWire::from(&proof.first),
                &proof.second,
                clock::wall().unwrap_or(0),
            )?;
            kept.senders.push(proof.clone());
        }
        for proof in &proofs.holders {
            let Ok(signer) = proof.first.signer(&self.domain) else {
                continue;
            };
            if proof.verify(&self.domain, &signer).is_ok()
                && self.keep_holder_equivocation(&signer, proof)?
            {
                kept.holders.push(proof.clone());
            }
        }
        for proof in &proofs.grants {
            if proof.verify().is_err() {
                continue;
            }
            if self.keep_grant_equivocation(&proof.first, &proof.second)? {
                kept.grants.push(proof.clone());
            }
        }
        Ok(kept)
    }
    /// The proofs learned after `after`, in learned order.
    pub(super) fn proofs(
        &self,
        after: u64,
        limit: usize,
    ) -> std::result::Result<ProofPage, Refusal> {
        let limit = limit.min(MAX_PROOFS);
        let mut proofs = Proofs::default();
        if limit == 0 {
            return Ok(ProofPage { proofs, next: None });
        }
        let names = self
            .store
            .state_namespaces_between(&proof_name(after), &format!("{PROOFS}~"), limit)
            .map_err(|_| Refusal::Storage)?;
        for name in &names {
            let (saved, _) = load::<SavedProof>(&self.store, name)?.ok_or(Refusal::Storage)?;
            match saved {
                SavedProof::Sender(wire) => proofs.senders.push(SenderEquivocation {
                    first: Stamp::try_from(&wire.first)?,
                    second: Stamp::try_from(&wire.second)?,
                }),
                SavedProof::Holder(wire) => {
                    proofs
                        .holders
                        .push(agentic_mailbox_swarm::proof::HolderEquivocation {
                            first: Receipt::try_from(&wire.first)?,
                            second: Receipt::try_from(&wire.second)?,
                        })
                }
                SavedProof::Grant(proof) => proofs.grants.push(proof),
            }
        }
        let next = if names.len() == limit {
            names
                .last()
                .and_then(|name| name[PROOFS.len()..].parse().ok())
        } else {
            None
        };
        Ok(ProofPage { proofs, next })
    }
    #[cfg(test)]
    pub(super) fn receipt_for_tests(
        &self,
        mailbox: [u8; 32],
        operation: [u8; 32],
        ticket: [u8; 32],
        stored_at: u64,
    ) -> Receipt {
        Receipt::sign(
            &self.domain,
            mailbox,
            operation,
            ticket,
            self.unit.unwrap_or_default(),
            stored_at,
            &self.key,
        )
    }
    /// Every slot this holder saw spent on two operations, as proofs.
    pub(super) fn equivocations(&self) -> std::result::Result<Vec<SenderEquivocation>, Refusal> {
        let through = format!("{EQUIVOCATIONS}{}", "f".repeat(64));
        let mut after = EQUIVOCATIONS.to_owned();
        let mut proofs = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(proofs);
            };
            for name in &names {
                let (saved, _) =
                    load::<SavedEquivocation>(&self.store, name)?.ok_or(Refusal::Storage)?;
                proofs.push(SenderEquivocation {
                    first: Stamp::try_from(&saved.first).map_err(|_| Refusal::Storage)?,
                    second: Stamp::try_from(&saved.second).map_err(|_| Refusal::Storage)?,
                });
            }
            after = last;
        }
    }
    fn keep_equivocation(
        &mut self,
        ticket: &[u8; 32],
        first: &StampWire,
        second: &Stamp,
        now: u64,
    ) -> std::result::Result<(), Refusal> {
        let name = equivocation_name(ticket);
        if load::<SavedEquivocation>(&self.store, &name)?.is_some() {
            return Ok(());
        }
        let proof = SavedEquivocation {
            first: first.clone(),
            second: StampWire::from(second),
        };
        let mut changes = vec![StateChange {
            namespace: name,
            expected_revision: 0,
            bytes: serde_json::to_vec(&proof).map_err(|_| Refusal::Storage)?,
        }];
        // One proof per book is passed on: the first blocks it everywhere,
        // from the moment this node learned it.
        let first_proof = !self.blocked.contains_key(&second.book);
        let mut report = false;
        if first_proof {
            changes.extend(self.log_change(SavedProof::Sender(SenderProofWire {
                first: first.clone(),
                second: StampWire::from(second),
            }))?);
            changes.push(StateChange {
                namespace: blocked_name(&second.book),
                expected_revision: 0,
                bytes: json(&now)?,
            });
            // A grant's identity answers for it at its identity server.
            if let Some(grant) = self.grants.get(&second.book) {
                report = true;
                changes.push(StateChange {
                    namespace: report_name(&second.book),
                    expected_revision: 0,
                    bytes: json(&SavedReport {
                        grant: grant.clone(),
                        first: first.clone(),
                        second: StampWire::from(second),
                    })?,
                });
                changes.push(StateChange {
                    namespace: expiry_name(grant.expiry, "p", &second.book),
                    expected_revision: 0,
                    bytes: vec![],
                });
            }
        }
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        if first_proof {
            self.blocked.insert(second.book, now);
        }
        self.new_reports |= report;
        Ok(())
    }
    /// Blocked books and when each proof was learned.
    fn load_blocked(&self) -> std::result::Result<BTreeMap<[u8; 32], u64>, Refusal> {
        let through = format!("{BLOCKED}~");
        let mut after = BLOCKED.to_owned();
        let mut blocked = BTreeMap::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(blocked);
            };
            for name in &names {
                let (at, _) = load::<u64>(&self.store, name)?.ok_or(Refusal::Storage)?;
                blocked.insert(bytes32(&name[BLOCKED.len()..])?, at);
            }
            after = last;
        }
    }
    /// The state that appends `proof` to the log of learned proofs.
    fn log_change(&self, proof: SavedProof) -> std::result::Result<Vec<StateChange>, Refusal> {
        let (last, revision) = load::<u64>(&self.store, PROOF_NEXT)?.unwrap_or((0, 0));
        Ok(vec![
            StateChange {
                namespace: proof_name(last + 1),
                expected_revision: 0,
                bytes: json(&proof)?,
            },
            StateChange {
                namespace: PROOF_NEXT.into(),
                expected_revision: revision,
                bytes: json(&(last + 1))?,
            },
        ])
    }
    /// Keep a holder's two receipts for one slot, once.
    fn keep_holder_equivocation(
        &mut self,
        account: &Account,
        proof: &agentic_mailbox_swarm::proof::HolderEquivocation,
    ) -> std::result::Result<bool, Refusal> {
        let name = holder_equivocation_name(account);
        if self
            .store
            .state(&name)
            .map_err(|_| Refusal::Storage)?
            .is_some()
        {
            return Ok(false);
        }
        let wire = HolderProofWire {
            first: ReceiptWire::from(&proof.first),
            second: ReceiptWire::from(&proof.second),
        };
        let mut changes = vec![StateChange {
            namespace: name,
            expected_revision: 0,
            bytes: json(&wire)?,
        }];
        changes.extend(self.log_change(SavedProof::Holder(wire))?);
        self.store
            .commit_states(changes)
            .map_err(|_| Refusal::Storage)?;
        Ok(true)
    }
    /// Entries of `mailbox` after cursor `after`, oldest first, at most
    /// `limit` (≤ MAX_PAGE).
    pub(super) fn read(
        &self,
        mailbox: &[u8; 32],
        after: u64,
        limit: usize,
    ) -> std::result::Result<Page, Refusal> {
        let limit = limit.min(MAX_PAGE);
        if limit == 0 {
            return Ok(Page {
                entries: vec![],
                next: after,
            });
        }
        let names = self
            .store
            .state_namespaces_between(
                &entry_name(mailbox, after),
                &entry_name(mailbox, u64::MAX),
                limit,
            )
            .map_err(|_| Refusal::Storage)?;
        let mut entries = Vec::with_capacity(names.len());
        for name in names {
            let (saved, _) = load::<Saved>(&self.store, &name)?.ok_or(Refusal::Storage)?;
            entries.push(Entry {
                seq: saved.seq,
                period: saved.period,
                envelope: hex::decode(&saved.envelope).map_err(|_| Refusal::Storage)?,
                stamp: Stamp::try_from(&saved.stamp)?,
                stored_at: saved.receipt.stored_at,
            });
        }
        let next = entries.last().map_or(after, |e| e.seq);
        Ok(Page { entries, next })
    }
}

// --- wire -------------------------------------------------------------------

/// A pass on the wire (`agentic_mailbox_swarm::access::AccessPass`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AccessPassWire {
    #[serde(with = "serde_bytes")]
    pub(super) book: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) peer: Vec<u8>,
    pub(super) day: u64,
    #[serde(with = "serde_bytes")]
    pub(super) signature: Vec<u8>,
}

impl From<&AccessPass> for AccessPassWire {
    fn from(pass: &AccessPass) -> Self {
        Self {
            book: pass.book.to_vec(),
            peer: pass.peer.to_vec(),
            day: pass.day,
            signature: pass.signature.to_vec(),
        }
    }
}
impl TryFrom<&AccessPassWire> for AccessPass {
    type Error = Refusal;
    fn try_from(wire: &AccessPassWire) -> std::result::Result<Self, Refusal> {
        let malformed = |_| Refusal::Malformed;
        Ok(Self {
            book: wire.book.as_slice().try_into().map_err(malformed)?,
            peer: wire.peer.as_slice().try_into().map_err(malformed)?,
            day: wire.day,
            signature: wire.signature.as_slice().try_into().map_err(malformed)?,
        })
    }
}

/// What a peer shows to be let in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum CredentialWire {
    /// A pass of the peer's active book, with its grant for a granted book.
    Pass {
        pass: AccessPassWire,
        grant: Option<agentic_grant_book::GrantBook>,
    },
    /// A registry unit's own record, signed by its transport key.
    Unit { record: UnitRecordWire },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StampWire {
    #[serde(with = "serde_bytes")]
    pub(super) book: Vec<u8>,
    pub(super) index: u32,
    #[serde(with = "serde_bytes")]
    pub(super) operation: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) signature: Vec<u8>,
    /// The units a mailbox stamp names; absent in stamps that name nobody,
    /// as every stamp before Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) holders: Option<Vec<serde_bytes::ByteBuf>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReceiptWire {
    #[serde(with = "serde_bytes")]
    pub(super) mailbox: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) operation: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) ticket: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) holder: Vec<u8>,
    pub(super) stored_at: u64,
    #[serde(with = "serde_bytes")]
    pub(super) signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EntryWire {
    pub(super) seq: u64,
    pub(super) period: u64,
    #[serde(with = "serde_bytes")]
    pub(super) envelope: Vec<u8>,
    pub(super) stamp: StampWire,
    pub(super) stored_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SummaryWire {
    #[serde(with = "serde_bytes")]
    pub(super) mailbox: Vec<u8>,
    pub(super) count: u64,
    #[serde(with = "serde_bytes")]
    pub(super) digest: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SenderProofWire {
    pub(super) first: StampWire,
    pub(super) second: StampWire,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HolderProofWire {
    pub(super) first: ReceiptWire,
    pub(super) second: ReceiptWire,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProofsWire {
    pub(super) senders: Vec<SenderProofWire>,
    pub(super) holders: Vec<HolderProofWire>,
    pub(super) grants: Vec<agentic_grant_book::GrantEquivocation>,
}

impl From<&Proofs> for ProofsWire {
    fn from(proofs: &Proofs) -> Self {
        Self {
            senders: proofs
                .senders
                .iter()
                .map(|p| SenderProofWire {
                    first: StampWire::from(&p.first),
                    second: StampWire::from(&p.second),
                })
                .collect(),
            holders: proofs
                .holders
                .iter()
                .map(|p| HolderProofWire {
                    first: ReceiptWire::from(&p.first),
                    second: ReceiptWire::from(&p.second),
                })
                .collect(),
            grants: proofs.grants.clone(),
        }
    }
}
impl TryFrom<&ProofsWire> for Proofs {
    type Error = Refusal;
    fn try_from(wire: &ProofsWire) -> std::result::Result<Self, Refusal> {
        if wire.senders.len() + wire.holders.len() + wire.grants.len() > MAX_PROOFS {
            return Err(Refusal::Malformed);
        }
        let receipt = |r: &ReceiptWire| Receipt::try_from(r).map_err(|_| Refusal::Malformed);
        Ok(Self {
            senders: wire
                .senders
                .iter()
                .map(|p| {
                    Ok(SenderEquivocation {
                        first: Stamp::try_from(&p.first)?,
                        second: Stamp::try_from(&p.second)?,
                    })
                })
                .collect::<std::result::Result<_, Refusal>>()?,
            holders: wire
                .holders
                .iter()
                .map(|p| {
                    Ok(agentic_mailbox_swarm::proof::HolderEquivocation {
                        first: receipt(&p.first)?,
                        second: receipt(&p.second)?,
                    })
                })
                .collect::<std::result::Result<_, Refusal>>()?,
            grants: wire.grants.clone(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum StatementWire {
    Ticket {
        stamp: StampWire,
    },
    Grant {
        grant: agentic_grant_book::GrantBook,
    },
    Commit {
        #[serde(with = "serde_bytes")]
        claim: Vec<u8>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum Request {
    Store {
        #[serde(with = "serde_bytes")]
        mailbox: Vec<u8>,
        /// The period the stamp pays for.
        period: u64,
        #[serde(with = "serde_bytes")]
        envelope: Vec<u8>,
        stamp: StampWire,
    },
    Read {
        #[serde(with = "serde_bytes")]
        mailbox: Vec<u8>,
        after: u64,
        limit: u16,
    },
    /// A swarm member's view of mailboxes both hold.
    Summaries { items: Vec<SummaryWire> },
    /// Put statements on record with one of their keys' notaries: at
    /// least one, at most `MAX_NOTARIZE`.
    Notarize { statements: Vec<StatementWire> },
    /// The proofs a node learned after cursor `after`.
    Proofs { after: u64 },
    /// A grant funding a book this holder does not know yet.
    LearnGrant {
        grant: agentic_grant_book::GrantBook,
    },
    /// Every unit record this node has verified.
    Directory,
    /// Be let in (access by book): the only request taken from a peer that
    /// is neither a unit nor showed an accepted pass.
    Access { credential: CredentialWire },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum Response {
    Stored {
        receipt: ReceiptWire,
    },
    Refused {
        code: String,
    },
    Page {
        entries: Vec<EntryWire>,
        next: u64,
        /// The grants of the page's granted books, each once, so a member
        /// that pulls them can learn the books.
        grants: Vec<agentic_grant_book::GrantBook>,
    },
    Summaries {
        items: Vec<SummaryWire>,
    },
    /// One answer per statement, in order.
    Notarized {
        answers: Vec<AnswerWire>,
    },
    Proofs {
        proofs: ProofsWire,
        next: Option<u64>,
    },
    /// The grant's book is known here.
    Learned,
    Directory {
        records: Vec<UnitRecordWire>,
    },
    /// Let in until this wall time.
    Access {
        until: u64,
    },
}

/// A unit record on the wire (`agentic_mailbox_swarm::directory::UnitRecord`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UnitRecordWire {
    #[serde(with = "serde_bytes")]
    pub(super) transport_key: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub(super) receipt: Vec<u8>,
    pub(super) addresses: Vec<String>,
    pub(super) issued_at: u64,
    #[serde(with = "serde_bytes")]
    pub(super) signature: Vec<u8>,
}

impl From<&agentic_mailbox_swarm::directory::UnitRecord> for UnitRecordWire {
    fn from(record: &agentic_mailbox_swarm::directory::UnitRecord) -> Self {
        Self {
            transport_key: record.transport_key.to_vec(),
            receipt: record.receipt.to_vec(),
            addresses: record.addresses.clone(),
            issued_at: record.issued_at,
            signature: record.signature.to_vec(),
        }
    }
}

impl TryFrom<&UnitRecordWire> for agentic_mailbox_swarm::directory::UnitRecord {
    type Error = Refusal;
    fn try_from(wire: &UnitRecordWire) -> std::result::Result<Self, Refusal> {
        Ok(Self {
            transport_key: wire
                .transport_key
                .as_slice()
                .try_into()
                .map_err(|_| Refusal::Malformed)?,
            receipt: wire
                .receipt
                .as_slice()
                .try_into()
                .map_err(|_| Refusal::Malformed)?,
            addresses: wire.addresses.clone(),
            issued_at: wire.issued_at,
            signature: wire
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| Refusal::Malformed)?,
        })
    }
}

/// A notary's answer about one statement: the first on record for its key
/// and when it was first seen, or why the statement was refused.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum AnswerWire {
    Noted {
        first: StatementWire,
        first_seen: u64,
    },
    Refused {
        code: String,
    },
}

impl From<&Statement> for StatementWire {
    fn from(statement: &Statement) -> Self {
        match statement {
            Statement::Ticket(stamp) => Self::Ticket {
                stamp: StampWire::from(stamp),
            },
            Statement::Grant(grant) => Self::Grant {
                grant: grant.clone(),
            },
            Statement::Commit(claim) => Self::Commit {
                claim: claim.clone(),
            },
        }
    }
}
impl TryFrom<&StatementWire> for Statement {
    type Error = Refusal;
    fn try_from(wire: &StatementWire) -> std::result::Result<Self, Refusal> {
        Ok(match wire {
            StatementWire::Ticket { stamp } => Self::Ticket(Stamp::try_from(stamp)?),
            StatementWire::Grant { grant } => Self::Grant(grant.clone()),
            StatementWire::Commit { claim } => {
                if claim.len() > agentic_protocol::MAX_DOCUMENT_BYTES {
                    return Err(Refusal::TooLarge);
                }
                Self::Commit(claim.clone())
            }
        })
    }
}
impl From<&Stamp> for StampWire {
    fn from(stamp: &Stamp) -> Self {
        Self {
            book: stamp.book.to_vec(),
            index: stamp.index,
            operation: stamp.operation.to_vec(),
            signature: stamp.signature.to_vec(),
            holders: stamp.holders.as_ref().map(|list| {
                list.iter()
                    .map(|unit| serde_bytes::ByteBuf::from(unit.to_vec()))
                    .collect()
            }),
        }
    }
}
impl TryFrom<&StampWire> for Stamp {
    type Error = Refusal;
    fn try_from(wire: &StampWire) -> std::result::Result<Self, Refusal> {
        let malformed = |_| Refusal::Malformed;
        Ok(Self {
            book: wire.book.as_slice().try_into().map_err(malformed)?,
            index: wire.index,
            operation: wire.operation.as_slice().try_into().map_err(malformed)?,
            signature: wire.signature.as_slice().try_into().map_err(malformed)?,
            holders: match &wire.holders {
                None => None,
                Some(list) if list.len() > agentic_mailbox_swarm::select::SWARM_SIZE => {
                    return Err(Refusal::Malformed);
                }
                Some(list) => Some(
                    list.iter()
                        .map(|unit| unit.as_slice().try_into().map_err(malformed))
                        .collect::<std::result::Result<_, _>>()?,
                ),
            },
        })
    }
}
impl From<&Receipt> for ReceiptWire {
    fn from(receipt: &Receipt) -> Self {
        Self {
            mailbox: receipt.mailbox.to_vec(),
            operation: receipt.operation.to_vec(),
            ticket: receipt.ticket.to_vec(),
            holder: receipt.holder.to_vec(),
            stored_at: receipt.stored_at,
            signature: receipt.signature.to_vec(),
        }
    }
}
impl TryFrom<&ReceiptWire> for Receipt {
    type Error = Refusal;
    fn try_from(wire: &ReceiptWire) -> std::result::Result<Self, Refusal> {
        let malformed = |_| Refusal::Storage;
        Ok(Self {
            mailbox: wire.mailbox.as_slice().try_into().map_err(malformed)?,
            operation: wire.operation.as_slice().try_into().map_err(malformed)?,
            ticket: wire.ticket.as_slice().try_into().map_err(malformed)?,
            holder: wire.holder.as_slice().try_into().map_err(malformed)?,
            stored_at: wire.stored_at,
            signature: wire.signature.as_slice().try_into().map_err(malformed)?,
        })
    }
}

pub(super) fn behaviour(
    budget: processing::Budget,
    gate: processing::Gate,
) -> processing::Cbor<Request, Response> {
    processing::gated_cbor(
        budget,
        Some(gate),
        PROTOCOL,
        (MAX_ENVELOPE + 16 * 1024) as u64,
        (MAX_PAGE * (MAX_ENVELOPE + 1024)) as u64,
        request_response::Config::default()
            .with_request_timeout(Duration::from_secs(10))
            .with_max_concurrent_streams(16),
    )
}

impl Runtime {
    /// The grants of a page's granted books, each once, in the order their
    /// books first appear.
    fn page_grants(&self, page: &Page) -> Vec<agentic_grant_book::GrantBook> {
        let mut grants: Vec<agentic_grant_book::GrantBook> = Vec::new();
        for entry in &page.entries {
            if let Ok(Some(grant)) = self.mailbox_holder.granted(&entry.stamp.book)
                && !grants.iter().any(|kept| kept.id() == grant.id())
            {
                grants.push(grant);
            }
        }
        grants
    }
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "used once books, units and senders are wired (phases 1b-3)"
        )
    )]
    pub(super) fn mailbox_request(
        &mut self,
        peer: PeerId,
        request: Request,
    ) -> request_response::OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .mailbox
            .send_request(&peer, request)
    }

    pub(super) fn mailbox_message(&mut self, event: request_response::Event<Request, Response>) {
        match event {
            request_response::Event::Message { peer, message, .. } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    let response = self.serve_mailbox(peer, request);
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .mailbox
                        .send_response(channel, response);
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => self.mailbox_outcome(request_id, Ok(response)),
            },
            request_response::Event::OutboundFailure {
                request_id, error, ..
            } => self.mailbox_outcome(request_id, Err(error.to_string())),
            _ => {}
        }
    }

    pub(super) fn serve_mailbox(&mut self, peer: PeerId, request: Request) -> Response {
        let refused = |refusal: Refusal| Response::Refused {
            code: refusal.code().into(),
        };
        let kind = match &request {
            Request::Store { .. } => "store",
            Request::Read { .. } => "read",
            Request::Summaries { .. } => "summaries",
            Request::Notarize { .. } => "notarize",
            Request::Proofs { .. } => "proofs",
            Request::LearnGrant { .. } => "learnGrant",
            Request::Directory => "directory",
            Request::Access { .. } => "access",
        };
        *self.mailbox_client.served.entry(kind).or_default() += 1;
        if self.access_gate.enabled() && !matches!(request, Request::Access { .. }) {
            match self.access_gate.principal(&peer) {
                Some(processing::Principal::Unit) => {}
                Some(processing::Principal::Book(_))
                    if !matches!(request, Request::Summaries { .. }) => {}
                _ => return refused(Refusal::AccessRequired),
            }
        }
        match request {
            Request::Store {
                mailbox,
                period,
                envelope,
                stamp,
            } => {
                let Ok(mailbox) = <[u8; 32]>::try_from(mailbox.as_slice()) else {
                    return refused(Refusal::Malformed);
                };
                let stored = Stamp::try_from(&stamp).and_then(|stamp| {
                    let now = clock::wall().map_err(|_| Refusal::Storage)?;
                    let stored = self
                        .mailbox_holder
                        .store(mailbox, period, &envelope, &stamp, now);
                    if stored == Err(Refusal::UnknownBook) {
                        // Refused at once; the sender retries while the
                        // book is read.
                        self.book_wanted(stamp.book, false);
                    }
                    stored
                });
                match stored {
                    Ok(receipt) => Response::Stored {
                        receipt: ReceiptWire::from(&receipt),
                    },
                    Err(refusal) => refused(refusal),
                }
            }
            Request::Summaries { items } => self.serve_summaries(peer, &items),
            Request::Proofs { after } => match self.mailbox_holder.proofs(after, MAX_PROOFS) {
                Ok(page) => Response::Proofs {
                    proofs: ProofsWire::from(&page.proofs),
                    next: page.next,
                },
                Err(refusal) => refused(refusal),
            },
            Request::Notarize { statements } => {
                if statements.is_empty() || statements.len() > MAX_NOTARIZE {
                    return refused(Refusal::Malformed);
                }
                let Ok(now) = clock::wall() else {
                    return refused(Refusal::Storage);
                };
                *self
                    .mailbox_client
                    .served
                    .entry("notarizeStatements")
                    .or_default() += statements.len() as u64;
                let answers = statements
                    .iter()
                    .map(|wire| {
                        match Statement::try_from(wire).and_then(|statement| {
                            let noted = self.mailbox_holder.notarize(&statement, now);
                            if let (Err(Refusal::UnknownBook), Statement::Ticket(stamp)) =
                                (&noted, &statement)
                            {
                                self.book_wanted(stamp.book, false);
                            }
                            noted
                        }) {
                            Ok(noted) => AnswerWire::Noted {
                                first: StatementWire::from(&noted.first),
                                first_seen: noted.first_seen,
                            },
                            Err(refusal) => AnswerWire::Refused {
                                code: refusal.code().into(),
                            },
                        }
                    })
                    .collect();
                Response::Notarized { answers }
            }
            Request::LearnGrant { grant } => self.offer_grant(grant),
            Request::Directory => Response::Directory {
                records: self.directory_records(),
            },
            Request::Access { credential } => self.serve_access(peer, credential),
            Request::Read {
                mailbox,
                after,
                limit,
            } => {
                let Ok(mailbox) = <[u8; 32]>::try_from(mailbox.as_slice()) else {
                    return refused(Refusal::Malformed);
                };
                match self
                    .mailbox_holder
                    .read(&mailbox, after, usize::from(limit))
                {
                    Ok(page) => Response::Page {
                        grants: self.page_grants(&page),
                        entries: page
                            .entries
                            .iter()
                            .map(|e| EntryWire {
                                seq: e.seq,
                                period: e.period,
                                envelope: e.envelope.clone(),
                                stamp: StampWire::from(&e.stamp),
                                stored_at: e.stored_at,
                            })
                            .collect(),
                        next: page.next,
                    },
                    Err(refusal) => refused(refusal),
                }
            }
        }
    }
}

// --- operator payouts (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md) --------------

/// Tickets of paid stamps that name this unit: `payout/ticket/{slot}`.
const PAYOUT_TICKETS: &str = "payout/ticket/";
/// How many messages this holder stored, and how many were paid.
const PAYOUT_HELD: &str = "payout/held";
/// Tickets claimed so far and their prizes, however long ago.
const PAYOUT_CLAIMED: &str = "payout/claimed";
/// Won tickets ending within this are shown as ending.
const ENDING_WITHIN: u64 = 30 * PERIOD_SECONDS;

/// Where a ticket stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum TicketState {
    /// Waiting for the seed of its book's purchase day.
    Drawing,
    /// Won: the operator may claim it.
    Won,
    /// Claimed from the pool.
    Claimed,
    /// The pool refused it: its place was paid already, or another
    /// operation of its slot was.
    Refused,
}

/// A ticket as a holder keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HeldTicket {
    pub(super) claim: super::chain::TicketClaim,
    /// Its book's end: the book was bought its validity before.
    pub(super) valid_until: u64,
    pub(super) state: TicketState,
}

/// Messages stored, and those paid by bought books.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Held {
    pub(super) messages: u64,
    pub(super) paid: u64,
}

/// Tickets claimed and their prizes, in USDC units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claimed {
    pub(super) tickets: u64,
    pub(super) usdc: u128,
}

/// What a draw needs: how long books last (a book was bought that long
/// before it ends), how long tickets last from then, and the threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DrawTerms {
    pub(super) validity: u64,
    pub(super) lifetime: u64,
    pub(super) threshold: [u8; 32],
}

impl DrawTerms {
    /// The UTC day a book ending at `valid_until` was bought.
    pub(super) fn purchase_day(&self, valid_until: u64) -> u64 {
        valid_until.saturating_sub(self.validity) / PERIOD_SECONDS
    }
    /// When the tickets of that book end.
    pub(super) fn ends_at(&self, valid_until: u64) -> u64 {
        valid_until
            .saturating_sub(self.validity)
            .saturating_add(self.lifetime)
    }
}

/// Won tickets ending soon, and in how many days, rounded up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Ending {
    pub(super) tickets: u64,
    pub(super) in_days: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedPayoutTicket {
    book: String,
    index: u32,
    mailbox: String,
    period: u64,
    envelope: String,
    holders: Vec<String>,
    position: u8,
    signature: String,
    valid_until: u64,
    state: TicketState,
}

impl SavedPayoutTicket {
    fn held(&self) -> std::result::Result<HeldTicket, Refusal> {
        let mut holders = [[0; 32]; 10];
        if self.holders.len() != holders.len() {
            return Err(Refusal::Storage);
        }
        for (slot, unit) in holders.iter_mut().zip(&self.holders) {
            *slot = bytes32(unit)?;
        }
        Ok(HeldTicket {
            claim: super::chain::TicketClaim {
                book: bytes32(&self.book)?,
                index: self.index,
                mailbox: bytes32(&self.mailbox)?,
                period: self.period,
                envelope: bytes32(&self.envelope)?,
                holders,
                position: self.position,
                signature: hex::decode(&self.signature)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or(Refusal::Storage)?,
            },
            valid_until: self.valid_until,
            state: self.state,
        })
    }
}

fn payout_ticket_name(slot: &[u8; 32]) -> String {
    format!("{PAYOUT_TICKETS}{}", hex::encode(slot))
}

impl Service {
    /// The counters and, for a paid stamp naming this unit once, the ticket
    /// a newly stored message adds.
    fn payout_changes(
        &self,
        mailbox: &[u8; 32],
        period: u64,
        envelope: &[u8],
        stamp: &Stamp,
        terms: &BookTerms,
        unit: [u8; 32],
    ) -> std::result::Result<Vec<StateChange>, Refusal> {
        let paid = !self.grants.contains_key(&stamp.book);
        let (mut held, revision) = load::<Held>(&self.store, PAYOUT_HELD)?.unwrap_or_default();
        held.messages += 1;
        held.paid += u64::from(paid);
        let mut changes = vec![StateChange {
            namespace: PAYOUT_HELD.into(),
            expected_revision: revision,
            bytes: json(&held)?,
        }];
        let (Some(position), Some(list), true) = (stamp.place_of(&unit), &stamp.holders, paid)
        else {
            return Ok(changes);
        };
        let mut holders: Vec<String> = list.iter().map(hex::encode).collect();
        holders.resize(
            agentic_mailbox_swarm::select::SWARM_SIZE,
            hex::encode([0; 32]),
        );
        let slot = stamp.ticket_id(&self.domain);
        changes.push(StateChange {
            namespace: payout_ticket_name(&slot),
            expected_revision: 0,
            bytes: json(&SavedPayoutTicket {
                book: hex::encode(stamp.book),
                index: stamp.index,
                mailbox: hex::encode(mailbox),
                period,
                envelope: hex::encode(alloy_primitives::keccak256(envelope)),
                holders,
                position: u8::try_from(position).map_err(|_| Refusal::Malformed)?,
                signature: hex::encode(stamp.signature),
                valid_until: terms.valid_until,
                state: TicketState::Drawing,
            })?,
        });
        Ok(changes)
    }

    /// Every ticket this holder keeps, by slot.
    pub(super) fn tickets(&self) -> std::result::Result<Vec<HeldTicket>, Refusal> {
        self.saved_tickets()?
            .into_iter()
            .map(|(_, _, saved)| saved.held())
            .collect()
    }

    fn saved_tickets(&self) -> std::result::Result<Vec<(String, u64, SavedPayoutTicket)>, Refusal> {
        let through = format!("{PAYOUT_TICKETS}~");
        let mut after = PAYOUT_TICKETS.to_owned();
        let mut tickets = Vec::new();
        loop {
            let names = self
                .store
                .state_namespaces_between(&after, &through, 64)
                .map_err(|_| Refusal::Storage)?;
            let Some(last) = names.last().cloned() else {
                return Ok(tickets);
            };
            for name in names {
                let (saved, revision) =
                    load::<SavedPayoutTicket>(&self.store, &name)?.ok_or(Refusal::Storage)?;
                tickets.push((name, revision, saved));
            }
            after = last;
        }
    }

    /// Messages stored and those paid by bought books.
    pub(super) fn held(&self) -> Held {
        load::<Held>(&self.store, PAYOUT_HELD)
            .ok()
            .flatten()
            .map(|(held, _)| held)
            .unwrap_or_default()
    }

    /// Tickets claimed and their prizes so far.
    pub(super) fn claimed(&self) -> Claimed {
        load::<Claimed>(&self.store, PAYOUT_CLAIMED)
            .ok()
            .flatten()
            .map(|(claimed, _)| claimed)
            .unwrap_or_default()
    }

    /// Draws the tickets whose book's purchase day has a seed in `seeds`: a
    /// winner stays to be claimed, a loser is dropped.
    #[cfg(test)]
    pub(super) fn draw(
        &mut self,
        terms: &DrawTerms,
        seeds: &BTreeMap<u64, [u8; 32]>,
    ) -> std::result::Result<(), Refusal> {
        self.draw_books(terms, seeds, None)
    }

    /// `draw`, only for tickets of `books` when given: books the pool's
    /// shop is known to have sold.
    pub(super) fn draw_books(
        &mut self,
        terms: &DrawTerms,
        seeds: &BTreeMap<u64, [u8; 32]>,
        books: Option<&BTreeSet<[u8; 32]>>,
    ) -> std::result::Result<(), Refusal> {
        let mut won = Vec::new();
        let mut lost = Vec::new();
        for (name, revision, mut saved) in self.saved_tickets()? {
            if saved.state != TicketState::Drawing {
                continue;
            }
            if let Some(books) = books
                && !books.contains(&bytes32(&saved.book)?)
            {
                continue;
            }
            let Some(seed) = seeds.get(&terms.purchase_day(saved.valid_until)) else {
                continue;
            };
            let slot = agentic_mailbox_swarm::stamp::ticket_id(
                &self.domain,
                &bytes32(&saved.book)?,
                saved.index,
            );
            if agentic_mailbox_swarm::payout::wins(seed, &slot, &terms.threshold) {
                saved.state = TicketState::Won;
                won.push(StateChange {
                    namespace: name,
                    expected_revision: revision,
                    bytes: json(&saved)?,
                });
            } else {
                lost.push((name, revision));
            }
        }
        if !won.is_empty() || !lost.is_empty() {
            self.store
                .commit_state_maintenance(won, lost)
                .map_err(|_| Refusal::Storage)?;
        }
        Ok(())
    }

    /// Won tickets ending within 30 days of `now`, and in how many days the
    /// first ends, rounded up.
    pub(super) fn ending(&self, now: u64, terms: &DrawTerms) -> Option<Ending> {
        let mut ending: Option<Ending> = None;
        for ticket in self.tickets().ok()? {
            let ends = terms.ends_at(ticket.valid_until);
            if ticket.state != TicketState::Won || ends <= now || ends - now > ENDING_WITHIN {
                continue;
            }
            let in_days = (ends - now).div_ceil(PERIOD_SECONDS);
            let entry = ending.get_or_insert(Ending {
                tickets: 0,
                in_days,
            });
            entry.tickets += 1;
            entry.in_days = entry.in_days.min(in_days);
        }
        ending
    }

    /// Drops the drawing tickets of `book`: the pool's shop never sold it (a
    /// book of a shop the network used before).
    pub(super) fn forget_book_tickets(
        &mut self,
        book: &[u8; 32],
    ) -> std::result::Result<(), Refusal> {
        let book = hex::encode(book);
        let dropped: Vec<_> = self
            .saved_tickets()?
            .into_iter()
            .filter(|(_, _, saved)| saved.book == book && saved.state == TicketState::Drawing)
            .map(|(name, revision, _)| (name, revision))
            .collect();
        if !dropped.is_empty() {
            self.store
                .commit_state_maintenance(vec![], dropped)
                .map_err(|_| Refusal::Storage)?;
        }
        Ok(())
    }

    /// Drops the tickets that ended: the pool pays none of them any more.
    pub(super) fn forget_ended(
        &mut self,
        now: u64,
        terms: &DrawTerms,
    ) -> std::result::Result<(), Refusal> {
        let ended: Vec<_> = self
            .saved_tickets()?
            .into_iter()
            .filter(|(_, _, saved)| terms.ends_at(saved.valid_until) <= now)
            .map(|(name, revision, _)| (name, revision))
            .collect();
        if !ended.is_empty() {
            self.store
                .commit_state_maintenance(vec![], ended)
                .map_err(|_| Refusal::Storage)?;
        }
        Ok(())
    }

    /// Moves the tickets of `slots` to `state`; claimed ones add `prize`
    /// each to the claimed total.
    pub(super) fn settle_tickets(
        &mut self,
        slots: &[[u8; 32]],
        state: TicketState,
        prize: u128,
    ) -> std::result::Result<(), Refusal> {
        let mut changes = Vec::new();
        let mut count = 0u64;
        for slot in slots {
            let name = payout_ticket_name(slot);
            let Some((mut saved, revision)) = load::<SavedPayoutTicket>(&self.store, &name)? else {
                continue;
            };
            saved.state = state;
            count += 1;
            changes.push(StateChange {
                namespace: name,
                expected_revision: revision,
                bytes: json(&saved)?,
            });
        }
        if state == TicketState::Claimed && count > 0 {
            let (mut claimed, revision) =
                load::<Claimed>(&self.store, PAYOUT_CLAIMED)?.unwrap_or_default();
            claimed.tickets += count;
            claimed.usdc += prize * u128::from(count);
            changes.push(StateChange {
                namespace: PAYOUT_CLAIMED.into(),
                expected_revision: revision,
                bytes: json(&claimed)?,
            });
        }
        if !changes.is_empty() {
            self.store
                .commit_states(changes)
                .map_err(|_| Refusal::Storage)?;
        }
        Ok(())
    }

    /// The receipt key, for the transactions the node sends to the pool.
    pub(super) fn transaction_key(&self) -> HolderKey {
        self.key.clone()
    }
}

/// The slot a claimed ticket spends.
pub(super) fn claim_slot(domain: &[u8; 32], claim: &super::chain::TicketClaim) -> [u8; 32] {
    agentic_mailbox_swarm::stamp::ticket_id(domain, &claim.book, claim.index)
}
