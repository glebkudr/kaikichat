//! Grant books: coins minted by the identity server for one stamp-book account.
//!
//! Digests are keccak256 over `keccak256(tag)` and fixed-width big-endian
//! fields (Solidity `abi.encodePacked`); signatures are secp256k1 `r ‖ s ‖ v`
//! with low `s` and `v` 27/28, so every statement can be checked with `ecrecover`.

use alloy_primitives::keccak256;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// An EVM account: the last 20 bytes of keccak256 of an uncompressed public key.
pub type Account = [u8; 20];

pub const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GrantError {
    #[error("signature does not recover the claimed signer")]
    BadSignature,
    #[error("grant belongs to another network domain")]
    WrongDomain,
    #[error("issuer is not active on the grant day")]
    IssuerInactive,
    #[error("grant size differs from the network book size")]
    CountMismatch,
    #[error("serial exceeds the daily emission cap")]
    OverCap,
    #[error("grant is dated in the future")]
    FutureDay,
    #[error("grant expiry exceeds the maximum validity")]
    ExpiryTooFar,
    #[error("grant has expired")]
    Expired,
    #[error("the two grants are not a conflicting issuance")]
    NotEquivocation,
}

/// A secp256k1 key that signs grants (server) or claim requests (book).
#[derive(Clone)]
pub struct SecpKey {
    key: SigningKey,
}

impl SecpKey {
    /// `None` for zero or out-of-range scalars.
    pub fn from_secret(secret: &[u8; 32]) -> Option<Self> {
        SigningKey::from_bytes(secret.into())
            .ok()
            .map(|key| Self { key })
    }

    pub fn account(&self) -> Account {
        account(self.key.verifying_key())
    }

    fn sign(&self, digest: &[u8; 32]) -> [u8; 65] {
        let mut out = [0; 65];
        // A 32-byte prehash is always accepted; RFC 6979 signing cannot fail.
        if let Ok((signature, recovery)) = self.key.sign_prehash_recoverable(digest) {
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
}

fn digest(tag: &str, fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = keccak256(tag.as_bytes()).to_vec();
    for field in fields {
        bytes.extend_from_slice(field);
    }
    keccak256(bytes).0
}

fn account(key: &VerifyingKey) -> Account {
    let point = key.to_encoded_point(false);
    let mut account = [0; 20];
    account.copy_from_slice(&keccak256(&point.as_bytes()[1..])[12..]);
    account
}

/// The signer of `digest`; `None` for malformed, high-s or non-27/28 signatures.
fn recover(digest: &[u8; 32], signature: &[u8; 65]) -> Option<Account> {
    let recovery = RecoveryId::from_byte(signature[64].checked_sub(27)?)?;
    let parsed = Signature::from_slice(&signature[..64]).ok()?;
    if parsed.normalize_s().is_some() {
        return None;
    }
    VerifyingKey::recover_from_prehash(digest, &parsed, recovery)
        .ok()
        .map(|key| account(&key))
}

/// Unsigned terms of one grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantTerms {
    pub domain: [u8; 32],
    pub book: Account,
    pub day: u64,
    pub serial: u32,
    pub count: u32,
    pub expiry: u64,
}

/// A signed grant of `count` coins to the stamp-book account `book`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantBook {
    #[serde(with = "hex_bytes")]
    pub domain: [u8; 32],
    #[serde(with = "hex_bytes")]
    pub server: Account,
    #[serde(with = "hex_bytes")]
    pub book: Account,
    pub day: u64,
    pub serial: u32,
    pub count: u32,
    pub expiry: u64,
    #[serde(with = "hex_bytes")]
    pub signature: [u8; 65],
}

/// Network rules for grants of one issuer on the grant day, read from the
/// `GrantIssuer` contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantRules {
    pub domain: [u8; 32],
    pub issuer_active: bool,
    pub cap_coins: u64,
    pub book_size: u32,
    pub max_validity_days: u64,
}

impl GrantBook {
    pub fn issue(terms: GrantTerms, server: &SecpKey) -> Self {
        let mut grant = Self {
            domain: terms.domain,
            server: server.account(),
            book: terms.book,
            day: terms.day,
            serial: terms.serial,
            count: terms.count,
            expiry: terms.expiry,
            signature: [0; 65],
        };
        grant.signature = server.sign(&grant.digest());
        grant
    }

    pub fn terms(&self) -> GrantTerms {
        GrantTerms {
            domain: self.domain,
            book: self.book,
            day: self.day,
            serial: self.serial,
            count: self.count,
            expiry: self.expiry,
        }
    }

    /// keccak256(keccak256("AIN_GRANT_V1") ‖ domain ‖ server ‖ book ‖ day ‖ serial ‖ count ‖ expiry).
    pub fn digest(&self) -> [u8; 32] {
        digest(
            "AIN_GRANT_V1",
            &[
                &self.domain,
                &self.server,
                &self.book,
                &self.day.to_be_bytes(),
                &self.serial.to_be_bytes(),
                &self.count.to_be_bytes(),
                &self.expiry.to_be_bytes(),
            ],
        )
    }

    /// keccak256(keccak256("AIN_GRANT_BOOK_V1") ‖ domain ‖ server ‖ day ‖ serial).
    /// The notary keys a grant by this id: one grant per issuer, day and serial.
    pub fn id(&self) -> [u8; 32] {
        digest(
            "AIN_GRANT_BOOK_V1",
            &[
                &self.domain,
                &self.server,
                &self.day.to_be_bytes(),
                &self.serial.to_be_bytes(),
            ],
        )
    }

    pub fn verify_signature(&self) -> Result<(), GrantError> {
        match recover(&self.digest(), &self.signature) {
            Some(signer) if signer == self.server => Ok(()),
            _ => Err(GrantError::BadSignature),
        }
    }

    /// Holder checks at `now` (Unix seconds), in this order: signature,
    /// domain, issuer active on the grant day, size, daily cap, day not in the
    /// future, expiry within the maximum validity, not yet expired. `rules`
    /// must be read for the grant's day, not for today.
    pub fn check(&self, rules: &GrantRules, now: u64) -> Result<(), GrantError> {
        self.verify_signature()?;
        if self.domain != rules.domain {
            return Err(GrantError::WrongDomain);
        }
        if !rules.issuer_active {
            return Err(GrantError::IssuerInactive);
        }
        if self.count != rules.book_size {
            return Err(GrantError::CountMismatch);
        }
        let used = (u128::from(self.serial) + 1) * u128::from(rules.book_size);
        if used > u128::from(rules.cap_coins) {
            return Err(GrantError::OverCap);
        }
        // Compare days, never day * 86400: a far-future day must not wrap.
        if self.day > now / SECONDS_PER_DAY {
            return Err(GrantError::FutureDay);
        }
        let limit = self
            .day
            .checked_add(1)
            .and_then(|day| day.checked_add(rules.max_validity_days))
            .and_then(|day| day.checked_mul(SECONDS_PER_DAY));
        if limit.is_none_or(|limit| self.expiry > limit) {
            return Err(GrantError::ExpiryTooFar);
        }
        if now >= self.expiry {
            return Err(GrantError::Expired);
        }
        Ok(())
    }

    /// Whether the notary first saw this grant on its own UTC day, allowing
    /// `tolerance` seconds of clock skew on either side.
    pub fn first_seen_in_time(&self, first_seen: u64, tolerance: u64) -> bool {
        let Some(start) = self.day.checked_mul(SECONDS_PER_DAY) else {
            return false;
        };
        let end = start.saturating_add(SECONDS_PER_DAY);
        first_seen.saturating_add(tolerance) >= start && first_seen < end.saturating_add(tolerance)
    }
}

/// A book-key-signed request that asks the identity server for a grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaimRequest {
    #[serde(with = "hex_bytes")]
    pub domain: [u8; 32],
    #[serde(with = "hex_bytes")]
    pub book: Account,
    #[serde(with = "hex_bytes")]
    pub nonce: [u8; 16],
    pub created_at: u64,
    #[serde(with = "hex_bytes")]
    pub signature: [u8; 65],
}

impl ClaimRequest {
    pub fn sign(domain: [u8; 32], nonce: [u8; 16], created_at: u64, book: &SecpKey) -> Self {
        let mut request = Self {
            domain,
            book: book.account(),
            nonce,
            created_at,
            signature: [0; 65],
        };
        request.signature = book.sign(&request.digest());
        request
    }

    /// keccak256(keccak256("AIN_GRANT_CLAIM_V1") ‖ domain ‖ book ‖ nonce ‖ created_at).
    pub fn digest(&self) -> [u8; 32] {
        digest(
            "AIN_GRANT_CLAIM_V1",
            &[
                &self.domain,
                &self.book,
                &self.nonce,
                &self.created_at.to_be_bytes(),
            ],
        )
    }

    pub fn verify(&self) -> Result<(), GrantError> {
        match recover(&self.digest(), &self.signature) {
            Some(signer) if signer == self.book => Ok(()),
            _ => Err(GrantError::BadSignature),
        }
    }
}

/// Two validly signed, different grants for one issuer, day and serial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantEquivocation {
    pub first: GrantBook,
    pub second: GrantBook,
}

impl GrantEquivocation {
    /// The misbehaving issuer when the proof holds.
    pub fn verify(&self) -> Result<Account, GrantError> {
        let (first, second) = (&self.first, &self.second);
        first.verify_signature()?;
        second.verify_signature()?;
        let same_slot = first.domain == second.domain
            && first.server == second.server
            && first.day == second.day
            && first.serial == second.serial;
        // Compare what was signed, not signature bytes: one grant may carry
        // several valid signatures.
        if !same_slot || first.digest() == second.digest() {
            return Err(GrantError::NotEquivocation);
        }
        Ok(first.server)
    }
}

/// The issuer withdraws a grant it signed: holders take no new stamps of
/// its book (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md). `expiry` is the
/// grant's, so a holder keeps the revocation no longer than the grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantRevocation {
    #[serde(with = "hex_bytes")]
    pub domain: [u8; 32],
    #[serde(with = "hex_bytes")]
    pub server: Account,
    /// The grant's book id (`GrantBook::id`).
    #[serde(with = "hex_bytes")]
    pub book: [u8; 32],
    pub expiry: u64,
    pub revoked_at: u64,
    #[serde(with = "hex_bytes")]
    pub signature: [u8; 65],
}

impl GrantRevocation {
    pub fn issue(grant: &GrantBook, revoked_at: u64, server: &SecpKey) -> Self {
        let mut revocation = Self {
            domain: grant.domain,
            server: server.account(),
            book: grant.id(),
            expiry: grant.expiry,
            revoked_at,
            signature: [0; 65],
        };
        revocation.signature = server.sign(&revocation.digest());
        revocation
    }

    /// keccak256(keccak256("AIN_GRANT_REVOCATION_V1") ‖ domain ‖ server ‖ book ‖ expiry ‖ revoked_at).
    pub fn digest(&self) -> [u8; 32] {
        digest(
            "AIN_GRANT_REVOCATION_V1",
            &[
                &self.domain,
                &self.server,
                &self.book,
                &self.expiry.to_be_bytes(),
                &self.revoked_at.to_be_bytes(),
            ],
        )
    }

    pub fn verify(&self) -> Result<(), GrantError> {
        match recover(&self.digest(), &self.signature) {
            Some(signer) if signer == self.server => Ok(()),
            _ => Err(GrantError::BadSignature),
        }
    }

    /// Whether this is `grant`'s own issuer withdrawing it.
    pub fn revokes(&self, grant: &GrantBook) -> bool {
        self.server == grant.server && self.book == grant.id() && self.verify().is_ok()
    }
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer, const N: usize>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        let text = String::deserialize(deserializer)?;
        let digits = text
            .strip_prefix("0x")
            .ok_or_else(|| D::Error::custom("expected 0x-prefixed hex"))?;
        let bytes = hex::decode(digits).map_err(D::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| D::Error::custom(format!("expected {N} bytes")))
    }
}
