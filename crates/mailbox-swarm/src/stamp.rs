//! A stamp pays for one message: the book key signs (book, index, operation).
//! A stamp of a mailbox message names the holders it pays
//! (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md): its operation commits to their
//! list, which travels with the stamp.

use crate::Account;
use crate::digest::{account, digest, recover, sign, signing_key};
use crate::select::SWARM_SIZE;
use alloy_primitives::keccak256;

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum StampError {
    #[error("malformed stamp signature")]
    Signature,
    #[error("stamp not signed by the book key")]
    Signer,
    #[error("ticket index outside the book")]
    Index,
    #[error("book expired")]
    Expired,
}

/// A secp256k1 key held by a book's buyer (or a grant provider).
pub struct BookKey(pub(crate) k256::ecdsa::SigningKey);

impl BookKey {
    /// Deterministic key from 32 secret bytes; `None` for an invalid scalar.
    pub fn from_bytes(secret: &[u8; 32]) -> Option<Self> {
        signing_key(secret).map(Self)
    }
    pub fn account(&self) -> Account {
        account(self.0.verifying_key())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub book: [u8; 32],
    pub index: u32,
    pub operation: [u8; 32],
    /// `r || s || v` with `v` in {27, 28}.
    pub signature: [u8; 65],
    /// The units a mailbox stamp names, best first; `None` for a stamp that
    /// names nobody (the first operation form, and discovery's).
    pub holders: Option<Vec<[u8; 32]>>,
}

/// The terms a holder learned once from the book's verified purchase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookTerms {
    pub key: Account,
    pub count: u32,
    pub valid_until: u64,
}

/// What a stamp pays for: delivering these exact envelope bytes to this
/// mailbox. The same bytes in another mailbox are another operation.
pub fn operation(mailbox: &[u8; 32], period: u64, envelope: &[u8]) -> [u8; 32] {
    digest(
        "AIN_OPERATION_V1",
        &[mailbox, &period.to_be_bytes(), envelope],
    )
}

/// The digest of the holders a stamp names, as `OperatorPool` reads it: the
/// list padded with empty units to a swarm's size.
pub fn swarm_digest(holders: &[[u8; 32]]) -> [u8; 32] {
    let mut list = [[0; 32]; SWARM_SIZE];
    for (place, unit) in list.iter_mut().zip(holders) {
        *place = *unit;
    }
    let fields: Vec<&[u8]> = list.iter().map(|unit| unit.as_slice()).collect();
    digest("AIN_SWARM_V1", &fields)
}

/// What a stamp naming its holders pays for: these envelope bytes, to this
/// mailbox, held by the units of `swarm` (`swarm_digest`). The envelope
/// enters as its hash, so a contract checks the stamp without the bytes.
pub fn named_operation(
    mailbox: &[u8; 32],
    period: u64,
    swarm: &[u8; 32],
    envelope: &[u8],
) -> [u8; 32] {
    named_operation_of(mailbox, period, swarm, &keccak256(envelope).0)
}

/// `named_operation` from the envelope's keccak256.
pub fn named_operation_of(
    mailbox: &[u8; 32],
    period: u64,
    swarm: &[u8; 32],
    envelope_hash: &[u8; 32],
) -> [u8; 32] {
    digest(
        "AIN_OPERATION_V2",
        &[mailbox, &period.to_be_bytes(), swarm, envelope_hash],
    )
}

/// A book's on-chain commitment: it binds the book key's account and a
/// salt, so a purchase reveals neither until a stamp is spent.
pub fn book_id(domain: &[u8; 32], account: &Account, salt: &[u8; 32]) -> [u8; 32] {
    digest("AIN_BOOK_V1", &[domain, account, salt])
}

/// The slot a stamp spends; the same for every operation.
pub fn ticket_id(domain: &[u8; 32], book: &[u8; 32], index: u32) -> [u8; 32] {
    digest("AIN_TICKET_V1", &[domain, book, &index.to_be_bytes()])
}

impl Stamp {
    /// The digest the book key signs; public so a contract can match it.
    pub fn digest(
        domain: &[u8; 32],
        book: &[u8; 32],
        index: u32,
        operation: &[u8; 32],
    ) -> [u8; 32] {
        digest(
            "AIN_STAMP_V1",
            &[domain, book, &index.to_be_bytes(), operation],
        )
    }
    pub fn sign(
        domain: &[u8; 32],
        book: [u8; 32],
        index: u32,
        operation: [u8; 32],
        key: &BookKey,
    ) -> Self {
        let signature = sign(&key.0, &Self::digest(domain, &book, index, &operation));
        Self {
            book,
            index,
            operation,
            signature,
            holders: None,
        }
    }
    /// Whether this stamp pays for `envelope` in `mailbox` and `period`: its
    /// operation is the named one of its list, or, naming nobody, the first
    /// form's.
    pub fn pays_for(&self, mailbox: &[u8; 32], period: u64, envelope: &[u8]) -> bool {
        match &self.holders {
            None => operation(mailbox, period, envelope) == self.operation,
            Some(list) => {
                list.len() <= SWARM_SIZE
                    && named_operation(mailbox, period, &swarm_digest(list), envelope)
                        == self.operation
            }
        }
    }
    /// Where the stamp names `unit`, if it names it exactly once.
    pub fn place_of(&self, unit: &[u8; 32]) -> Option<usize> {
        let mut places = self
            .holders
            .as_ref()?
            .iter()
            .enumerate()
            .filter(|(_, named)| *named == unit)
            .map(|(place, _)| place);
        let place = places.next()?;
        places.next().is_none().then_some(place)
    }
    /// The account whose key produced this signature over these fields.
    pub fn signer(&self, domain: &[u8; 32]) -> Result<Account, StampError> {
        recover(
            &Self::digest(domain, &self.book, self.index, &self.operation),
            &self.signature,
        )
        .ok_or(StampError::Signature)
    }
    pub fn ticket_id(&self, domain: &[u8; 32]) -> [u8; 32] {
        ticket_id(domain, &self.book, self.index)
    }
    /// Signed by the book key, inside the book, before it expires at `now`.
    pub fn verify(&self, domain: &[u8; 32], terms: &BookTerms, now: u64) -> Result<(), StampError> {
        if self.signer(domain)? != terms.key {
            return Err(StampError::Signer);
        }
        if self.index >= terms.count {
            return Err(StampError::Index);
        }
        if now >= terms.valid_until {
            return Err(StampError::Expired);
        }
        Ok(())
    }
}
