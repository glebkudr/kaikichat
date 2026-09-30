//! Who a registry unit is: its on-chain commitment binds the node's Ed25519
//! transport key and its secp256k1 receipt account, and a record signed by
//! the transport key tells where to reach it.

use crate::Account;
use crate::digest::digest;

/// Addresses one record may list.
pub const MAX_ADDRESSES: usize = 8;
/// Bytes of one address.
pub const MAX_ADDRESS_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum RecordError {
    #[error("record not signed by its transport key")]
    Signature,
    #[error("too many addresses")]
    TooManyAddresses,
    #[error("address too long")]
    AddressTooLong,
}

/// `keccak256(keccak256("AIN_UNIT_V1") ‖ domain ‖ transport key ‖ receipt)`:
/// what an operator bonds in `NodeRegistry`.
pub fn unit_commitment(domain: &[u8; 32], transport_key: &[u8; 32], receipt: &Account) -> [u8; 32] {
    digest("AIN_UNIT_V1", &[domain, transport_key, receipt])
}

/// Where a unit is reached, signed by its transport key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRecord {
    pub transport_key: [u8; 32],
    pub receipt: Account,
    pub addresses: Vec<String>,
    pub issued_at: u64,
    pub signature: [u8; 64],
}

impl UnitRecord {
    /// A record signed by `sign`, the transport key's Ed25519 signer.
    pub fn sign(
        domain: &[u8; 32],
        transport_key: [u8; 32],
        receipt: Account,
        addresses: Vec<String>,
        issued_at: u64,
        sign: impl FnOnce(&[u8; 32]) -> [u8; 64],
    ) -> Self {
        let mut record = Self {
            transport_key,
            receipt,
            addresses,
            issued_at,
            signature: [0; 64],
        };
        record.signature = sign(&record.digest(domain));
        record
    }

    /// What the transport key signs: the fields, and each address behind
    /// its length so no two lists sign alike.
    pub fn digest(&self, domain: &[u8; 32]) -> [u8; 32] {
        let mut addresses = Vec::new();
        addresses.extend_from_slice(&(self.addresses.len() as u64).to_be_bytes());
        for address in &self.addresses {
            addresses.extend_from_slice(&(address.len() as u64).to_be_bytes());
            addresses.extend_from_slice(address.as_bytes());
        }
        digest(
            "AIN_UNIT_RECORD_V1",
            &[
                domain,
                &self.transport_key,
                &self.receipt,
                &self.issued_at.to_be_bytes(),
                &addresses,
            ],
        )
    }

    /// The unit this record is for, if it is well-formed and signed by its
    /// transport key.
    pub fn verify(&self, domain: &[u8; 32]) -> Result<[u8; 32], RecordError> {
        if self.addresses.len() > MAX_ADDRESSES {
            return Err(RecordError::TooManyAddresses);
        }
        if self.addresses.iter().any(|a| a.len() > MAX_ADDRESS_BYTES) {
            return Err(RecordError::AddressTooLong);
        }
        let key = ed25519_dalek::VerifyingKey::from_bytes(&self.transport_key)
            .map_err(|_| RecordError::Signature)?;
        key.verify_strict(
            &self.digest(domain),
            &ed25519_dalek::Signature::from_bytes(&self.signature),
        )
        .map_err(|_| RecordError::Signature)?;
        Ok(unit_commitment(domain, &self.transport_key, &self.receipt))
    }
}
