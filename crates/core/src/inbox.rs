//! Durable read cursors and bounded processing leases, invoked only through the signed broker.
use super::broker::{change, scope};
use super::{AppCore, CoreError, invalid, parse_id, random_id};
use agentic_capabilities::{Action, AuthContext, GrantChain};
use agentic_store::{StateChange, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SCAN_LIMIT: usize = 1000;
/// Long enough for a reasoning agent to handle a page before its ack.
const MAX_LEASE_SECONDS: u64 = 600;
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PollRequest {
    conversation_id: String,
    operation_id: String,
    limit: usize,
    max_bytes: u64,
    lease_seconds: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AckRequest {
    conversation_id: String,
    lease_id: String,
}
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct InboxCursor {
    cursor: u64,
    active: Option<ActiveLease>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveLease {
    id: String,
    expires_at: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InboxItem {
    id: String,
    author: String,
    text: String,
    created_at: u64,
    sequence: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InboxPage {
    conversation_id: String,
    items: Vec<InboxItem>,
    cursor: u64,
    lease_id: Option<String>,
    expires_at: Option<u64>,
    has_more: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PollOperation {
    request_hash: [u8; 32],
    grant_id: [u8; 32],
    principal: [u8; 32],
    agent: [u8; 32],
    service: [u8; 32],
    page: InboxPage,
    acknowledged: bool,
}
pub(super) struct PreparedInbox {
    pub value: Value,
    pub states: Vec<StateChange>,
}
impl AppCore {
    fn check_inbox(
        &self,
        chain: &GrantChain,
        conversation: &str,
        bytes: u64,
    ) -> Result<(), CoreError> {
        chain.authorize(&scope(Action::ReadInbox, conversation, None, bytes))?;
        let (data, _) = self.data()?;
        if !data.contacts.contains_key(conversation) {
            return Err(CoreError::UnknownConversation);
        }
        Ok(())
    }
    fn poll_operation(&self, namespace: &str) -> Result<Option<(PollOperation, u64)>, CoreError> {
        self.store
            .state(namespace)?
            .map(|saved| {
                Ok((
                    serde_json::from_slice(&saved.bytes).map_err(|_| CoreError::InvalidState)?,
                    saved.revision,
                ))
            })
            .transpose()
    }
    pub(super) fn prepare_inbox_poll(
        &self,
        input: Value,
        chain: &GrantChain,
        context: &AuthContext,
        grant_id: [u8; 32],
    ) -> Result<PreparedInbox, CoreError> {
        let request: PollRequest = serde_json::from_value(input).map_err(invalid)?;
        if request.limit == 0
            || request.limit > 100
            || request.max_bytes == 0
            || request.max_bytes > 48_000
            || request.lease_seconds == 0
            || request.lease_seconds > MAX_LEASE_SECONDS
            || request.operation_id.is_empty()
            || request.operation_id.len() > 128
            || request.operation_id.chars().any(char::is_control)
        {
            return Err(CoreError::InvalidInput);
        }
        self.check_inbox(chain, &request.conversation_id, request.max_bytes)?;
        let operation_namespace = namespace(
            "operation",
            &[&context.principal, request.operation_id.as_bytes()],
        );
        let mut binding = Sha256::new();
        binding.update(b"AgenticInternet/inbox-poll/v1\0");
        binding.update(grant_id);
        binding.update(serde_json::to_vec(&request).map_err(invalid)?);
        let request_hash = binding.finalize().into();
        if let Some((operation, _)) = self.poll_operation(&operation_namespace)? {
            if operation.request_hash != request_hash {
                return Err(StoreError::IdempotencyConflict.into());
            }
            owns_operation(&operation, context, &grant_id, &request.conversation_id)?;
            return Ok(PreparedInbox {
                value: json!(operation.page),
                states: vec![],
            });
        }
        let cursor_namespace = cursor_namespace(context, &request.conversation_id);
        let (mut inbox, revision) = self.authorization_state::<InboxCursor>(&cursor_namespace)?;
        if inbox
            .active
            .as_ref()
            .is_some_and(|active| active.expires_at > context.now)
        {
            return Err(CoreError::InboxBusy);
        }
        let records = self
            .store
            .messages(&request.conversation_id, inbox.cursor, SCAN_LIMIT)?;
        let mut page = InboxPage {
            conversation_id: request.conversation_id,
            items: vec![],
            cursor: inbox.cursor,
            lease_id: None,
            expires_at: None,
            has_more: records.len() == SCAN_LIMIT,
        };
        let mut byte_count = 0;
        for stored in records {
            if !stored.record.own
                && let Some(message) = self.present_message(&stored)?
            {
                let bytes = message.text.len() as u64;
                if page.items.len() >= request.limit || bytes > request.max_bytes - byte_count {
                    if page.items.is_empty() {
                        return Err(CoreError::InboxItemTooLarge {
                            required_bytes: bytes,
                        });
                    }
                    page.has_more = true;
                    break;
                }
                byte_count += bytes;
                page.items.push(InboxItem {
                    id: message.id,
                    author: message.author,
                    text: message.text,
                    created_at: message.created_at,
                    sequence: stored.sequence,
                });
            }
            // Skipped owner/control records also need an acknowledged cursor to avoid an endless scan.
            page.cursor = stored.sequence;
        }
        inbox.active = None;
        let mut states = Vec::new();
        if page.cursor > inbox.cursor {
            let lease_id = hex::encode(random_id()?);
            let expires_at = context
                .now
                .checked_add(request.lease_seconds)
                .ok_or(CoreError::InvalidInput)?;
            inbox.active = Some(ActiveLease {
                id: lease_id.clone(),
                expires_at,
            });
            page.lease_id = Some(lease_id.clone());
            page.expires_at = Some(expires_at);
            states.push(StateChange {
                namespace: lease_namespace(&lease_id),
                expected_revision: 0,
                bytes: operation_namespace.as_bytes().to_vec(),
            });
        }
        let operation = PollOperation {
            request_hash,
            grant_id,
            principal: context.principal,
            agent: context.agent,
            service: context.service,
            page,
            acknowledged: false,
        };
        let value = json!(operation.page);
        states.push(change(&cursor_namespace, revision, &inbox)?);
        states.push(change(&operation_namespace, 0, &operation)?);
        Ok(PreparedInbox { value, states })
    }
    pub(super) fn prepare_inbox_ack(
        &self,
        input: Value,
        chain: &GrantChain,
        context: &AuthContext,
        grant_id: [u8; 32],
    ) -> Result<PreparedInbox, CoreError> {
        let request: AckRequest = serde_json::from_value(input).map_err(invalid)?;
        self.check_inbox(chain, &request.conversation_id, 0)?;
        // Require the canonical hex token returned by poll, not a path or arbitrary state namespace.
        if hex::encode(parse_id(&request.lease_id)?) != request.lease_id {
            return Err(CoreError::InvalidInput);
        }
        let index = self
            .store
            .state(&lease_namespace(&request.lease_id))?
            .ok_or(CoreError::Unauthorized)?;
        let operation_namespace =
            std::str::from_utf8(&index.bytes).map_err(|_| CoreError::InvalidState)?;
        if !operation_namespace.starts_with("authorization/inbox/operation/") {
            return Err(CoreError::InvalidState);
        }
        let (mut operation, operation_revision) = self
            .poll_operation(operation_namespace)?
            .ok_or(CoreError::InvalidState)?;
        owns_operation(&operation, context, &grant_id, &request.conversation_id)?;
        if operation.page.lease_id.as_ref() != Some(&request.lease_id) {
            return Err(CoreError::Unauthorized);
        }
        let value = json!({"leaseId":request.lease_id,"cursor":operation.page.cursor});
        if operation.acknowledged {
            return Ok(PreparedInbox {
                value,
                states: vec![],
            });
        }
        let cursor_namespace = cursor_namespace(context, &request.conversation_id);
        let (mut inbox, revision) = self.authorization_state::<InboxCursor>(&cursor_namespace)?;
        let active = inbox.active.as_ref().ok_or(CoreError::InboxLeaseExpired)?;
        if active.id != request.lease_id || active.expires_at <= context.now {
            return Err(CoreError::InboxLeaseExpired);
        }
        if operation.page.cursor <= inbox.cursor {
            return Err(CoreError::InvalidState);
        }
        inbox.cursor = operation.page.cursor;
        inbox.active = None;
        operation.acknowledged = true;
        Ok(PreparedInbox {
            value,
            states: vec![
                change(&cursor_namespace, revision, &inbox)?,
                change(operation_namespace, operation_revision, &operation)?,
            ],
        })
    }
}
fn owns_operation(
    operation: &PollOperation,
    context: &AuthContext,
    grant: &[u8; 32],
    conversation: &str,
) -> Result<(), CoreError> {
    if &operation.grant_id != grant
        || operation.principal != context.principal
        || operation.agent != context.agent
        || operation.service != context.service
        || operation.page.conversation_id != conversation
    {
        return Err(CoreError::Unauthorized);
    }
    Ok(())
}
fn namespace(kind: &str, pieces: &[&[u8]]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"AgenticInternet/inbox-state/v1\0");
    for piece in pieces {
        hash.update((piece.len() as u64).to_be_bytes());
        hash.update(piece);
    }
    format!(
        "authorization/inbox/{kind}/{}",
        hex::encode(hash.finalize())
    )
}
fn cursor_namespace(context: &AuthContext, conversation: &str) -> String {
    namespace(
        "cursor",
        &[&context.agent, &context.service, conversation.as_bytes()],
    )
}
fn lease_namespace(id: &str) -> String {
    format!("authorization/inbox/lease/{id}")
}
