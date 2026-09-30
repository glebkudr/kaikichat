//! Contact by ID (spec/contact-by-id-v1.md): intro cards published in the
//! intro mailbox, requests sealed to them, and the recipient's policy.
use super::group_parts::TreeRef;
use super::groups::GroupInvite;
use super::*;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, intro_mailbox_id, period};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use x25519_dalek::{PublicKey, StaticSecret};

const CARDS: &str = "intro/cards";
const POLICY: &str = "intro/policy";
const INTAKE: &str = "intro/intake";
const SENT: &str = "intro/sent/";
const READ: &str = "intro/read";
const CARD_ENTRY: u8 = 1;
pub(super) const REQUEST_ENTRY: u8 = 2;
const CARD_LIFETIME: u64 = 30 * PERIOD_SECONDS;
/// A new card once the current one has less than this left.
const RENEW_BEFORE: u64 = 23 * PERIOD_SECONDS;
/// A card's keys outlive it by the mailbox's retention and one period.
const KEEP_AFTER: u64 = 31 * PERIOD_SECONDS;
const MAX_WAITING: usize = 32;
const MAX_ALLOWED: usize = 1000;
const MAX_DAILY_LIMIT: u32 = 1000;
/// Requesters whose last decided request is remembered.
const MAX_DECIDED: usize = 4096;

/// The owner's current card, as published in its intro mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntroCard {
    pub id: String,
    pub envelope: Vec<u8>,
    pub expires_at: u64,
}

/// A verified card of another profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntroCardInfo {
    pub network_id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub expires_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntroMode {
    All,
    List,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntroPolicy {
    pub mode: IntroMode,
    pub daily_limit: u32,
    pub allowed: Vec<String>,
}

impl Default for IntroPolicy {
    fn default() -> Self {
        Self {
            mode: IntroMode::All,
            daily_limit: 100,
            allowed: vec![],
        }
    }
}

/// A request waiting for the owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntroRequest {
    pub request_id: String,
    pub network_id: String,
    pub name: String,
    pub received_at: u64,
    /// A group invitation: the group's id (its name is `name`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug)]
pub enum IntroOutcome {
    Joined(Conversation),
    Pending(IntroRequest),
    Ignored,
    /// A big group's invitation, let in: it joins once its tree, read from
    /// the tree's own mailbox, is whole (the group's id).
    AwaitingTree(String),
    /// A closed channel's keys, taken: it is read from now on (its
    /// conversation, `G` in hex).
    Subscribed(String),
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cards {
    cards: BTreeMap<String, StoredCard>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCard {
    wire: String,
    expires_at: u64,
    seal_secret: String,
}

impl StoredCard {
    fn keys_gone(&self, now: u64) -> bool {
        now >= self.expires_at.saturating_add(KEEP_AFTER)
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intake {
    day: u64,
    joined: u32,
    waiting: BTreeMap<String, Waiting>,
    /// The issue time of each requester's last request that was rejected or
    /// replaced: that one and older ones are ignored when read again.
    decided: BTreeMap<String, u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Waiting {
    request: IntroRequest,
    card: String,
    group: String,
    root: String,
    issued_at: u64,
    welcome: String,
    addresses: Vec<String>,
    /// A group invitation's owner and roster.
    #[serde(default)]
    invite: Option<StoredInvite>,
    /// A closed channel's keys: the packet in hex.
    #[serde(default)]
    keys: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredInvite {
    owner: String,
    roster: String,
    #[serde(default)]
    mailbox: String,
    #[serde(default)]
    epoch: u64,
    /// The tree in hex when it came along, else empty.
    #[serde(default)]
    tree: String,
    /// The awaited tree's whole and number of parts, when it did not.
    #[serde(default)]
    whole: String,
    #[serde(default)]
    count: u16,
    /// The certificate of membership in hex, when it came along.
    #[serde(default)]
    membership: String,
}

impl StoredInvite {
    fn from(invite: &GroupInvite) -> Self {
        let (tree, whole, count) = match &invite.tree {
            TreeRef::Inline(tree) => (hex::encode(tree), String::new(), 0),
            TreeRef::Parts { whole, count } => (String::new(), hex::encode(whole), *count),
        };
        Self {
            owner: hex::encode(invite.owner),
            roster: hex::encode(&invite.roster),
            mailbox: hex::encode(invite.mailbox),
            epoch: invite.epoch,
            tree,
            whole,
            count,
            membership: hex::encode(&invite.membership),
        }
    }
    fn invite(&self) -> Result<GroupInvite, CoreError> {
        let bytes = |text: &str| hex::decode(text).map_err(|_| CoreError::InvalidState);
        Ok(GroupInvite {
            owner: unhex32(&self.owner)?,
            roster: bytes(&self.roster)?,
            mailbox: unhex32(&self.mailbox)?,
            epoch: self.epoch,
            tree: if self.whole.is_empty() {
                TreeRef::Inline(bytes(&self.tree)?)
            } else {
                TreeRef::Parts {
                    whole: unhex32(&self.whole)?,
                    count: self.count,
                }
            },
            membership: bytes(&self.membership)?,
        })
    }
}

struct OpenedRequest {
    verified: VerifiedDocument,
    group: [u8; 32],
    card: [u8; 32],
    name: String,
    welcome: Vec<u8>,
    invite: Option<GroupInvite>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadThrough {
    period: Option<u64>,
}

/// What sealing a sent request into its recipient's intro mailbox needs.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sent {
    card: String,
    seal_key: String,
    identity: String,
    seed: String,
    /// Where the direct copy goes, when no contact names it (a group
    /// invitation).
    #[serde(default)]
    addresses: Vec<String>,
}

/// The 32 bytes a network id spells.
pub(super) fn identity_digest(network_id: &str) -> Result<[u8; 32], CoreError> {
    let digits = network_id
        .strip_prefix("ain1")
        .ok_or(CoreError::InvalidInput)?;
    let digest = parse_id(digits)?;
    if hex::encode(digest) != digits {
        return Err(CoreError::InvalidInput);
    }
    Ok(digest)
}

fn unhex32(text: &str) -> Result<[u8; 32], CoreError> {
    parse_id(text).map_err(|_| CoreError::InvalidState)
}

/// The key sealing a request: HKDF-SHA256 of the X25519 secret, salted with
/// both public keys, bound to the domain.
fn seal_cipher(
    domain: &[u8; 32],
    shared: &[u8; 32],
    ephemeral: &[u8; 32],
    seal_key: &[u8; 32],
) -> Result<ChaCha20Poly1305, CoreError> {
    let mut salt = [0; 64];
    salt[..32].copy_from_slice(ephemeral);
    salt[32..].copy_from_slice(seal_key);
    let mut info = b"AIN_INTRO_SEAL_V1".to_vec();
    info.extend_from_slice(domain);
    let mut key = zeroize::Zeroizing::new([0; 32]);
    hkdf::Hkdf::<Sha256>::new(Some(&salt), shared)
        .expand(&info, key.as_mut())
        .map_err(|_| CoreError::InvalidState)?;
    ChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| CoreError::InvalidState)
}

/// The ephemeral key of a sent request in `period`: one per request and
/// period, the same on every retry.
pub(super) fn ephemeral(seed: &[u8; 32], period: u64) -> StaticSecret {
    let mut hash = Sha256::new();
    hash.update(b"AIN_INTRO_EPHEMERAL_V1");
    hash.update(seed);
    hash.update(period.to_be_bytes());
    StaticSecret::from(<[u8; 32]>::from(hash.finalize()))
}

pub(super) fn seal(
    domain: &[u8; 32],
    card: &[u8; 32],
    seal_key: &[u8; 32],
    secret: &StaticSecret,
    plaintext: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let public = PublicKey::from(secret).to_bytes();
    let shared = secret.diffie_hellman(&PublicKey::from(*seal_key));
    if !shared.was_contributory() {
        return Err(CoreError::InvalidInput);
    }
    let sealed = seal_cipher(domain, shared.as_bytes(), &public, seal_key)?
        .encrypt(
            &Nonce::default(),
            Payload {
                msg: plaintext,
                aad: card,
            },
        )
        .map_err(|_| CoreError::InvalidState)?;
    let mut envelope = Vec::with_capacity(65 + sealed.len());
    envelope.push(REQUEST_ENTRY);
    envelope.extend_from_slice(card);
    envelope.extend_from_slice(&public);
    envelope.extend_from_slice(&sealed);
    Ok(envelope)
}

pub(super) fn open(
    domain: &[u8; 32],
    secret: &StaticSecret,
    envelope: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let card = &envelope[1..33];
    let ephemeral: [u8; 32] = envelope[33..65].try_into().map_err(invalid)?;
    let seal_key = PublicKey::from(secret).to_bytes();
    let shared = secret.diffie_hellman(&PublicKey::from(ephemeral));
    if !shared.was_contributory() {
        return Err(CoreError::InvalidInput);
    }
    seal_cipher(domain, shared.as_bytes(), &ephemeral, &seal_key)?
        .decrypt(
            &Nonce::default(),
            Payload {
                msg: &envelope[65..],
                aad: card,
            },
        )
        .map_err(|_| CoreError::InvalidInput)
}

impl AppCore {
    fn intro_state<T: DeserializeOwned + Default>(
        &self,
        name: &str,
    ) -> Result<(T, u64), CoreError> {
        match self.store.state(name)? {
            None => Ok((T::default(), 0)),
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
        }
    }

    /// The intro mailbox of `network_id` for the period containing `at`.
    pub fn intro_mailbox(&self, network_id: &str, at: u64) -> Result<[u8; 32], CoreError> {
        Ok(intro_mailbox_id(
            &self.domain,
            &identity_digest(network_id)?,
            period(at),
        ))
    }

    /// The card to publish now: the current one while at least 23 days are
    /// left, else a new one. Cards whose keys are gone are dropped, with the
    /// requests that wait on them.
    pub fn intro_card(&mut self, addresses: Vec<String>, now: u64) -> Result<IntroCard, CoreError> {
        valid_addresses(&addresses)?;
        let (data, _) = self.data()?;
        let identity = self.identity_for(&data)?;
        let (mut cards, cards_revision) = self.intro_state::<Cards>(CARDS)?;
        let (mut intake, intake_revision) = self.intro_state::<Intake>(INTAKE)?;
        let before = (cards.cards.len(), intake.waiting.len());
        cards.cards.retain(|_, card| !card.keys_gone(now));
        intake
            .waiting
            .retain(|_, waiting| cards.cards.contains_key(&waiting.card));
        let mut changes = vec![];
        if (cards.cards.len(), intake.waiting.len()) != before {
            changes.push(StateChange {
                namespace: INTAKE.into(),
                expected_revision: intake_revision,
                bytes: serde_json::to_vec(&intake).map_err(invalid)?,
            });
        }
        let current = cards
            .cards
            .iter()
            .max_by_key(|(_, card)| card.expires_at)
            .filter(|(_, card)| card.expires_at.saturating_sub(now) >= RENEW_BEFORE)
            .map(|(id, card)| (id.clone(), card.clone()));
        if let Some((id, card)) = current {
            if !changes.is_empty() {
                changes.push(StateChange {
                    namespace: CARDS.into(),
                    expected_revision: cards_revision,
                    bytes: serde_json::to_vec(&cards).map_err(invalid)?,
                });
                self.store.commit_states(changes)?;
            }
            let mut envelope = vec![CARD_ENTRY];
            envelope.extend(hex::decode(&card.wire).map_err(|_| CoreError::InvalidState)?);
            return Ok(IntroCard {
                id,
                envelope,
                expires_at: card.expires_at,
            });
        }
        let (crypto, crypto_revision) = self.crypto(None)?;
        let prepared = crypto.last_resort_key_package()?;
        let mut secret = zeroize::Zeroizing::new([0; 32]);
        getrandom::fill(secret.as_mut()).map_err(|_| CoreError::Randomness)?;
        let seal_key = PublicKey::from(&StaticSecret::from(*secret)).to_bytes();
        let expires_at = now
            .checked_add(CARD_LIFETIME)
            .ok_or(CoreError::InvalidInput)?;
        let wire = self.sign(
            Packet::IntroCard {
                name: identity.name,
                package: prepared.value,
                addresses,
                seal_key,
            },
            now,
            Some(expires_at),
        )?;
        let id = hex::encode(VerifiedDocument::decode(&wire, self.domain, now)?.id());
        cards.cards.insert(
            id.clone(),
            StoredCard {
                wire: hex::encode(&wire),
                expires_at,
                seal_secret: hex::encode(*secret),
            },
        );
        changes.push(StateChange {
            namespace: CARDS.into(),
            expected_revision: cards_revision,
            bytes: serde_json::to_vec(&cards).map_err(invalid)?,
        });
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, crypto_revision);
        changes.push(mls);
        self.store
            .commit_states_with_records(changes, vec![records])?;
        let mut envelope = vec![CARD_ENTRY];
        envelope.extend(wire);
        Ok(IntroCard {
            id,
            envelope,
            expires_at,
        })
    }

    /// A card read from `network_id`'s intro mailbox: its signed document,
    /// verified as that identity's and still valid.
    pub(super) fn verified_card(
        &self,
        network_id: &str,
        envelope: &[u8],
        now: u64,
    ) -> Result<(VerifiedDocument, Packet), CoreError> {
        let digest = identity_digest(network_id)?;
        if envelope.first() != Some(&CARD_ENTRY) {
            return Err(CoreError::InvalidInput);
        }
        let verified = VerifiedDocument::decode(&envelope[1..], self.domain, now)?;
        root_epoch(&verified)?;
        let packet = Packet::decode(verified.body(), verified.kind())?;
        let Packet::IntroCard { package, .. } = &packet else {
            return Err(CoreError::InvalidInput);
        };
        let expires_at = verified.expires_at().ok_or(CoreError::InvalidInvitation)?;
        if now >= expires_at || expires_at.saturating_sub(verified.issued_at()) > CARD_LIFETIME {
            return Err(CoreError::InvalidInvitation);
        }
        let author = network_id_of(verified.author());
        if <[u8; 32]>::from(Sha256::digest(verified.author())) != digest
            || inspect_key_package(package)?.identity != author.as_bytes()
        {
            return Err(CoreError::Unauthorized);
        }
        Ok((verified, packet))
    }

    pub fn open_intro_card(
        &self,
        network_id: &str,
        envelope: &[u8],
        now: u64,
    ) -> Result<IntroCardInfo, CoreError> {
        let (verified, packet) = self.verified_card(network_id, envelope, now)?;
        let Packet::IntroCard {
            name, addresses, ..
        } = packet
        else {
            return Err(CoreError::InvalidInput);
        };
        Ok(IntroCardInfo {
            network_id: network_id_of(verified.author()),
            name,
            addresses,
            expires_at: verified.expires_at().ok_or(CoreError::InvalidInvitation)?,
        })
    }

    /// Ask `network_id` for a conversation with its card: the Welcome goes
    /// directly to the card's addresses and, sealed, to its intro mailbox.
    /// The same operation id is the same request.
    pub fn request_contact(
        &mut self,
        name: &str,
        network_id: &str,
        card: &[u8],
        operation_id: &str,
        now: u64,
    ) -> Result<Conversation, CoreError> {
        valid_name(name)?;
        if operation_id.is_empty()
            || operation_id.len() > 128
            || operation_id.chars().any(char::is_control)
        {
            return Err(CoreError::InvalidInput);
        }
        let digest = identity_digest(network_id)?;
        let (mut data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        if identity.network_id == network_id {
            return Err(CoreError::Unauthorized);
        }
        let operation = hex::encode(Sha256::digest(operation_id.as_bytes()));
        let key = format!("intro-request:{operation}");
        if let Some(group) = data.imported.get(&key) {
            let contact = data.contacts.get(group).ok_or(CoreError::InvalidState)?;
            if <[u8; 32]>::from(Sha256::digest(contact.root)) != digest {
                return Err(agentic_store::StoreError::IdempotencyConflict.into());
            }
            return self.conversation(group, contact);
        }
        let (verified, packet) = self.verified_card(network_id, card, now)?;
        let Packet::IntroCard {
            package,
            addresses,
            seal_key,
            ..
        } = packet
        else {
            return Err(CoreError::InvalidInput);
        };
        let group = random_id()?;
        let group_id = hex::encode(group);
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let created = crypto.create_group(group)?;
        let added = MlsClient::from_state(created.next_state).add_members(group, &[package])?;
        // Initial two-party group creation has no earlier membership to order.
        let activated = MlsClient::from_state(added.next_state).activate_pending_commit(group)?;
        let wire = self.sign(
            Packet::Welcome {
                group,
                invitation: verified.id(),
                name: identity.name,
                welcome: added.value.welcome,
            },
            now,
            None,
        )?;
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
        data.imported.insert(key, group_id.clone());
        let seed = random_id()?;
        let sent = StateChange {
            namespace: format!("{SENT}{message_id}"),
            expected_revision: 0,
            bytes: serde_json::to_vec(&Sent {
                card: hex::encode(verified.id()),
                seal_key: hex::encode(seal_key),
                identity: hex::encode(digest),
                seed: hex::encode(seed),
                addresses: vec![],
            })
            .map_err(invalid)?,
        };
        let (mls, records) = crypto_change(Some(&crypto), &activated.next_state, crypto_revision);
        self.store.commit_outgoing_with_retry_states_and_records(
            OutgoingCommit {
                operation_id: format!("intro:{operation}"),
                request_hash: Sha256::digest(&wire).into(),
                message: record(
                    message_id,
                    &group_id,
                    &identity.network_id,
                    now,
                    true,
                    Event::Request,
                )?,
                destination: network_id.into(),
                wire,
                states: vec![data_change(&data, revision)?, mls, sent],
            },
            vec![],
            vec![records],
        )?;
        self.conversation(&group_id, &contact)
    }

    /// What sealing an invitation into `network_id`'s intro mailbox needs,
    /// with the addresses of its direct copy.
    pub(super) fn intro_sent_state(
        &self,
        message_id: &str,
        card: [u8; 32],
        seal_key: [u8; 32],
        network_id: &str,
        addresses: Vec<String>,
    ) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{SENT}{message_id}"),
            expected_revision: 0,
            bytes: serde_json::to_vec(&Sent {
                card: hex::encode(card),
                seal_key: hex::encode(seal_key),
                identity: hex::encode(identity_digest(network_id)?),
                seed: hex::encode(random_id()?),
                addresses,
            })
            .map_err(invalid)?,
        })
    }

    /// The direct addresses of a sent invitation, if `message_id` is one.
    pub(super) fn intro_sent_addresses(
        &self,
        message_id: &str,
    ) -> Result<Option<Vec<String>>, CoreError> {
        self.store
            .state(&format!("{SENT}{message_id}"))?
            .map(|state| {
                serde_json::from_slice::<Sent>(&state.bytes)
                    .map(|sent| sent.addresses)
                    .map_err(|_| CoreError::InvalidState)
            })
            .transpose()
    }

    /// The intro mailbox and sealed envelope of a sent request in `period`.
    pub(super) fn intro_delivery(
        &self,
        message_id: &str,
        period: u64,
        wire: &[u8],
    ) -> Result<([u8; 32], Vec<u8>), CoreError> {
        let state = self
            .store
            .state(&format!("{SENT}{message_id}"))?
            .ok_or(CoreError::InvalidState)?;
        let sent: Sent =
            serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
        let mailbox = intro_mailbox_id(&self.domain, &unhex32(&sent.identity)?, period);
        let envelope = seal(
            &self.domain,
            &unhex32(&sent.card)?,
            &unhex32(&sent.seal_key)?,
            &ephemeral(&unhex32(&sent.seed)?, period),
            wire,
        )?;
        Ok((mailbox, envelope))
    }

    /// An entry read from this profile's intro mailbox: a card is nothing to
    /// do; a request is opened and meets the policy.
    pub fn receive_intro_envelope(
        &mut self,
        envelope: &[u8],
        now: u64,
    ) -> Result<IntroOutcome, CoreError> {
        if envelope.first() == Some(&CARD_ENTRY) {
            return Ok(IntroOutcome::Ignored);
        }
        let (verified, card_id) = self.open_intro_document(envelope, now)?;
        // A member of a private group back after a long absence asks its
        // owner for its place: nothing for the owner to decide.
        if let Ok(Packet::GroupApplication {
            group,
            card,
            rejoin: true,
            ..
        }) = Packet::decode(verified.body(), verified.kind())
        {
            self.receive_rejoin(&verified, group, &card, now)?;
            return Ok(IntroOutcome::Ignored);
        }
        // A closed channel's keys.
        if let Ok(packet @ Packet::ChannelKeys { .. }) =
            Packet::decode(verified.body(), verified.kind())
        {
            return self.take_channel_keys(&verified, packet, Some(&card_id), now);
        }
        let opened = self.intro_request_of(verified, &card_id)?;
        self.intake(
            &opened.verified,
            opened.group,
            opened.card,
            opened.name,
            opened.welcome,
            vec![],
            opened.invite,
            now,
        )
    }

    /// A request delivered directly by the requester's own node: taken like
    /// one read from the intro mailbox, answered with a receipt whatever the
    /// policy decides.
    pub fn receive_intro_from(
        &mut self,
        envelope: &[u8],
        node_record: &[u8],
        actual_peer: &str,
        now: u64,
    ) -> Result<(IntroOutcome, ReceiveOutcome), CoreError> {
        let record = self.verify_node_record(node_record, actual_peer, now)?;
        let opened = self.open_intro_request(envelope, now)?;
        if opened.verified.author() != &record.author {
            return Err(CoreError::Unauthorized);
        }
        let (group, id) = (opened.group, opened.verified.id());
        let outcome = self.intake(
            &opened.verified,
            group,
            opened.card,
            opened.name,
            opened.welcome,
            record.addresses.clone(),
            opened.invite,
            now,
        )?;
        let receipt = self.receipt(group, id, now)?;
        self.remember_verified_node_record(record, now)?;
        Ok((outcome, receipt))
    }

    /// Open a sealed request to one of this profile's cards whose keys are
    /// still kept.
    fn open_intro_request(&self, envelope: &[u8], now: u64) -> Result<OpenedRequest, CoreError> {
        let (verified, card_id) = self.open_intro_document(envelope, now)?;
        self.intro_request_of(verified, &card_id)
    }

    /// The signed document sealed to one of this profile's cards whose keys
    /// are still kept, and that card's id.
    fn open_intro_document(
        &self,
        envelope: &[u8],
        now: u64,
    ) -> Result<(VerifiedDocument, String), CoreError> {
        match envelope.first() {
            Some(&REQUEST_ENTRY) if envelope.len() > 65 => {}
            _ => return Err(CoreError::InvalidInput),
        }
        let card_id = hex::encode(&envelope[1..33]);
        let (cards, _) = self.intro_state::<Cards>(CARDS)?;
        let card = cards
            .cards
            .get(&card_id)
            .filter(|card| !card.keys_gone(now))
            .ok_or(CoreError::InvalidInvitation)?;
        let secret = zeroize::Zeroizing::new(unhex32(&card.seal_secret)?);
        let wire = open(&self.domain, &StaticSecret::from(*secret), envelope)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        root_epoch(&verified)?;
        Ok((verified, card_id))
    }

    fn intro_request_of(
        &self,
        verified: VerifiedDocument,
        card_id: &str,
    ) -> Result<OpenedRequest, CoreError> {
        let (group, invitation, name, welcome, invite) =
            match Packet::decode(verified.body(), verified.kind())? {
                Packet::Welcome {
                    group,
                    invitation,
                    name,
                    welcome,
                } => (group, invitation, name, welcome, None),
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
                } => (
                    group,
                    card,
                    name,
                    welcome,
                    Some(GroupInvite {
                        owner,
                        roster,
                        mailbox,
                        epoch,
                        tree,
                        membership,
                    }),
                ),
                _ => return Err(CoreError::InvalidInput),
            };
        if hex::encode(invitation) != card_id {
            return Err(CoreError::InvalidInvitation);
        }
        Ok(OpenedRequest {
            verified,
            group,
            card: invitation,
            name,
            welcome,
            invite,
        })
    }

    /// The intro mailbox of this profile for the period containing `at`.
    pub fn own_intro_mailbox(&self, at: u64) -> Result<[u8; 32], CoreError> {
        let network = self.store.identity()?.network_id;
        self.intro_mailbox(&network, at)
    }

    /// When this profile's oldest kept card was made: nothing in its intro
    /// mailboxes predates it.
    pub fn intro_since(&self) -> Result<Option<u64>, CoreError> {
        let (cards, _) = self.intro_state::<Cards>(CARDS)?;
        Ok(cards
            .cards
            .values()
            .map(|card| card.expires_at.saturating_sub(CARD_LIFETIME))
            .min())
    }

    /// The last period of this profile's intro mailboxes read completely.
    pub fn intro_read_through(&self) -> Result<Option<u64>, CoreError> {
        Ok(self.intro_state::<ReadThrough>(READ)?.0.period)
    }

    pub fn set_intro_read_through(&mut self, period: u64) -> Result<(), CoreError> {
        let (mut read, revision) = self.intro_state::<ReadThrough>(READ)?;
        if read.period.is_some_and(|through| through >= period) {
            return Ok(());
        }
        read.period = Some(period);
        self.store.commit_states(vec![StateChange {
            namespace: READ.into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&read).map_err(invalid)?,
        }])?;
        Ok(())
    }

    /// The conversation a request made under `operation_id` created.
    pub fn requested_contact(&self, operation_id: &str) -> Result<Option<Conversation>, CoreError> {
        let (data, _) = self.data()?;
        let key = format!(
            "intro-request:{}",
            hex::encode(Sha256::digest(operation_id.as_bytes()))
        );
        data.imported
            .get(&key)
            .map(|group| {
                let contact = data.contacts.get(group).ok_or(CoreError::InvalidState)?;
                self.conversation(group, contact)
            })
            .transpose()
    }

    /// Whether an unstamped Welcome names one of this profile's cards
    /// rather than an invitation it issued.
    pub(super) fn welcome_needs_stamp(&self, invitation: &[u8; 32]) -> Result<bool, CoreError> {
        let (data, _) = self.data()?;
        Ok(!data.issued.contains_key(&hex::encode(invitation)))
    }

    /// A Welcome made with one of this profile's cards meets the policy.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn intake(
        &mut self,
        verified: &VerifiedDocument,
        group: [u8; 32],
        card_id: [u8; 32],
        name: String,
        welcome: Vec<u8>,
        addresses: Vec<String>,
        invite: Option<GroupInvite>,
        now: u64,
    ) -> Result<IntroOutcome, CoreError> {
        let (data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        let peer = network_id_of(verified.author());
        // A group invitation is decided per inviter and group.
        let requester = match &invite {
            Some(_) => format!("{peer}:{}", hex::encode(group)),
            None => peer.clone(),
        };
        if peer == identity.network_id {
            return Err(CoreError::Unauthorized);
        }
        let card = hex::encode(card_id);
        let (cards, _) = self.intro_state::<Cards>(CARDS)?;
        let stored = cards
            .cards
            .get(&card)
            .filter(|stored| !stored.keys_gone(now))
            .ok_or(CoreError::InvalidInvitation)?;
        if verified.issued_at() >= stored.expires_at {
            return Err(CoreError::InvalidInvitation);
        }
        let request_id = hex::encode(verified.id());
        let contact = data.contacts.values().any(|c| c.root == *verified.author());
        let joined = match &invite {
            None => contact,
            // A member back after a long absence, who asked for its place,
            // takes the invitation although it still holds the group.
            Some(_) => {
                let id = hex::encode(group);
                self.member_of(&id)? && !self.applied_to(&id)?
            }
        };
        if self.store.message(&request_id)?.is_some() || joined {
            return Ok(IntroOutcome::Ignored);
        }
        let (mut intake, intake_revision) = self.intro_state::<Intake>(INTAKE)?;
        if let Some(waiting) = intake.waiting.get(&request_id) {
            return Ok(IntroOutcome::Pending(waiting.request.clone()));
        }
        let issued_at = verified.issued_at();
        if intake
            .decided
            .get(&requester)
            .is_some_and(|decided| issued_at <= *decided)
        {
            return Ok(IntroOutcome::Ignored);
        }
        let waiting_for = |waiting: &Waiting| match &waiting.request.group {
            Some(g) => format!("{}:{g}", waiting.request.network_id),
            None => waiting.request.network_id.clone(),
        };
        let previous = intake
            .waiting
            .iter()
            .find(|(_, waiting)| waiting_for(waiting) == requester)
            .map(|(id, waiting)| (id.clone(), waiting.issued_at));
        if previous
            .as_ref()
            .is_some_and(|(_, previous)| issued_at <= *previous)
        {
            return Ok(IntroOutcome::Ignored);
        }
        let (policy, _) = self.intro_state::<IntroPolicy>(POLICY)?;
        let today = period(now);
        if intake.day != today {
            intake.day = today;
            intake.joined = 0;
        }
        // A contact inviting to a group is let in, and so is a group this
        // profile knocked on.
        let listed = policy.allowed.contains(&peer)
            || (invite.is_some() && (contact || self.applied_to(&hex::encode(group))?));
        let by_limit =
            !listed && policy.mode == IntroMode::All && intake.joined < policy.daily_limit;
        if let Some((id, previous)) = previous {
            intake.waiting.remove(&id);
            decide(&mut intake, &requester, previous, now);
        }
        if listed || by_limit {
            if by_limit {
                intake.joined += 1;
            }
            let intake_change = StateChange {
                namespace: INTAKE.into(),
                expected_revision: intake_revision,
                bytes: serde_json::to_vec(&intake).map_err(invalid)?,
            };
            if let Some(invite) = &invite {
                return self.admit_group(
                    *verified.author(),
                    request_id,
                    issued_at,
                    group,
                    name,
                    invite,
                    &welcome,
                    now,
                    vec![intake_change],
                );
            }
            let conversation = self.join_welcome(
                data,
                revision,
                Joining {
                    group,
                    root: *verified.author(),
                    message_id: request_id,
                    issued_at,
                    name,
                    welcome,
                    addresses,
                },
                |_, _| {},
                vec![intake_change],
            )?;
            return Ok(IntroOutcome::Joined(conversation));
        }
        if intake.waiting.len() >= MAX_WAITING {
            return Ok(IntroOutcome::Ignored);
        }
        let request = IntroRequest {
            request_id: request_id.clone(),
            network_id: peer,
            name,
            received_at: now,
            group: invite.as_ref().map(|_| hex::encode(group)),
        };
        intake.waiting.insert(
            request_id,
            Waiting {
                request: request.clone(),
                card,
                group: hex::encode(group),
                root: hex::encode(verified.author()),
                issued_at,
                welcome: hex::encode(welcome),
                addresses,
                invite: invite.as_ref().map(StoredInvite::from),
                keys: String::new(),
            },
        );
        self.store.commit_states(vec![StateChange {
            namespace: INTAKE.into(),
            expected_revision: intake_revision,
            bytes: serde_json::to_vec(&intake).map_err(invalid)?,
        }])?;
        Ok(IntroOutcome::Pending(request))
    }

    /// Requests waiting for the owner, oldest first.
    pub fn intro_requests(&self) -> Result<Vec<IntroRequest>, CoreError> {
        let (intake, _) = self.intro_state::<Intake>(INTAKE)?;
        let mut requests: Vec<IntroRequest> = intake
            .waiting
            .into_values()
            .map(|waiting| waiting.request)
            .collect();
        requests
            .sort_by(|a, b| (a.received_at, &a.request_id).cmp(&(b.received_at, &b.request_id)));
        Ok(requests)
    }

    /// Whether one of this profile's cards, `card`, takes what was signed
    /// at `issued_at`: it still opens what is sealed to it, and was valid
    /// then.
    pub(super) fn own_card_takes(
        &self,
        card: &[u8; 32],
        issued_at: u64,
        now: u64,
    ) -> Result<bool, CoreError> {
        let (cards, _) = self.intro_state::<Cards>(CARDS)?;
        Ok(cards
            .cards
            .get(&hex::encode(card))
            .is_some_and(|stored| !stored.keys_gone(now) && issued_at < stored.expires_at))
    }

    /// Whether a closed channel's keys from `verified`'s author are let in:
    /// a channel this profile knocked on, a contact, one the policy lists,
    /// or anyone within the day's limit when it lets strangers in.
    pub(super) fn keys_allowed(
        &mut self,
        verified: &VerifiedDocument,
        group: &[u8; 32],
        reference: &[u8; 32],
        now: u64,
    ) -> Result<bool, CoreError> {
        let (data, _) = self.data()?;
        let peer = network_id_of(verified.author());
        let contact = data.contacts.values().any(|c| c.root == *verified.author());
        let (policy, _) = self.intro_state::<IntroPolicy>(POLICY)?;
        // A knock counts for the channel knocked on: its owner's, not one
        // who reuses its id.
        let knocked = self.knocked_reference(&hex::encode(group))?.as_deref()
            == Some(hex::encode(reference).as_str());
        if policy.allowed.contains(&peer) || contact || knocked {
            return Ok(true);
        }
        if policy.mode != IntroMode::All {
            return Ok(false);
        }
        let (mut intake, revision) = self.intro_state::<Intake>(INTAKE)?;
        let today = period(now);
        if intake.day != today {
            intake.day = today;
            intake.joined = 0;
        }
        if intake.joined >= policy.daily_limit {
            return Ok(false);
        }
        intake.joined += 1;
        self.store.commit_states(vec![StateChange {
            namespace: INTAKE.into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&intake).map_err(invalid)?,
        }])?;
        Ok(true)
    }

    /// A closed channel's keys kept for this profile's decision.
    pub(super) fn keys_waiting(
        &mut self,
        verified: &VerifiedDocument,
        packet: &Packet,
        now: u64,
    ) -> Result<IntroOutcome, CoreError> {
        let Packet::ChannelKeys {
            group,
            card,
            name,
            owner,
            ..
        } = packet
        else {
            return Err(CoreError::InvalidInput);
        };
        let (mut intake, revision) = self.intro_state::<Intake>(INTAKE)?;
        let request_id = hex::encode(verified.id());
        if let Some(waiting) = intake.waiting.get(&request_id) {
            return Ok(IntroOutcome::Pending(waiting.request.clone()));
        }
        if intake.waiting.len() >= MAX_WAITING {
            return Ok(IntroOutcome::Ignored);
        }
        let conversation = hex::encode(agentic_protocol::group::group_ref(
            &self.domain,
            owner,
            group,
        ));
        let request = IntroRequest {
            request_id: request_id.clone(),
            network_id: network_id_of(verified.author()),
            name: name.clone(),
            received_at: now,
            group: Some(conversation),
        };
        intake.waiting.insert(
            request_id,
            Waiting {
                request: request.clone(),
                card: hex::encode(card),
                group: hex::encode(group),
                root: hex::encode(verified.author()),
                issued_at: verified.issued_at(),
                welcome: String::new(),
                addresses: vec![],
                invite: None,
                keys: hex::encode(packet.encode()?),
            },
        );
        self.store.commit_states(vec![StateChange {
            namespace: INTAKE.into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&intake).map_err(invalid)?,
        }])?;
        Ok(IntroOutcome::Pending(request))
    }

    pub fn accept_intro_request(
        &mut self,
        request_id: &str,
        now: u64,
    ) -> Result<Conversation, CoreError> {
        let (mut intake, intake_revision) = self.intro_state::<Intake>(INTAKE)?;
        let waiting = intake
            .waiting
            .remove(request_id)
            .ok_or(CoreError::InvalidInput)?;
        let (cards, _) = self.intro_state::<Cards>(CARDS)?;
        if cards
            .cards
            .get(&waiting.card)
            .is_none_or(|card| card.keys_gone(now))
        {
            return Err(CoreError::InvalidInvitation);
        }
        let (data, revision) = self.data()?;
        let intake_change = StateChange {
            namespace: INTAKE.into(),
            expected_revision: intake_revision,
            bytes: serde_json::to_vec(&intake).map_err(invalid)?,
        };
        // A closed channel's keys: followed from now.
        if !waiting.keys.is_empty() {
            let packet = Packet::decode(
                &hex::decode(&waiting.keys).map_err(|_| CoreError::InvalidState)?,
                DocumentKind::GroupControl,
            )?;
            let id = self.subscribe_with(packet, waiting.issued_at.min(now), now)?;
            self.store.commit_states(vec![intake_change])?;
            return Ok(Conversation {
                id,
                title: waiting.request.name,
                unread: 0,
                messages: vec![],
            });
        }
        if let Some(invite) = &waiting.invite {
            let group = waiting.group.clone();
            let name = waiting.request.name.clone();
            return match self.admit_group(
                unhex32(&waiting.root)?,
                waiting.request.request_id,
                waiting.issued_at,
                unhex32(&waiting.group)?,
                waiting.request.name,
                &invite.invite()?,
                &hex::decode(&waiting.welcome).map_err(|_| CoreError::InvalidState)?,
                now,
                vec![intake_change],
            )? {
                IntroOutcome::Joined(conversation) => Ok(conversation),
                // A big group appears once its tree is whole.
                _ => Ok(Conversation {
                    id: group,
                    title: name,
                    unread: 0,
                    messages: vec![],
                }),
            };
        }
        self.join_welcome(
            data,
            revision,
            Joining {
                group: unhex32(&waiting.group)?,
                root: unhex32(&waiting.root)?,
                message_id: waiting.request.request_id,
                issued_at: waiting.issued_at,
                name: waiting.request.name,
                welcome: hex::decode(&waiting.welcome).map_err(|_| CoreError::InvalidState)?,
                addresses: waiting.addresses,
            },
            |_, _| {},
            vec![intake_change],
        )
    }

    pub fn reject_intro_request(&mut self, request_id: &str) -> Result<(), CoreError> {
        let (mut intake, revision) = self.intro_state::<Intake>(INTAKE)?;
        let waiting = intake
            .waiting
            .remove(request_id)
            .ok_or(CoreError::InvalidInput)?;
        let requester = match &waiting.request.group {
            Some(group) => format!("{}:{group}", waiting.request.network_id),
            None => waiting.request.network_id.clone(),
        };
        decide(
            &mut intake,
            &requester,
            waiting.issued_at,
            waiting.issued_at,
        );
        self.store.commit_states(vec![StateChange {
            namespace: INTAKE.into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&intake).map_err(invalid)?,
        }])?;
        Ok(())
    }

    pub fn intro_policy(&self) -> Result<IntroPolicy, CoreError> {
        Ok(self.intro_state::<IntroPolicy>(POLICY)?.0)
    }

    pub fn set_intro_policy(&mut self, policy: IntroPolicy) -> Result<(), CoreError> {
        if policy.daily_limit > MAX_DAILY_LIMIT || policy.allowed.len() > MAX_ALLOWED {
            return Err(CoreError::InvalidInput);
        }
        for id in &policy.allowed {
            identity_digest(id)?;
        }
        let (_, revision) = self.intro_state::<IntroPolicy>(POLICY)?;
        self.store.commit_states(vec![StateChange {
            namespace: POLICY.into(),
            expected_revision: revision,
            bytes: serde_json::to_vec(&policy).map_err(invalid)?,
        }])?;
        Ok(())
    }
}

/// Remember that `peer`'s request issued at `issued_at` was decided, so it
/// and older ones are ignored when read again. Entries go once no card
/// whose keys are kept could have served them.
fn decide(intake: &mut Intake, peer: &str, issued_at: u64, now: u64) {
    let latest = intake.decided.entry(peer.into()).or_insert(issued_at);
    *latest = (*latest).max(issued_at);
    let horizon = CARD_LIFETIME + KEEP_AFTER;
    intake
        .decided
        .retain(|_, issued| issued.saturating_add(horizon) > now);
    while intake.decided.len() > MAX_DECIDED {
        let Some(oldest) = intake
            .decided
            .iter()
            .min_by_key(|(_, issued)| **issued)
            .map(|(peer, _)| peer.clone())
        else {
            break;
        };
        intake.decided.remove(&oldest);
    }
}

fn network_id_of(root: &[u8; 32]) -> String {
    network_id(root)
}
