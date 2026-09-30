//! The trusted shared boundary between runtime signatures and owner application operations.
use super::{AppCore, CoreError, invalid, network_id, valid_name, valid_text};
use agentic_capabilities::{
    Action, AuthContext, GrantChain, GrantClaims, ScopeRequest, prepare_debit,
};
use agentic_protocol::{DocumentDraft, DocumentKind, VerifiedDocument};
use agentic_store::StateChange;
use minicbor::{Decoder, Encoder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[path = "runtime_provisioning.rs"]
mod provisioning;
pub use provisioning::{ProvisionRuntimeRequest, ProvisionedRuntime, RuntimeInfo};

const REGISTRY: &str = "authorization/registry";
const NONCES: &str = "authorization/nonces";
const PROOF_LIFETIME: u64 = 30;
const MAX_REGISTRATIONS: usize = 1024;
const MAX_ACTIVE_NONCES: usize = 4096;
const MAX_CONTEXT_BYTES: usize = 16 * 1024;
const POSTAGE_ASSET: &str = "transport-credit-v1";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeGrantRequest {
    pub name: String,
    pub principal: [u8; 32],
    pub agent_id: [u8; 32],
    pub service_id: [u8; 32],
    pub conversation_ids: Vec<String>,
    pub actions: BTreeSet<Action>,
    pub expires_at: u64,
    pub max_data_bytes: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeGrant {
    pub grant_id: [u8; 32],
    pub wire: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    request: RuntimeGrantRequest,
    grant: RuntimeGrant,
    device_epoch: u64,
    service_epoch: u64,
    revoked: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u8,
    ownership_epoch: u64,
    runtimes: BTreeMap<String, Registration>,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            version: 1,
            ownership_epoch: 0,
            runtimes: BTreeMap::new(),
        }
    }
}

/// Public request material. Only the signature verified by the broker establishes the principal.
pub struct AgentCall {
    pub grant_id: [u8; 32],
    pub method: String,
    pub request: Value,
    pub nonce: [u8; 32],
}
impl AgentCall {
    pub fn draft(
        &self,
        domain: [u8; 32],
        ownership_epoch: u64,
        now: u64,
    ) -> Result<DocumentDraft, CoreError> {
        Ok(DocumentDraft {
            domain,
            kind: DocumentKind::Identity,
            authority_epoch: ownership_epoch,
            issued_at: now,
            expires_at: Some(
                now.checked_add(PROOF_LIFETIME)
                    .ok_or(CoreError::InvalidInput)?,
            ),
            body: self.body()?,
            extensions: BTreeMap::new(),
        })
    }
    fn body(&self) -> Result<Vec<u8>, CoreError> {
        if self.method.is_empty()
            || self.method.len() > 64
            || !self
                .method
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c == b'_')
            || !self.request.is_object()
        {
            return Err(CoreError::InvalidInput);
        }
        let mut request = self.request.clone();
        request.sort_all_objects();
        let request = serde_json::to_vec(&request).map_err(invalid)?;
        let mut e = Encoder::new(Vec::new());
        e.array(5)
            .map_err(invalid)?
            .u8(3)
            .map_err(invalid)?
            .bytes(&self.grant_id)
            .map_err(invalid)?
            .str(&self.method)
            .map_err(invalid)?
            .bytes(&request)
            .map_err(invalid)?
            .bytes(&self.nonce)
            .map_err(invalid)?;
        Ok(e.into_writer())
    }
    fn decode(document: &VerifiedDocument, now: u64) -> Result<Self, CoreError> {
        let expiry = document.expires_at().ok_or(CoreError::Unauthorized)?;
        if document.kind() != DocumentKind::Identity
            || document.issued_at() > now
            || expiry <= now
            || expiry <= document.issued_at()
            || expiry - document.issued_at() > PROOF_LIFETIME
            || !document.extensions().is_empty()
        {
            return Err(CoreError::Unauthorized);
        }
        let mut d = Decoder::new(document.body());
        if d.array().map_err(invalid)? != Some(5) || d.u8().map_err(invalid)? != 3 {
            return Err(CoreError::InvalidInput);
        }
        let call = Self {
            grant_id: d.bytes().map_err(invalid)?.try_into().map_err(invalid)?,
            method: d.str().map_err(invalid)?.into(),
            request: serde_json::from_slice(d.bytes().map_err(invalid)?).map_err(invalid)?,
            nonce: d.bytes().map_err(invalid)?.try_into().map_err(invalid)?,
        };
        if d.position() != document.body().len() || call.body()? != document.body() {
            return Err(CoreError::InvalidInput);
        }
        Ok(call)
    }
}

#[derive(Default)]
pub(super) struct AuthorizationChanges {
    pub new_operation: Vec<StateChange>,
    pub retry: Vec<StateChange>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeContextRequest {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SendRequest {
    conversation_id: String,
    text: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeliveryRequest {
    conversation_id: String,
    operation_id: String,
}

impl AppCore {
    pub(super) fn authorization_state<T: DeserializeOwned + Default>(
        &self,
        namespace: &str,
    ) -> Result<(T, u64), CoreError> {
        match self.store.state(namespace)? {
            None => Ok((T::default(), 0)),
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
        }
    }
    fn registry(&self) -> Result<(Registry, u64), CoreError> {
        let (registry, revision) = self.authorization_state::<Registry>(REGISTRY)?;
        if registry.version != 1 || registry.runtimes.len() > MAX_REGISTRATIONS {
            return Err(CoreError::InvalidState);
        }
        Ok((registry, revision))
    }
    /// Owner service operation; adapters must authenticate the owner before calling this method.
    pub fn grant_runtime(
        &mut self,
        request: RuntimeGrantRequest,
        now: u64,
    ) -> Result<RuntimeGrant, CoreError> {
        let (grant, state) = self.prepare_runtime_grant(request, now)?;
        self.store.commit_states(vec![state])?;
        Ok(grant)
    }
    fn prepare_runtime_grant(
        &self,
        request: RuntimeGrantRequest,
        now: u64,
    ) -> Result<(RuntimeGrant, StateChange), CoreError> {
        valid_name(&request.name)?;
        if request.conversation_ids.is_empty()
            || request.conversation_ids.len() > 32
            || request.actions.is_empty()
            || !request
                .actions
                .iter()
                .all(|a| matches!(a, Action::ReadInbox | Action::SendMessage))
            || request.expires_at <= now
            || request.max_data_bytes == 0
            || request.max_data_bytes > 48_000
        {
            return Err(CoreError::InvalidInput);
        }
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        let (mut registry, revision) = self.registry()?;
        if registry.runtimes.len() >= MAX_REGISTRATIONS
            || registry.runtimes.values().any(|r| {
                r.request.principal == request.principal && !r.revoked && r.request.expires_at > now
            })
        {
            return Err(CoreError::Unauthorized);
        }
        let mut resources = BTreeSet::new();
        let mut recipients = BTreeSet::new();
        for id in &request.conversation_ids {
            let contact = data
                .contacts
                .get(id)
                .ok_or(CoreError::UnknownConversation)?;
            if !resources.insert(format!("conversation:{id}")) {
                return Err(CoreError::InvalidInput);
            }
            recipients.insert(network_id(&contact.root));
        }
        let claims = GrantClaims {
            owner: self.store.identity()?.public_key,
            agent: request.agent_id,
            service: request.service_id,
            subject: request.principal,
            parent: None,
            device_epoch: 1,
            service_epoch: 1,
            actions: request.actions.clone(),
            resources,
            recipients,
            budget_asset: POSTAGE_ASSET.into(),
            budget_units: 0,
            max_data_bytes: request.max_data_bytes,
            remaining_depth: 0,
            no_subcontract: true,
        };
        let signed = self.store.sign_document(claims.draft(
            self.domain,
            registry.ownership_epoch,
            now,
            request.expires_at,
        )?)?;
        let wire = signed.to_wire();
        let grant = RuntimeGrant {
            grant_id: Sha256::digest(&wire).into(),
            wire,
        };
        // A revoked document must not be resurrected by reissuing identical claims in the same second.
        if registry.runtimes.contains_key(&hex::encode(grant.grant_id)) {
            return Err(CoreError::Unauthorized);
        }
        registry.runtimes.insert(
            hex::encode(grant.grant_id),
            Registration {
                request,
                grant: grant.clone(),
                device_epoch: 1,
                service_epoch: 1,
                revoked: false,
            },
        );
        Ok((grant, change(REGISTRY, revision, &registry)?))
    }
    pub fn revoke_runtime(&mut self, grant_id: [u8; 32], _now: u64) -> Result<(), CoreError> {
        let (mut registry, revision) = self.registry()?;
        let registration = registry
            .runtimes
            .get_mut(&hex::encode(grant_id))
            .ok_or(CoreError::Unauthorized)?;
        if registration.revoked {
            return Ok(());
        }
        registration.revoked = true;
        self.store
            .commit_states(vec![change(REGISTRY, revision, &registry)?])?;
        Ok(())
    }
    fn runtime_authority(
        &self,
        registry: &Registry,
        registration: &Registration,
        principal: [u8; 32],
        now: u64,
    ) -> Result<(AuthContext, GrantChain), CoreError> {
        if registration.revoked
            || Sha256::digest(&registration.grant.wire).as_slice() != registration.grant.grant_id
        {
            return Err(CoreError::Unauthorized);
        }
        let context = AuthContext {
            domain: self.domain,
            owner: self.store.identity()?.public_key,
            ownership_epoch: registry.ownership_epoch,
            agent: registration.request.agent_id,
            service: registration.request.service_id,
            service_epoch: registration.service_epoch,
            subject_epochs: [(registration.request.principal, registration.device_epoch)].into(),
            principal,
            now,
            revoked: BTreeSet::new(),
        };
        let chain = GrantChain::verify(std::slice::from_ref(&registration.grant.wire), &context)?;
        Ok((context, chain))
    }

    pub fn agent_call(&mut self, wire: &[u8], now: u64) -> Result<Value, CoreError> {
        let document = VerifiedDocument::decode(wire, self.domain, now)?;
        let call = AgentCall::decode(&document, now)?;
        let (registry, registry_revision) = self.registry()?;
        let registration = registry
            .runtimes
            .get(&hex::encode(call.grant_id))
            .ok_or(CoreError::Unauthorized)?;
        if registration.revoked
            || document.authority_epoch() != registry.ownership_epoch
            || call.grant_id != registration.grant.grant_id
        {
            return Err(CoreError::Unauthorized);
        }
        let (context, chain) =
            self.runtime_authority(&registry, registration, *document.author(), now)?;
        let (mut nonces, nonce_revision) =
            self.authorization_state::<BTreeMap<String, u64>>(NONCES)?;
        nonces.retain(|_, expiry| *expiry > now);
        let nonce_id = hex::encode(Sha256::digest([call.grant_id, call.nonce].concat()));
        if nonces.contains_key(&nonce_id) || nonces.len() >= MAX_ACTIVE_NONCES {
            return Err(CoreError::Unauthorized);
        }
        nonces.insert(
            nonce_id,
            document.expires_at().ok_or(CoreError::Unauthorized)?,
        );
        let fence = vec![
            change(REGISTRY, registry_revision, &registry)?,
            change(NONCES, nonce_revision, &nonces)?,
        ];
        let (data, _) = self.data()?;
        self.identity_for(&data)?;
        match call.method.as_str() {
            "inbox_poll" | "inbox_ack" => {
                let prepared = if call.method == "inbox_poll" {
                    self.prepare_inbox_poll(call.request, &chain, &context, call.grant_id)?
                } else {
                    self.prepare_inbox_ack(call.request, &chain, &context, call.grant_id)?
                };
                let mut states = fence;
                states.extend(prepared.states);
                self.store.commit_states(states)?;
                Ok(prepared.value)
            }
            "runtime_context" | "snapshot" => {
                let _: RuntimeContextRequest =
                    serde_json::from_value(call.request).map_err(invalid)?;
                let can_read = registration.request.actions.contains(&Action::ReadInbox);
                if call.method == "snapshot" && !can_read {
                    return Err(CoreError::Unauthorized);
                }
                let action = if can_read {
                    Action::ReadInbox
                } else {
                    *registration
                        .request
                        .actions
                        .first()
                        .ok_or(CoreError::Unauthorized)?
                };
                let mut conversations = Vec::new();
                for id in &registration.request.conversation_ids {
                    let contact = data
                        .contacts
                        .get(id)
                        .ok_or(CoreError::UnknownConversation)?;
                    // Authorize metadata discovery without loading or exposing message history.
                    chain.authorize(&scope(action, id, Some(network_id(&contact.root)), 0))?;
                    conversations.push(json!({"id":id,"title":contact.title,"networkId":network_id(&contact.root)}));
                }
                let snapshot = json!({
                    "version":1,"networkId":network_id(&context.owner),"grantId":hex::encode(call.grant_id),
                    "agentId":hex::encode(context.agent),"serviceId":hex::encode(context.service),
                    "principal":hex::encode(context.principal),
                    "expiresAt":registration.request.expires_at,
                    "maxDataBytes":registration.request.max_data_bytes,
                    "actions":registration.request.actions,"conversations":conversations
                });
                // Scope metadata has its own finite bound, independent of the message text limit.
                if serde_json::to_vec(&snapshot).map_err(invalid)?.len() > MAX_CONTEXT_BYTES {
                    return Err(CoreError::InvalidState);
                }
                self.store.commit_states(fence)?;
                Ok(snapshot)
            }
            "delivery_get" => {
                let request: DeliveryRequest =
                    serde_json::from_value(call.request).map_err(invalid)?;
                if request.operation_id.is_empty() || request.operation_id.len() > 128 {
                    return Err(CoreError::InvalidInput);
                }
                let contact = data
                    .contacts
                    .get(&request.conversation_id)
                    .ok_or(CoreError::Unauthorized)?;
                chain.authorize(&scope(
                    Action::SendMessage,
                    &request.conversation_id,
                    Some(network_id(&contact.root)),
                    0,
                ))?;
                let stored = self
                    .store
                    .operation_message(&agent_operation_id(
                        &context.principal,
                        &request.operation_id,
                    ))?
                    .ok_or(CoreError::Unauthorized)?;
                if stored.record.conversation_id != request.conversation_id
                    || !stored.record.own
                    || stored.record.author != network_id(&context.owner)
                {
                    return Err(CoreError::Unauthorized);
                }
                let result = json!({"conversationId":request.conversation_id,"operationId":request.operation_id,"messageId":stored.record.id,"delivery":self.delivery_status(&stored.record)?});
                self.store.commit_states(fence)?;
                Ok(result)
            }
            "send_message" => {
                let request: SendRequest = serde_json::from_value(call.request).map_err(invalid)?;
                valid_text(&request.text)?;
                let contact = data
                    .contacts
                    .get(&request.conversation_id)
                    .ok_or(CoreError::UnknownConversation)?;
                let scope = scope(
                    Action::SendMessage,
                    &request.conversation_id,
                    Some(network_id(&contact.root)),
                    request.text.len() as u64,
                );
                let debit = prepare_debit(&self.store, &chain, &request.operation_id, &scope)?;
                let mut states = fence.clone();
                states.extend(debit.states);
                let operation_id = agent_operation_id(&context.principal, &request.operation_id);
                let message = self.send_authorized(
                    &request.conversation_id,
                    &request.text,
                    &operation_id,
                    now,
                    AuthorizationChanges {
                        new_operation: states,
                        retry: fence,
                    },
                )?;
                serde_json::to_value(message).map_err(|_| CoreError::InvalidState)
            }
            _ => Err(CoreError::Unauthorized),
        }
    }
}
fn agent_operation_id(principal: &[u8; 32], operation_id: &str) -> String {
    let mut operation = Sha256::new();
    operation.update(b"AgenticInternet/agent-message-operation/v1\0");
    operation.update(principal);
    operation.update(operation_id.as_bytes());
    format!("agent/{}", hex::encode(operation.finalize()))
}
pub(super) fn change<T: Serialize>(
    namespace: &str,
    expected_revision: u64,
    value: &T,
) -> Result<StateChange, CoreError> {
    Ok(StateChange {
        namespace: namespace.into(),
        expected_revision,
        bytes: serde_json::to_vec(value).map_err(|_| CoreError::InvalidState)?,
    })
}
pub(super) fn scope(
    action: Action,
    conversation: &str,
    recipient: Option<String>,
    data_bytes: u64,
) -> ScopeRequest {
    ScopeRequest {
        action,
        resource: format!("conversation:{conversation}"),
        recipient,
        asset: POSTAGE_ASSET.into(),
        units: 0,
        data_bytes,
        subcontract: false,
    }
}
