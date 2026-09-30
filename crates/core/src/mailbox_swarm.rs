//! Core side of the mailbox swarm (Docs/V1_STORAGE_REDESIGN_2026_09_24.md):
//! the rotating address of each conversation direction, and the profile's
//! stamp key with the verified books (bought or granted) it spends.
use super::*;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::access::AccessPass;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period, writable};
use agentic_mailbox_swarm::receipt::Receipt;
use agentic_mailbox_swarm::select::{Member, QUORUM, SWARM_SIZE, rendezvous};
use agentic_mailbox_swarm::stamp::{
    BookKey, Stamp, book_id, named_operation, operation, swarm_digest,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;

/// The profile's secp256k1 stamp signer.
const KEY: &str = "swarm/key";
const BOOKS: &str = "swarm/books/";
const STAMPS: &str = "swarm/stamp/";
const SENDS: &str = "swarm/send/";
const READS: &str = "swarm/read/";
/// The grant that funds a granted book, by book id.
const GRANTS: &str = "swarm/grant/";
/// Purchase requests not confirmed yet: `swarm/purchase/{book}`.
const PURCHASES: &str = "swarm/purchase/";
/// The grant claim in progress at the identity server.
const CLAIM: &str = "swarm/claim";
/// The registry's units, as the node last told them.
const UNITS: &str = "swarm/units";
/// The holders a mailbox's stamps name, fixed at its first stamp:
/// `swarm/named/{mailbox}`.
const NAMED: &str = "swarm/named/";

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredUnits {
    version: u8,
    units: Vec<String>,
}

/// The holders a mailbox's stamps name; `None`: nobody.
type Named = Option<Vec<[u8; 32]>>;

/// `None`: the mailbox was first stamped before the units were known, and
/// its stamps name nobody.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredNamed {
    version: u8,
    holders: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRead {
    version: u8,
    period: u64,
}
/// The event kind a Welcome is stored with.
const WELCOME_EVENT: &str = "welcome";
/// Messages that go in parts: `swarm/whole/{message id}`, their number of
/// parts. Each part is its own delivery, `{message id}:{index}`.
const WHOLES: &str = "swarm/whole/";

/// A delivery's message, and its part when it goes in parts.
fn delivery_id(id: &str) -> Result<(&str, Option<u16>), CoreError> {
    match id.split_once(':') {
        None => {
            parse_id(id)?;
            Ok((id, None))
        }
        Some((message, index)) => {
            parse_id(message)?;
            let index = index.parse().map_err(|_| CoreError::InvalidInput)?;
            Ok((message, Some(index)))
        }
    }
}
const MAX_SWARM_OUTBOX: usize = 64;

/// A message's swarm delivery: the period it is pinned to, what its receipts
/// must pay for, and the receipts it was stored with.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredSend {
    version: u8,
    period: u64,
    mailbox: String,
    operation: String,
    ticket: String,
    receipts: Option<Vec<StoredReceipt>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredReceipt {
    holder: String,
    stored_at: u64,
    signature: String,
}

impl From<&Receipt> for StoredReceipt {
    fn from(receipt: &Receipt) -> Self {
        Self {
            holder: hex::encode(receipt.holder),
            stored_at: receipt.stored_at,
            signature: hex::encode(receipt.signature),
        }
    }
}

/// Mailbox, operation and ticket a delivery's receipts must match.
type Pinned = ([u8; 32], [u8; 32], [u8; 32]);

impl StoredSend {
    fn pinned(&self) -> Result<Pinned, CoreError> {
        Ok((
            bytes32(&self.mailbox)?,
            bytes32(&self.operation)?,
            bytes32(&self.ticket)?,
        ))
    }
}

fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    serde_json::to_vec(value).map_err(|_| CoreError::InvalidState)
}

/// A verified book (purchase or grant) signed by the profile's book key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxBook {
    pub book: [u8; 32],
    pub count: u32,
    pub valid_until: u64,
    /// Slots already spent.
    pub used: u32,
}

/// What the profile shows holders to be let in
/// (Docs/V1_DISCOVERY_2026_09_27.md, part 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxAccess {
    pub pass: AccessPass,
    /// The grant of a granted book, for a holder that does not know it yet.
    pub grant: Option<agentic_grant_book::GrantBook>,
}

/// A book purchase asked for and not seen confirmed on the chain yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxPurchase {
    pub book: [u8; 32],
    /// The profile's book key the purchase names.
    pub key: Account,
    pub salt: [u8; 32],
    pub created_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredClaim {
    version: u8,
    request: agentic_grant_book::ClaimRequest,
    claim_id: Option<String>,
    login_url: Option<String>,
    expires_at: Option<u64>,
}

impl From<StoredClaim> for MailboxClaim {
    fn from(stored: StoredClaim) -> Self {
        Self {
            request: stored.request,
            claim_id: stored.claim_id,
            login_url: stored.login_url,
            expires_at: stored.expires_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredPurchase {
    version: u8,
    salt: String,
    created_at: u64,
}

/// A claim for a grant at the identity server: the signed request, and the
/// server's claim once it answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxClaim {
    pub request: agentic_grant_book::ClaimRequest,
    pub claim_id: Option<String>,
    pub login_url: Option<String>,
    pub expires_at: Option<u64>,
}

/// A pending own conversation message that travels through the swarm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmPending {
    pub message_id: String,
    pub conversation_id: String,
}

/// Whether the recipient's node checked the stamp of a message it took
/// directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectPayment {
    /// Checked as a holder checks it, or not asked for: the node reads no
    /// chain.
    Checked,
    /// Not checked: its book is unknown here and the chain did not answer.
    /// The message is shown with low trust.
    Unchecked,
}

/// What a sender stores at every holder of the swarm of `mailbox`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmDelivery {
    pub message_id: String,
    pub conversation_id: String,
    pub period: u64,
    pub mailbox: [u8; 32],
    pub envelope: Vec<u8>,
    pub stamp: Stamp,
}

/// A message's place in its sender's MLS order: a sender's messages are
/// taken in generation order within an epoch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvelopeOrder {
    pub epoch: u64,
    /// The sender's credential.
    pub sender: Vec<u8>,
    pub generation: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmReceived {
    pub message_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredKey {
    version: u8,
    secret: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredBook {
    version: u8,
    count: u32,
    valid_until: u64,
    used: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGrant {
    version: u8,
    grant: agentic_grant_book::GrantBook,
}

/// The book and slot an operation spent.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredSlot {
    version: u8,
    book: String,
    index: u32,
}

fn bytes32(hex_text: &str) -> Result<[u8; 32], CoreError> {
    hex::decode(hex_text)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(CoreError::InvalidState)
}

impl AppCore {
    /// The mailbox secret of one direction of a conversation, derived from its
    /// MLS exporter: incoming for this profile's own mailbox, else the peer's.
    pub(super) fn mailbox_context(
        &self,
        conversation_id: &str,
        incoming: bool,
    ) -> Result<agentic_crypto::mailbox::EpochMailbox, CoreError> {
        let (data, _) = self.data()?;
        let own = self.identity_for(&data)?;
        let contact = data
            .contacts
            .get(conversation_id)
            .ok_or(CoreError::UnknownConversation)?;
        let recipient = if incoming {
            own.network_id
        } else {
            network_id(&contact.root)
        };
        Ok(self
            .crypto(Some(parse_id(conversation_id)?))?
            .0
            .mailbox_secret(
                parse_id(conversation_id)?,
                self.domain,
                recipient.as_bytes(),
            )?)
    }

    /// A queued message's pin in the swarm: its state's name and revision,
    /// and whether a quorum of holders stored it.
    pub(super) fn swarm_pin(
        &self,
        message_id: &str,
    ) -> Result<Option<(String, u64, bool)>, CoreError> {
        Ok(self.stored_send(message_id)?.map(|(send, revision)| {
            (
                format!("{SENDS}{message_id}"),
                revision,
                send.receipts.is_some(),
            )
        }))
    }

    /// This profile's messages waiting for the swarm, oldest first, but
    /// Welcomes, which go directly.
    pub(super) fn swarm_queue(&self) -> Result<Vec<(String, String)>, CoreError> {
        Ok(self
            .store
            .pending_outbox_ids_excluding(WELCOME_EVENT, 1000)?)
    }

    fn stored_send(&self, message_id: &str) -> Result<Option<(StoredSend, u64)>, CoreError> {
        let Some(state) = self.store.state(&format!("{SENDS}{message_id}"))? else {
            return Ok(None);
        };
        let send: StoredSend =
            serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
        if send.version != 1 {
            return Err(CoreError::InvalidState);
        }
        Ok(Some((send, state.revision)))
    }

    /// Mark a queued message as going in `count` parts.
    pub(super) fn whole_parts(message_id: &str, count: u16) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{WHOLES}{message_id}"),
            expected_revision: 0,
            bytes: json(&count)?,
        })
    }

    pub(super) fn parts_of(&self, message_id: &str) -> Result<Option<u16>, CoreError> {
        Ok(self
            .state_of::<u16>(&format!("{WHOLES}{message_id}"))?
            .map(|(count, _)| count))
    }

    fn part_stored(&self, message_id: &str, index: u16) -> Result<bool, CoreError> {
        Ok(self
            .stored_send(&format!("{message_id}:{index}"))?
            .is_some_and(|(send, _)| send.receipts.is_some()))
    }

    fn require_conversation(&self, conversation_id: &str) -> Result<(), CoreError> {
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        if !data.contacts.contains_key(conversation_id) && !self.is_group(conversation_id)? {
            return Err(CoreError::UnknownConversation);
        }
        Ok(())
    }

    /// The swarm mailbox of a conversation direction (`incoming` = the
    /// mailbox this profile reads) for the period containing `at`.
    pub fn swarm_mailbox(
        &self,
        conversation_id: &str,
        incoming: bool,
        at: u64,
    ) -> Result<[u8; 32], CoreError> {
        self.require_conversation(conversation_id)?;
        if self.is_group(conversation_id)? {
            // One mailbox for every member, both ways.
            return Ok(self
                .group_secret(conversation_id, None)?
                .swarm_mailbox(&self.domain, period(at)));
        }
        let context = self.mailbox_context(conversation_id, incoming)?;
        Ok(context.secret.swarm_mailbox(&self.domain, period(at)))
    }

    fn state_of<T: DeserializeOwned>(&self, name: &str) -> Result<Option<(T, u64)>, CoreError> {
        self.store
            .state(name)?
            .map(|state| {
                serde_json::from_slice(&state.bytes)
                    .map(|value| (value, state.revision))
                    .map_err(|_| CoreError::InvalidState)
            })
            .transpose()
    }

    fn book_key(&self) -> Result<Option<BookKey>, CoreError> {
        let Some((key, _)) = self.state_of::<StoredKey>(KEY)? else {
            return Ok(None);
        };
        if key.version != 1 {
            return Err(CoreError::InvalidState);
        }
        BookKey::from_bytes(&bytes32(&key.secret)?)
            .map(Some)
            .ok_or(CoreError::InvalidState)
    }

    /// The profile's stamp signer, created on first use: every verified
    /// book of this profile, bought or granted, is signed by it.
    pub fn mailbox_book_account(&mut self) -> Result<Account, CoreError> {
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        if let Some(key) = self.book_key()? {
            return Ok(key.account());
        }
        let secret = loop {
            let candidate = random_id()?;
            if BookKey::from_bytes(&candidate).is_some() {
                break candidate;
            }
        };
        self.store.commit_states(vec![StateChange {
            namespace: KEY.into(),
            expected_revision: 0,
            bytes: json(&StoredKey {
                version: 1,
                secret: hex::encode(secret),
            })?,
        }])?;
        BookKey::from_bytes(&secret)
            .map(|key| key.account())
            .ok_or(CoreError::InvalidState)
    }

    /// Record a verified book signed by the profile's key. A book's terms
    /// never change; adding it again keeps its spent slots.
    pub fn add_mailbox_book(
        &mut self,
        book: [u8; 32],
        count: u32,
        valid_until: u64,
    ) -> Result<MailboxBook, CoreError> {
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        let name = format!("{BOOKS}{}", hex::encode(book));
        if let Some((stored, _)) = self.state_of::<StoredBook>(&name)? {
            if (stored.count, stored.valid_until) != (count, valid_until) {
                return Err(CoreError::InvalidInput);
            }
            return Ok(MailboxBook {
                book,
                count,
                valid_until,
                used: stored.used,
            });
        }
        self.store.commit_states(vec![StateChange {
            namespace: name,
            expected_revision: 0,
            bytes: json(&StoredBook {
                version: 1,
                count,
                valid_until,
                used: 0,
            })?,
        }])?;
        Ok(MailboxBook {
            book,
            count,
            valid_until,
            used: 0,
        })
    }

    /// Record a grant book of the identity server: a book of the profile's
    /// key whose id is the grant's. The grant is kept to show holders.
    pub fn add_mailbox_grant(
        &mut self,
        grant: &agentic_grant_book::GrantBook,
    ) -> Result<MailboxBook, CoreError> {
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        let key = self.book_key()?.ok_or(CoreError::InvalidInput)?;
        if grant.verify_signature().is_err()
            || grant.domain != self.domain
            || grant.book != key.account()
        {
            return Err(CoreError::InvalidInput);
        }
        let book = grant.id();
        let added = MailboxBook {
            book,
            count: grant.count,
            valid_until: grant.expiry,
            used: 0,
        };
        let name = format!("{BOOKS}{}", hex::encode(book));
        let grant_name = format!("{GRANTS}{}", hex::encode(book));
        if let Some((stored, _)) = self.state_of::<StoredBook>(&name)? {
            let kept = self.state_of::<StoredGrant>(&grant_name)?;
            if (stored.count, stored.valid_until) != (grant.count, grant.expiry)
                || kept.is_none_or(|(kept, _)| kept.grant != *grant)
            {
                return Err(CoreError::InvalidInput);
            }
            return Ok(MailboxBook {
                used: stored.used,
                ..added
            });
        }
        self.store.commit_states(vec![
            StateChange {
                namespace: name,
                expected_revision: 0,
                bytes: json(&StoredBook {
                    version: 1,
                    count: grant.count,
                    valid_until: grant.expiry,
                    used: 0,
                })?,
            },
            StateChange {
                namespace: grant_name,
                expected_revision: 0,
                bytes: json(&StoredGrant {
                    version: 1,
                    grant: grant.clone(),
                })?,
            },
        ])?;
        Ok(added)
    }

    /// Ask for a book of the profile's key under a fresh salt; it funds
    /// sending once the chain confirmed its purchase. While one request is
    /// unpaid, asking again gives it back.
    pub fn start_mailbox_purchase(&mut self, now: u64) -> Result<MailboxPurchase, CoreError> {
        let key = self.mailbox_book_account()?;
        if let Some(unpaid) = self.mailbox_purchases()?.into_iter().next() {
            return Ok(unpaid);
        }
        let salt = random_id()?;
        let book = book_id(&self.domain, &key, &salt);
        self.store.commit_states(vec![StateChange {
            namespace: format!("{PURCHASES}{}", hex::encode(book)),
            expected_revision: 0,
            bytes: json(&StoredPurchase {
                version: 1,
                salt: hex::encode(salt),
                created_at: now,
            })?,
        }])?;
        Ok(MailboxPurchase {
            book,
            key,
            salt,
            created_at: now,
        })
    }

    /// Purchases asked for and not confirmed yet, oldest first. They are
    /// kept until paid, however late.
    pub fn mailbox_purchases(&self) -> Result<Vec<MailboxPurchase>, CoreError> {
        let Some(key) = self.book_key()?.map(|key| key.account()) else {
            return Ok(Vec::new());
        };
        let through = format!("{PURCHASES}{}", "f".repeat(64));
        let mut after = PURCHASES.to_owned();
        let mut purchases = Vec::new();
        loop {
            let names = self.store.state_namespaces_between(&after, &through, 64)?;
            let Some(last) = names.last().cloned() else {
                break;
            };
            for name in &names {
                let (stored, _) = self
                    .state_of::<StoredPurchase>(name)?
                    .ok_or(CoreError::InvalidState)?;
                if stored.version != 1 {
                    return Err(CoreError::InvalidState);
                }
                purchases.push(MailboxPurchase {
                    book: bytes32(&name[PURCHASES.len()..])?,
                    key,
                    salt: bytes32(&stored.salt)?,
                    created_at: stored.created_at,
                });
            }
            after = last;
        }
        purchases.sort_by_key(|p| (p.created_at, p.book));
        Ok(purchases)
    }

    /// The chain confirmed a pending purchase of the profile's key: its
    /// book funds sending. Seeing it again returns the book.
    pub fn confirm_mailbox_purchase(
        &mut self,
        book: [u8; 32],
        key: Account,
        count: u32,
        valid_until: u64,
    ) -> Result<MailboxBook, CoreError> {
        if self.book_key()?.map(|own| own.account()) != Some(key) {
            return Err(CoreError::InvalidInput);
        }
        let purchase = format!("{PURCHASES}{}", hex::encode(book));
        let Some((_, revision)) = self.state_of::<StoredPurchase>(&purchase)? else {
            return self
                .mailbox_books()?
                .into_iter()
                .find(|b| (b.book, b.count, b.valid_until) == (book, count, valid_until))
                .ok_or(CoreError::InvalidInput);
        };
        let name = format!("{BOOKS}{}", hex::encode(book));
        if self.state_of::<StoredBook>(&name)?.is_some() {
            return Err(CoreError::InvalidState);
        }
        self.store.commit_state_maintenance(
            vec![StateChange {
                namespace: name,
                expected_revision: 0,
                bytes: json(&StoredBook {
                    version: 1,
                    count,
                    valid_until,
                    used: 0,
                })?,
            }],
            vec![(purchase, revision)],
        )?;
        Ok(MailboxBook {
            book,
            count,
            valid_until,
            used: 0,
        })
    }

    fn stored_claim(&self) -> Result<Option<(StoredClaim, u64)>, CoreError> {
        match self.state_of::<StoredClaim>(CLAIM)? {
            Some((stored, _)) if stored.version != 1 => Err(CoreError::InvalidState),
            stored => Ok(stored),
        }
    }

    /// The current claim, or a new request signed by the profile's book key.
    pub fn start_mailbox_claim(&mut self, now: u64) -> Result<MailboxClaim, CoreError> {
        if let Some((stored, _)) = self.stored_claim()? {
            return Ok(stored.into());
        }
        self.mailbox_book_account()?;
        let (key, _) = self
            .state_of::<StoredKey>(KEY)?
            .ok_or(CoreError::InvalidState)?;
        let signer = agentic_grant_book::SecpKey::from_secret(&bytes32(&key.secret)?)
            .ok_or(CoreError::InvalidState)?;
        let mut nonce = [0; 16];
        nonce.copy_from_slice(&random_id()?[..16]);
        let stored = StoredClaim {
            version: 1,
            request: agentic_grant_book::ClaimRequest::sign(self.domain, nonce, now, &signer),
            claim_id: None,
            login_url: None,
            expires_at: None,
        };
        self.store.commit_states(vec![StateChange {
            namespace: CLAIM.into(),
            expected_revision: 0,
            bytes: json(&stored)?,
        }])?;
        Ok(stored.into())
    }

    /// The server answered the current claim's request.
    pub fn open_mailbox_claim(
        &mut self,
        claim_id: &str,
        login_url: &str,
        expires_at: u64,
    ) -> Result<MailboxClaim, CoreError> {
        let (mut stored, revision) = self.stored_claim()?.ok_or(CoreError::InvalidInput)?;
        stored.claim_id = Some(claim_id.to_owned());
        stored.login_url = Some(login_url.to_owned());
        stored.expires_at = Some(expires_at);
        self.store.commit_states(vec![StateChange {
            namespace: CLAIM.into(),
            expected_revision: revision,
            bytes: json(&stored)?,
        }])?;
        Ok(stored.into())
    }

    /// The claim in progress, if any.
    pub fn mailbox_claim(&self) -> Result<Option<MailboxClaim>, CoreError> {
        Ok(self.stored_claim()?.map(|(stored, _)| stored.into()))
    }

    /// The claim ended: granted, denied or expired.
    pub fn finish_mailbox_claim(&mut self) -> Result<(), CoreError> {
        if let Some((_, revision)) = self.stored_claim()? {
            self.store
                .commit_state_maintenance(vec![], vec![(CLAIM.into(), revision)])?;
        }
        Ok(())
    }

    /// A pass for `peer` on `day`, signed by the book key, for the active
    /// book that lasts longest; `None` without an active book. A book with
    /// nothing left to spend still lets its owner in.
    pub fn mailbox_access(
        &self,
        peer: [u8; 32],
        day: u64,
        now: u64,
    ) -> Result<Option<MailboxAccess>, CoreError> {
        let Some(key) = self.book_key()? else {
            return Ok(None);
        };
        let Some(book) = self
            .mailbox_books()?
            .into_iter()
            .filter(|b| now < b.valid_until)
            .max_by_key(|b| (b.valid_until, b.book))
        else {
            return Ok(None);
        };
        Ok(Some(MailboxAccess {
            pass: AccessPass::sign(&self.domain, book.book, peer, day, &key),
            grant: self.mailbox_book_grant(&book.book)?,
        }))
    }

    /// The grant a book was funded by, if it was granted.
    pub fn mailbox_book_grant(
        &self,
        book: &[u8; 32],
    ) -> Result<Option<agentic_grant_book::GrantBook>, CoreError> {
        match self.state_of::<StoredGrant>(&format!("{GRANTS}{}", hex::encode(book)))? {
            Some((stored, _)) if stored.version != 1 => Err(CoreError::InvalidState),
            stored => Ok(stored.map(|(stored, _)| stored.grant)),
        }
    }

    /// The profile's books, the soonest ending first.
    pub fn mailbox_books(&self) -> Result<Vec<MailboxBook>, CoreError> {
        let through = format!("{BOOKS}{}", "f".repeat(64));
        let mut after = BOOKS.to_owned();
        let mut books = Vec::new();
        loop {
            let names = self.store.state_namespaces_between(&after, &through, 64)?;
            let Some(last) = names.last().cloned() else {
                break;
            };
            for name in &names {
                let (stored, _) = self
                    .state_of::<StoredBook>(name)?
                    .ok_or(CoreError::InvalidState)?;
                if stored.version != 1 {
                    return Err(CoreError::InvalidState);
                }
                books.push(MailboxBook {
                    book: bytes32(&name[BOOKS.len()..])?,
                    count: stored.count,
                    valid_until: stored.valid_until,
                    used: stored.used,
                });
            }
            after = last;
        }
        books.sort_by_key(|b| (b.valid_until, b.book));
        Ok(books)
    }

    /// The registry's units, as the node's directory lists them: stamps of
    /// mailboxes stamped from now on name their holders among these
    /// (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md). Kept across restarts.
    pub fn set_swarm_units(&mut self, units: Vec<[u8; 32]>) -> Result<(), CoreError> {
        let stored = StoredUnits {
            version: 1,
            units: units.iter().map(hex::encode).collect(),
        };
        let current = self.state_of::<StoredUnits>(UNITS)?;
        if current.as_ref().is_some_and(|(kept, _)| *kept == stored) {
            return Ok(());
        }
        self.store.commit_states(vec![StateChange {
            namespace: UNITS.into(),
            expected_revision: current.map_or(0, |(_, revision)| revision),
            bytes: json(&stored)?,
        }])?;
        Ok(())
    }

    /// The holders `mailbox`'s stamps name, and the change that fixes them
    /// when this is the mailbox's first stamp: its holders among the units
    /// known now, or nobody before any are known.
    fn named_holders(&self, mailbox: &[u8; 32]) -> Result<(Named, Option<StateChange>), CoreError> {
        let name = format!("{NAMED}{}", hex::encode(mailbox));
        if let Some((kept, _)) = self.state_of::<StoredNamed>(&name)? {
            let holders = kept
                .holders
                .map(|list| list.iter().map(|unit| bytes32(unit)).collect())
                .transpose()?;
            return Ok((holders, None));
        }
        let members = match self.state_of::<StoredUnits>(UNITS)? {
            Some((kept, _)) => kept
                .units
                .iter()
                .map(|unit| bytes32(unit).map(|commitment| Member { commitment }))
                .collect::<Result<Vec<_>, _>>()?,
            None => vec![],
        };
        let holders = (!members.is_empty()).then(|| {
            rendezvous(mailbox, &members, SWARM_SIZE)
                .into_iter()
                .map(|member| member.commitment)
                .collect::<Vec<_>>()
        });
        let pin = StateChange {
            namespace: name,
            expected_revision: 0,
            bytes: json(&StoredNamed {
                version: 1,
                holders: holders
                    .as_ref()
                    .map(|list| list.iter().map(hex::encode).collect()),
            })?,
        };
        Ok((holders, Some(pin)))
    }

    /// A stamp paying for `envelope` in `mailbox`. The same operation always
    /// gets the same slot; a new one durably takes the next free slot.
    pub fn stamp_mailbox(
        &mut self,
        mailbox: &[u8; 32],
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<Stamp, CoreError> {
        let (stamp, changes) = self.stamp_changes(mailbox, period, envelope, now)?;
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(stamp)
    }

    /// The stamp for an operation and the state that spends its slot, if
    /// new: the same operation keeps its book and slot; a new one takes the
    /// next slot of the valid book that ends soonest.
    pub(super) fn stamp_changes(
        &self,
        mailbox: &[u8; 32],
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<(Stamp, Vec<StateChange>), CoreError> {
        self.stamp_changes_as(mailbox, period, envelope, now, false)
    }

    /// `stamp_changes`, naming nobody when `unnamed`: a message pinned
    /// before its mailbox named holders keeps its first stamp.
    fn stamp_changes_as(
        &self,
        mailbox: &[u8; 32],
        period: u64,
        envelope: &[u8],
        now: u64,
        unnamed: bool,
    ) -> Result<(Stamp, Vec<StateChange>), CoreError> {
        let (holders, pin) = if unnamed {
            (None, None)
        } else {
            self.named_holders(mailbox)?
        };
        let operation = match &holders {
            Some(list) => named_operation(mailbox, period, &swarm_digest(list), envelope),
            None => operation(mailbox, period, envelope),
        };
        // A new slot only for a period holders still take fresh; a spent
        // operation is re-signed whatever the clock says.
        let (mut stamp, mut changes) = self.slot_changes(operation, writable(period, now), now)?;
        stamp.holders = holders;
        changes.extend(pin);
        Ok((stamp, changes))
    }

    /// A stamp paying for `operation` of the discovery service
    /// (spec/discovery-v1.md): the same slot for the same operation again,
    /// the next slot of an active book for a new one.
    pub fn stamp_operation(&mut self, operation: [u8; 32], now: u64) -> Result<Stamp, CoreError> {
        let (stamp, changes) = self.slot_changes(operation, true, now)?;
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(stamp)
    }

    /// A document for the discovery service, signed by the profile's root
    /// key.
    pub fn directory_document(&self, body: Vec<u8>, now: u64) -> Result<Vec<u8>, CoreError> {
        Ok(self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::Directory,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body,
                extensions: BTreeMap::new(),
            })?
            .to_wire())
    }

    /// The stamp for `operation` and the state that spends its slot, if
    /// new; a new slot is taken only when `fresh` allows.
    fn slot_changes(
        &self,
        operation: [u8; 32],
        fresh: bool,
        now: u64,
    ) -> Result<(Stamp, Vec<StateChange>), CoreError> {
        let key = self.book_key()?.ok_or(CoreError::MailboxBookMissing)?;
        let slot = format!("{STAMPS}{}", hex::encode(operation));
        if let Some((saved, _)) = self.state_of::<StoredSlot>(&slot)? {
            let book = bytes32(&saved.book)?;
            let (stored, _) = self
                .state_of::<StoredBook>(&format!("{BOOKS}{}", saved.book))?
                .ok_or(CoreError::InvalidState)?;
            if now >= stored.valid_until {
                return Err(CoreError::MailboxBookExpired);
            }
            return Ok((
                Stamp::sign(&self.domain, book, saved.index, operation, &key),
                vec![],
            ));
        }
        let books = self.mailbox_books()?;
        if books.is_empty() {
            return Err(CoreError::MailboxBookMissing);
        }
        let mut valid = books.into_iter().filter(|b| now < b.valid_until).peekable();
        if valid.peek().is_none() {
            return Err(CoreError::MailboxBookExpired);
        }
        if !fresh {
            return Err(CoreError::InvalidInput);
        }
        // The soonest ending of the books lasting another period, so a
        // message is not pinned to a slot its holders will refuse before
        // quorum; a book ending sooner only when nothing else is left.
        let book = valid
            .filter(|b| b.used < b.count)
            .min_by_key(|b| {
                (
                    b.valid_until < now.saturating_add(PERIOD_SECONDS),
                    b.valid_until,
                    b.book,
                )
            })
            .ok_or(CoreError::MailboxBookExhausted)?;
        let name = format!("{BOOKS}{}", hex::encode(book.book));
        let (stored, revision) = self
            .state_of::<StoredBook>(&name)?
            .ok_or(CoreError::InvalidState)?;
        let changes = vec![
            StateChange {
                namespace: slot,
                expected_revision: 0,
                bytes: json(&StoredSlot {
                    version: 1,
                    book: hex::encode(book.book),
                    index: book.used,
                })?,
            },
            StateChange {
                namespace: name,
                expected_revision: revision,
                bytes: json(&StoredBook {
                    used: stored.used + 1,
                    ..stored
                })?,
            },
        ];
        Ok((
            Stamp::sign(&self.domain, book.book, book.used, operation, &key),
            changes,
        ))
    }

    /// Own pending messages that travel through the swarm, oldest first.
    /// A Welcome of an invitation stays on direct delivery: its recipient
    /// cannot derive the conversation's mailbox before joining. A request
    /// made with an intro card goes to the recipient's intro mailbox.
    pub fn swarm_outbox(&self, limit: usize) -> Result<Vec<SwarmPending>, CoreError> {
        if !(1..=MAX_SWARM_OUTBOX).contains(&limit) {
            return Err(CoreError::InvalidInput);
        }
        // A message in parts is one delivery per part not yet stored.
        let mut pending = vec![];
        for (message_id, conversation_id) in self
            .store
            .pending_outbox_ids_excluding(WELCOME_EVENT, limit)?
        {
            let Some(count) = self.parts_of(&message_id)? else {
                pending.push(SwarmPending {
                    message_id,
                    conversation_id,
                });
                continue;
            };
            for index in 0..count {
                if !self.part_stored(&message_id, index)? {
                    pending.push(SwarmPending {
                        message_id: format!("{message_id}:{index}"),
                        conversation_id: conversation_id.clone(),
                    });
                }
            }
        }
        pending.truncate(limit);
        Ok(pending)
    }

    /// Seal and stamp a pending message for its mailbox. The first call pins
    /// the period; the message keeps it, and its stamp slot, while the
    /// recipient still reads that period, then moves to the current one.
    pub fn prepare_swarm_delivery(
        &mut self,
        message_id: &str,
        now: u64,
    ) -> Result<SwarmDelivery, CoreError> {
        let (whole_id, part) = delivery_id(message_id)?;
        let item = self
            .store
            .pending_outbox_item(whole_id)?
            .ok_or(CoreError::InvalidInput)?;
        let record = &item.message.record;
        let event: Event =
            serde_json::from_slice(&record.content).map_err(|_| CoreError::InvalidState)?;
        if !record.own || matches!(event, Event::Welcome) {
            return Err(CoreError::InvalidInput);
        }
        let conversation_id = record.conversation_id.clone();
        let send = self.stored_send(message_id)?;
        let current = period(now);
        let pinned = match &send {
            Some((send, _)) if current <= send.period + 1 => send.period,
            _ => current,
        };
        let wire = match part {
            None if self.parts_of(whole_id)?.is_none() => item.wire.clone(),
            None => return Err(CoreError::InvalidInput),
            Some(index) => {
                // Parts are signed again as they were: the same bytes, the
                // same slot on every retry.
                let issued_at =
                    VerifiedDocument::decode_large(&item.wire, self.domain, now)?.issued_at();
                self.store
                    .split_document(&item.wire, self.domain, issued_at)?
                    .get(usize::from(index))
                    .filter(|_| self.parts_of(whole_id).ok().flatten() > Some(index))
                    .ok_or(CoreError::InvalidInput)?
                    .to_wire()
            }
        };
        let (mailbox, envelope) = if matches!(event, Event::Knock) {
            // An application, sealed to the door it goes to.
            self.knock_delivery(whole_id, pinned, &wire)?
        } else if let Event::Tree { epoch } = event {
            // A group's tree, in the mailbox of its own that newcomers read.
            let group = self.group_secret(&conversation_id, Some(epoch))?;
            let secret = super::group_parts::tree_secret(&group.expose(), epoch);
            (
                secret.swarm_mailbox(&self.domain, pinned),
                secret.seal_swarm_envelope(&self.domain, pinned, &wire)?,
            )
        } else if matches!(event, Event::Request | Event::Invite) {
            self.intro_delivery(message_id, pinned, &item.wire)?
        } else if self.is_public_send(message_id)? {
            // A post of an open group, in the clear; of a closed channel,
            // sealed under its key.
            self.public_entry_target(
                &conversation_id,
                [&[PUBLIC_POST][..], &item.wire].concat(),
                pinned,
            )?
        } else if self.is_group(&conversation_id)? {
            // The mailbox of the epoch it was written in.
            let epoch = self.group_send_epoch(whole_id)?;
            let secret = self.group_secret(&conversation_id, epoch)?;
            (
                secret.swarm_mailbox(&self.domain, pinned),
                secret.seal_swarm_envelope(&self.domain, pinned, &wire)?,
            )
        } else {
            let context = self.mailbox_context(&conversation_id, false)?;
            (
                context.secret.swarm_mailbox(&self.domain, pinned),
                context
                    .secret
                    .seal_swarm_envelope(&self.domain, pinned, &item.wire)?,
            )
        };
        // A message pinned with a stamp that names nobody keeps it.
        let unnamed = send.as_ref().is_some_and(|(send, _)| {
            send.period == pinned
                && send.operation == hex::encode(operation(&mailbox, pinned, &envelope))
        });
        let (stamp, mut changes) =
            self.stamp_changes_as(&mailbox, pinned, &envelope, now, unnamed)?;
        if send.as_ref().is_none_or(|(send, _)| send.period != pinned) {
            changes.push(StateChange {
                namespace: format!("{SENDS}{message_id}"),
                expected_revision: send.as_ref().map_or(0, |(_, revision)| *revision),
                bytes: json(&StoredSend {
                    version: 1,
                    period: pinned,
                    mailbox: hex::encode(mailbox),
                    operation: hex::encode(stamp.operation),
                    ticket: hex::encode(stamp.ticket_id(&self.domain)),
                    receipts: None,
                })?,
            });
        }
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(SwarmDelivery {
            message_id: message_id.into(),
            conversation_id,
            period: pinned,
            mailbox,
            envelope,
            stamp,
        })
    }

    /// Record a quorum of receipts from `swarm`, the holders the node
    /// selected, for the pinned delivery and acknowledge the message.
    pub fn complete_swarm_delivery(
        &mut self,
        message_id: &str,
        swarm: &[Account],
        receipts: &[Receipt],
    ) -> Result<(), CoreError> {
        let (whole_id, part) = delivery_id(message_id)?;
        let (mut send, revision) = self
            .stored_send(message_id)?
            .ok_or(CoreError::InvalidInput)?;
        if send.receipts.is_none() {
            let (mailbox, operation, ticket) = send.pinned()?;
            let mut signers = BTreeSet::new();
            for receipt in receipts {
                let signer = receipt
                    .signer(&self.domain)
                    .map_err(|_| CoreError::InvalidInput)?;
                if (receipt.mailbox, receipt.operation, receipt.ticket)
                    != (mailbox, operation, ticket)
                    || !swarm.contains(&signer)
                    || !signers.insert(signer)
                {
                    return Err(CoreError::InvalidInput);
                }
            }
            if signers.len() < QUORUM {
                return Err(CoreError::InvalidInput);
            }
            send.receipts = Some(receipts.iter().map(StoredReceipt::from).collect());
            self.store.commit_states(vec![StateChange {
                namespace: format!("{SENDS}{message_id}"),
                expected_revision: revision,
                bytes: json(&send)?,
            }])?;
        }
        let Some(_) = part else {
            // Repeats a lost acknowledgment; a no-op once acknowledged.
            self.store.acknowledge_outbox(message_id)?;
            self.forget_notice_body(message_id)?;
            return Ok(());
        };
        // A message in parts is stored when every part is; its first part's
        // receipts stand for it.
        let count = self.parts_of(whole_id)?.ok_or(CoreError::InvalidState)?;
        for index in 0..count {
            if !self.part_stored(whole_id, index)? {
                return Ok(());
            }
        }
        if self.stored_send(whole_id)?.is_none() {
            let (first, _) = self
                .stored_send(&format!("{whole_id}:0"))?
                .ok_or(CoreError::InvalidState)?;
            self.store.commit_states(vec![StateChange {
                namespace: format!("{SENDS}{whole_id}"),
                expected_revision: 0,
                bytes: json(&first)?,
            }])?;
        }
        self.store.acknowledge_outbox(whole_id)?;
        Ok(())
    }

    /// The receipts a delivery was stored with.
    pub fn swarm_receipts(&self, message_id: &str) -> Result<Option<Vec<Receipt>>, CoreError> {
        parse_id(message_id)?;
        let Some((send, _)) = self.stored_send(message_id)? else {
            return Ok(None);
        };
        let Some(stored) = &send.receipts else {
            return Ok(None);
        };
        let (mailbox, operation, ticket) = send.pinned()?;
        stored
            .iter()
            .map(|r| {
                Ok(Receipt {
                    mailbox,
                    operation,
                    ticket,
                    holder: bytes32(&r.holder)?,
                    stored_at: r.stored_at,
                    signature: hex::decode(&r.signature)
                        .ok()
                        .and_then(|bytes| bytes.try_into().ok())
                        .ok_or(CoreError::InvalidState)?,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()
            .map(Some)
    }

    /// Import one envelope read from this profile's incoming mailbox of a
    /// conversation. Holder receipts acknowledge swarm deliveries, so no MLS
    /// receipt is queued.
    pub fn receive_swarm_envelope(
        &mut self,
        conversation_id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<SwarmReceived, CoreError> {
        self.require_conversation(conversation_id)?;
        if self.is_group(conversation_id)? {
            return self.receive_group_envelope(conversation_id, period, envelope, now);
        }
        let group = parse_id(conversation_id)?;
        let context = self.mailbox_context(conversation_id, true)?;
        let wire = context
            .secret
            .open_swarm_envelope(&self.domain, period, envelope)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        match Packet::decode(verified.body(), verified.kind())? {
            Packet::Application { group: g, .. } if g == group => {}
            _ => return Err(CoreError::InvalidInput),
        }
        let message_id = hex::encode(verified.id());
        self.receive_verified(verified, vec![], now)?;
        Ok(SwarmReceived { message_id })
    }

    /// A direct delivery paid like a swarm store: the sealed envelope of the
    /// conversation's incoming mailbox for `period`, from its author's node.
    /// A new message taken `Unchecked` is kept with low trust.
    #[allow(clippy::too_many_arguments)]
    pub fn receive_stamped_from(
        &mut self,
        conversation_id: &str,
        period: u64,
        envelope: &[u8],
        node_record: &[u8],
        actual_peer: &str,
        payment: DirectPayment,
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        let record = self.verify_node_record(node_record, actual_peer, now)?;
        self.require_conversation(conversation_id)?;
        let group = parse_id(conversation_id)?;
        let context = self.mailbox_context(conversation_id, true)?;
        let wire = context
            .secret
            .open_swarm_envelope(&self.domain, period, envelope)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        if verified.author() != &record.author {
            return Err(CoreError::Unauthorized);
        }
        match Packet::decode(verified.body(), verified.kind())? {
            Packet::Application { group: g, .. } if g == group => {}
            _ => return Err(CoreError::InvalidInput),
        }
        let outcome = self.receive_verified_with_states(
            verified,
            record.addresses.clone(),
            now,
            vec![],
            payment == DirectPayment::Unchecked,
        )?;
        self.remember_verified_node_record(record, now)?;
        Ok(outcome)
    }

    /// Where an envelope of this conversation's incoming mailbox falls in its
    /// sender's MLS order, read without consuming the ratchet, so a reader
    /// can hold envelopes that arrived early in order.
    pub fn swarm_envelope_order(
        &self,
        conversation_id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<EnvelopeOrder, CoreError> {
        self.require_conversation(conversation_id)?;
        if self.is_group(conversation_id)? {
            return self.group_envelope_order(conversation_id, period, envelope, now);
        }
        let group = parse_id(conversation_id)?;
        let context = self.mailbox_context(conversation_id, true)?;
        let wire = context
            .secret
            .open_swarm_envelope(&self.domain, period, envelope)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        let Packet::Application { group: g, message } =
            Packet::decode(verified.body(), verified.kind())?
        else {
            return Err(CoreError::InvalidInput);
        };
        if g != group {
            return Err(CoreError::InvalidInput);
        }
        let (crypto, _) = self.crypto(Some(group))?;
        let aad = application_aad_for(self.domain, group)?;
        let metadata = crypto.inspect_application_message(group, &message, &aad)?;
        Ok(EnvelopeOrder {
            epoch: metadata.epoch,
            sender: metadata.sender,
            generation: metadata.generation,
        })
    }

    fn stored_read(&self, conversation_id: &str) -> Result<Option<(StoredRead, u64)>, CoreError> {
        let Some(state) = self.store.state(&format!("{READS}{conversation_id}"))? else {
            return Ok(None);
        };
        let read: StoredRead =
            serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
        if read.version != 1 {
            return Err(CoreError::InvalidState);
        }
        Ok(Some((read, state.revision)))
    }

    /// The last period of this conversation's incoming mailboxes that was
    /// read completely.
    pub fn swarm_read_through(&self, conversation_id: &str) -> Result<Option<u64>, CoreError> {
        self.require_conversation(conversation_id)?;
        Ok(self
            .stored_read(conversation_id)?
            .map(|(read, _)| read.period))
    }

    /// Record that every period up to `period` was read; it never moves back.
    /// A group's reading moved past a period no longer kept leaves the group
    /// behind (`stale_groups`).
    pub fn set_swarm_read_through(
        &mut self,
        conversation_id: &str,
        period: u64,
        now: u64,
    ) -> Result<(), CoreError> {
        self.require_conversation(conversation_id)?;
        let stored = self.stored_read(conversation_id)?;
        if stored
            .as_ref()
            .is_some_and(|(read, _)| read.period >= period)
        {
            return Ok(());
        }
        if self.is_group(conversation_id)? {
            let read = stored.as_ref().map(|(read, _)| read.period);
            self.note_group_reading(conversation_id, read, period, now)?;
        }
        self.store.commit_states(vec![StateChange {
            namespace: format!("{READS}{conversation_id}"),
            expected_revision: stored.map_or(0, |(_, revision)| revision),
            bytes: json(&StoredRead { version: 1, period })?,
        }])?;
        Ok(())
    }

    /// When the conversation began: the issue time of its first message,
    /// the Welcome. No message of it can be older.
    pub fn conversation_started_at(&self, conversation_id: &str) -> Result<u64, CoreError> {
        self.require_conversation(conversation_id)?;
        self.store
            .messages(conversation_id, 0, 1)?
            .first()
            .map(|first| first.record.created_at)
            .ok_or(CoreError::InvalidState)
    }
}
