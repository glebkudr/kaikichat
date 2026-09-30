//! Shared trusted Rust conversation service. All crypto state changes commit with durable work.
use agentic_crypto::{MlsClient, SecretState, inspect_key_package};
use agentic_protocol::{DocumentDraft, DocumentKind, VerifiedDocument, network_id};
use agentic_store::{
    IncomingCommit, MessageRecord, OutgoingCommit, ProfileStore, StateChange, StateRecordBatch,
    StateRecordChange, StoredMessage,
};
use minicbor::{Decoder, Encoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;
mod agent_failure;
mod broker;
mod inbox;
mod mailbox_swarm;
pub use mailbox_swarm::{
    DirectPayment, EnvelopeOrder, MailboxAccess, MailboxBook, MailboxClaim, MailboxPurchase,
    SwarmDelivery, SwarmPending, SwarmReceived,
};
mod door;
pub use door::{DoorApplication, DoorEntry, DoorInfo, DoorRequest};
mod group_parts;
mod groups;
pub use groups::{
    BannedId, CommitDecision, GroupChange, GroupClaim, GroupCommitMade, GroupInfo, Invitee,
    StaleGroup,
};
mod public_groups;
pub use public_groups::{FollowInfo, PUBLIC_POST, PUBLIC_ROSTER, PublicEntry};
mod channels;
pub use channels::{ChannelStorage, PUBLIC_ARCHIVE};
mod closed_channels;
mod intro;
mod key_tree;
mod network_preferences;
pub use intro::{IntroCard, IntroCardInfo, IntroMode, IntroOutcome, IntroPolicy, IntroRequest};
mod owner_inbox;
pub use owner_inbox::{OwnerInboxItem, OwnerInboxPage, OwnerUnread};
mod peer_records;
mod transport_binding;
pub use agent_failure::AgentFailure;
pub use broker::{
    AgentCall, ProvisionRuntimeRequest, ProvisionedRuntime, RuntimeGrant, RuntimeGrantRequest,
    RuntimeInfo,
};
pub use network_preferences::{NetworkPreferences, StoredNetworkPreferences};
pub use transport_binding::NodeRecord;

const APP_STATE: &str = "application";
const MLS_STATE: &str = "mls";
const INVITE_PREFIX: &str = "ain-invite1:";
const INVITE_LIFETIME: u64 = 7 * 86400;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("profile has not been created")]
    NoProfile,
    #[error("profile already exists; use profile settings to rename it")]
    ProfileExists,
    #[error("invalid input or application packet")]
    InvalidInput,
    #[error("profile or packet belongs to another network")]
    WrongNetwork,
    #[error("invalid persisted application state")]
    InvalidState,
    #[error("original outgoing packet is unavailable")]
    OriginalPacketUnavailable,
    #[error("unknown conversation")]
    UnknownConversation,
    #[error("invitation is missing, expired or already consumed")]
    InvalidInvitation,
    #[error("root signature and MLS participant do not match")]
    Unauthorized,
    #[error("this conversation has no active stamp book")]
    MailboxBookMissing,
    #[error("every stamp of this conversation's book is spent")]
    MailboxBookExhausted,
    #[error("this conversation's stamp book has expired")]
    MailboxBookExpired,
    #[error("a message needs a stamp at this node")]
    PaymentRequired,
    #[error("a commit of this group waits for the notary")]
    GroupBusy,
    #[error("this id is banned in the group; lift the ban first")]
    Banned,
    #[error("operating system randomness unavailable")]
    Randomness,
    #[error("this inbox already has an active processing lease")]
    InboxBusy,
    #[error("inbox processing lease expired or was replaced")]
    InboxLeaseExpired,
    #[error("the next inbox item requires at least {required_bytes} bytes")]
    InboxItemTooLarge { required_bytes: u64 },
    #[error(transparent)]
    Store(#[from] agentic_store::StoreError),
    #[error(transparent)]
    Wire(#[from] agentic_protocol::WireError),
    #[error(transparent)]
    Crypto(#[from] agentic_crypto::CryptoError),
    #[error(transparent)]
    Mailbox(#[from] agentic_crypto::mailbox::MailboxError),
    #[error(transparent)]
    Capability(#[from] agentic_capabilities::CapabilityError),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub name: String,
    pub network_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub phase: String,
    pub replicas: u32,
    pub target: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub author: String,
    pub own: bool,
    pub text: String,
    pub created_at: u64,
    pub delivery: Delivery,
    /// Taken directly while its stamp could not be checked.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub low_trust: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub unread: u32,
    pub messages: Vec<Message>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub connected_peers: u32,
    pub state: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub identity: Option<Identity>,
    pub network: NetworkStatus,
    pub conversations: Vec<Conversation>,
}
mod desktop_history;
pub use desktop_history::{ConversationHistory, DesktopOverview};
pub struct PendingDelivery {
    pub message_id: String,
    pub destination: String,
    pub addresses: Vec<String>,
    pub wire: Vec<u8>,
}
pub struct ReceiveOutcome {
    pub reply: Option<Vec<u8>>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReceiveOrder {
    LegacyBounded,
    Contiguous,
}
#[derive(Clone, Serialize, Deserialize)]
struct Contact {
    title: String,
    root: [u8; 32],
    addresses: Vec<String>,
    read_through: u64,
    #[serde(default)]
    route_version: Option<peer_records::RouteVersion>,
    #[serde(default)]
    receive_order: Option<ReceiveOrder>,
}
#[derive(Clone, Serialize, Deserialize)]
struct IssuedInvitation {
    expires_at: u64,
    consumed_by: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct AppState {
    version: u8,
    name: Option<String>,
    domain: Option<[u8; 32]>,
    contacts: BTreeMap<String, Contact>,
    issued: BTreeMap<String, IssuedInvitation>,
    imported: BTreeMap<String, String>,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            version: 2,
            name: None,
            domain: None,
            contacts: BTreeMap::new(),
            issued: BTreeMap::new(),
            imported: BTreeMap::new(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    Welcome,
    /// A Welcome sent through an intro card, which also travels through the
    /// recipient's intro swarm.
    Request,
    /// A group invitation, sent like a request.
    Invite,
    /// A group commit this profile made.
    Commit,
    /// A group's ratchet tree of an epoch, for newcomers, in its own mailbox.
    Tree {
        epoch: u64,
    },
    /// An application to join a group, left at its door.
    Knock,
    /// A notice among a group's members, never shown as talk.
    Notice,
    Text {
        text: String,
        /// Taken directly while its stamp could not be checked.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        low_trust: bool,
    },
}

/// What joining a Welcome's group needs.
struct Joining {
    group: [u8; 32],
    root: [u8; 32],
    message_id: String,
    issued_at: u64,
    name: String,
    welcome: Vec<u8>,
    addresses: Vec<String>,
}

struct ApplicationSend<'a> {
    conversation_id: &'a str,
    operation_id: &'a str,
    request_hash: [u8; 32],
    now: u64,
    payload: Vec<u8>,
    event: Event,
}

pub struct AppCore {
    store: ProfileStore,
    domain: [u8; 32],
}
impl AppCore {
    pub fn new(store: ProfileStore, domain: [u8; 32]) -> Result<Self, CoreError> {
        let core = Self { store, domain };
        let (data, _) = core.data()?;
        if data.domain.is_some_and(|stored| stored != domain) {
            return Err(CoreError::WrongNetwork);
        }
        Ok(core)
    }
    fn data(&self) -> Result<(AppState, u64), CoreError> {
        match self.store.state(APP_STATE)? {
            None => Ok((AppState::default(), 0)),
            Some(state) => {
                let data: AppState =
                    serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
                if !matches!(data.version, 1 | 2)
                    || data
                        .contacts
                        .values()
                        .any(|contact| (data.version == 1) != contact.receive_order.is_none())
                {
                    return Err(CoreError::InvalidState);
                }
                Ok((data, state.revision))
            }
        }
    }
    /// The MLS state an operation needs: the profile's own records and, with
    /// `group`, that group's or conversation's alone
    /// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 1, phase 1b).
    fn crypto(&self, group: Option<[u8; 32]>) -> Result<(MlsClient, u64), CoreError> {
        let saved = self
            .store
            .state_records_in(MLS_STATE, &agentic_crypto::record_scope(group))?
            .ok_or(CoreError::NoProfile)?;
        Ok((
            MlsClient::restore_scope(&saved.state.bytes, saved.records, group)?,
            saved.state.revision,
        ))
    }
    fn identity_for(&self, data: &AppState) -> Result<Identity, CoreError> {
        Ok(Identity {
            name: data.name.clone().ok_or(CoreError::NoProfile)?,
            network_id: self.store.identity()?.network_id,
        })
    }
    pub fn create_profile(&mut self, name: &str) -> Result<Identity, CoreError> {
        valid_name(name)?;
        let name = name.trim();
        let (mut data, revision) = self.data()?;
        if let Some(existing) = &data.name {
            if existing == name {
                return self.identity_for(&data);
            }
            return Err(CoreError::ProfileExists);
        }
        let root = self.store.identity()?;
        let crypto = MlsClient::new(root.network_id.as_bytes())?;
        data.name = Some(name.into());
        data.domain = Some(self.domain);
        let (mls, records) = crypto_change(None, crypto.snapshot(), 0);
        self.store
            .commit_states_with_records(vec![data_change(&data, revision)?, mls], vec![records])?;
        self.identity_for(&data)
    }
    pub fn create_invitation(
        &mut self,
        now: u64,
        addresses: Vec<String>,
    ) -> Result<String, CoreError> {
        valid_addresses(&addresses)?;
        let (mut data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        let (crypto, crypto_revision) = self.crypto(None)?;
        let prepared = crypto.key_package()?;
        let packet = Packet::Invitation {
            name: identity.name,
            package: prepared.value,
            addresses,
            nonce: random_id()?,
        };
        let expires_at = now
            .checked_add(INVITE_LIFETIME)
            .ok_or(CoreError::InvalidInput)?;
        let wire = self.sign(packet, now, Some(expires_at))?;
        data.issued.insert(
            hex::encode(Sha256::digest(&wire)),
            IssuedInvitation {
                expires_at,
                consumed_by: None,
            },
        );
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, crypto_revision);
        self.store
            .commit_states_with_records(vec![data_change(&data, revision)?, mls], vec![records])?;
        Ok(format!("{INVITE_PREFIX}{}", hex::encode(wire)))
    }
    pub fn invitation_addresses(
        &self,
        invitation: &str,
        now: u64,
    ) -> Result<Vec<String>, CoreError> {
        let (_, packet) = self.decode_invitation(invitation, now)?;
        match packet {
            Packet::Invitation { addresses, .. } => Ok(addresses),
            _ => Err(CoreError::InvalidInvitation),
        }
    }
    fn decode_invitation(
        &self,
        invitation: &str,
        now: u64,
    ) -> Result<(VerifiedDocument, Packet), CoreError> {
        if invitation.len() > INVITE_PREFIX.len() + 2 * agentic_protocol::MAX_DOCUMENT_BYTES {
            return Err(CoreError::InvalidInput);
        }
        let wire = hex::decode(
            invitation
                .strip_prefix(INVITE_PREFIX)
                .ok_or(CoreError::InvalidInput)?,
        )
        .map_err(invalid)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        root_epoch(&verified)?;
        let packet = Packet::decode(verified.body(), verified.kind())?;
        let expiry = verified.expires_at().ok_or(CoreError::InvalidInvitation)?;
        if !matches!(packet, Packet::Invitation { .. })
            || expiry.saturating_sub(verified.issued_at()) > INVITE_LIFETIME
        {
            return Err(CoreError::InvalidInvitation);
        }
        Ok((verified, packet))
    }
    pub fn add_contact(
        &mut self,
        name: &str,
        invitation: &str,
        now: u64,
    ) -> Result<Conversation, CoreError> {
        valid_name(name)?;
        let (verified, packet) = self.decode_invitation(invitation, now)?;
        let (mut data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        let Packet::Invitation {
            package, addresses, ..
        } = packet
        else {
            return Err(CoreError::InvalidInvitation);
        };
        let peer = network_id(verified.author());
        if peer == identity.network_id {
            return Err(CoreError::Unauthorized);
        }
        if inspect_key_package(&package)?.identity != peer.as_bytes() {
            return Err(CoreError::Unauthorized);
        }
        let invite_id = hex::encode(verified.id());
        if let Some(group) = data.imported.get(&invite_id) {
            return self.conversation(
                group,
                data.contacts.get(group).ok_or(CoreError::InvalidState)?,
            );
        }
        let group = random_id()?;
        let group_id = hex::encode(group);
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let created = crypto.create_group(group)?;
        let created_client = MlsClient::from_state(created.next_state);
        let added = created_client.add_members(group, &[package])?;
        let pending = MlsClient::from_state(added.next_state);
        // Initial two-party group creation has no earlier membership to order.
        let activated = pending.activate_pending_commit(group)?;
        let welcome = Packet::Welcome {
            group,
            invitation: verified.id(),
            name: identity.name,
            welcome: added.value.welcome,
        };
        let wire = self.sign(welcome, now, None)?;
        let message_id = hex::encode(Sha256::digest(&wire));
        enable_receive_order(&mut data);
        let contact = Contact {
            title: name.trim().into(),
            root: *verified.author(),
            addresses,
            read_through: 0,
            route_version: None,
            receive_order: Some(ReceiveOrder::Contiguous),
        };
        data.contacts.insert(group_id.clone(), contact.clone());
        data.imported.insert(invite_id.clone(), group_id.clone());
        let (mls, records) = crypto_change(Some(&crypto), &activated.next_state, crypto_revision);
        self.store.commit_outgoing_with_retry_states_and_records(
            OutgoingCommit {
                operation_id: format!("invite:{invite_id}"),
                request_hash: Sha256::digest(&wire).into(),
                message: record(
                    message_id,
                    &group_id,
                    &identity.network_id,
                    now,
                    true,
                    Event::Welcome,
                )?,
                destination: peer,
                wire,
                states: vec![data_change(&data, revision)?, mls],
            },
            vec![],
            vec![records],
        )?;
        self.conversation(&group_id, &contact)
    }
    pub fn send_message(
        &mut self,
        conversation_id: &str,
        text: &str,
        operation_id: &str,
        now: u64,
    ) -> Result<Message, CoreError> {
        self.send_authorized(
            conversation_id,
            text,
            operation_id,
            now,
            broker::AuthorizationChanges::default(),
        )
    }
    fn send_authorized(
        &mut self,
        conversation_id: &str,
        text: &str,
        operation_id: &str,
        now: u64,
        authorization: broker::AuthorizationChanges,
    ) -> Result<Message, CoreError> {
        valid_text(text)?;
        let (data, _) = self.data()?;
        let identity = self.identity_for(&data)?;
        let group = parse_id(conversation_id)?;
        let mut command = Encoder::new(Vec::new());
        command
            .array(5)
            .map_err(invalid)?
            .str("send-message-v1")
            .map_err(invalid)?
            .bytes(&self.domain)
            .map_err(invalid)?
            .str(&identity.network_id)
            .map_err(invalid)?
            .bytes(&group)
            .map_err(invalid)?
            .str(text)
            .map_err(invalid)?;
        let request_hash = Sha256::digest(command.into_writer()).into();
        let saved = self.send_application(
            ApplicationSend {
                conversation_id,
                operation_id,
                request_hash,
                now,
                payload: text.as_bytes().to_vec(),
                event: Event::Text {
                    text: text.into(),
                    low_trust: false,
                },
            },
            authorization,
        )?;
        self.present_message(&saved)?.ok_or(CoreError::InvalidState)
    }
    fn send_application(
        &mut self,
        input: ApplicationSend<'_>,
        authorization: broker::AuthorizationChanges,
    ) -> Result<StoredMessage, CoreError> {
        if self.is_group(input.conversation_id)? {
            let Event::Text { text, .. } = &input.event else {
                return Err(CoreError::InvalidInput);
            };
            let text = text.clone();
            return self.send_group_text(
                input.conversation_id,
                &text,
                input.operation_id,
                input.request_hash,
                input.now,
                authorization,
            );
        }
        let (data, _) = self.data()?;
        let identity = self.identity_for(&data)?;
        let contact = data
            .contacts
            .get(input.conversation_id)
            .ok_or(CoreError::UnknownConversation)?;
        let group = parse_id(input.conversation_id)?;
        let (crypto, revision) = self.crypto(Some(group))?;
        let prepared = crypto.encrypt(
            group,
            &input.payload,
            &application_aad_for(self.domain, group)?,
        )?;
        let packet = Packet::Application {
            group,
            message: prepared.value.wire,
        };
        let wire = self.sign(packet, input.now, None)?;
        let message = record(
            hex::encode(Sha256::digest(&wire)),
            input.conversation_id,
            &identity.network_id,
            input.now,
            true,
            input.event,
        )?;
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, revision);
        let mut states = vec![mls];
        states.extend(authorization.new_operation);
        Ok(self.store.commit_outgoing_with_retry_states_and_records(
            OutgoingCommit {
                operation_id: input.operation_id.into(),
                request_hash: input.request_hash,
                message,
                destination: network_id(&contact.root),
                wire,
                states,
            },
            authorization.retry,
            vec![records],
        )?)
    }
    pub fn receive(&mut self, wire: &[u8], now: u64) -> Result<ReceiveOutcome, CoreError> {
        let verified = VerifiedDocument::decode(wire, self.domain, now)?;
        self.receive_verified(verified, vec![], now)
    }
    fn receive_verified(
        &mut self,
        verified: VerifiedDocument,
        addresses: Vec<String>,
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        self.receive_verified_with_states(verified, addresses, now, vec![], false)
    }
    /// `low_trust` marks a new text message taken with an unchecked stamp.
    fn receive_verified_with_states(
        &mut self,
        verified: VerifiedDocument,
        addresses: Vec<String>,
        now: u64,
        completion: Vec<StateChange>,
        low_trust: bool,
    ) -> Result<ReceiveOutcome, CoreError> {
        root_epoch(&verified)?;
        let packet = Packet::decode(verified.body(), verified.kind())?;
        if !completion.is_empty() && !matches!(packet, Packet::Application { .. }) {
            return Err(CoreError::InvalidInput);
        }
        let (data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        let peer = network_id(verified.author());
        let message_id = hex::encode(verified.id());
        match packet {
            Packet::Welcome {
                group,
                invitation,
                name,
                welcome,
            } => {
                if peer == identity.network_id {
                    return Err(CoreError::Unauthorized);
                }
                let group_id = hex::encode(group);
                if let Some(existing) = self.store.message(&message_id)? {
                    if existing.record.own
                        || existing.record.conversation_id != group_id
                        || existing.record.author != peer
                    {
                        return Err(CoreError::Unauthorized);
                    }
                    require_peer(&data, &group_id, verified.author())?;
                    return self.receipt(group, verified.id(), now);
                }
                let invitation_key = hex::encode(invitation);
                let Some(issued) = data.issued.get(&invitation_key) else {
                    // A Welcome made with an intro card meets the policy.
                    self.intake(
                        &verified, group, invitation, name, welcome, addresses, None, now,
                    )?;
                    return self.receipt(group, verified.id(), now);
                };
                if issued.expires_at <= now || issued.consumed_by.is_some() {
                    return Err(CoreError::InvalidInvitation);
                }
                self.join_welcome(
                    data,
                    revision,
                    Joining {
                        group,
                        root: *verified.author(),
                        message_id,
                        issued_at: verified.issued_at(),
                        name,
                        welcome,
                        addresses,
                    },
                    |data, group_id| {
                        if let Some(issued) = data.issued.get_mut(&invitation_key) {
                            issued.consumed_by = Some(group_id.into());
                        }
                    },
                    vec![],
                )?;
                self.receipt(group, verified.id(), now)
            }
            Packet::Application { group, message } => {
                let group_id = hex::encode(group);
                if self.is_group(&group_id)? {
                    return Err(CoreError::InvalidInput);
                }
                require_peer(&data, &group_id, verified.author())?;
                if let Some(existing) = self.store.message(&message_id)? {
                    if existing.record.own
                        || existing.record.conversation_id != group_id
                        || existing.record.author != peer
                    {
                        return Err(CoreError::Unauthorized);
                    }
                    if !completion.is_empty() {
                        self.store.commit_states(completion)?;
                    }
                    return self.receipt(group, verified.id(), now);
                }
                let (crypto, crypto_revision) = self.crypto(Some(group))?;
                let aad = application_aad_for(self.domain, group)?;
                let contiguous = data
                    .contacts
                    .get(&group_id)
                    .is_some_and(|contact| contact.receive_order == Some(ReceiveOrder::Contiguous));
                let received = if contiguous {
                    crypto.decrypt_contiguous(group, &message, &aad)
                } else {
                    crypto.decrypt(group, &message, &aad)
                }?;
                if received.value.sender != peer.as_bytes() {
                    return Err(CoreError::Unauthorized);
                }
                let text = String::from_utf8(received.value.plaintext).map_err(invalid)?;
                valid_text(&text)?;
                let event = Event::Text { text, low_trust };
                let mut states = vec![];
                let (mls, records) =
                    crypto_change(Some(&crypto), &received.next_state, crypto_revision);
                states.push(mls);
                states.extend(completion);
                self.store.commit_incoming_with_records(
                    IncomingCommit {
                        message: record(
                            message_id,
                            &group_id,
                            &peer,
                            verified.issued_at(),
                            false,
                            event,
                        )?,
                        states,
                    },
                    vec![records],
                )?;
                self.receipt(group, verified.id(), now)
            }
            Packet::Receipt { group, accepted } => {
                let group_id = hex::encode(group);
                let accepted_id = hex::encode(accepted);
                if self.is_group(&group_id)? {
                    // An invitee answers the invitation sent to it.
                    let item = self
                        .store
                        .pending_outbox_item(&accepted_id)?
                        .ok_or(CoreError::Unauthorized)?;
                    if item.message.record.conversation_id != group_id || item.destination != peer {
                        return Err(CoreError::Unauthorized);
                    }
                    self.store.acknowledge_outbox(&accepted_id)?;
                    return Ok(ReceiveOutcome { reply: None });
                }
                require_peer(&data, &group_id, verified.author())?;
                let original = self
                    .store
                    .message(&accepted_id)?
                    .ok_or(CoreError::Unauthorized)?;
                if !original.record.own
                    || original.record.conversation_id != group_id
                    || original.record.author != identity.network_id
                {
                    return Err(CoreError::Unauthorized);
                }
                self.store.acknowledge_outbox(&accepted_id)?;
                Ok(ReceiveOutcome { reply: None })
            }
            Packet::GroupWelcome {
                group,
                card,
                name,
                owner,
                roster,
                welcome,
                mailbox,
                epoch,
                tree,
                membership,
            } => {
                let id = verified.id();
                self.intake(
                    &verified,
                    group,
                    card,
                    name,
                    welcome,
                    addresses,
                    Some(groups::GroupInvite {
                        owner,
                        roster,
                        mailbox,
                        epoch,
                        tree,
                        membership,
                    }),
                    now,
                )?;
                self.receipt(group, id, now)
            }
            Packet::Invitation { .. }
            | Packet::IntroCard { .. }
            | Packet::GroupCommit { .. }
            | Packet::GroupTree { .. }
            | Packet::GroupApplication { .. } => Err(CoreError::InvalidInput),
            packet @ Packet::ChannelKeys { group, .. } => {
                let id = verified.id();
                self.take_channel_keys(&verified, packet, None, now)?;
                self.receipt(group, id, now)
            }
        }
    }
    /// Join the group of a Welcome from `joining.root` and add the contact,
    /// with `update` and `states` committed alongside.
    fn join_welcome(
        &mut self,
        mut data: AppState,
        revision: u64,
        joining: Joining,
        update: impl FnOnce(&mut AppState, &str),
        mut states: Vec<StateChange>,
    ) -> Result<Conversation, CoreError> {
        let identity = self.identity_for(&data)?;
        let peer = network_id(&joining.root);
        let group_id = hex::encode(joining.group);
        let (crypto, crypto_revision) = self.crypto(Some(joining.group))?;
        let joined = crypto.join(joining.group, &joining.welcome)?;
        let joined_client = MlsClient::from_state(joined.next_state);
        let members = joined_client.members(joining.group)?;
        let mut actual: Vec<_> = members.iter().map(|m| m.identity.clone()).collect();
        actual.sort();
        let mut expected = vec![
            identity.network_id.as_bytes().to_vec(),
            peer.as_bytes().to_vec(),
        ];
        expected.sort();
        if actual != expected {
            return Err(CoreError::Unauthorized);
        }
        enable_receive_order(&mut data);
        let contact = Contact {
            title: joining.name,
            root: joining.root,
            addresses: joining.addresses,
            read_through: 0,
            route_version: None,
            receive_order: Some(ReceiveOrder::Contiguous),
        };
        data.contacts.insert(group_id.clone(), contact.clone());
        update(&mut data, &group_id);
        let (mls, records) =
            crypto_change(Some(&crypto), joined_client.snapshot(), crypto_revision);
        states.insert(0, mls);
        states.insert(0, data_change(&data, revision)?);
        self.store.commit_incoming_with_records(
            IncomingCommit {
                message: record(
                    joining.message_id,
                    &group_id,
                    &peer,
                    joining.issued_at,
                    false,
                    Event::Welcome,
                )?,
                states,
            },
            vec![records],
        )?;
        self.conversation(&group_id, &contact)
    }
    fn receipt(
        &self,
        group: [u8; 32],
        accepted: [u8; 32],
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        Ok(ReceiveOutcome {
            reply: Some(self.sign(Packet::Receipt { group, accepted }, now, None)?),
        })
    }
    /// A group document of any size: past one envelope's body it is signed
    /// large and travels in parts.
    fn sign_whole(&self, packet: Packet, now: u64) -> Result<Vec<u8>, CoreError> {
        let draft = DocumentDraft {
            domain: self.domain,
            kind: packet.kind(),
            authority_epoch: 0,
            issued_at: now,
            expires_at: None,
            body: packet.encode()?,
            extensions: BTreeMap::new(),
        };
        Ok(if draft.body.len() > agentic_protocol::MAX_BODY_BYTES {
            self.store.sign_large_document(draft)?
        } else {
            self.store.sign_document(draft)?
        }
        .to_wire())
    }
    fn sign(
        &self,
        packet: Packet,
        now: u64,
        expires_at: Option<u64>,
    ) -> Result<Vec<u8>, CoreError> {
        let draft = DocumentDraft {
            domain: self.domain,
            kind: packet.kind(),
            authority_epoch: 0,
            issued_at: now,
            expires_at,
            body: packet.encode()?,
            extensions: BTreeMap::new(),
        };
        Ok(self.store.sign_document(draft)?.to_wire())
    }
    /// Bounded read-only planning without loading message histories or MLS state.
    pub fn conversation_ids(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, CoreError> {
        if !(1..=32).contains(&limit) {
            return Err(CoreError::InvalidInput);
        }
        if let Some(cursor) = after {
            parse_id(cursor)?;
        }
        let (data, _) = self.data()?;
        use std::ops::Bound::{Excluded, Unbounded};
        let lower = after.map_or(Unbounded, Excluded);
        Ok(data
            .contacts
            .range::<str, _>((lower, Unbounded))
            .take(limit)
            .map(|(id, _)| id.clone())
            .collect())
    }
    pub fn snapshot(&self) -> Result<Snapshot, CoreError> {
        let (data, _) = self.data()?;
        let identity = if data.name.is_some() {
            Some(self.identity_for(&data)?)
        } else {
            None
        };
        let mut conversations = data
            .contacts
            .iter()
            .map(|(id, c)| self.conversation(id, c))
            .collect::<Result<Vec<_>, _>>()?;
        conversations.extend(self.group_conversations()?);
        conversations.extend(self.follow_conversations()?);
        Ok(Snapshot {
            identity,
            network: NetworkStatus {
                connected_peers: 0,
                state: "offline".into(),
            },
            conversations,
        })
    }
    fn conversation(&self, id: &str, contact: &Contact) -> Result<Conversation, CoreError> {
        let mut messages = vec![];
        let mut unread = 0;
        for stored in self.store.messages(id, 0, 1000)? {
            if let Some(message) = self.present_message(&stored)? {
                if !message.own && stored.sequence > contact.read_through {
                    unread += 1;
                }
                messages.push(message);
            }
        }
        Ok(Conversation {
            id: id.into(),
            title: contact.title.clone(),
            messages,
            unread,
        })
    }
    fn present_message(&self, stored: &StoredMessage) -> Result<Option<Message>, CoreError> {
        let event: Event =
            serde_json::from_slice(&stored.record.content).map_err(|_| CoreError::InvalidState)?;
        let Event::Text { text, low_trust } = event else {
            return Ok(None);
        };
        Ok(Some(Message {
            id: stored.record.id.clone(),
            author: stored.record.author.clone(),
            own: stored.record.own,
            text,
            created_at: stored.record.created_at,
            delivery: self.delivery_status(&stored.record)?,
            low_trust,
        }))
    }
    fn delivery_status(&self, record: &MessageRecord) -> Result<Delivery, CoreError> {
        let phase = if record.own && self.store.is_pending(&record.id)? {
            "queued"
        } else {
            "delivered"
        };
        Ok(Delivery {
            phase: phase.into(),
            replicas: 0,
            target: 10,
        })
    }
    pub fn outbox(&self, limit: usize) -> Result<Vec<PendingDelivery>, CoreError> {
        let (data, _) = self.data()?;
        self.store
            .pending_outbox(limit)?
            .into_iter()
            .filter_map(|item| {
                let addresses = match data.contacts.get(&item.message.record.conversation_id) {
                    Some(contact) => contact.addresses.clone(),
                    // A group: only its invitations go directly, to the
                    // addresses of the invitee's card.
                    None => match self.intro_sent_addresses(&item.message.record.id) {
                        Ok(Some(addresses)) => addresses,
                        Ok(None) => return None,
                        Err(error) => return Some(Err(error)),
                    },
                };
                Some(Ok(PendingDelivery {
                    message_id: item.message.record.id,
                    destination: item.destination,
                    addresses,
                    wire: item.wire,
                }))
            })
            .collect()
    }
}
// Called only when staging an initial contact/Welcome. Never certify an
// existing ratchet whose skipped generations are unknown to this Core version.
fn enable_receive_order(data: &mut AppState) {
    if data.version == 1 {
        for contact in data.contacts.values_mut() {
            contact.receive_order = Some(ReceiveOrder::LegacyBounded);
        }
        data.version = 2;
    }
}

fn require_peer(data: &AppState, group: &str, author: &[u8; 32]) -> Result<(), CoreError> {
    if &data
        .contacts
        .get(group)
        .ok_or(CoreError::UnknownConversation)?
        .root
        != author
    {
        return Err(CoreError::Unauthorized);
    }
    Ok(())
}
fn root_epoch(document: &VerifiedDocument) -> Result<(), CoreError> {
    if document.authority_epoch() != 0 {
        return Err(CoreError::Unauthorized);
    }
    Ok(())
}
fn invalid<E>(_: E) -> CoreError {
    CoreError::InvalidInput
}
fn random_id() -> Result<[u8; 32], CoreError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| CoreError::Randomness)?;
    Ok(bytes)
}
fn parse_id(id: &str) -> Result<[u8; 32], CoreError> {
    hex::decode(id)
        .map_err(invalid)?
        .try_into()
        .map_err(invalid)
}
fn valid_name(name: &str) -> Result<(), CoreError> {
    if name.trim().is_empty()
        || name.chars().count() > 80
        || name.len() > 320
        || name.chars().any(char::is_control)
    {
        return Err(CoreError::InvalidInput);
    }
    Ok(())
}
fn valid_text(text: &str) -> Result<(), CoreError> {
    if text.trim().is_empty() || text.len() > 48_000 || text.chars().count() > 12_000 {
        return Err(CoreError::InvalidInput);
    }
    Ok(())
}
fn valid_addresses(addresses: &[String]) -> Result<(), CoreError> {
    if addresses.len() > 8
        || addresses
            .iter()
            .any(|a| a.is_empty() || a.len() > 256 || a.chars().any(char::is_control))
    {
        return Err(CoreError::InvalidInput);
    }
    Ok(())
}
fn data_change(data: &AppState, revision: u64) -> Result<StateChange, CoreError> {
    Ok(StateChange {
        namespace: APP_STATE.into(),
        expected_revision: revision,
        bytes: serde_json::to_vec(data).map_err(|_| CoreError::InvalidState)?,
    })
}
fn crypto_change(
    previous: Option<&MlsClient>,
    next: &SecretState,
    revision: u64,
) -> (StateChange, StateRecordBatch) {
    let base = previous.and_then(MlsClient::persisted_record_state);
    (
        StateChange {
            namespace: MLS_STATE.into(),
            expected_revision: revision,
            bytes: next.record_prefix().to_vec(),
        },
        StateRecordBatch {
            namespace: MLS_STATE.into(),
            changes: next
                .record_changes(base)
                .map(|(key, bytes)| StateRecordChange {
                    key: key.to_vec(),
                    bytes: bytes.map(<[u8]>::to_vec),
                })
                .collect(),
        },
    )
}
fn record(
    id: String,
    group: &str,
    author: &str,
    now: u64,
    own: bool,
    event: Event,
) -> Result<MessageRecord, CoreError> {
    Ok(MessageRecord {
        id,
        conversation_id: group.into(),
        author: author.into(),
        created_at: now,
        own,
        content: serde_json::to_vec(&event).map_err(invalid)?,
    })
}
fn application_aad_for(domain: [u8; 32], group: [u8; 32]) -> Result<Vec<u8>, CoreError> {
    let mut e = Encoder::new(Vec::new());
    e.array(3)
        .map_err(invalid)?
        .str("AgenticInternet/message/v1")
        .map_err(invalid)?
        .bytes(&domain)
        .map_err(invalid)?
        .bytes(&group)
        .map_err(invalid)?;
    Ok(e.into_writer())
}

// Domain bodies use fixed-order deterministic CBOR in the signed document's bounded body.
enum Packet {
    Invitation {
        name: String,
        package: Vec<u8>,
        addresses: Vec<String>,
        nonce: [u8; 32],
    },
    Welcome {
        group: [u8; 32],
        invitation: [u8; 32],
        name: String,
        welcome: Vec<u8>,
    },
    Application {
        group: [u8; 32],
        message: Vec<u8>,
    },
    Receipt {
        group: [u8; 32],
        accepted: [u8; 32],
    },
    IntroCard {
        name: String,
        package: Vec<u8>,
        addresses: Vec<String>,
        seal_key: [u8; 32],
    },
    GroupWelcome {
        group: [u8; 32],
        card: [u8; 32],
        name: String,
        owner: [u8; 32],
        roster: Vec<u8>,
        welcome: Vec<u8>,
        /// The group mailbox's secret, which a join does not change.
        mailbox: [u8; 32],
        /// The epoch the Welcome joins.
        epoch: u64,
        tree: group_parts::TreeRef,
        /// The invitee's certificate of membership, signed by the inviter;
        /// empty when there is none.
        membership: Vec<u8>,
    },
    GroupCommit {
        group: [u8; 32],
        commit: Vec<u8>,
        claim: Vec<u8>,
    },
    /// A group's ratchet tree at an epoch, for newcomers, in its own mailbox.
    GroupTree {
        group: [u8; 32],
        epoch: u64,
        tree: Vec<u8>,
    },
    /// An application to join a group, sealed to its door: the applicant's
    /// intro card and a note.
    GroupApplication {
        group: [u8; 32],
        card: Vec<u8>,
        note: String,
        /// A member back from a long absence asks for its place again.
        rejoin: bool,
    },
    /// A closed channel's keys for one subscriber, sealed to its card like
    /// an invitation (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 10c).
    ChannelKeys {
        group: [u8; 32],
        card: [u8; 32],
        name: String,
        owner: [u8; 32],
        roster: Vec<u8>,
        generation: u32,
        leaf: u32,
        /// The root's version: removals so far.
        version: u32,
        /// The leaf's path, root first: each node's version and key.
        keys: Vec<(u32, [u8; 32])>,
    },
}
impl Packet {
    fn kind(&self) -> DocumentKind {
        match self {
            Self::Invitation { .. } | Self::IntroCard { .. } => DocumentKind::Invitation,
            Self::Welcome { .. }
            | Self::GroupWelcome { .. }
            | Self::GroupCommit { .. }
            | Self::GroupTree { .. }
            | Self::GroupApplication { .. }
            | Self::ChannelKeys { .. } => DocumentKind::GroupControl,
            Self::Application { .. } | Self::Receipt { .. } => DocumentKind::Message,
        }
    }
    fn encode(&self) -> Result<Vec<u8>, CoreError> {
        let mut e = Encoder::new(Vec::new());
        match self {
            Self::Invitation {
                name,
                package,
                addresses,
                nonce,
            } => {
                e.array(5)
                    .map_err(invalid)?
                    .u8(1)
                    .map_err(invalid)?
                    .str(name)
                    .map_err(invalid)?
                    .bytes(package)
                    .map_err(invalid)?
                    .array(addresses.len() as u64)
                    .map_err(invalid)?;
                for address in addresses {
                    e.str(address).map_err(invalid)?;
                }
                e.bytes(nonce).map_err(invalid)?;
            }
            Self::Welcome {
                group,
                invitation,
                name,
                welcome,
            } => {
                e.array(5)
                    .map_err(invalid)?
                    .u8(2)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(invitation)
                    .map_err(invalid)?
                    .str(name)
                    .map_err(invalid)?
                    .bytes(welcome)
                    .map_err(invalid)?;
            }
            Self::Application { group, message } => {
                e.array(3)
                    .map_err(invalid)?
                    .u8(3)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(message)
                    .map_err(invalid)?;
            }
            Self::Receipt { group, accepted } => {
                e.array(3)
                    .map_err(invalid)?
                    .u8(4)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(accepted)
                    .map_err(invalid)?;
            }
            Self::IntroCard {
                name,
                package,
                addresses,
                seal_key,
            } => {
                e.array(5)
                    .map_err(invalid)?
                    .u8(5)
                    .map_err(invalid)?
                    .str(name)
                    .map_err(invalid)?
                    .bytes(package)
                    .map_err(invalid)?
                    .array(addresses.len() as u64)
                    .map_err(invalid)?;
                for address in addresses {
                    e.str(address).map_err(invalid)?;
                }
                e.bytes(seal_key).map_err(invalid)?;
            }
            Self::GroupWelcome {
                group,
                card,
                name,
                owner,
                roster,
                welcome,
                mailbox,
                epoch,
                tree,
                membership,
            } => {
                e.array(11)
                    .map_err(invalid)?
                    .u8(6)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(card)
                    .map_err(invalid)?
                    .str(name)
                    .map_err(invalid)?
                    .bytes(owner)
                    .map_err(invalid)?
                    .bytes(roster)
                    .map_err(invalid)?
                    .bytes(welcome)
                    .map_err(invalid)?
                    .bytes(mailbox)
                    .map_err(invalid)?
                    .u64(*epoch)
                    .map_err(invalid)?;
                match tree {
                    group_parts::TreeRef::Inline(tree) => e
                        .array(2)
                        .and_then(|e| e.u8(0))
                        .and_then(|e| e.bytes(tree))
                        .map_err(invalid)?,
                    group_parts::TreeRef::Parts { whole, count } => e
                        .array(3)
                        .and_then(|e| e.u8(1))
                        .and_then(|e| e.bytes(whole))
                        .and_then(|e| e.u16(*count))
                        .map_err(invalid)?,
                };
                e.bytes(membership).map_err(invalid)?;
            }
            Self::GroupTree { group, epoch, tree } => {
                e.array(4)
                    .map_err(invalid)?
                    .u8(8)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .u64(*epoch)
                    .map_err(invalid)?
                    .bytes(tree)
                    .map_err(invalid)?;
            }
            Self::GroupApplication {
                group,
                card,
                note,
                rejoin,
            } => {
                e.array(5)
                    .map_err(invalid)?
                    .u8(9)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(card)
                    .map_err(invalid)?
                    .str(note)
                    .map_err(invalid)?
                    .bool(*rejoin)
                    .map_err(invalid)?;
            }
            Self::ChannelKeys {
                group,
                card,
                name,
                owner,
                roster,
                generation,
                leaf,
                version,
                keys,
            } => {
                e.array(10)
                    .and_then(|e| e.u8(10))
                    .and_then(|e| e.bytes(group))
                    .and_then(|e| e.bytes(card))
                    .and_then(|e| e.str(name))
                    .and_then(|e| e.bytes(owner))
                    .and_then(|e| e.bytes(roster))
                    .and_then(|e| e.u32(*generation))
                    .and_then(|e| e.u32(*leaf))
                    .and_then(|e| e.u32(*version))
                    .and_then(|e| e.array(keys.len() as u64))
                    .map_err(invalid)?;
                for (version, key) in keys {
                    e.array(2)
                        .and_then(|e| e.u32(*version))
                        .and_then(|e| e.bytes(key))
                        .map_err(invalid)?;
                }
            }
            Self::GroupCommit {
                group,
                commit,
                claim,
            } => {
                e.array(4)
                    .map_err(invalid)?
                    .u8(7)
                    .map_err(invalid)?
                    .bytes(group)
                    .map_err(invalid)?
                    .bytes(commit)
                    .map_err(invalid)?
                    .bytes(claim)
                    .map_err(invalid)?;
            }
        }
        Ok(e.into_writer())
    }
    fn decode(bytes: &[u8], kind: DocumentKind) -> Result<Self, CoreError> {
        let mut d = Decoder::new(bytes);
        let length = d.array().map_err(invalid)?;
        let tag = d.u8().map_err(invalid)?;
        let result = match (tag, length) {
            (1, Some(5)) => {
                let name = d.str().map_err(invalid)?.to_owned();
                valid_name(&name)?;
                let package = d.bytes().map_err(invalid)?.to_vec();
                let count = d.array().map_err(invalid)?.ok_or(CoreError::InvalidInput)?;
                if count > 8 {
                    return Err(CoreError::InvalidInput);
                }
                let mut addresses = vec![];
                for _ in 0..count {
                    addresses.push(d.str().map_err(invalid)?.to_owned());
                }
                valid_addresses(&addresses)?;
                Self::Invitation {
                    name,
                    package,
                    addresses,
                    nonce: read_id(&mut d)?,
                }
            }
            (2, Some(5)) => {
                let group = read_id(&mut d)?;
                let invitation = read_id(&mut d)?;
                let name = d.str().map_err(invalid)?.to_owned();
                valid_name(&name)?;
                Self::Welcome {
                    group,
                    invitation,
                    name,
                    welcome: d.bytes().map_err(invalid)?.to_vec(),
                }
            }
            (3, Some(3)) => Self::Application {
                group: read_id(&mut d)?,
                message: d.bytes().map_err(invalid)?.to_vec(),
            },
            (4, Some(3)) => Self::Receipt {
                group: read_id(&mut d)?,
                accepted: read_id(&mut d)?,
            },
            (5, Some(5)) => {
                let name = d.str().map_err(invalid)?.to_owned();
                valid_name(&name)?;
                let package = d.bytes().map_err(invalid)?.to_vec();
                let count = d.array().map_err(invalid)?.ok_or(CoreError::InvalidInput)?;
                if count > 8 {
                    return Err(CoreError::InvalidInput);
                }
                let mut addresses = vec![];
                for _ in 0..count {
                    addresses.push(d.str().map_err(invalid)?.to_owned());
                }
                valid_addresses(&addresses)?;
                Self::IntroCard {
                    name,
                    package,
                    addresses,
                    seal_key: read_id(&mut d)?,
                }
            }
            (6, Some(11)) => {
                let group = read_id(&mut d)?;
                let card = read_id(&mut d)?;
                let name = d.str().map_err(invalid)?.to_owned();
                valid_name(&name)?;
                let owner = read_id(&mut d)?;
                let roster = d.bytes().map_err(invalid)?.to_vec();
                let welcome = d.bytes().map_err(invalid)?.to_vec();
                let mailbox = read_id(&mut d)?;
                let epoch = d.u64().map_err(invalid)?;
                let tree = match (d.array().map_err(invalid)?, d.u8().map_err(invalid)?) {
                    (Some(2), 0) => {
                        group_parts::TreeRef::Inline(d.bytes().map_err(invalid)?.to_vec())
                    }
                    (Some(3), 1) => group_parts::TreeRef::Parts {
                        whole: read_id(&mut d)?,
                        count: d.u16().map_err(invalid)?,
                    },
                    _ => return Err(CoreError::InvalidInput),
                };
                Self::GroupWelcome {
                    group,
                    card,
                    name,
                    owner,
                    roster,
                    welcome,
                    mailbox,
                    epoch,
                    tree,
                    membership: d.bytes().map_err(invalid)?.to_vec(),
                }
            }
            (7, Some(4)) => Self::GroupCommit {
                group: read_id(&mut d)?,
                commit: d.bytes().map_err(invalid)?.to_vec(),
                claim: d.bytes().map_err(invalid)?.to_vec(),
            },
            (8, Some(4)) => Self::GroupTree {
                group: read_id(&mut d)?,
                epoch: d.u64().map_err(invalid)?,
                tree: d.bytes().map_err(invalid)?.to_vec(),
            },
            (9, Some(5)) => Self::GroupApplication {
                group: read_id(&mut d)?,
                card: d.bytes().map_err(invalid)?.to_vec(),
                note: d.str().map_err(invalid)?.to_owned(),
                rejoin: d.bool().map_err(invalid)?,
            },
            (10, Some(10)) => {
                let group = read_id(&mut d)?;
                let card = read_id(&mut d)?;
                let name = d.str().map_err(invalid)?.to_owned();
                valid_name(&name)?;
                let owner = read_id(&mut d)?;
                let roster = d.bytes().map_err(invalid)?.to_vec();
                let generation = d.u32().map_err(invalid)?;
                let leaf = d.u32().map_err(invalid)?;
                let version = d.u32().map_err(invalid)?;
                if d.array().map_err(invalid)? != Some(u64::from(key_tree::DEPTH) + 1) {
                    return Err(CoreError::InvalidInput);
                }
                let mut keys = vec![];
                for _ in 0..=key_tree::DEPTH {
                    if d.array().map_err(invalid)? != Some(2) {
                        return Err(CoreError::InvalidInput);
                    }
                    keys.push((d.u32().map_err(invalid)?, read_id(&mut d)?));
                }
                Self::ChannelKeys {
                    group,
                    card,
                    name,
                    owner,
                    roster,
                    generation,
                    leaf,
                    version,
                    keys,
                }
            }
            _ => return Err(CoreError::InvalidInput),
        };
        if d.position() != bytes.len() || result.kind() != kind || result.encode()? != bytes {
            return Err(CoreError::InvalidInput);
        }
        Ok(result)
    }
}
fn read_id(d: &mut Decoder<'_>) -> Result<[u8; 32], CoreError> {
    d.bytes().map_err(invalid)?.try_into().map_err(invalid)
}
