//! Read-only, bounded views for the local owner desktop. Agent inbox leases are separate.
use super::{AppCore, Conversation, CoreError, Identity, Message, NetworkStatus, parse_id};
use serde::Serialize;

const PAGE_MESSAGES: usize = 50;
const PAGE_BYTES: usize = 512 * 1024;
const CONTACTS: usize = 32;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopOverview {
    pub identity: Option<Identity>,
    pub network: NetworkStatus,
    pub conversations: Vec<Conversation>,
    pub next_after: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationHistory {
    pub conversation_id: String,
    pub messages: Vec<Message>,
    pub next_before: Option<String>,
}

impl AppCore {
    pub fn desktop_revision(&self) -> u64 {
        self.store.change_count()
    }

    pub fn desktop_overview(&self, after: Option<&str>) -> Result<DesktopOverview, CoreError> {
        if let Some(cursor) = after {
            parse_id(cursor)?;
        }
        let (data, _) = self.data()?;
        // Contacts, groups and follows in one id order, so one cursor pages
        // them all. A follow keeps no read mark: it shows nothing unread.
        let mut listed: std::collections::BTreeMap<String, (String, Option<u64>)> = data
            .contacts
            .iter()
            .map(|(id, contact)| {
                (
                    id.clone(),
                    (contact.title.clone(), Some(contact.read_through)),
                )
            })
            .collect();
        for id in self.group_ids()? {
            if let Some((title, read_through)) = self.group_listing(&id)? {
                listed.insert(id, (title, Some(read_through)));
            }
        }
        for id in self.follow_ids()? {
            if let Some(title) = self.follow_name(&id)? {
                listed.insert(id, (title, None));
            }
        }
        let mut contacts = listed
            .iter()
            .filter(|(id, _)| after.is_none_or(|cursor| id.as_str() > cursor));
        let mut conversations = Vec::new();
        for (id, (title, read_through)) in contacts.by_ref().take(CONTACTS) {
            let mut messages = Vec::new();
            if let Some(stored) = self
                .store
                .event_messages_before(id, "text", None, 1)?
                .first()
            {
                let mut message = self
                    .present_message(stored)?
                    .ok_or(CoreError::InvalidState)?;
                let mut chars = message.text.chars();
                let mut preview = chars.by_ref().take(256).collect::<String>();
                if chars.next().is_some() {
                    preview.push('…');
                }
                message.text = preview;
                messages.push(message);
            }
            conversations.push(Conversation {
                id: id.clone(),
                title: title.clone(),
                messages,
                unread: match read_through {
                    Some(read_through) => self
                        .store
                        .unread_events(id, "text", *read_through)?
                        .min(u32::MAX as u64) as u32,
                    None => 0,
                },
            });
        }
        let next_after = if contacts.next().is_some() {
            conversations.last().map(|c| c.id.clone())
        } else {
            None
        };
        Ok(DesktopOverview {
            identity: data
                .name
                .as_ref()
                .map(|_| self.identity_for(&data))
                .transpose()?,
            network: NetworkStatus {
                connected_peers: 0,
                state: "offline".into(),
            },
            conversations,
            next_after,
        })
    }

    pub fn conversation_history(
        &self,
        conversation: &str,
        before: Option<&str>,
    ) -> Result<ConversationHistory, CoreError> {
        let (data, _) = self.data()?;
        if !data.contacts.contains_key(conversation)
            && self.group_listing(conversation)?.is_none()
            && !self.is_follow(conversation)?
        {
            return Err(CoreError::UnknownConversation);
        }
        let before = before
            .map(|id| {
                parse_id(id)?;
                let stored = self.store.message(id)?.ok_or(CoreError::InvalidInput)?;
                if stored.record.conversation_id != conversation
                    || self.present_message(&stored)?.is_none()
                {
                    return Err(CoreError::InvalidInput);
                }
                Ok(stored.sequence)
            })
            .transpose()?;
        let records =
            self.store
                .event_messages_before(conversation, "text", before, PAGE_MESSAGES + 1)?;
        let mut page = ConversationHistory {
            conversation_id: conversation.into(),
            messages: vec![],
            next_before: None,
        };
        // Reserve the wrapper, comma separators and a full cursor. Account for JSON
        // escaping by measuring encoded messages, rather than just UTF-8 text bytes.
        let mut bytes = serde_json::to_vec(&page)
            .map_err(|_| CoreError::InvalidState)?
            .len()
            + 128;
        let mut more = false;
        for stored in records {
            let message = self
                .present_message(&stored)?
                .ok_or(CoreError::InvalidState)?;
            let size = serde_json::to_vec(&message)
                .map_err(|_| CoreError::InvalidState)?
                .len()
                + 1;
            if page.messages.len() == PAGE_MESSAGES || bytes + size > PAGE_BYTES {
                more = true;
                break;
            }
            bytes += size;
            page.messages.push(message);
        }
        if more {
            // One valid wire message always fits the page budget.
            page.next_before = Some(
                page.messages
                    .last()
                    .ok_or(CoreError::InvalidState)?
                    .id
                    .clone(),
            );
        }
        page.messages.reverse();
        Ok(page)
    }
}
