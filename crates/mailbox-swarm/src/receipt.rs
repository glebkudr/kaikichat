//! What a holder signs when it stores one message of a mailbox.

use crate::Account;
use crate::digest::{account, digest, recover, sign, signing_key};
use crate::stamp::StampError;

/// A holder's secp256k1 receipt key, bound to its registry unit. Its
/// account also sends the node's transactions to the operator pool.
#[derive(Clone)]
pub struct HolderKey(pub(crate) k256::ecdsa::SigningKey);

impl HolderKey {
    pub fn from_bytes(secret: &[u8; 32]) -> Option<Self> {
        signing_key(secret).map(Self)
    }
    pub fn account(&self) -> Account {
        account(self.0.verifying_key())
    }
    /// `r || s || v` (low s, v 27/28) over a transaction's signing hash:
    /// keccak256 of its typed payload, never a tagged digest.
    pub fn sign_transaction(&self, hash: &[u8; 32]) -> [u8; 65] {
        sign(&self.0, hash)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub mailbox: [u8; 32],
    pub operation: [u8; 32],
    pub ticket: [u8; 32],
    /// The holder's registry unit commitment.
    pub holder: [u8; 32],
    pub stored_at: u64,
    pub signature: [u8; 65],
}

fn receipt_digest(domain: &[u8; 32], r: &Receipt) -> [u8; 32] {
    digest(
        "AIN_RECEIPT_V1",
        &[
            domain,
            &r.mailbox,
            &r.operation,
            &r.ticket,
            &r.holder,
            &r.stored_at.to_be_bytes(),
        ],
    )
}

impl Receipt {
    /// The digest the holder key signs; public so a contract can match it.
    pub fn digest(&self, domain: &[u8; 32]) -> [u8; 32] {
        receipt_digest(domain, self)
    }
    pub fn sign(
        domain: &[u8; 32],
        mailbox: [u8; 32],
        operation: [u8; 32],
        ticket: [u8; 32],
        holder: [u8; 32],
        stored_at: u64,
        key: &HolderKey,
    ) -> Self {
        let mut receipt = Self {
            mailbox,
            operation,
            ticket,
            holder,
            stored_at,
            signature: [0; 65],
        };
        receipt.signature = sign(&key.0, &receipt_digest(domain, &receipt));
        receipt
    }
    pub fn signer(&self, domain: &[u8; 32]) -> Result<Account, StampError> {
        recover(&receipt_digest(domain, self), &self.signature).ok_or(StampError::Signature)
    }
}
