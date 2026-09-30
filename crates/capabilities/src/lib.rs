//! Signed attenuating grants and transaction-ready shared budget reservations.
use agentic_protocol::{DocumentDraft, DocumentKind, VerifiedDocument};
use agentic_store::{ProfileStore, StateChange};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
mod wire;

const MAX_GRANT_BYTES: usize = 16 * 1024;
const MAX_LIFETIME: u64 = 30 * 86400;
type Key = [u8; 32];
pub type Result<T> = std::result::Result<T, CapabilityError>;

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("invalid or noncanonical delegation grant")]
    InvalidGrant,
    #[error("action is outside current delegated authority")]
    Unauthorized,
    #[error("delegated budget is exhausted")]
    BudgetExceeded,
    #[error("operation ID is already bound to another authorized action")]
    IdempotencyConflict,
    #[error("invalid persisted authorization state")]
    InvalidState,
    #[error(transparent)]
    Wire(#[from] agentic_protocol::WireError),
    #[error(transparent)]
    Store(#[from] agentic_store::StoreError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Action {
    ReadInbox = 1,
    SendMessage = 2,
    CreateGroup = 3,
    ManageGroup = 4,
    ReadArtifact = 7,
    WriteArtifact = 8,
    PublishReview = 9,
    AmendReview = 10,
    WithdrawReview = 11,
    ReplyReview = 12,
    SpendPostage = 13,
}
impl TryFrom<u64> for Action {
    type Error = CapabilityError;
    fn try_from(value: u64) -> Result<Self> {
        match value {
            1 => Ok(Self::ReadInbox),
            2 => Ok(Self::SendMessage),
            3 => Ok(Self::CreateGroup),
            4 => Ok(Self::ManageGroup),
            7 => Ok(Self::ReadArtifact),
            8 => Ok(Self::WriteArtifact),
            9 => Ok(Self::PublishReview),
            10 => Ok(Self::AmendReview),
            11 => Ok(Self::WithdrawReview),
            12 => Ok(Self::ReplyReview),
            13 => Ok(Self::SpendPostage),
            _ => Err(CapabilityError::InvalidGrant),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantClaims {
    pub owner: Key,
    pub agent: Key,
    pub service: Key,
    pub subject: Key,
    pub parent: Option<Key>,
    pub device_epoch: u64,
    pub service_epoch: u64,
    pub actions: BTreeSet<Action>,
    pub resources: BTreeSet<String>,
    pub recipients: BTreeSet<String>,
    pub budget_asset: String,
    pub budget_units: u64,
    pub max_data_bytes: u64,
    pub remaining_depth: u8,
    pub no_subcontract: bool,
}
impl GrantClaims {
    pub fn draft(
        &self,
        domain: Key,
        ownership_epoch: u64,
        issued_at: u64,
        expires_at: u64,
    ) -> Result<DocumentDraft> {
        self.validate()?;
        lifetime(issued_at, expires_at)?;
        Ok(DocumentDraft {
            domain,
            kind: DocumentKind::Identity,
            authority_epoch: ownership_epoch,
            issued_at,
            expires_at: Some(expires_at),
            body: wire::encode(self)?,
            extensions: BTreeMap::new(),
        })
    }
    fn validate(&self) -> Result<()> {
        if self.actions.is_empty()
            || self.actions.len() > 11
            || self.resources.len() > 32
            || self.recipients.len() > 32
            || self.remaining_depth > 4
            || !identifier(&self.budget_asset, 96)
            || self
                .resources
                .iter()
                .chain(&self.recipients)
                .any(|v| !identifier(v, 160))
        {
            return Err(CapabilityError::InvalidGrant);
        }
        Ok(())
    }
    fn attenuates(&self, parent: &Self) -> bool {
        self.owner == parent.owner
            && self.agent == parent.agent
            && self.service == parent.service
            && self.service_epoch == parent.service_epoch
            && self.actions.is_subset(&parent.actions)
            && self.resources.is_subset(&parent.resources)
            && self.recipients.is_subset(&parent.recipients)
            && self.budget_asset == parent.budget_asset
            && self.budget_units <= parent.budget_units
            && self.max_data_bytes <= parent.max_data_bytes
            && self.remaining_depth < parent.remaining_depth
            && (!parent.no_subcontract || self.no_subcontract)
    }
}

/// Supplied by the broker from authenticated identity and current authority state.
/// Never deserialize this context directly from an agent request.
#[derive(Clone)]
pub struct AuthContext {
    pub domain: Key,
    pub owner: Key,
    pub ownership_epoch: u64,
    pub agent: Key,
    pub service: Key,
    pub service_epoch: u64,
    pub subject_epochs: BTreeMap<Key, u64>,
    pub principal: Key,
    pub now: u64,
    pub revoked: BTreeSet<Key>,
}
struct Grant {
    claims: GrantClaims,
    document: VerifiedDocument,
    expiry: u64,
}
/// A validation result for one broker request, not a cached authorization session.
pub struct GrantChain {
    grants: Vec<Grant>,
    owner: Key,
    principal: Key,
}
impl GrantChain {
    pub fn verify(wires: &[Vec<u8>], context: &AuthContext) -> Result<Self> {
        if wires.is_empty() || wires.len() > 5 || wires.iter().any(|w| w.len() > MAX_GRANT_BYTES) {
            return Err(CapabilityError::InvalidGrant);
        }
        let mut grants: Vec<Grant> = Vec::with_capacity(wires.len());
        for wire in wires {
            let document = VerifiedDocument::decode(wire, context.domain, context.now)?;
            let expiry = document.expires_at().ok_or(CapabilityError::InvalidGrant)?;
            lifetime(document.issued_at(), expiry)?;
            if document.kind() != DocumentKind::Identity {
                return Err(CapabilityError::InvalidGrant);
            }
            let claims = wire::decode(document.body())?;
            if document.authority_epoch() != context.ownership_epoch
                || document.issued_at() > context.now
                || claims.owner != context.owner
                || claims.agent != context.agent
                || claims.service != context.service
                || claims.service_epoch != context.service_epoch
                || context.subject_epochs.get(&claims.subject) != Some(&claims.device_epoch)
                || context.revoked.contains(&document.id())
            {
                return Err(CapabilityError::Unauthorized);
            }
            match grants.last() {
                None => {
                    if document.author() != &context.owner || claims.parent.is_some() {
                        return Err(CapabilityError::Unauthorized);
                    }
                }
                Some(parent) => {
                    if claims.parent != Some(parent.document.id())
                        || document.author() != &parent.claims.subject
                        || !claims.attenuates(&parent.claims)
                        || expiry > parent.expiry
                        || document.issued_at() < parent.document.issued_at()
                    {
                        return Err(CapabilityError::Unauthorized);
                    }
                }
            }
            grants.push(Grant {
                claims,
                document,
                expiry,
            });
        }
        if grants.last().map(|g| g.claims.subject) != Some(context.principal) {
            return Err(CapabilityError::Unauthorized);
        }
        Ok(Self {
            grants,
            owner: context.owner,
            principal: context.principal,
        })
    }
    pub fn authorize(&self, request: &ScopeRequest) -> Result<()> {
        let grant = &self
            .grants
            .last()
            .ok_or(CapabilityError::InvalidGrant)?
            .claims;
        if !identifier(&request.resource, 160)
            || !identifier(&request.asset, 96)
            || !grant.actions.contains(&request.action)
            || !grant.resources.contains(&request.resource)
            || request.asset != grant.budget_asset
            || request.units > grant.budget_units
            || request.data_bytes > grant.max_data_bytes
            || (request.subcontract && grant.no_subcontract)
            || (request.units > 0 && request.action != Action::SpendPostage)
        {
            return Err(CapabilityError::Unauthorized);
        }
        match &request.recipient {
            Some(recipient)
                if !identifier(recipient, 160) || !grant.recipients.contains(recipient) =>
            {
                return Err(CapabilityError::Unauthorized);
            }
            None if matches!(request.action, Action::SendMessage | Action::SpendPostage) => {
                return Err(CapabilityError::Unauthorized);
            }
            _ => {}
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeRequest {
    pub action: Action,
    pub resource: String,
    pub recipient: Option<String>,
    pub asset: String,
    pub units: u64,
    pub data_bytes: u64,
    pub subcontract: bool,
}
pub struct PreparedDebit {
    pub states: Vec<StateChange>,
    pub already_committed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Counter {
    asset: String,
    used: u64,
}

/// Prepare only. The caller commits these changes with its actual application operation.
pub fn prepare_debit(
    store: &ProfileStore,
    chain: &GrantChain,
    operation_id: &str,
    request: &ScopeRequest,
) -> Result<PreparedDebit> {
    chain.authorize(request)?;
    if store.identity()?.public_key != chain.owner || !identifier(operation_id, 128) {
        return Err(CapabilityError::Unauthorized);
    }
    let mut key_hash = Sha256::new();
    key_hash.update(b"AgenticInternet/authorization-operation/v1\0");
    key_hash.update(chain.principal);
    key_hash.update(operation_id.as_bytes());
    let operation_namespace = format!("authorization/op/{}", hex::encode(key_hash.finalize()));
    let mut binding = Sha256::new();
    binding.update(b"AgenticInternet/authorization-binding/v1\0");
    // Fixed-width IDs bind the complete chain; the typed JSON encodes every action parameter.
    for grant in &chain.grants {
        binding.update(grant.document.id());
    }
    binding.update(serde_json::to_vec(request).map_err(|_| CapabilityError::InvalidState)?);
    let binding: [u8; 32] = binding.finalize().into();
    if let Some(saved) = store.state(&operation_namespace)? {
        if saved.bytes != binding {
            return Err(CapabilityError::IdempotencyConflict);
        }
        return Ok(PreparedDebit {
            states: Vec::new(),
            already_committed: true,
        });
    }
    let mut states = Vec::with_capacity(chain.grants.len() + 1);
    for grant in &chain.grants {
        let namespace = format!("authorization/budget/{}", hex::encode(grant.document.id()));
        let saved = store.state(&namespace)?;
        let (revision, used) = if let Some(saved) = saved {
            if saved.bytes.len() > 512 {
                return Err(CapabilityError::InvalidState);
            }
            let counter: Counter =
                serde_json::from_slice(&saved.bytes).map_err(|_| CapabilityError::InvalidState)?;
            if counter.asset != request.asset {
                return Err(CapabilityError::InvalidState);
            }
            (saved.revision, counter.used)
        } else {
            (0, 0)
        };
        let used = used
            .checked_add(request.units)
            .filter(|used| *used <= grant.claims.budget_units)
            .ok_or(CapabilityError::BudgetExceeded)?;
        states.push(StateChange {
            namespace,
            expected_revision: revision,
            bytes: serde_json::to_vec(&Counter {
                asset: request.asset.clone(),
                used,
            })
            .map_err(|_| CapabilityError::InvalidState)?,
        });
    }
    states.push(StateChange {
        namespace: operation_namespace,
        expected_revision: 0,
        bytes: binding.to_vec(),
    });
    Ok(PreparedDebit {
        states,
        already_committed: false,
    })
}
fn identifier(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '*' | '?'))
}
fn lifetime(issued: u64, expires: u64) -> Result<()> {
    if !matches!(expires.checked_sub(issued), Some(1..=MAX_LIFETIME)) {
        return Err(CapabilityError::InvalidGrant);
    }
    Ok(())
}
