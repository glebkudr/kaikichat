//! Root attestation for an authenticated transport session, independent of libp2p types.
use super::*;
pub(super) const LIFETIME: u64 = 86400;
const MAX_RECORD: usize = 4096;
/// Verified by `AppCore::verify_node_record`; fields contain public routing information only.
#[derive(Clone, Debug)]
pub struct NodeRecord {
    pub author: [u8; 32],
    pub peer_id: String,
    pub addresses: Vec<String>,
    /// Zero denotes an unsequenced legacy v1 record.
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub id: [u8; 32],
    pub wire: Vec<u8>,
}
fn body(peer: &str, addresses: &[String], sequence: Option<u64>) -> Result<Vec<u8>, CoreError> {
    if peer.is_empty() || peer.len() > 128 || peer.chars().any(char::is_control) {
        return Err(CoreError::InvalidInput);
    }
    valid_addresses(addresses)?;
    if sequence == Some(0) {
        return Err(CoreError::InvalidInput);
    }
    let mut e = Encoder::new(Vec::new());
    e.array(if sequence.is_some() { 4 } else { 3 })
        .map_err(invalid)?
        .u8(if sequence.is_some() { 2 } else { 1 })
        .map_err(invalid)?
        .str(peer)
        .map_err(invalid)?
        .array(addresses.len() as u64)
        .map_err(invalid)?;
    for address in addresses {
        e.str(address).map_err(invalid)?;
    }
    if let Some(sequence) = sequence {
        e.u64(sequence).map_err(invalid)?;
    }
    Ok(e.into_writer())
}
impl AppCore {
    /// Trusted runtime only. Agent capabilities must never expose unrestricted root signing.
    pub fn create_node_record(
        &self,
        peer: &str,
        addresses: Vec<String>,
        now: u64,
    ) -> Result<Vec<u8>, CoreError> {
        self.sign_node_record(peer, &addresses, None, now)
    }
    pub(super) fn sign_node_record(
        &self,
        peer: &str,
        addresses: &[String],
        sequence: Option<u64>,
        now: u64,
    ) -> Result<Vec<u8>, CoreError> {
        let draft = DocumentDraft {
            domain: self.domain,
            kind: DocumentKind::Identity,
            authority_epoch: 0,
            issued_at: now,
            expires_at: Some(now.checked_add(LIFETIME).ok_or(CoreError::InvalidInput)?),
            body: body(peer, addresses, sequence)?,
            extensions: BTreeMap::new(),
        };
        Ok(self.store.sign_document(draft)?.to_wire())
    }
    pub fn verify_node_record(
        &self,
        wire: &[u8],
        actual_peer: &str,
        now: u64,
    ) -> Result<NodeRecord, CoreError> {
        if wire.len() > MAX_RECORD {
            return Err(CoreError::InvalidInput);
        }
        let verified = VerifiedDocument::decode(wire, self.domain, now)?;
        root_epoch(&verified)?;
        if verified.kind() != DocumentKind::Identity
            || verified
                .expires_at()
                .is_none_or(|expiry| expiry.saturating_sub(verified.issued_at()) > LIFETIME)
        {
            return Err(CoreError::Unauthorized);
        }
        let mut d = Decoder::new(verified.body());
        let sequenced = match (d.array().map_err(invalid)?, d.u8().map_err(invalid)?) {
            (Some(3), 1) => false,
            (Some(4), 2) => true,
            _ => return Err(CoreError::InvalidInput),
        };
        let peer = d.str().map_err(invalid)?;
        if peer != actual_peer {
            return Err(CoreError::Unauthorized);
        }
        let count = d.array().map_err(invalid)?.ok_or(CoreError::InvalidInput)?;
        if count > 8 {
            return Err(CoreError::InvalidInput);
        }
        let mut addresses = Vec::with_capacity(count as usize);
        for _ in 0..count {
            addresses.push(d.str().map_err(invalid)?.to_owned());
        }
        let sequence = if sequenced {
            Some(d.u64().map_err(invalid)?)
        } else {
            None
        };
        if body(peer, &addresses, sequence)? != verified.body() {
            return Err(CoreError::InvalidInput);
        }
        Ok(NodeRecord {
            author: *verified.author(),
            peer_id: peer.into(),
            addresses,
            sequence: sequence.unwrap_or(0),
            issued_at: verified.issued_at(),
            expires_at: verified.expires_at().ok_or(CoreError::InvalidInput)?,
            id: verified.id(),
            wire: wire.to_vec(),
        })
    }
    pub fn receive_from(
        &mut self,
        wire: &[u8],
        node_record: &[u8],
        actual_peer: &str,
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        let record = self.verify_node_record(node_record, actual_peer, now)?;
        let verified = VerifiedDocument::decode(wire, self.domain, now)?;
        if verified.author() != &record.author {
            return Err(CoreError::Unauthorized);
        }
        let outcome = self.receive_verified(verified, record.addresses.clone(), now)?;
        // Envelope admission and MLS/history commit precede route learning. If the separate
        // cache transaction fails, retrying this envelope uses the reducer's durable dedupe.
        self.remember_verified_node_record(record, now)?;
        Ok(outcome)
    }

    /// A direct delivery without a stamp, at a node that takes payment: only
    /// control messages (a Welcome, receipts) are received.
    pub fn receive_control_from(
        &mut self,
        wire: &[u8],
        node_record: &[u8],
        actual_peer: &str,
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        let record = self.verify_node_record(node_record, actual_peer, now)?;
        let verified = VerifiedDocument::decode(wire, self.domain, now)?;
        if verified.author() != &record.author {
            return Err(CoreError::Unauthorized);
        }
        match Packet::decode(verified.body(), verified.kind())? {
            Packet::Application { .. } => return Err(CoreError::PaymentRequired),
            // A request made with an intro card pays like a message.
            Packet::Welcome { invitation, .. } if self.welcome_needs_stamp(&invitation)? => {
                return Err(CoreError::PaymentRequired);
            }
            Packet::GroupWelcome { .. } => return Err(CoreError::PaymentRequired),
            _ => {}
        }
        let outcome = self.receive_verified(verified, record.addresses.clone(), now)?;
        self.remember_verified_node_record(record, now)?;
        Ok(outcome)
    }
}
