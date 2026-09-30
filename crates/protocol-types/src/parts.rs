//! Documents too big for one mailbox envelope travel in parts
//! (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 2): the ratchet tree of
//! a big group, a big commit. Each part is a signed document of kind `Part`,
//! paid like any message; it names the SHA-256 of the whole wire, its place
//! and the number of parts. The whole is itself a signed document: its own
//! signature, checked by `VerifiedDocument::decode_large`, says who made it.
use crate::{DocumentDraft, DocumentKind, SignedDocument, VerifiedDocument, WireError};
use ed25519_dalek::SigningKey;
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Parts of one whole at most.
pub const MAX_PARTS: usize = 64;
/// Bytes of the whole one part carries at most: its signed document still
/// fits one envelope.
pub const MAX_PART_BYTES: usize = 65_280;
/// The biggest whole wire.
pub const MAX_WHOLE_BYTES: usize = MAX_PARTS * MAX_PART_BYTES;
/// A part's body: its place and bytes.
pub(crate) const MAX_PART_BODY: usize = MAX_PART_BYTES + 64;

/// One piece of a whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// SHA-256 of the whole wire.
    pub whole: [u8; 32],
    pub index: u16,
    pub count: u16,
    pub bytes: Vec<u8>,
}

impl Part {
    fn encode(&self) -> Result<Vec<u8>, WireError> {
        let mut e = Encoder::new(Vec::new());
        e.array(4)
            .and_then(|e| e.bytes(&self.whole))
            .and_then(|e| e.u16(self.index))
            .and_then(|e| e.u16(self.count))
            .and_then(|e| e.bytes(&self.bytes))
            .map_err(|_| WireError::Malformed)?;
        Ok(e.into_writer())
    }

    fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(body);
        let malformed = |_| WireError::Malformed;
        if d.array().map_err(malformed)? != Some(4) {
            return Err(WireError::Malformed);
        }
        let whole = d
            .bytes()
            .map_err(malformed)?
            .try_into()
            .map_err(|_| WireError::Malformed)?;
        let index = d.u16().map_err(malformed)?;
        let count = d.u16().map_err(malformed)?;
        let bytes = d.bytes().map_err(malformed)?.to_vec();
        if d.position() != body.len() {
            return Err(WireError::Malformed);
        }
        Ok(Self {
            whole,
            index,
            count,
            bytes,
        })
    }

    /// Sign this part as it is; its place is checked by the reader.
    pub fn sign(
        &self,
        domain: [u8; 32],
        key: &SigningKey,
        issued_at: u64,
    ) -> Result<SignedDocument, WireError> {
        SignedDocument::sign(
            DocumentDraft {
                domain,
                kind: DocumentKind::Part,
                authority_epoch: 0,
                issued_at,
                expires_at: None,
                body: self.encode()?,
                extensions: BTreeMap::new(),
            },
            key,
        )
    }
}

/// What names a whole: the SHA-256 of its wire and its number of parts.
pub fn reference(whole: &SignedDocument) -> ([u8; 32], u16) {
    wire_reference(whole.as_wire())
}

/// `reference` of a whole's wire.
pub fn wire_reference(wire: &[u8]) -> ([u8; 32], u16) {
    (
        Sha256::digest(wire).into(),
        wire.len().div_ceil(MAX_PART_BYTES) as u16,
    )
}

/// Cut a whole into parts signed with `key`, in order. Signatures are
/// deterministic: the same whole, key and time give the same parts.
pub fn split(
    whole: &SignedDocument,
    domain: [u8; 32],
    key: &SigningKey,
    issued_at: u64,
) -> Result<Vec<SignedDocument>, WireError> {
    split_wire(whole.as_wire(), domain, key, issued_at)
}

/// `split` of a whole's wire.
pub fn split_wire(
    wire: &[u8],
    domain: [u8; 32],
    key: &SigningKey,
    issued_at: u64,
) -> Result<Vec<SignedDocument>, WireError> {
    if wire.is_empty() || wire.len() > MAX_WHOLE_BYTES {
        return Err(WireError::TooLarge);
    }
    let (hash, count) = wire_reference(wire);
    wire.chunks(MAX_PART_BYTES)
        .enumerate()
        .map(|(index, bytes)| {
            Part {
                whole: hash,
                index: index as u16,
                count,
                bytes: bytes.to_vec(),
            }
            .sign(domain, key, issued_at)
        })
        .collect()
}

/// A part whose signature and place hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPart {
    /// Who signed the part.
    pub author: [u8; 32],
    pub part: Part,
}

pub fn verify_part(wire: &[u8], domain: [u8; 32], now: u64) -> Result<VerifiedPart, WireError> {
    let verified = VerifiedDocument::decode(wire, domain, now)?;
    if verified.kind() != DocumentKind::Part {
        return Err(WireError::UnsupportedKind(verified.kind() as u64));
    }
    let part = Part::decode(verified.body())?;
    let count = usize::from(part.count);
    if count == 0
        || count > MAX_PARTS
        || part.index >= part.count
        || part.bytes.is_empty()
        || part.bytes.len() > MAX_PART_BYTES
    {
        return Err(WireError::Malformed);
    }
    Ok(VerifiedPart {
        author: *verified.author(),
        part,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssemblyError {
    /// The part belongs to another author's or another whole's assembly.
    #[error("part of another assembly")]
    Foreign,
    /// The parts do not add up to the whole they name.
    #[error("parts do not add up to their whole")]
    Mismatch,
}

/// The parts of one whole by one author, as they arrive.
#[derive(Clone, Debug)]
pub struct Assembly {
    author: [u8; 32],
    whole: [u8; 32],
    count: u16,
    parts: BTreeMap<u16, Vec<u8>>,
    done: bool,
}

impl Assembly {
    pub fn new(first: &VerifiedPart) -> Self {
        Self {
            author: first.author,
            whole: first.part.whole,
            count: first.part.count,
            parts: BTreeMap::new(),
            done: false,
        }
    }

    /// Take a part: the whole wire when it is the one that completes it,
    /// once; nothing more after that.
    pub fn add(&mut self, part: &VerifiedPart) -> Result<Option<Vec<u8>>, AssemblyError> {
        if part.author != self.author
            || part.part.whole != self.whole
            || part.part.count != self.count
        {
            return Err(AssemblyError::Foreign);
        }
        if self.done {
            return Ok(None);
        }
        self.parts
            .entry(part.part.index)
            .or_insert_with(|| part.part.bytes.clone());
        if self.parts.len() < usize::from(self.count) {
            return Ok(None);
        }
        let wire: Vec<u8> = self.parts.values().flatten().copied().collect();
        if <[u8; 32]>::from(Sha256::digest(&wire)) != self.whole {
            return Err(AssemblyError::Mismatch);
        }
        self.done = true;
        self.parts.clear();
        Ok(Some(wire))
    }
}
