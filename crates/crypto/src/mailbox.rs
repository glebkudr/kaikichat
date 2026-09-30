//! The sealed envelopes of a conversation direction's swarm mailbox. Only holders
//! of the MLS exporter capability derive the mailbox, the envelope key and the
//! plaintext; holders see fixed-size buckets.
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::{
    OpenMlsProvider,
    crypto::OpenMlsCrypto,
    types::{AeadType, HashType},
};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Error)]
pub enum MailboxError {
    #[error("invalid mailbox envelope")]
    Invalid,
    #[error("mailbox size limit exceeded")]
    Limit,
    #[error("mailbox capability or authentication mismatch")]
    Authentication,
}

/// Deliberately neither Debug nor Serialize: never expose the capability in public IPC.
pub struct MailboxSecret(Zeroizing<[u8; 32]>);
pub struct EpochMailbox {
    pub epoch: u64,
    pub secret: MailboxSecret,
}
/// Swarm envelopes pad the sealed wire to whole buckets.
pub const SWARM_ENVELOPE_BUCKET: usize = 1024;
const SWARM_NONCE: usize = 12;
const SWARM_TAG: usize = 16;
const SWARM_LENGTH: usize = 4;
/// The sealed size of the largest signed document.
pub const MAX_SWARM_ENVELOPE: usize = SWARM_NONCE
    + (SWARM_LENGTH + agentic_protocol::MAX_DOCUMENT_BYTES).div_ceil(SWARM_ENVELOPE_BUCKET)
        * SWARM_ENVELOPE_BUCKET
    + SWARM_TAG;

struct EnvelopeKeys {
    encryption: Zeroizing<[u8; 16]>,
    nonce: Zeroizing<Vec<u8>>,
}

impl MailboxSecret {
    /// The capability's bytes, for the profile's encrypted store only.
    pub fn expose(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(*self.0)
    }

    /// The swarm mailbox of this direction in one period; the capability
    /// itself never leaves this type.
    pub fn swarm_mailbox(&self, domain: &[u8; 32], period: u64) -> [u8; 32] {
        agentic_mailbox_swarm::address::mailbox_id(domain, &self.0, period)
    }

    /// What holders of this direction's mailbox store: the signed wire
    /// sealed under a key only conversation members derive, padded to whole
    /// buckets. Deterministic, so a retried send pays for the same bytes.
    pub fn seal_swarm_envelope(
        &self,
        domain: &[u8; 32],
        period: u64,
        wire: &[u8],
    ) -> Result<Vec<u8>, MailboxError> {
        if wire.len() > agentic_protocol::MAX_DOCUMENT_BYTES {
            return Err(MailboxError::Limit);
        }
        let size =
            (SWARM_LENGTH + wire.len()).div_ceil(SWARM_ENVELOPE_BUCKET) * SWARM_ENVELOPE_BUCKET;
        let mut padded = Zeroizing::new(Vec::with_capacity(size));
        padded.extend_from_slice(&(wire.len() as u32).to_be_bytes());
        padded.extend_from_slice(wire);
        padded.resize(size, 0);
        let provider = OpenMlsRustCrypto::default();
        let keys = self.envelope_keys(&provider, domain, period)?;
        let nonce = provider
            .crypto()
            .hkdf_extract(HashType::Sha2_256, &keys.nonce, &padded)
            .map_err(authentication)?;
        let nonce = &nonce.as_slice()[..SWARM_NONCE];
        let ciphertext = provider
            .crypto()
            .aead_encrypt(
                AeadType::Aes128Gcm,
                &*keys.encryption,
                &padded,
                nonce,
                &self.swarm_mailbox(domain, period),
            )
            .map_err(authentication)?;
        let mut envelope = nonce.to_vec();
        envelope.extend_from_slice(&ciphertext);
        Ok(envelope)
    }

    /// The signed wire inside an envelope of this direction's mailbox.
    pub fn open_swarm_envelope(
        &self,
        domain: &[u8; 32],
        period: u64,
        envelope: &[u8],
    ) -> Result<Vec<u8>, MailboxError> {
        let sealed = envelope
            .len()
            .checked_sub(SWARM_NONCE + SWARM_TAG)
            .ok_or(MailboxError::Invalid)?;
        if envelope.len() > MAX_SWARM_ENVELOPE || sealed == 0 || sealed % SWARM_ENVELOPE_BUCKET != 0
        {
            return Err(MailboxError::Invalid);
        }
        let provider = OpenMlsRustCrypto::default();
        let keys = self.envelope_keys(&provider, domain, period)?;
        let padded = Zeroizing::new(
            provider
                .crypto()
                .aead_decrypt(
                    AeadType::Aes128Gcm,
                    &*keys.encryption,
                    &envelope[SWARM_NONCE..],
                    &envelope[..SWARM_NONCE],
                    &self.swarm_mailbox(domain, period),
                )
                .map_err(authentication)?,
        );
        let length =
            u32::from_be_bytes(padded[..SWARM_LENGTH].try_into().map_err(invalid)?) as usize;
        let end = SWARM_LENGTH
            .checked_add(length)
            .filter(|end| {
                length <= agentic_protocol::MAX_DOCUMENT_BYTES
                    && end.div_ceil(SWARM_ENVELOPE_BUCKET) * SWARM_ENVELOPE_BUCKET == padded.len()
            })
            .ok_or(MailboxError::Invalid)?;
        if padded[end..].iter().any(|byte| *byte != 0) {
            return Err(MailboxError::Invalid);
        }
        Ok(padded[SWARM_LENGTH..end].to_vec())
    }

    fn envelope_keys(
        &self,
        provider: &OpenMlsRustCrypto,
        domain: &[u8; 32],
        period: u64,
    ) -> Result<EnvelopeKeys, MailboxError> {
        let mut salt = b"AgenticInternet/mailbox-envelope/v1\0".to_vec();
        salt.extend_from_slice(domain);
        let prk = provider
            .crypto()
            .hkdf_extract(HashType::Sha2_256, &salt, &*self.0)
            .map_err(authentication)?;
        let expand = |label: &[u8], size| {
            let mut info = label.to_vec();
            info.extend_from_slice(&period.to_be_bytes());
            provider
                .crypto()
                .hkdf_expand(HashType::Sha2_256, prk.as_slice(), &info, size)
                .map_err(authentication)
        };
        Ok(EnvelopeKeys {
            encryption: Zeroizing::new(
                expand(b"encryption\0", 16)?
                    .as_slice()
                    .try_into()
                    .map_err(invalid)?,
            ),
            nonce: Zeroizing::new(expand(b"nonce\0", 32)?.as_slice().to_vec()),
        })
    }
}

impl MailboxSecret {
    /// Trusted Core/fixture boundary. This is not an IPC import/export endpoint.
    pub fn from_bytes(mut bytes: [u8; 32]) -> Self {
        let secret = Self(Zeroizing::new(bytes));
        bytes.zeroize();
        secret
    }
}

fn invalid<E>(_: E) -> MailboxError {
    MailboxError::Invalid
}
fn authentication<E>(_: E) -> MailboxError {
    MailboxError::Authentication
}
