//! The owner's inbox (spec/owner-cli-v1.md): a processing cursor per
//! conversation with one lease, apart from the history and every agent's
//! cursor. A lease keeps the range it covers, so a repeated poll rebuilds
//! the same page.
use super::broker::change;
use super::{AppCore, CoreError, parse_id, random_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCAN_LIMIT: usize = 1000;
const MAX_LIMIT: usize = 100;
const MAX_LEASE_SECONDS: u64 = 600;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerInboxItem {
    pub id: String,
    pub author: String,
    pub text: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerInboxPage {
    pub conversation_id: String,
    pub items: Vec<OwnerInboxItem>,
    pub lease_id: Option<String>,
    pub expires_at: Option<u64>,
    pub has_more: bool,
}

/// A conversation with unacknowledged incoming messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerUnread {
    pub conversation_id: String,
    pub unread: u64,
    /// Under an active lease: someone is processing it.
    pub leased: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    cursor: u64,
    lease: Option<Lease>,
    acknowledged: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lease {
    id: String,
    expires_at: u64,
    /// The last sequence the page covers.
    end: u64,
    has_more: bool,
}

/// What a scan after the cursor found.
struct Scan {
    items: Vec<OwnerInboxItem>,
    end: u64,
    has_more: bool,
}

fn namespace(conversation: &str) -> String {
    format!(
        "owner/inbox/{}",
        hex::encode(Sha256::digest(conversation.as_bytes()))
    )
}

impl AppCore {
    fn owner_cursor(&self, conversation: &str) -> Result<(Cursor, u64), CoreError> {
        let (data, _) = self.data()?;
        if !data.contacts.contains_key(conversation)
            && !self.is_group(conversation)?
            && !self.is_follow(conversation)?
        {
            return Err(CoreError::UnknownConversation);
        }
        self.authorization_state(&namespace(conversation))
    }

    /// The incoming text messages after `cursor`, at most `limit`, up to the
    /// sequence `until` when given. Records passed over (the owner's own,
    /// control) are covered by `end` too.
    fn owner_scan(
        &self,
        conversation: &str,
        cursor: u64,
        limit: usize,
        until: Option<u64>,
    ) -> Result<Scan, CoreError> {
        let records = self.store.messages(conversation, cursor, SCAN_LIMIT)?;
        let mut scan = Scan {
            items: vec![],
            end: cursor,
            has_more: records.len() == SCAN_LIMIT,
        };
        for stored in records {
            if until.is_some_and(|until| stored.sequence > until) {
                break;
            }
            if !stored.record.own
                && let Some(message) = self.present_message(&stored)?
            {
                if scan.items.len() == limit {
                    scan.has_more = true;
                    break;
                }
                scan.items.push(OwnerInboxItem {
                    id: message.id,
                    author: message.author,
                    text: message.text,
                    created_at: message.created_at,
                });
            }
            scan.end = stored.sequence;
        }
        Ok(scan)
    }

    /// Conversations and groups with unacknowledged incoming messages
    /// (counted up to a thousand), contacts first.
    pub fn owner_inbox_unread(&self, now: u64) -> Result<Vec<OwnerUnread>, CoreError> {
        let (data, _) = self.data()?;
        let mut found = Vec::new();
        for conversation in data
            .contacts
            .keys()
            .cloned()
            .chain(self.group_ids()?)
            .chain(self.follow_ids()?)
        {
            let (state, _) = self.authorization_state::<Cursor>(&namespace(&conversation))?;
            let scan = self.owner_scan(&conversation, state.cursor, SCAN_LIMIT, None)?;
            if !scan.items.is_empty() {
                found.push(OwnerUnread {
                    conversation_id: conversation,
                    unread: scan.items.len() as u64,
                    leased: state.lease.is_some_and(|lease| lease.expires_at > now),
                });
            }
        }
        Ok(found)
    }

    /// The next incoming messages under a lease; the same page while the
    /// lease is active. An empty page takes no lease.
    pub fn owner_inbox_poll(
        &mut self,
        conversation: &str,
        limit: usize,
        lease_seconds: u64,
        now: u64,
    ) -> Result<OwnerInboxPage, CoreError> {
        if !(1..=MAX_LIMIT).contains(&limit) || !(1..=MAX_LEASE_SECONDS).contains(&lease_seconds) {
            return Err(CoreError::InvalidInput);
        }
        let (mut state, revision) = self.owner_cursor(conversation)?;
        if let Some(lease) = state.lease.as_ref().filter(|lease| lease.expires_at > now) {
            let scan = self.owner_scan(conversation, state.cursor, SCAN_LIMIT, Some(lease.end))?;
            return Ok(OwnerInboxPage {
                conversation_id: conversation.into(),
                items: scan.items,
                lease_id: Some(lease.id.clone()),
                expires_at: Some(lease.expires_at),
                has_more: lease.has_more,
            });
        }
        let scan = self.owner_scan(conversation, state.cursor, limit, None)?;
        let mut page = OwnerInboxPage {
            conversation_id: conversation.into(),
            items: scan.items,
            lease_id: None,
            expires_at: None,
            has_more: scan.has_more,
        };
        if page.items.is_empty() {
            // Only records passed over: move past them without a lease.
            if scan.end > state.cursor {
                state.cursor = scan.end;
                state.lease = None;
                self.store.commit_states(vec![change(
                    &namespace(conversation),
                    revision,
                    &state,
                )?])?;
            }
            return Ok(page);
        }
        let lease = Lease {
            id: hex::encode(random_id()?),
            expires_at: now
                .checked_add(lease_seconds)
                .ok_or(CoreError::InvalidInput)?,
            end: scan.end,
            has_more: scan.has_more,
        };
        page.lease_id = Some(lease.id.clone());
        page.expires_at = Some(lease.expires_at);
        state.lease = Some(lease);
        self.store
            .commit_states(vec![change(&namespace(conversation), revision, &state)?])?;
        Ok(page)
    }

    /// Move the cursor past a leased page. Idempotent; an expired lease still
    /// counts until a later poll replaces it.
    pub fn owner_inbox_ack(
        &mut self,
        conversation: &str,
        lease_id: &str,
        _now: u64,
    ) -> Result<(), CoreError> {
        let (mut state, revision) = self.owner_cursor(conversation)?;
        if hex::encode(parse_id(lease_id)?) != lease_id {
            return Err(CoreError::InvalidInput);
        }
        match state.lease.take() {
            Some(lease) if lease.id == lease_id => {
                state.cursor = lease.end;
                state.acknowledged = Some(lease.id);
                self.store.commit_states(vec![change(
                    &namespace(conversation),
                    revision,
                    &state,
                )?])?;
                Ok(())
            }
            _ if state.acknowledged.as_deref() == Some(lease_id) => Ok(()),
            _ => Err(CoreError::InboxLeaseExpired),
        }
    }
}
