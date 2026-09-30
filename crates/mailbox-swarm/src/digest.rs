//! keccak256 over a domain tag and fixed-width big-endian fields, and the
//! secp256k1 recoverable signatures EVM `ecrecover` accepts (low s, v 27/28).

use crate::Account;
use alloy_primitives::keccak256;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};

pub(crate) fn digest(tag: &str, fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = keccak256(tag.as_bytes()).to_vec();
    for field in fields {
        bytes.extend_from_slice(field);
    }
    keccak256(bytes).0
}

pub(crate) fn account(key: &VerifyingKey) -> Account {
    let point = key.to_encoded_point(false);
    let mut account = [0; 20];
    account.copy_from_slice(&keccak256(&point.as_bytes()[1..])[12..]);
    account
}

pub(crate) fn signing_key(secret: &[u8; 32]) -> Option<SigningKey> {
    SigningKey::from_bytes(secret.into()).ok()
}

pub(crate) fn sign(key: &SigningKey, digest: &[u8; 32]) -> [u8; 65] {
    let mut out = [0; 65];
    // A 32-byte prehash is always accepted; RFC 6979 signing cannot fail.
    if let Ok((signature, recovery)) = key.sign_prehash_recoverable(digest) {
        let (signature, recovery) = match signature.normalize_s() {
            Some(low) => (
                low,
                RecoveryId::from_byte(recovery.to_byte() ^ 1).unwrap_or(recovery),
            ),
            None => (signature, recovery),
        };
        out[..64].copy_from_slice(&signature.to_bytes());
        out[64] = 27 + recovery.to_byte();
    }
    out
}

/// The signer of `digest`, or `None` for a malformed or high-s signature.
pub(crate) fn recover(digest: &[u8; 32], signature: &[u8; 65]) -> Option<Account> {
    let recovery = RecoveryId::from_byte(signature[64].checked_sub(27)?)?;
    let parsed = Signature::from_slice(&signature[..64]).ok()?;
    if parsed.normalize_s().is_some() {
        return None;
    }
    VerifyingKey::recover_from_prehash(digest, &parsed, recovery)
        .ok()
        .map(|key| account(&key))
}
