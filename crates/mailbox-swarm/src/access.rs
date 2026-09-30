//! Access by book (Docs/V1_DISCOVERY_2026_09_27.md, part 1): once a UTC day
//! a node signs, with its book key, a pass naming its transport key; holders
//! serve free requests only to peers that showed a pass of an active book.

use crate::Account;
use crate::address::period;
use crate::digest::{digest, recover, sign};
use crate::stamp::{BookKey, BookTerms};

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum AccessError {
    #[error("malformed pass signature")]
    Signature,
    #[error("pass not signed by the book key")]
    Signer,
    #[error("book expired")]
    Expired,
    #[error("pass for another day")]
    Day,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPass {
    pub book: [u8; 32],
    /// The Ed25519 transport key of the peer the pass lets in.
    pub peer: [u8; 32],
    /// The UTC day (mailbox period) it is for.
    pub day: u64,
    /// `r || s || v` with `v` in {27, 28}.
    pub signature: [u8; 65],
}

impl AccessPass {
    /// The digest the book key signs.
    pub fn digest(domain: &[u8; 32], book: &[u8; 32], peer: &[u8; 32], day: u64) -> [u8; 32] {
        digest("AIN_ACCESS_V1", &[domain, book, peer, &day.to_be_bytes()])
    }
    pub fn sign(
        domain: &[u8; 32],
        book: [u8; 32],
        peer: [u8; 32],
        day: u64,
        key: &BookKey,
    ) -> Self {
        let signature = sign(&key.0, &Self::digest(domain, &book, &peer, day));
        Self {
            book,
            peer,
            day,
            signature,
        }
    }
    /// The account whose key produced this signature over these fields.
    pub fn signer(&self, domain: &[u8; 32]) -> Result<Account, AccessError> {
        recover(
            &Self::digest(domain, &self.book, &self.peer, self.day),
            &self.signature,
        )
        .ok_or(AccessError::Signature)
    }
    /// Signed by the book key, for a day next to `now`'s, before the book
    /// ends. A book with nothing left to spend still lets its owner in.
    pub fn verify(
        &self,
        domain: &[u8; 32],
        terms: &BookTerms,
        now: u64,
    ) -> Result<(), AccessError> {
        if self.signer(domain)? != terms.key {
            return Err(AccessError::Signer);
        }
        if now >= terms.valid_until {
            return Err(AccessError::Expired);
        }
        if self.day.abs_diff(period(now)) > 1 {
            return Err(AccessError::Day);
        }
        Ok(())
    }
}
