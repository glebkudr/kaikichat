//! Canonical, bounded, authenticated application documents.
//! Body schemas and authorization are deliberately enforced by their consumers.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use thiserror::Error;

pub const MAX_DOCUMENT_BYTES: usize = 65_536;
pub const MAX_BODY_BYTES: usize = 49_152;
pub const MAX_EXTENSIONS: usize = 16;
pub const MAX_EXTENSION_BYTES: usize = 1024;
const SIGNATURE_DOMAIN: &[u8] = b"AgenticInternet/signed-document/v1\0";
const VERSION: u64 = 1;
const CLOCK_TOLERANCE_SECONDS: u64 = 30;
/// Distinct documents whose signature check a process remembers.
#[cfg(not(test))]
pub(crate) const VERIFIED_DOCUMENTS: usize = 16_384;
/// A small bound keeps the eviction test short; the mechanism is the same.
#[cfg(test)]
pub(crate) const VERIFIED_DOCUMENTS: usize = 256;

pub mod directory;
pub mod group;
pub mod parts;

#[cfg(test)]
#[path = "verified_cache_tests.rs"]
mod verified_cache_tests;

/// Ids of wires whose Ed25519 check succeeded in this process. The id is the
/// SHA-256 of the whole canonical wire: the unsigned bytes, which carry the
/// author key, and the signature. A remembered id therefore proves that this
/// exact deterministic check already passed; it grants nothing else, and every
/// other check of `decode` still runs. Failures are never remembered.
static VERIFIED: LazyLock<Mutex<Verified>> = LazyLock::new(Default::default);

#[derive(Default)]
struct Verified {
    ids: HashSet<[u8; 32]>,
    order: VecDeque<[u8; 32]>,
}
impl Verified {
    fn contains(id: &[u8; 32]) -> bool {
        VERIFIED
            .lock()
            .is_ok_and(|verified| verified.ids.contains(id))
    }
    fn insert(id: [u8; 32]) {
        let Ok(mut verified) = VERIFIED.lock() else {
            return;
        };
        if !verified.ids.insert(id) {
            return;
        }
        verified.order.push_back(id);
        while verified.order.len() > VERIFIED_DOCUMENTS {
            if let Some(oldest) = verified.order.pop_front() {
                verified.ids.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Ed25519 checks actually performed on this thread.
    static SIGNATURE_CHECKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DocumentKind {
    Identity = 1,
    Invitation = 2,
    Message = 3,
    GroupControl = 4,
    Credential = 5,
    ServiceCard = 6,
    Review = 8,
    Resource = 9,
    /// A group's roster of admins, signed by its owner.
    GroupRoster = 10,
    /// A claim on a group epoch's commit, signed by its committer.
    GroupCommit = 11,
    /// A post of an open group, in the clear, signed by its author.
    PublicPost = 12,
    /// Who may write in an open group at an epoch, signed by its committer.
    PublicRoster = 13,
    /// What a profile sends the discovery service, and the bindings the
    /// service signs (spec/discovery-v1.md).
    Directory = 14,
    /// One part of a document too big for one envelope (`parts`).
    Part = 15,
    /// A group's door, signed by its owner or an admin (`group::DoorCard`).
    GroupDoor = 16,
    /// A member's certificate, signed by whoever let it in
    /// (`group::MemberCert`).
    GroupMember = 17,
    /// New keys of a closed channel's key tree, signed by its owner or an
    /// admin (`group::KeyUpdate`).
    ChannelKeys = 18,
    /// A closed channel's subscriber's own key, signed by the subscriber
    /// (`group::SubscriberKey`).
    ChannelSubscriber = 19,
}

impl TryFrom<u64> for DocumentKind {
    type Error = WireError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Identity),
            2 => Ok(Self::Invitation),
            3 => Ok(Self::Message),
            4 => Ok(Self::GroupControl),
            5 => Ok(Self::Credential),
            6 => Ok(Self::ServiceCard),
            8 => Ok(Self::Review),
            9 => Ok(Self::Resource),
            10 => Ok(Self::GroupRoster),
            11 => Ok(Self::GroupCommit),
            12 => Ok(Self::PublicPost),
            13 => Ok(Self::PublicRoster),
            14 => Ok(Self::Directory),
            15 => Ok(Self::Part),
            16 => Ok(Self::GroupDoor),
            17 => Ok(Self::GroupMember),
            18 => Ok(Self::ChannelKeys),
            19 => Ok(Self::ChannelSubscriber),
            other => Err(WireError::UnsupportedKind(other)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentDraft {
    pub domain: [u8; 32],
    pub kind: DocumentKind,
    pub authority_epoch: u64,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub body: Vec<u8>,
    pub extensions: BTreeMap<u16, Vec<u8>>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WireError {
    #[error("malformed signed document")]
    Malformed,
    #[error("noncanonical signed document")]
    NonCanonical,
    #[error("document exceeds resource limits")]
    TooLarge,
    #[error("unsupported protocol version {0}")]
    UnsupportedVersion(u64),
    #[error("unsupported document kind {0}")]
    UnsupportedKind(u64),
    #[error("unsupported critical extension {0}")]
    UnsupportedCriticalExtension(u16),
    #[error("invalid signature or author key")]
    InvalidSignature,
    #[error("document belongs to another network")]
    WrongDomain,
    #[error("expiry must be later than issuance")]
    InvalidLifetime,
    #[error("document has expired")]
    Expired,
    #[error("document is issued too far in the future")]
    NotYetValid,
    #[error("group roster or commit claim does not hold")]
    GroupRule,
}

#[derive(Clone)]
pub struct SignedDocument {
    wire: Vec<u8>,
}

impl std::fmt::Debug for SignedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedDocument")
            .field("bytes", &self.wire.len())
            .finish()
    }
}

impl SignedDocument {
    pub fn sign(draft: DocumentDraft, key: &SigningKey) -> Result<Self, WireError> {
        Self::sign_within(draft, key, Size::Envelope)
    }

    /// A document too big for one envelope, up to `parts::MAX_WHOLE_BYTES`:
    /// it travels in parts (`parts::split`).
    pub fn sign_large(draft: DocumentDraft, key: &SigningKey) -> Result<Self, WireError> {
        Self::sign_within(draft, key, Size::Whole)
    }

    fn sign_within(draft: DocumentDraft, key: &SigningKey, size: Size) -> Result<Self, WireError> {
        validate_fields(&draft, size)?;
        let unsigned = encode_unsigned(&draft, &key.verifying_key().to_bytes())?;
        // Check the entire wire budget before expensive signing.
        let mut wire = encode_outer(&unsigned, &[0; 64])?;
        if wire.len() > size.wire() {
            return Err(WireError::TooLarge);
        }
        let signature = key.sign(&signing_message(&unsigned));
        let signature_start = wire.len() - 64;
        wire[signature_start..].copy_from_slice(&signature.to_bytes());
        Ok(Self { wire })
    }

    pub fn to_wire(&self) -> Vec<u8> {
        self.wire.clone()
    }
    pub fn as_wire(&self) -> &[u8] {
        &self.wire
    }
}

#[derive(Clone)]
pub struct VerifiedDocument {
    draft: DocumentDraft,
    author: [u8; 32],
    id: [u8; 32],
}

impl std::fmt::Debug for VerifiedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedDocument")
            .field("kind", &self.draft.kind)
            .field("authority_epoch", &self.draft.authority_epoch)
            .field("body_bytes", &self.draft.body.len())
            .finish()
    }
}

impl VerifiedDocument {
    pub fn decode(wire: &[u8], expected_domain: [u8; 32], now: u64) -> Result<Self, WireError> {
        Self::decode_within(wire, expected_domain, now, Size::Envelope)
    }

    /// A document put together from its parts (`parts::Assembly`), up to
    /// `parts::MAX_WHOLE_BYTES`; checked like any other.
    pub fn decode_large(
        wire: &[u8],
        expected_domain: [u8; 32],
        now: u64,
    ) -> Result<Self, WireError> {
        Self::decode_within(wire, expected_domain, now, Size::Whole)
    }

    fn decode_within(
        wire: &[u8],
        expected_domain: [u8; 32],
        now: u64,
        size: Size,
    ) -> Result<Self, WireError> {
        if wire.len() > size.wire() {
            return Err(WireError::TooLarge);
        }
        let mut outer = Decoder::new(wire);
        if outer.array().map_err(malformed)? != Some(2) {
            return Err(WireError::NonCanonical);
        }
        // Decoder::bytes borrows the input after checking the declared range. No declared-size allocation.
        let unsigned = outer.bytes().map_err(malformed)?;
        let signature: [u8; 64] = outer
            .bytes()
            .map_err(malformed)?
            .try_into()
            .map_err(malformed)?;
        if outer.position() != wire.len() || encode_outer(unsigned, &signature)? != wire {
            return Err(WireError::NonCanonical);
        }
        let (draft, author) = decode_unsigned(unsigned, size)?;
        validate_fields(&draft, size)?;
        if encode_unsigned(&draft, &author)? != unsigned {
            return Err(WireError::NonCanonical);
        }
        let id: [u8; 32] = Sha256::digest(wire).into();
        if !Verified::contains(&id) {
            #[cfg(test)]
            SIGNATURE_CHECKS.with(|checks| checks.set(checks.get() + 1));
            let public =
                VerifyingKey::from_bytes(&author).map_err(|_| WireError::InvalidSignature)?;
            public
                .verify_strict(
                    &signing_message(unsigned),
                    &Signature::from_bytes(&signature),
                )
                .map_err(|_| WireError::InvalidSignature)?;
            Verified::insert(id);
        }
        if draft.domain != expected_domain {
            return Err(WireError::WrongDomain);
        }
        if draft.issued_at.saturating_sub(now) > CLOCK_TOLERANCE_SECONDS {
            return Err(WireError::NotYetValid);
        }
        if draft.expires_at.is_some_and(|expiry| now >= expiry) {
            return Err(WireError::Expired);
        }
        Ok(Self { draft, author, id })
    }

    pub fn author(&self) -> &[u8; 32] {
        &self.author
    }
    pub fn domain(&self) -> &[u8; 32] {
        &self.draft.domain
    }
    pub fn kind(&self) -> DocumentKind {
        self.draft.kind
    }
    pub fn authority_epoch(&self) -> u64 {
        self.draft.authority_epoch
    }
    pub fn issued_at(&self) -> u64 {
        self.draft.issued_at
    }
    pub fn expires_at(&self) -> Option<u64> {
        self.draft.expires_at
    }
    pub fn body(&self) -> &[u8] {
        &self.draft.body
    }
    pub fn extensions(&self) -> &BTreeMap<u16, Vec<u8>> {
        &self.draft.extensions
    }
    pub fn id(&self) -> [u8; 32] {
        self.id
    }
}

fn malformed<E>(_: E) -> WireError {
    WireError::Malformed
}

/// How a document travels: in one envelope, or put together from parts.
#[derive(Clone, Copy)]
enum Size {
    Envelope,
    Whole,
}

impl Size {
    fn wire(self) -> usize {
        match self {
            Self::Envelope => MAX_DOCUMENT_BYTES,
            Self::Whole => parts::MAX_WHOLE_BYTES,
        }
    }
    /// A part's body fills its envelope; other bodies leave room for
    /// extensions.
    fn body(self, kind: DocumentKind) -> usize {
        match (self, kind) {
            (Self::Whole, _) => parts::MAX_WHOLE_BYTES,
            (Self::Envelope, DocumentKind::Part) => parts::MAX_PART_BODY,
            (Self::Envelope, _) => MAX_BODY_BYTES,
        }
    }
}

fn validate_fields(draft: &DocumentDraft, size: Size) -> Result<(), WireError> {
    if draft.body.len() > size.body(draft.kind) || draft.extensions.len() > MAX_EXTENSIONS {
        return Err(WireError::TooLarge);
    }
    for (id, value) in &draft.extensions {
        if *id >= 32768 {
            return Err(WireError::UnsupportedCriticalExtension(*id));
        }
        if value.len() > MAX_EXTENSION_BYTES {
            return Err(WireError::TooLarge);
        }
    }
    if draft
        .expires_at
        .is_some_and(|expiry| expiry <= draft.issued_at)
    {
        return Err(WireError::InvalidLifetime);
    }
    Ok(())
}

fn signing_message(unsigned: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(SIGNATURE_DOMAIN.len() + unsigned.len());
    message.extend_from_slice(SIGNATURE_DOMAIN);
    message.extend_from_slice(unsigned);
    message
}

fn encode_outer(unsigned: &[u8], signature: &[u8; 64]) -> Result<Vec<u8>, WireError> {
    let mut encoder = Encoder::new(Vec::new());
    encoder.array(2).map_err(malformed)?;
    encoder.bytes(unsigned).map_err(malformed)?;
    encoder.bytes(signature).map_err(malformed)?;
    Ok(encoder.into_writer())
}

fn encode_unsigned(draft: &DocumentDraft, author: &[u8; 32]) -> Result<Vec<u8>, WireError> {
    let mut encoder = Encoder::new(Vec::new());
    encoder.array(9).map_err(malformed)?;
    encoder.bytes(&draft.domain).map_err(malformed)?;
    encoder.u64(VERSION).map_err(malformed)?;
    encoder.u8(draft.kind as u8).map_err(malformed)?;
    encoder.bytes(author).map_err(malformed)?;
    encoder.u64(draft.authority_epoch).map_err(malformed)?;
    encoder.u64(draft.issued_at).map_err(malformed)?;
    match draft.expires_at {
        Some(expiry) => {
            encoder.u64(expiry).map_err(malformed)?;
        }
        None => {
            encoder.null().map_err(malformed)?;
        }
    }
    encoder.bytes(&draft.body).map_err(malformed)?;
    encoder
        .map(draft.extensions.len() as u64)
        .map_err(malformed)?;
    for (id, value) in &draft.extensions {
        encoder.u16(*id).map_err(malformed)?;
        encoder.bytes(value).map_err(malformed)?;
    }
    Ok(encoder.into_writer())
}

fn decode_unsigned(bytes: &[u8], size: Size) -> Result<(DocumentDraft, [u8; 32]), WireError> {
    let mut decoder = Decoder::new(bytes);
    if decoder.array().map_err(malformed)? != Some(9) {
        return Err(WireError::NonCanonical);
    }
    let domain = decoder
        .bytes()
        .map_err(malformed)?
        .try_into()
        .map_err(malformed)?;
    let version = decoder.u64().map_err(malformed)?;
    if version != VERSION {
        return Err(WireError::UnsupportedVersion(version));
    }
    let kind = DocumentKind::try_from(decoder.u64().map_err(malformed)?)?;
    let author = decoder
        .bytes()
        .map_err(malformed)?
        .try_into()
        .map_err(malformed)?;
    let authority_epoch = decoder.u64().map_err(malformed)?;
    let issued_at = decoder.u64().map_err(malformed)?;
    let expires_at = if decoder.datatype().map_err(malformed)? == minicbor::data::Type::Null {
        decoder.null().map_err(malformed)?;
        None
    } else {
        Some(decoder.u64().map_err(malformed)?)
    };
    let body = decoder.bytes().map_err(malformed)?;
    if body.len() > size.body(kind) {
        return Err(WireError::TooLarge);
    }
    let count = decoder
        .map()
        .map_err(malformed)?
        .ok_or(WireError::NonCanonical)?;
    if count > MAX_EXTENSIONS as u64 {
        return Err(WireError::TooLarge);
    }
    let mut extensions = BTreeMap::new();
    let mut previous = None;
    for _ in 0..count {
        let id = decoder.u16().map_err(malformed)?;
        if previous.is_some_and(|old| id <= old) {
            return Err(WireError::NonCanonical);
        }
        previous = Some(id);
        if id >= 32768 {
            return Err(WireError::UnsupportedCriticalExtension(id));
        }
        let value = decoder.bytes().map_err(malformed)?;
        if value.len() > MAX_EXTENSION_BYTES {
            return Err(WireError::TooLarge);
        }
        extensions.insert(id, value.to_vec());
    }
    if decoder.position() != bytes.len() {
        return Err(WireError::NonCanonical);
    }
    Ok((
        DocumentDraft {
            domain,
            kind,
            authority_epoch,
            issued_at,
            expires_at,
            body: body.to_vec(),
            extensions,
        },
        author,
    ))
}

/// Stable human-facing root address; network/genesis separation is enforced by signed documents.
pub fn network_id(public_key: &[u8; 32]) -> String {
    format!("ain1{}", hex::encode(Sha256::digest(public_key)))
}
