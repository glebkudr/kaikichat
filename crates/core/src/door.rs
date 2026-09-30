//! A group's door (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 5): a
//! public group or one by request lets people ask to join. Its owner and
//! admins publish a door card into the door mailbox `H(domain, G, period)`;
//! an applicant leaves there an application sealed to the card's key, paid
//! like a message. In a public group an owner's or admin's node lets it in
//! at once, in one by request it waits for their decision; once a minute a
//! batch commit adds everyone let in, invited as usual. The applicant's node
//! takes the invitation of a group it knocked on whatever its own policy.
use super::groups::GroupChange;
use super::intro::{REQUEST_ENTRY, ephemeral, identity_digest, open, seal};
use super::*;
use agentic_mailbox_swarm::address::{door_mailbox_id, intro_mailbox_id, period};
use agentic_protocol::group::{Access, DoorCard, VerifiedDoor, verify_door_card};
use std::collections::BTreeMap;
use x25519_dalek::{PublicKey, StaticSecret};

/// An owner's or admin's intake of a group's door.
const DOORS: &str = "groups/door/";
/// Groups this profile knocked on, by group id.
const KNOCKED: &str = "groups/knocked/";
/// How a queued application is sealed, by message id.
const KNOCKS: &str = "groups/knock/";
/// The door card entry of a door mailbox; applications are `REQUEST_ENTRY`.
pub const DOOR_CARD: u8 = 1;
/// Characters of an applicant's note at most.
const MAX_NOTE: usize = 280;
/// Applications waiting for a decision per group at most.
const MAX_WAITING: usize = 1000;
/// Applicants let in and not yet added per group at most.
const MAX_ADMITTED: usize = 2000;
/// Refusals remembered per group.
const MAX_REFUSED: usize = 4096;
/// Members one batch adds at most.
const MAX_BATCH: usize = 300;

/// What an application read from a door came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoorEntry {
    /// A public group's applicant, let in: the next batch adds it.
    Admitted(String),
    /// An application to a group by request, waiting for a decision.
    Waiting(DoorRequest),
    /// Not for this door, already known, from a member, a banned id or a
    /// refused applicant, or the door is closed.
    Ignored,
}

/// An application waiting for the owner's or an admin's decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorRequest {
    pub request_id: String,
    pub network_id: String,
    pub note: String,
    pub received_at: u64,
    /// A member back from a long absence.
    pub rejoin: bool,
}

/// A group's door as an applicant reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorInfo {
    pub group_id: String,
    pub name: String,
    /// The owner's network id.
    pub owner: String,
    /// `public` or `request`.
    pub access: String,
}

/// An application queued for a group's door.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorApplication {
    pub group_id: String,
    pub message_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Waiting {
    network_id: String,
    note: String,
    received_at: u64,
    issued_at: u64,
    /// The applicant's intro card envelope, in hex.
    card: String,
    rejoin: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Admitted {
    card: String,
    issued_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Door {
    /// By request id.
    waiting: BTreeMap<String, Waiting>,
    /// By network id.
    admitted: BTreeMap<String, Admitted>,
    /// Refused applicants by network id: their applications issued up to
    /// then stay refused.
    refused: BTreeMap<String, u64>,
    /// Members back after a long absence, by network id: the next batch
    /// gives them their place again.
    #[serde(default)]
    rejoining: BTreeMap<String, Admitted>,
    /// Their applications already answered with a batch, by network id.
    #[serde(default)]
    rejoined: BTreeMap<String, u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Knock {
    /// `G` and the door key the application is sealed to, in hex.
    group: String,
    door_key: String,
    seed: String,
    /// Sealed instead to the owner's intro card: the owner's id digest, the
    /// card's id and seal key, in hex.
    #[serde(default)]
    owner: String,
    #[serde(default)]
    card: String,
    #[serde(default)]
    seal_key: String,
}

/// The door key of a group mailbox secret: it changes with the mailbox.
fn door_secret(mailbox: &[u8; 32]) -> StaticSecret {
    let mut hash = Sha256::new();
    hash.update(b"AIN_GROUP_DOOR_KEY_V1");
    hash.update(mailbox);
    StaticSecret::from(<[u8; 32]>::from(hash.finalize()))
}

fn access_name(access: Access) -> &'static str {
    match access {
        Access::Public => "public",
        Access::Request => "request",
        Access::Private => "private",
    }
}

fn unhex32(text: &str) -> Result<[u8; 32], CoreError> {
    hex::decode(text)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(CoreError::InvalidState)
}

impl AppCore {
    fn door_state(&self, id: &str) -> Result<(Door, u64), CoreError> {
        match self.store.state(&format!("{DOORS}{id}"))? {
            None => Ok((Door::default(), 0)),
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
        }
    }

    fn save_door(&mut self, id: &str, door: &Door, revision: u64) -> Result<(), CoreError> {
        self.store.commit_states(vec![StateChange {
            namespace: format!("{DOORS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(door).map_err(invalid)?,
        }])?;
        Ok(())
    }

    /// A closed door forgets whom it let in or kept waiting.
    fn drop_door(&mut self, id: &str) -> Result<(), CoreError> {
        let name = format!("{DOORS}{id}");
        if let Some(state) = self.store.state(&name)? {
            self.store
                .commit_state_maintenance(vec![], vec![(name, state.revision)])?;
        }
        Ok(())
    }

    /// The door mailbox of group `G` for the period containing `at`.
    pub fn door_mailbox(&self, group: &[u8; 32], at: u64) -> [u8; 32] {
        door_mailbox_id(&self.domain, group, period(at))
    }

    /// The door mailboxes of the groups this profile opens as owner or admin,
    /// for the period containing `at`.
    pub fn door_mailboxes(&self, at: u64) -> Result<Vec<(String, [u8; 32])>, CoreError> {
        let mut doors = vec![];
        for info in self.groups()? {
            if let Some(view) = self.door_view(&info.id)?
                && view.admin
                && view.door
            {
                doors.push((info.id, self.door_mailbox(&view.group, at)));
            }
        }
        Ok(doors)
    }

    /// The door card an owner or admin of a group with a door publishes
    /// now, stamped for this period's door mailbox. `None` for a plain
    /// member or a private group.
    pub fn door_card(&mut self, id: &str, now: u64) -> Result<Option<SwarmDelivery>, CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Ok(None);
        };
        if !view.admin || !view.door {
            return Ok(None);
        }
        let current = view.mailboxes.first().ok_or(CoreError::InvalidState)?;
        let card = DoorCard {
            group_id: parse_id(id)?,
            owner: view.owner,
            roster: view.roster,
            name: view.name,
            door_key: PublicKey::from(&door_secret(current)).to_bytes(),
        };
        let wire = self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::GroupDoor,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: card.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire();
        let envelope = [&[DOOR_CARD][..], &wire].concat();
        let pinned = period(now);
        let mailbox = door_mailbox_id(&self.domain, &view.group, pinned);
        let (stamp, changes) = self.stamp_changes(&mailbox, pinned, &envelope, now)?;
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(Some(SwarmDelivery {
            message_id: String::new(),
            conversation_id: id.into(),
            period: pinned,
            mailbox,
            envelope,
            stamp,
        }))
    }

    fn verified_door(
        &self,
        group: &[u8; 32],
        envelope: &[u8],
        now: u64,
    ) -> Result<VerifiedDoor, CoreError> {
        if envelope.first() != Some(&DOOR_CARD) {
            return Err(CoreError::InvalidInput);
        }
        Ok(verify_door_card(&envelope[1..], self.domain, now, group)?)
    }

    /// A door card read from group `G`'s door mailbox, as an applicant sees
    /// it.
    pub fn open_door_card(
        &self,
        group: &[u8; 32],
        envelope: &[u8],
        now: u64,
    ) -> Result<DoorInfo, CoreError> {
        let door = self.verified_door(group, envelope, now)?;
        Ok(DoorInfo {
            group_id: hex::encode(door.card.group_id),
            name: door.card.name,
            owner: network_id(&door.card.owner),
            access: access_name(door.access).into(),
        })
    }

    /// Knock at the door of group `G` whose card is `card`: an application
    /// with this profile's intro card and `note`, queued for the door
    /// mailbox and paid like a message. The same operation id is the same
    /// application.
    pub fn apply_to_group(
        &mut self,
        group: &[u8; 32],
        card: &[u8],
        note: &str,
        operation_id: &str,
        now: u64,
    ) -> Result<DoorApplication, CoreError> {
        if note.chars().count() > MAX_NOTE || note.chars().any(char::is_control) {
            return Err(CoreError::InvalidInput);
        }
        let door = self.verified_door(group, card, now)?;
        let knock = Knock {
            group: hex::encode(group),
            door_key: hex::encode(door.card.door_key),
            seed: hex::encode(random_id()?),
            owner: String::new(),
            card: String::new(),
            seal_key: String::new(),
        };
        self.knock(door.card.group_id, knock, note, false, operation_id, now)
    }

    /// Back after a long absence in group `G` (a stale group of this
    /// profile), ask for its place again at the door of `card`.
    pub fn rejoin_group(
        &mut self,
        group: &[u8; 32],
        card: &[u8],
        operation_id: &str,
        now: u64,
    ) -> Result<DoorApplication, CoreError> {
        let door = self.verified_door(group, card, now)?;
        if !self.member_of(&hex::encode(door.card.group_id))? {
            return Err(CoreError::UnknownConversation);
        }
        let knock = Knock {
            group: hex::encode(group),
            door_key: hex::encode(door.card.door_key),
            seed: hex::encode(random_id()?),
            owner: String::new(),
            card: String::new(),
            seal_key: String::new(),
        };
        self.knock(door.card.group_id, knock, "", true, operation_id, now)
    }

    /// Back after a long absence in a group without a door, ask its owner
    /// for its place again, sealed to the owner's intro `card`.
    pub fn rejoin_by_owner(
        &mut self,
        id: &str,
        card: &[u8],
        operation_id: &str,
        now: u64,
    ) -> Result<DoorApplication, CoreError> {
        let owner = self.group(id)?.owner;
        let (verified, packet) = self.verified_card(&owner, card, now)?;
        let Packet::IntroCard { seal_key, .. } = packet else {
            return Err(CoreError::InvalidInput);
        };
        let knock = Knock {
            group: String::new(),
            door_key: String::new(),
            seed: hex::encode(random_id()?),
            owner: hex::encode(identity_digest(&owner)?),
            card: hex::encode(verified.id()),
            seal_key: hex::encode(seal_key),
        };
        self.knock(parse_id(id)?, knock, "", true, operation_id, now)
    }

    /// Queue an application to group `group_id` with this profile's intro
    /// card, to be sealed as `knock` says.
    fn knock(
        &mut self,
        group_id: [u8; 32],
        knock: Knock,
        note: &str,
        rejoin: bool,
        operation_id: &str,
        now: u64,
    ) -> Result<DoorApplication, CoreError> {
        let group_id_hex = hex::encode(group_id);
        if operation_id.is_empty() || operation_id.len() > 128 {
            return Err(CoreError::InvalidInput);
        }
        let operation = format!(
            "group-knock:{}",
            hex::encode(Sha256::digest(operation_id.as_bytes()))
        );
        let group_id = group_id_hex;
        if let Some(sent) = self.store.operation_message(&operation)? {
            return Ok(DoorApplication {
                group_id,
                message_id: sent.record.id,
            });
        }
        let own = self.intro_card(vec![], now)?.envelope;
        let wire = self.sign(
            Packet::GroupApplication {
                group: parse_id(&group_id)?,
                card: own,
                note: note.into(),
                rejoin,
            },
            now,
            None,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let identity = self.store.identity()?.network_id;
        let states = vec![
            StateChange {
                namespace: format!("{KNOCKS}{message_id}"),
                expected_revision: 0,
                bytes: serde_json::to_vec(&knock).map_err(invalid)?,
            },
            StateChange {
                namespace: format!("{KNOCKED}{group_id}"),
                expected_revision: self
                    .store
                    .state(&format!("{KNOCKED}{group_id}"))?
                    .map_or(0, |state| state.revision),
                // When, and at whose group's door: `G`.
                bytes: serde_json::to_vec(&(now, knock.group.clone())).map_err(invalid)?,
            },
        ];
        self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: operation,
                request_hash: Sha256::digest(&wire).into(),
                message: agentic_store::MessageRecord {
                    id: message_id.clone(),
                    conversation_id: group_id.clone(),
                    author: identity,
                    created_at: now,
                    own: true,
                    content: serde_json::to_vec(&Event::Knock).map_err(invalid)?,
                },
                destination: group_id.clone(),
                wire,
                states,
            },
            vec![],
            vec![],
        )?;
        Ok(DoorApplication {
            group_id,
            message_id,
        })
    }

    /// Where a queued application goes in `period`, sealed to its door.
    pub(super) fn knock_delivery(
        &self,
        message_id: &str,
        period: u64,
        wire: &[u8],
    ) -> Result<([u8; 32], Vec<u8>), CoreError> {
        let state = self
            .store
            .state(&format!("{KNOCKS}{message_id}"))?
            .ok_or(CoreError::InvalidState)?;
        let knock: Knock =
            serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
        let seed = unhex32(&knock.seed)?;
        if !knock.owner.is_empty() {
            // To the owner's intro mailbox, sealed to its card.
            let envelope = seal(
                &self.domain,
                &unhex32(&knock.card)?,
                &unhex32(&knock.seal_key)?,
                &ephemeral(&seed, period),
                wire,
            )?;
            return Ok((
                intro_mailbox_id(&self.domain, &unhex32(&knock.owner)?, period),
                envelope,
            ));
        }
        let door_key = unhex32(&knock.door_key)?;
        let envelope = seal(
            &self.domain,
            &door_key,
            &door_key,
            &ephemeral(&seed, period),
            wire,
        )?;
        Ok((
            door_mailbox_id(&self.domain, &unhex32(&knock.group)?, period),
            envelope,
        ))
    }

    /// Whether this profile knocked on group `id` and is not in it yet.
    pub(super) fn applied_to(&self, id: &str) -> Result<bool, CoreError> {
        Ok(self.store.state(&format!("{KNOCKED}{id}"))?.is_some())
    }

    /// `G` of the group `id` this profile knocked on, in hex; `None` when it
    /// did not, or knocked before the knock named it.
    pub(super) fn knocked_reference(&self, id: &str) -> Result<Option<String>, CoreError> {
        Ok(self
            .store
            .state(&format!("{KNOCKED}{id}"))?
            .and_then(|state| serde_json::from_slice::<(u64, String)>(&state.bytes).ok())
            .map(|(_, reference)| reference))
    }

    /// Applications of `network_id` at group `id`'s door, answered by
    /// another admin: dropped here too.
    pub(super) fn door_answered(
        &self,
        id: &str,
        network_id: &str,
    ) -> Result<Vec<StateChange>, CoreError> {
        let (mut door, revision) = self.door_state(id)?;
        let before = door.waiting.len();
        door.waiting.retain(|_, w| w.network_id != network_id);
        if door.waiting.len() == before {
            return Ok(vec![]);
        }
        Ok(vec![StateChange {
            namespace: format!("{DOORS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(&door).map_err(invalid)?,
        }])
    }

    /// The knock at group `id` was answered: this profile joined.
    pub(super) fn forget_knock(&mut self, id: &str) -> Result<(), CoreError> {
        let name = format!("{KNOCKED}{id}");
        if let Some(state) = self.store.state(&name)? {
            self.store
                .commit_state_maintenance(vec![], vec![(name, state.revision)])?;
        }
        Ok(())
    }

    /// An entry read from the door mailbox of group `id`, which this
    /// profile opens as owner or admin.
    pub fn receive_door_entry(
        &mut self,
        id: &str,
        _period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<DoorEntry, CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Ok(DoorEntry::Ignored);
        };
        if !view.admin || !view.door {
            self.drop_door(id)?;
            return Ok(DoorEntry::Ignored);
        }
        if envelope.first() != Some(&REQUEST_ENTRY) || envelope.len() <= 65 {
            return Ok(DoorEntry::Ignored);
        }
        // Sealed to the key of one of the mailboxes kept.
        let Some(secret) = view
            .mailboxes
            .iter()
            .map(door_secret)
            .find(|secret| PublicKey::from(secret).to_bytes().as_slice() == &envelope[1..33])
        else {
            return Ok(DoorEntry::Ignored);
        };
        let Ok(wire) = open(&self.domain, &secret, envelope) else {
            return Ok(DoorEntry::Ignored);
        };
        let Ok(verified) = VerifiedDocument::decode(&wire, self.domain, now) else {
            return Ok(DoorEntry::Ignored);
        };
        let Ok(Packet::GroupApplication {
            group,
            card,
            note,
            rejoin,
        }) = Packet::decode(verified.body(), verified.kind())
        else {
            return Ok(DoorEntry::Ignored);
        };
        let applicant = network_id(verified.author());
        if hex::encode(group) != id
            || note.chars().count() > MAX_NOTE
            || self.verified_card(&applicant, &card, now).is_err()
            || view.banned.contains(&applicant)
        {
            return Ok(DoorEntry::Ignored);
        }
        if view.members.contains(&applicant) {
            // A member back after a long absence takes its place again,
            // whatever the mode; any other member has nothing to ask.
            return match rejoin && self.take_rejoin(id, &view, &verified, &card)? {
                true => Ok(DoorEntry::Admitted(applicant)),
                false => Ok(DoorEntry::Ignored),
            };
        }
        let request_id = hex::encode(verified.id());
        let issued_at = verified.issued_at();
        let (mut door, revision) = self.door_state(id)?;
        if door
            .refused
            .get(&applicant)
            .is_some_and(|refused| issued_at <= *refused)
            || door.waiting.contains_key(&request_id)
            || door
                .admitted
                .get(&applicant)
                .is_some_and(|admitted| issued_at <= admitted.issued_at)
        {
            return Ok(DoorEntry::Ignored);
        }
        // One application per applicant: a later one replaces it.
        if door
            .waiting
            .values()
            .any(|w| w.network_id == applicant && w.issued_at >= issued_at)
        {
            return Ok(DoorEntry::Ignored);
        }
        door.waiting.retain(|_, w| w.network_id != applicant);
        let entry = if view.access == Access::Public {
            if door.admitted.len() >= MAX_ADMITTED {
                return Ok(DoorEntry::Ignored);
            }
            door.admitted.insert(
                applicant.clone(),
                Admitted {
                    card: hex::encode(&card),
                    issued_at,
                },
            );
            DoorEntry::Admitted(applicant)
        } else {
            if door.waiting.len() >= MAX_WAITING {
                return Ok(DoorEntry::Ignored);
            }
            let waiting = Waiting {
                network_id: applicant,
                note,
                received_at: now,
                issued_at,
                card: hex::encode(&card),
                rejoin,
            };
            let request = request_of(&request_id, &waiting);
            door.waiting.insert(request_id, waiting);
            DoorEntry::Waiting(request)
        };
        self.save_door(id, &door, revision)?;
        Ok(entry)
    }

    /// A member of group `id` asks for its place again: let in for the next
    /// batch when it is a plain member, not banned, with a valid card.
    fn take_rejoin(
        &mut self,
        id: &str,
        view: &super::groups::DoorView,
        verified: &VerifiedDocument,
        card: &[u8],
    ) -> Result<bool, CoreError> {
        let member = network_id(verified.author());
        if !view.admin
            || !view.members.contains(&member)
            || view.banned.contains(&member)
            || *verified.author() == view.owner
            || view.admins.contains(verified.author())
        {
            return Ok(false);
        }
        let issued_at = verified.issued_at();
        let (mut door, revision) = self.door_state(id)?;
        if door
            .rejoined
            .get(&member)
            .is_some_and(|answered| issued_at <= *answered)
            || door
                .rejoining
                .get(&member)
                .is_some_and(|known| issued_at <= known.issued_at)
        {
            return Ok(false);
        }
        door.rejoining.insert(
            member,
            Admitted {
                card: hex::encode(card),
                issued_at,
            },
        );
        self.save_door(id, &door, revision)?;
        Ok(true)
    }

    /// An application read from this profile's intro mailbox asking for a
    /// place again in group `group`, of which this profile is the owner.
    pub(super) fn receive_rejoin(
        &mut self,
        verified: &VerifiedDocument,
        group: [u8; 32],
        card: &[u8],
        now: u64,
    ) -> Result<(), CoreError> {
        let id = hex::encode(group);
        let member = network_id(verified.author());
        let Some(view) = self.door_view(&id)? else {
            return Ok(());
        };
        if self.verified_card(&member, card, now).is_ok() {
            self.take_rejoin(&id, &view, verified, card)?;
        }
        Ok(())
    }

    /// A notice among group `id`'s members from `author`: a refusal at its
    /// door by its owner or an admin holds at this admin's door too.
    pub(super) fn group_notice(
        &self,
        id: &str,
        author: &[u8; 32],
        body: &[u8],
    ) -> Result<Vec<StateChange>, CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Ok(vec![]);
        };
        if !view.admin || (*author != view.owner && !view.admins.contains(author)) {
            return Ok(vec![]);
        }
        let mut d = minicbor::Decoder::new(body);
        let refused = (|| {
            if d.array().ok()? != Some(3) || d.str().ok()? != "door-refused" {
                return None;
            }
            Some((d.str().ok()?.to_owned(), d.u64().ok()?))
        })();
        let Some((network_id, issued_at)) = refused else {
            return Ok(vec![]);
        };
        let (mut door, revision) = self.door_state(id)?;
        door.waiting
            .retain(|_, w| w.network_id != network_id || w.issued_at > issued_at);
        let refused = door.refused.entry(network_id).or_default();
        *refused = (*refused).max(issued_at);
        Ok(vec![StateChange {
            namespace: format!("{DOORS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(&door).map_err(invalid)?,
        }])
    }

    /// Applications to group `id` waiting for a decision, oldest first.
    pub fn door_requests(&self, id: &str) -> Result<Vec<DoorRequest>, CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Err(CoreError::UnknownConversation);
        };
        if !view.admin {
            return Err(CoreError::Unauthorized);
        }
        if !view.door {
            return Ok(vec![]);
        }
        let (door, _) = self.door_state(id)?;
        let mut requests: Vec<DoorRequest> = door
            .waiting
            .iter()
            .map(|(request_id, waiting)| request_of(request_id, waiting))
            .collect();
        requests
            .sort_by(|a, b| (a.received_at, &a.request_id).cmp(&(b.received_at, &b.request_id)));
        Ok(requests)
    }

    /// Let the applicant of a waiting application in, or refuse it.
    pub fn decide_door_request(
        &mut self,
        id: &str,
        request_id: &str,
        accept: bool,
        now: u64,
    ) -> Result<(), CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Err(CoreError::UnknownConversation);
        };
        if !view.admin {
            return Err(CoreError::Unauthorized);
        }
        let (mut door, revision) = self.door_state(id)?;
        let waiting = door
            .waiting
            .remove(request_id)
            .ok_or(CoreError::InvalidInput)?;
        // A closed channel's applicant gets its keys at once, alone; the
        // application is answered once they are.
        if accept && view.channel {
            let card = hex::decode(&waiting.card).map_err(|_| CoreError::InvalidState)?;
            self.channel_subscribe(
                id,
                &Invitee {
                    network_id: waiting.network_id,
                    card,
                },
                &format!("door-{request_id}"),
                now,
            )?;
            let (mut door, revision) = self.door_state(id)?;
            door.waiting.remove(request_id);
            return self.save_door(id, &door, revision);
        }
        if accept {
            if door.admitted.len() >= MAX_ADMITTED {
                return Err(CoreError::InvalidInput);
            }
            door.admitted.insert(
                waiting.network_id.clone(),
                Admitted {
                    card: waiting.card.clone(),
                    issued_at: waiting.issued_at,
                },
            );
        } else {
            door.refused
                .insert(waiting.network_id.clone(), waiting.issued_at);
            while door.refused.len() > MAX_REFUSED {
                door.refused.pop_first();
            }
        }
        self.save_door(id, &door, revision)?;
        if !accept {
            // The other admins drop it too.
            let mut body = minicbor::Encoder::new(Vec::new());
            body.array(3)
                .and_then(|e| e.str("door-refused"))
                .and_then(|e| e.str(&waiting.network_id))
                .and_then(|e| e.u64(waiting.issued_at))
                .map_err(invalid)?;
            self.send_group_notice(id, &body.into_writer(), now)?;
        }
        Ok(())
    }

    /// Add everyone let in and not yet a member, at most 300, with one
    /// commit: the batch of this minute. Asked again within the minute it is
    /// the same commit; `None` when nobody is to be added, or while another
    /// commit of this profile waits for the notary.
    pub fn door_batch(&mut self, id: &str, now: u64) -> Result<Option<GroupCommitMade>, CoreError> {
        let Some(view) = self.door_view(id)? else {
            return Ok(None);
        };
        if !view.admin {
            return Ok(None);
        }
        let operation = format!("door:{}", now / 60);
        if let Some(made) = self.group_operation(id, &operation)? {
            return Ok(Some(made));
        }
        let (mut door, revision) = self.door_state(id)?;
        let before = (
            door.admitted.len(),
            door.waiting.len(),
            door.rejoining.len(),
        );
        if !view.door {
            // A closed door forgets whom it let in or kept waiting; members
            // back after a long absence still take their place.
            door.admitted.clear();
            door.waiting.clear();
        }
        let card_holds = |network_id: &String, admitted: &Admitted| {
            hex::decode(&admitted.card)
                .ok()
                .is_some_and(|card| self.verified_card(network_id, &card, now).is_ok())
        };
        // Those in by now, banned meanwhile, or whose card lapsed are let go;
        // so are rejoiners no longer members.
        door.admitted.retain(|network_id, admitted| {
            !view.members.contains(network_id)
                && !view.banned.contains(network_id)
                && card_holds(network_id, admitted)
        });
        door.rejoining.retain(|network_id, admitted| {
            view.members.contains(network_id)
                && !view.banned.contains(network_id)
                && card_holds(network_id, admitted)
        });
        let invitee = |(network_id, admitted): (&String, &Admitted)| {
            Ok(Invitee {
                network_id: network_id.clone(),
                card: hex::decode(&admitted.card).map_err(|_| CoreError::InvalidState)?,
            })
        };
        let replace: Vec<Invitee> = door
            .rejoining
            .iter()
            .take(MAX_BATCH)
            .map(invitee)
            .collect::<Result<_, CoreError>>()?;
        let add: Vec<Invitee> = door
            .admitted
            .iter()
            .take(MAX_BATCH - replace.len())
            .map(invitee)
            .collect::<Result<_, CoreError>>()?;
        let changed = before
            != (
                door.admitted.len(),
                door.waiting.len(),
                door.rejoining.len(),
            );
        if (add.is_empty() && replace.is_empty()) || view.busy {
            if changed {
                self.save_door(id, &door, revision)?;
            }
            return Ok(None);
        }
        let answered: Vec<(String, u64)> = replace
            .iter()
            .filter_map(|r| {
                door.rejoining
                    .get(&r.network_id)
                    .map(|a| (r.network_id.clone(), a.issued_at))
            })
            .collect();
        let made = match self.change_group(
            id,
            GroupChange {
                add,
                replace,
                ..GroupChange::default()
            },
            &operation,
            now,
        ) {
            Ok(made) => made,
            Err(CoreError::GroupBusy) => return Ok(None),
            Err(error) => return Err(error),
        };
        // Rejoiners are answered by this batch; asked again, they wait for
        // the next absence.
        let (mut door, revision) = (door, self.door_state(id)?.1);
        for (network_id, issued_at) in answered {
            door.rejoining.remove(&network_id);
            door.rejoined.insert(network_id, issued_at);
        }
        self.save_door(id, &door, revision)?;
        Ok(Some(made))
    }
}

fn request_of(request_id: &str, waiting: &Waiting) -> DoorRequest {
    DoorRequest {
        request_id: request_id.into(),
        network_id: waiting.network_id.clone(),
        note: waiting.note.clone(),
        received_at: waiting.received_at,
        rejoin: waiting.rejoin,
    }
}
