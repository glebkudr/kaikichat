//! Self-contained evidence of misbehaviour. Nothing here trusts a complaint:
//! every proof verifies from its own signatures.

use crate::Account;
use crate::receipt::Receipt;
use crate::stamp::Stamp;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProofError {
    #[error("the two statements do not conflict")]
    NoConflict,
    #[error("a statement is not signed by the accused key")]
    Signer,
}

/// A book key signed two different operations for one slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SenderEquivocation {
    pub first: Stamp,
    pub second: Stamp,
}

impl SenderEquivocation {
    pub fn verify(&self, domain: &[u8; 32], book_key: &Account) -> Result<(), ProofError> {
        for stamp in [&self.first, &self.second] {
            if stamp.signer(domain).ok().as_ref() != Some(book_key) {
                return Err(ProofError::Signer);
            }
        }
        let (a, b) = (&self.first, &self.second);
        if a.book != b.book || a.index != b.index || a.operation == b.operation {
            return Err(ProofError::NoConflict);
        }
        Ok(())
    }
}

/// A holder receipted two different operations spending one slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HolderEquivocation {
    pub first: Receipt,
    pub second: Receipt,
}

impl HolderEquivocation {
    pub fn verify(&self, domain: &[u8; 32], holder_key: &Account) -> Result<(), ProofError> {
        for receipt in [&self.first, &self.second] {
            if receipt.signer(domain).ok().as_ref() != Some(holder_key) {
                return Err(ProofError::Signer);
            }
        }
        let (a, b) = (&self.first, &self.second);
        if a.ticket != b.ticket || a.operation == b.operation {
            return Err(ProofError::NoConflict);
        }
        Ok(())
    }
}
