//! Groups (spec/groups-v1.md): MLS groups with an owner-signed roster, one
//! group mailbox per epoch, commits applied in the order the notary names,
//! and invitations through the invitees' intro mailboxes.
use super::group_parts::TreeRef;
use super::intro::identity_digest;
use super::*;
use agentic_crypto::mailbox::MailboxSecret;
use agentic_mailbox_swarm::address::period;
use agentic_protocol::group::{
    Access, BANNED, Ban, Bans, CommitClaim, GroupKind, KeyTree, MAX_ADMINS, MAX_BANS, MAX_BRANCHES,
    MAX_TREE_LOG, MemberCert, PublicPost, PublicRoster, Retention, Roster, check_bans,
    check_transition, group_ref, verify_claim, verify_roster,
};
use std::collections::BTreeMap;

/// Invitees' KeyPackages, and their root keys by network id.
type InviteeKeys = (Vec<Vec<u8>>, BTreeMap<String, String>);
/// An open group's reference `G` and a roster document of it.
pub(super) type PublicRosterWire = ([u8; 32], Vec<u8>);

const GROUPS: &str = "groups/state/";
/// The epoch a group message or commit was written in, by message id.
const SENDS: &str = "groups/send/";
/// Messages sent as public posts of an open group, by message id.
const PUBLIC_SENDS: &str = "groups/public-send/";
/// What a notice of this profile says, by message id, until holders store
/// it: sealed again from it should it wait too long.
const NOTICE_BODIES: &str = "groups/notice/";
/// Epochs a message may wait in its sender's outbox before it is sealed
/// again: members keep the keys of three past epochs only.
const RESEAL_EPOCHS: u64 = 3;
/// A group's invitations to send once its pending commit wins, apart from
/// its state, which every look at the group reads.
const INVITATIONS: &str = "groups/invite/";
/// The owner's record of members its admins let in: network id → the
/// admin's. Should it demote that admin, it gives them certificates of its
/// own.
const ISSUED: &str = "groups/issued/";
/// Certificates one notice carries at most.
const CERTS_PER_NOTICE: usize = 200;
const MAX_MEMBERS: usize = 2000;
/// Members one commit adds at most: their Welcome still fits one
/// invitation.
const MAX_ADDS: usize = 300;
/// Epochs whose removals are remembered: older epochs no longer open.
const KEPT_EPOCHS: usize = 4;
/// Mailbox secrets kept, each from the epoch it began: a late message or
/// commit still goes to the mailbox of the epoch it was written in; readers
/// read the newest two.
const KEPT_MAILBOXES: usize = 4;
/// Room an invitation keeps beside its Welcome and a tree that comes along.
const INVITATION_ROOM: usize = 2048;
/// The first byte of a notice among a group's members; talk is text, which
/// never begins so.
const NOTICE: u8 = 0;
/// Operations remembered per group for retries.
const MAX_OPERATIONS: usize = 64;

/// A member's view of a group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub epoch: u64,
    pub owner: String,
    pub admins: Vec<String>,
    pub members: Vec<String>,
    /// This profile's role: `owner`, `admin` or `member`.
    pub role: String,
    /// `private`, or `public`: anyone reads it (open-read groups).
    pub access: String,
    /// `G`, the group's reference, in hex: what readers follow.
    pub group_ref: String,
    /// Ids nobody adds until they are unbanned.
    pub banned: Vec<BannedId>,
    /// `group`, or `channel`: only its owner and admins are in it and
    /// write.
    pub kind: String,
    /// Days its history is kept for its readers, `None` for ever: a
    /// channel's retention, a day for a group.
    pub retention: Option<u32>,
}

/// A group this profile fell behind in: away longer than its mailboxes
/// keep, it asks for its place again (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
/// part 7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleGroup {
    pub group_id: String,
    /// `G` in hex: where its door is.
    pub group_ref: String,
    /// The owner's network id: whom to ask when it has no door.
    pub owner: String,
    /// `public`, `request` or `private`, as last known.
    pub access: String,
}

/// An id banned from a group, and whether the owner banned it: then only
/// the owner lifts the ban.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BannedId {
    pub id: String,
    pub by_owner: bool,
}

/// Someone to add: a network id and its intro card.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invitee {
    pub network_id: String,
    pub card: Vec<u8>,
}

/// A change of a group's members, admins or bans.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupChange {
    pub add: Vec<Invitee>,
    pub remove: Vec<String>,
    /// The new list of admins (the owner's only).
    pub admins: Option<Vec<String>>,
    /// Open the group to anyone's reading, or close it (the owner's only).
    pub access: Option<Access>,
    /// Ids to ban: members among them are removed by the same commit.
    pub ban: Vec<String>,
    pub unban: Vec<String>,
    /// Plain members back after a long absence: their leaf is replaced by
    /// the KeyPackage of their card, which is no removal.
    pub replace: Vec<Invitee>,
    /// How long a channel keeps its history (its owner's or an admin's).
    pub retention: Option<Retention>,
    /// Subscribers of a closed channel to remove, by network id: their
    /// keys stop opening anything new.
    pub unsubscribe: Vec<String>,
    /// Give a closed channel's subscribers keys of a new seed.
    pub reseed: bool,
}

/// A commit this profile made, waiting for the notary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupCommitMade {
    pub epoch: u64,
    pub commit: String,
    pub message_id: String,
}

/// A claim on an epoch's commit, as the notary takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupClaim {
    pub epoch: u64,
    pub round: u32,
    pub commit: String,
    pub claim: Vec<u8>,
    /// This profile's own pending commit: the message that stores it.
    pub message_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitDecision {
    /// The named commit is applied: the group is at the next epoch.
    Applied,
    /// This profile's own commit lost; it made nothing.
    Lost,
    /// The named commit is not read yet; it is applied when it is.
    Waiting,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    epoch: u64,
    round: u32,
    claim: String,
    commit: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredInvitee {
    network_id: String,
    card: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    epoch: u64,
    commit: String,
    operation: String,
    message_id: String,
    /// Kept apart now (`INVITATIONS`); older states still name it.
    #[serde(default, skip_serializing, rename = "invitees")]
    _invitees: Option<serde::de::IgnoredAny>,
    /// The roster after the commit.
    roster: String,
    /// The roster before it, which its claims carry.
    claim_roster: String,
    issued_at: u64,
    /// Members the commit removes.
    #[serde(default)]
    removed: Vec<String>,
    /// A closed channel's key tree before the commit, and the leaves it
    /// removes: what the committer publishes new keys for once it wins.
    #[serde(default)]
    tree_before: Option<StoredTree>,
    #[serde(default)]
    leaves: Vec<u32>,
    /// Its reseed is onto subscribers' own keys: the owner's, after a team
    /// member left.
    #[serde(default)]
    hard: bool,
}

/// Invitations to send once a commit that adds members is applied.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Invitations {
    epoch: u64,
    welcome: String,
    invitees: Vec<StoredInvitee>,
    /// The tree in hex when it goes with the invitations, else empty.
    #[serde(default)]
    tree: String,
    /// The tree document in parts, when it does not: its whole and count.
    #[serde(default)]
    whole: String,
    #[serde(default)]
    count: u16,
}

impl Invitations {
    fn tree_ref(&self) -> Result<TreeRef, CoreError> {
        Ok(if self.whole.is_empty() {
            TreeRef::Inline(unhex(&self.tree)?)
        } else {
            TreeRef::Parts {
                whole: unhex32(&self.whole)?,
                count: self.count,
            }
        })
    }
    fn set_tree(&mut self, tree: &TreeRef) {
        match tree {
            TreeRef::Inline(tree) => {
                self.tree = hex::encode(tree);
                self.whole.clear();
                self.count = 0;
            }
            TreeRef::Parts { whole, count } => {
                self.tree.clear();
                self.whole = hex::encode(whole);
                self.count = *count;
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupState {
    version: u8,
    name: String,
    owner: String,
    /// The owner-signed roster in force, and what it says.
    roster: String,
    roster_version: u64,
    admins: Vec<String>,
    /// Root keys of members this profile learned, by network id.
    roots: BTreeMap<String, String>,
    active: bool,
    read_through: u64,
    /// Mailbox secrets of the latest epochs.
    mailboxes: BTreeMap<u64, String>,
    /// The commit named for recent epochs.
    decided: BTreeMap<u64, String>,
    /// Commits of the current epoch read or made, by hash.
    candidates: BTreeMap<String, Candidate>,
    /// Commit wires of the candidates, by hash.
    wires: BTreeMap<String, String>,
    pending: Option<Pending>,
    /// Kept apart now (`INVITATIONS`); older states still name it.
    #[serde(default, skip_serializing, rename = "invitations")]
    _invitations: Option<serde::de::IgnoredAny>,
    /// Operations already made: operation hash → result.
    operations: BTreeMap<String, GroupCommitMade>,
    /// The roster in force opens the group to anyone's reading.
    #[serde(default)]
    public: bool,
    /// The roster in force lets one join through the door once let in.
    #[serde(default)]
    by_request: bool,
    /// The epoch and day the group was closed again: its closing roster is
    /// published then.
    #[serde(default)]
    closed: Option<(u64, u64)>,
    /// Members removed within the kept epochs, by network id: the last
    /// epoch each was a member in. Nothing more of theirs is taken.
    #[serde(default)]
    removed: BTreeMap<String, u64>,
    /// The epoch this profile joined at: what was written before is passed
    /// over unread.
    #[serde(default)]
    joined: u64,
    /// When this profile joined or made the group.
    #[serde(default)]
    joined_at: u64,
    /// Its reading moved past days the mailboxes no longer keep: it asks
    /// for its place again, until it joins anew.
    #[serde(default)]
    behind: bool,
    /// What the group shows, kept with every change of its MLS state so a
    /// look at it opens nothing.
    #[serde(default)]
    view: Option<View>,
    /// This profile's certificate of membership in hex, as its invitation
    /// or the owner brought it: what its posts in the clear carry.
    #[serde(default)]
    membership: Option<String>,
    /// Members removed, by network id digest in hex: the last epoch each
    /// was a member in. The latest `MAX_DEPARTED`, for readers in the clear.
    #[serde(default)]
    departed: BTreeMap<String, u64>,
    /// The roster in force makes it a channel.
    #[serde(default)]
    channel: bool,
}

/// A group's epoch, members and bans as its MLS state says.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct View {
    epoch: u64,
    members: Vec<String>,
    /// Banned ids' digests in hex, and whether the owner banned them.
    bans: Vec<(String, bool)>,
    /// How long a channel keeps its history: days, 0 for ever.
    #[serde(default = "default_retention")]
    retention: u32,
    /// A closed channel's key tree, as the team keeps it.
    #[serde(default)]
    tree: Option<StoredTree>,
}

/// A key tree kept in a group's view.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredTree {
    generation: u32,
    seed: String,
    removed: Vec<u32>,
    branches: Vec<(String, u8)>,
}

impl StoredTree {
    fn of(tree: &KeyTree) -> Self {
        Self {
            generation: tree.generation,
            seed: hex::encode(tree.seed),
            removed: tree.removed.clone(),
            branches: tree
                .branches
                .iter()
                .map(|(member, branch)| (hex::encode(member), *branch))
                .collect(),
        }
    }

    pub(super) fn tree(&self) -> Result<KeyTree, CoreError> {
        Ok(KeyTree {
            generation: self.generation,
            seed: unhex32(&self.seed)?,
            removed: self.removed.clone(),
            branches: self
                .branches
                .iter()
                .map(|(member, branch)| Ok((unhex32(member)?, *branch)))
                .collect::<Result<_, CoreError>>()?,
        })
    }
}

fn default_retention() -> u32 {
    Retention::DEFAULT.code()
}

fn view_of(client: &MlsClient, group: [u8; 32]) -> Result<View, CoreError> {
    Ok(View {
        epoch: client.epoch(group)?,
        members: client
            .members(group)?
            .into_iter()
            .filter_map(|m| String::from_utf8(m.identity).ok())
            .collect(),
        bans: bans_of(client, group)?
            .entries
            .iter()
            .map(|ban| (hex::encode(ban.id), ban.by_owner))
            .collect(),
        retention: bans_of(client, group)?.retention.code(),
        tree: bans_of(client, group)?.tree.as_ref().map(StoredTree::of),
    })
}

/// A group as its door sees it.
pub(super) struct DoorView {
    /// `G`.
    pub group: [u8; 32],
    pub access: Access,
    /// It takes applications through a door.
    pub door: bool,
    pub channel: bool,
    /// This profile is the owner or an admin.
    pub admin: bool,
    /// The admins' root keys.
    pub admins: Vec<[u8; 32]>,
    pub name: String,
    pub owner: [u8; 32],
    /// The owner-signed roster in force.
    pub roster: Vec<u8>,
    /// Kept mailbox secrets, the current first.
    pub mailboxes: Vec<[u8; 32]>,
    pub members: Vec<String>,
    /// Banned network ids.
    pub banned: Vec<String>,
    /// A commit of this profile waits for the notary.
    pub busy: bool,
}

/// An invitation's group parts, as the invitee receives them.
#[derive(Clone)]
pub(super) struct GroupInvite {
    pub owner: [u8; 32],
    pub roster: Vec<u8>,
    /// The group mailbox's secret, and the epoch the Welcome joins.
    pub mailbox: [u8; 32],
    pub epoch: u64,
    pub tree: TreeRef,
    /// The invitee's certificate of membership; empty when none came.
    pub membership: Vec<u8>,
}

/// The mailbox secret in use at `epoch`: the one of the latest mailbox that
/// began at or before it.
/// A message of this profile waiting in a group's mailbox queue.
struct WaitingSend {
    message_id: String,
    group: [u8; 32],
    /// The epoch it was sealed in.
    epoch: u64,
    /// The members no longer read the mailbox of that epoch.
    mailbox_left: bool,
    plaintext: Vec<u8>,
    written_at: u64,
}

fn mailbox_at(state: &GroupState, epoch: u64) -> Option<&String> {
    state
        .mailboxes
        .range(..=epoch)
        .next_back()
        .map(|(_, secret)| secret)
}

fn operation_key(kind: &str, operation_id: &str) -> Result<String, CoreError> {
    if operation_id.is_empty()
        || operation_id.len() > 128
        || operation_id.chars().any(char::is_control)
    {
        return Err(CoreError::InvalidInput);
    }
    Ok(format!(
        "{kind}:{}",
        hex::encode(Sha256::digest(operation_id.as_bytes()))
    ))
}

fn unhex(text: &str) -> Result<Vec<u8>, CoreError> {
    hex::decode(text).map_err(|_| CoreError::InvalidState)
}

fn unhex32(text: &str) -> Result<[u8; 32], CoreError> {
    unhex(text)?.try_into().map_err(|_| CoreError::InvalidState)
}

fn commit_hash(wire: &[u8]) -> [u8; 32] {
    Sha256::digest(wire).into()
}

/// The digest a root key's network id spells.
fn root_digest(root: &[u8; 32]) -> [u8; 32] {
    Sha256::digest(root).into()
}

/// The group's bans, from the MLS group context.
fn bans_of(crypto: &MlsClient, group: [u8; 32]) -> Result<Bans, CoreError> {
    Ok(crypto
        .group_data(group)?
        .map(|data| Bans::decode(&data))
        .transpose()?
        .unwrap_or_default())
}

/// Remember who a commit leaving `epoch` removed; forget removals older
/// than the kept epochs, whose messages no longer open.
fn note_removed(state: &mut GroupState, removed: &[String], epoch: u64) {
    for member in removed {
        state.removed.insert(member.clone(), epoch);
    }
    state
        .removed
        .retain(|_, last| *last + KEPT_EPOCHS as u64 > epoch);
}

/// Removals readers in the clear are told of at most, the latest kept.
const MAX_DEPARTED: usize = 1024;

/// Remember for readers in the clear who left at `epoch` (the last epoch
/// they were members in): not those whose leaf a commit replaced.
fn note_departed(state: &mut GroupState, departed: &[String], epoch: u64) {
    for member in departed {
        if let Ok(digest) = identity_digest(member) {
            state.departed.insert(hex::encode(digest), epoch);
        }
    }
    while state.departed.len() > MAX_DEPARTED {
        let Some(oldest) = state
            .departed
            .iter()
            .min_by_key(|(_, epoch)| **epoch)
            .map(|(id, _)| id.clone())
        else {
            break;
        };
        state.departed.remove(&oldest);
    }
}

impl AppCore {
    fn group_state(&self, id: &str) -> Result<Option<(GroupState, u64)>, CoreError> {
        self.store
            .state(&format!("{GROUPS}{id}"))?
            .map(|state| {
                Ok((
                    serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                    state.revision,
                ))
            })
            .transpose()
    }

    fn group_change(id: &str, state: &GroupState, revision: u64) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{GROUPS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(state).map_err(invalid)?,
        })
    }

    pub(super) fn is_group(&self, id: &str) -> Result<bool, CoreError> {
        Ok(self.group_state(id)?.is_some())
    }

    fn invitations_of(&self, id: &str) -> Result<Option<(Invitations, u64)>, CoreError> {
        self.store
            .state(&format!("{INVITATIONS}{id}"))?
            .map(|state| {
                serde_json::from_slice(&state.bytes)
                    .map(|invitations| (invitations, state.revision))
                    .map_err(|_| CoreError::InvalidState)
            })
            .transpose()
    }

    fn invitations_change(
        id: &str,
        invitations: &Invitations,
        revision: u64,
    ) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{INVITATIONS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(invitations).map_err(invalid)?,
        })
    }

    fn drop_invitations(&mut self, id: &str) -> Result<(), CoreError> {
        if let Some((_, revision)) = self.invitations_of(id)? {
            self.store
                .commit_state_maintenance(vec![], vec![(format!("{INVITATIONS}{id}"), revision)])?;
        }
        Ok(())
    }

    fn require_group(&self, id: &str) -> Result<(GroupState, u64), CoreError> {
        parse_id(id)?;
        self.group_state(id)?.ok_or(CoreError::UnknownConversation)
    }

    /// The roster in force, as verified when it was taken.
    fn roster_of(&self, state: &GroupState, id: &str) -> Result<Roster, CoreError> {
        let owner = unhex32(&state.owner)?;
        Ok(Roster {
            group: group_ref(&self.domain, &owner, &parse_id(id)?),
            version: state.roster_version,
            admins: state
                .admins
                .iter()
                .map(|a| unhex32(a))
                .collect::<Result<_, _>>()?,
            access: if state.public {
                Access::Public
            } else if state.by_request {
                Access::Request
            } else {
                Access::Private
            },
            kind: if state.channel {
                GroupKind::Channel
            } else {
                GroupKind::Group
            },
        })
    }

    /// Take a verified roster and its wire.
    fn set_roster(state: &mut GroupState, wire: &[u8], roster: &Roster) {
        state.roster = hex::encode(wire);
        state.roster_version = roster.version;
        state.admins = roster.admins.iter().map(hex::encode).collect();
        state.public = roster.access == Access::Public;
        state.by_request = roster.access == Access::Request;
        state.channel = roster.kind == GroupKind::Channel;
    }

    /// A roster that closed an open group at `epoch`, on the day of `now`.
    fn note_closing(state: &mut GroupState, was_public: bool, epoch: u64, now: u64) {
        if was_public && !state.public {
            state.closed = Some((epoch, period(now)));
        }
    }

    /// Group ids this profile belongs to or belonged to.
    pub(super) fn group_ids(&self) -> Result<Vec<String>, CoreError> {
        let mut ids = vec![];
        let mut after = GROUPS.to_owned();
        let through = format!("{GROUPS}~");
        loop {
            let page = self.store.state_namespaces_between(&after, &through, 64)?;
            let Some(last) = page.last().cloned() else {
                return Ok(ids);
            };
            ids.extend(
                page.iter()
                    .filter_map(|namespace| namespace.strip_prefix(GROUPS).map(str::to_owned)),
            );
            after = last;
        }
    }

    fn info_of(&self, id: &str, state: &GroupState) -> Result<GroupInfo, CoreError> {
        let group = parse_id(id)?;
        let roster = self.roster_of(state, id)?;
        let own = self.store.identity()?.network_id;
        let owner = network_id(&unhex32(&state.owner)?);
        let admins: Vec<String> = roster.admins.iter().map(network_id).collect();
        // A group kept before its view was: from its MLS state.
        let view = match &state.view {
            Some(view) => view.clone(),
            None if state.active => view_of(&self.crypto(Some(group))?.0, group)?,
            None => View {
                epoch: state.mailboxes.keys().next_back().copied().unwrap_or(0),
                members: vec![],
                bans: vec![],
                retention: default_retention(),
                tree: None,
            },
        };
        let (members, banned) = if state.active {
            let banned = view
                .bans
                .iter()
                .map(|(ban, by_owner)| BannedId {
                    id: format!("ain1{ban}"),
                    by_owner: *by_owner,
                })
                .collect();
            (view.members.clone(), banned)
        } else {
            (vec![], vec![])
        };
        let role = if own == owner {
            "owner"
        } else if admins.contains(&own) {
            "admin"
        } else {
            "member"
        };
        Ok(GroupInfo {
            access: if state.public {
                "public"
            } else if state.by_request {
                "request"
            } else {
                "private"
            }
            .into(),
            group_ref: hex::encode(roster.group),
            id: id.into(),
            name: state.name.clone(),
            epoch: view.epoch,
            owner,
            admins,
            members,
            role: role.into(),
            banned,
            kind: if state.channel { "channel" } else { "group" }.into(),
            retention: history_of(state, &view).days(),
        })
    }

    /// What the door of a group this profile is in needs of it.
    pub(super) fn door_view(&self, id: &str) -> Result<Option<DoorView>, CoreError> {
        let Some((state, _)) = self.group_state(id)? else {
            return Ok(None);
        };
        if !state.active {
            return Ok(None);
        }
        let roster = self.roster_of(&state, id)?;
        let owner = unhex32(&state.owner)?;
        let own = self.store.identity()?.public_key;
        let info = self.info_of(id, &state)?;
        Ok(Some(DoorView {
            group: roster.group,
            access: roster.access,
            door: roster.has_door(),
            channel: roster.kind == GroupKind::Channel,
            admin: own == owner || roster.admins.contains(&own),
            admins: roster.admins.clone(),
            name: state.name.clone(),
            owner,
            roster: unhex(&state.roster)?,
            mailboxes: state
                .mailboxes
                .values()
                .rev()
                .map(|secret| unhex32(secret))
                .collect::<Result<_, _>>()?,
            members: info.members,
            banned: info.banned.into_iter().map(|b| b.id).collect(),
            busy: state.pending.is_some(),
        }))
    }

    pub fn group(&self, id: &str) -> Result<GroupInfo, CoreError> {
        let (state, _) = self.require_group(id)?;
        self.info_of(id, &state)
    }

    pub fn groups(&self) -> Result<Vec<GroupInfo>, CoreError> {
        let mut groups = vec![];
        for id in self.group_ids()? {
            if let Some((state, _)) = self.group_state(&id)?
                && state.active
            {
                groups.push(self.info_of(&id, &state)?);
            }
        }
        Ok(groups)
    }

    /// Group conversations, as the snapshot lists them.
    pub(super) fn group_conversations(&self) -> Result<Vec<Conversation>, CoreError> {
        let mut conversations = vec![];
        for id in self.group_ids()? {
            if let Some((state, _)) = self.group_state(&id)? {
                conversations.push(self.present_group(&id, &state)?);
            }
        }
        Ok(conversations)
    }

    fn present_group(&self, id: &str, state: &GroupState) -> Result<Conversation, CoreError> {
        let mut messages = vec![];
        let mut unread = 0;
        for stored in self.store.messages(id, 0, 1000)? {
            if let Some(message) = self.present_message(&stored)? {
                if !message.own && stored.sequence > state.read_through {
                    unread += 1;
                }
                messages.push(message);
            }
        }
        Ok(Conversation {
            id: id.into(),
            title: state.name.clone(),
            unread,
            messages,
        })
    }

    /// A group's title and the last message it has read, for the desktop's
    /// list; `None` for an id that is not a group.
    pub(super) fn group_listing(&self, id: &str) -> Result<Option<(String, u64)>, CoreError> {
        Ok(self
            .group_state(id)?
            .map(|(state, _)| (state.name, state.read_through)))
    }

    pub(super) fn group_conversation(&self, id: &str) -> Result<Conversation, CoreError> {
        let (state, _) = self.require_group(id)?;
        self.present_group(id, &state)
    }

    fn sign_roster(&self, roster: &Roster, now: u64) -> Result<Vec<u8>, CoreError> {
        Ok(self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::GroupRoster,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: roster.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire())
    }

    fn sign_claim(&self, claim: &CommitClaim, now: u64) -> Result<Vec<u8>, CoreError> {
        Ok(self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::GroupCommit,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: claim.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire())
    }

    /// Verified cards of the invitees: their KeyPackages, each once, none
    /// this profile's own.
    fn invitee_packages(
        &self,
        invitees: &[Invitee],
        own: &str,
        now: u64,
    ) -> Result<InviteeKeys, CoreError> {
        let mut seen = std::collections::BTreeSet::new();
        let mut packages = vec![];
        let mut roots = BTreeMap::new();
        for invitee in invitees {
            if invitee.network_id == own || !seen.insert(invitee.network_id.clone()) {
                return Err(CoreError::InvalidInput);
            }
            let (verified, packet) = self.verified_card(&invitee.network_id, &invitee.card, now)?;
            let Packet::IntroCard { package, .. } = packet else {
                return Err(CoreError::InvalidInput);
            };
            packages.push(package);
            roots.insert(invitee.network_id.clone(), hex::encode(verified.author()));
        }
        Ok((packages, roots))
    }

    /// Make a group of this profile (its owner) and the invitees; each is
    /// invited through its intro mailbox and directly. The same operation id
    /// is the same group.
    pub fn create_group(
        &mut self,
        name: &str,
        invitees: &[Invitee],
        operation_id: &str,
        now: u64,
    ) -> Result<GroupInfo, CoreError> {
        self.create(
            name,
            invitees,
            GroupKind::Group,
            Access::Private,
            operation_id,
            now,
        )
    }

    /// Make a channel of this profile (its owner) with `team` as its
    /// admins, the only others in it, open to anyone's reading or not as
    /// `access` says (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 8).
    pub fn create_channel(
        &mut self,
        name: &str,
        team: &[Invitee],
        access: Access,
        operation_id: &str,
        now: u64,
    ) -> Result<GroupInfo, CoreError> {
        if team.len() > MAX_ADMINS {
            return Err(CoreError::InvalidInput);
        }
        self.create(name, team, GroupKind::Channel, access, operation_id, now)
    }

    fn create(
        &mut self,
        name: &str,
        invitees: &[Invitee],
        kind: GroupKind,
        access: Access,
        operation_id: &str,
        now: u64,
    ) -> Result<GroupInfo, CoreError> {
        valid_name(name)?;
        let key = operation_key("group-create", operation_id)?;
        let (mut data, revision) = self.data()?;
        let identity = self.identity_for(&data)?;
        if let Some(id) = data.imported.get(&key).cloned() {
            self.send_invitations(&id, now)?;
            return self.group(&id);
        }
        if invitees.len() >= MAX_MEMBERS || invitees.len() > MAX_ADDS {
            return Err(CoreError::InvalidInput);
        }
        let (packages, mut roots) = self.invitee_packages(invitees, &identity.network_id, now)?;
        let root = self.store.identity()?.public_key;
        roots.insert(identity.network_id.clone(), hex::encode(root));
        let group = random_id()?;
        let id = hex::encode(group);
        // A channel's team are its admins from the start.
        let mut admins = vec![];
        if kind == GroupKind::Channel {
            for invitee in invitees {
                let admin = roots
                    .get(&invitee.network_id)
                    .ok_or(CoreError::InvalidInput)?;
                admins.push(unhex32(admin)?);
            }
            admins.sort();
        }
        let first = Roster {
            group: group_ref(&self.domain, &root, &group),
            version: 1,
            admins,
            access,
            kind,
        };
        let roster = self.sign_roster(&first, now)?;
        // A closed channel's key tree: a seed, and a branch for each of its
        // team.
        let group_data = if kind == GroupKind::Channel && access != Access::Public {
            let mut seed = [0; 32];
            getrandom::fill(&mut seed).map_err(|_| CoreError::InvalidState)?;
            let mut branches = vec![(root_digest(&root), 0)];
            for (n, invitee) in invitees.iter().enumerate() {
                branches.push((identity_digest(&invitee.network_id)?, n as u8 + 1));
            }
            Some(
                Bans {
                    tree: Some(KeyTree {
                        generation: 0,
                        seed,
                        removed: vec![],
                        branches,
                    }),
                    ..Bans::default()
                }
                .encode(),
            )
        } else {
            None
        };
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let created = MlsClient::from_state(crypto.create_group(group)?.next_state);
        // Its Welcomes go without the tree, however big it grows.
        let created = MlsClient::from_state(created.set_tree_apart(group)?.next_state);
        let (next, welcome) = if packages.is_empty() && group_data.is_none() {
            (created, None)
        } else {
            let made =
                created.commit_changes(group, &packages, &[], &roster, group_data.as_deref())?;
            let pending = MlsClient::from_state(made.next_state);
            // A new group has no earlier membership to order.
            let activated = pending.activate_pending_commit(group)?;
            (
                MlsClient::from_state(activated.next_state),
                made.value.welcome,
            )
        };
        let secret = next.group_mailbox_secret(group, self.domain)?;
        let mailbox = *secret.secret.expose();
        let mut invitations = welcome.map(|welcome| Invitations {
            epoch: secret.epoch,
            welcome: hex::encode(welcome),
            invitees: invitees
                .iter()
                .map(|i| StoredInvitee {
                    network_id: i.network_id.clone(),
                    card: hex::encode(&i.card),
                })
                .collect(),
            tree: String::new(),
            whole: String::new(),
            count: 0,
        });
        let tree = match &mut invitations {
            Some(invitations) => {
                self.invitation_tree(&id, invitations, &next, &roster, &identity.network_id, now)?
            }
            None => None,
        };
        let mut state = GroupState {
            version: 1,
            name: name.trim().into(),
            owner: hex::encode(root),
            roster: hex::encode(&roster),
            roster_version: 1,
            admins: vec![],
            roots,
            active: true,
            read_through: 0,
            mailboxes: BTreeMap::from([(secret.epoch, hex::encode(mailbox))]),
            decided: BTreeMap::new(),
            candidates: BTreeMap::new(),
            wires: BTreeMap::new(),
            pending: None,
            _invitations: None,
            operations: BTreeMap::new(),
            public: false,
            by_request: false,
            closed: None,
            removed: BTreeMap::new(),
            joined: secret.epoch,
            joined_at: now,
            behind: false,
            view: Some(view_of(&next, group)?),
            membership: None,
            departed: BTreeMap::new(),
            channel: false,
        };
        Self::set_roster(&mut state, &roster, &first);
        data.imported.insert(key, id.clone());
        let (mls, records) = crypto_change(Some(&crypto), next.snapshot(), crypto_revision);
        let mut states = vec![
            data_change(&data, revision)?,
            mls,
            Self::group_change(&id, &state, 0)?,
        ];
        if let Some(invitations) = &invitations {
            states.push(Self::invitations_change(&id, invitations, 0)?);
        }
        self.commit_with_tree(states, records, tree)?;
        self.send_invitations(&id, now)?;
        self.group(&id)
    }

    /// Decide how the tree of `client`'s current epoch reaches the
    /// invitees: with the invitations when both fit one, else as a document
    /// in parts into the tree's own mailbox, to be queued with the commit
    /// that applies the epoch — so before any invitation.
    fn invitation_tree(
        &self,
        id: &str,
        invitations: &mut Invitations,
        client: &MlsClient,
        roster: &[u8],
        own: &str,
        now: u64,
    ) -> Result<Option<agentic_store::OutgoingCommit>, CoreError> {
        let group = parse_id(id)?;
        let tree = client.ratchet_tree(group)?;
        // Each invitation carries one newcomer's Welcome; all are as big.
        let first = invitations
            .invitees
            .first()
            .ok_or(CoreError::InvalidState)?;
        let (_, card) = self.verified_card(&first.network_id, &unhex(&first.card)?, now)?;
        let Packet::IntroCard { package, .. } = card else {
            return Err(CoreError::InvalidState);
        };
        let welcome = agentic_crypto::welcome_for(&unhex(&invitations.welcome)?, &package)?.len();
        if welcome + tree.len() + roster.len() + INVITATION_ROOM <= agentic_protocol::MAX_BODY_BYTES
        {
            invitations.set_tree(&TreeRef::Inline(tree));
            return Ok(None);
        }
        let epoch = invitations.epoch;
        let wire = self.sign_whole(Packet::GroupTree { group, epoch, tree }, now)?;
        let (whole, count) = agentic_protocol::parts::wire_reference(&wire);
        invitations.set_tree(&TreeRef::Parts { whole, count });
        let message_id = hex::encode(whole);
        Ok(Some(agentic_store::OutgoingCommit {
            operation_id: format!("group-tree:{id}:{epoch}"),
            request_hash: whole,
            message: record(
                message_id.clone(),
                id,
                own,
                now,
                true,
                Event::Tree { epoch },
            )?,
            destination: id.into(),
            wire,
            states: vec![
                Self::send_epoch(&message_id, epoch)?,
                Self::whole_parts(&message_id, count)?,
            ],
        }))
    }

    /// Commit a group's `states` and MLS `records`, with the tree to queue
    /// when there is one.
    fn commit_with_tree(
        &mut self,
        states: Vec<StateChange>,
        records: StateRecordBatch,
        tree: Option<agentic_store::OutgoingCommit>,
    ) -> Result<(), CoreError> {
        match tree {
            None => self
                .store
                .commit_states_with_records(states, vec![records])?,
            Some(mut tree) => {
                tree.states.extend(states);
                self.store.commit_outgoing_with_retry_states_and_records(
                    tree,
                    vec![],
                    vec![records],
                )?;
            }
        }
        Ok(())
    }

    /// A certificate of membership in group `id` since `epoch` for the
    /// network id digest `member`, signed by this profile: the owner's
    /// stands alone, an admin's carries the owner's roster in force.
    fn member_cert(
        &self,
        id: &str,
        state: &GroupState,
        member: [u8; 32],
        epoch: u64,
        now: u64,
    ) -> Result<Vec<u8>, CoreError> {
        let owner = unhex32(&state.owner)?;
        let own = self.store.identity()?.public_key;
        let cert = MemberCert {
            group: group_ref(&self.domain, &owner, &parse_id(id)?),
            member,
            epoch,
            roster: if own == owner {
                vec![]
            } else {
                unhex(&state.roster)?
            },
        };
        Ok(self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::GroupMember,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: cert.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire())
    }

    /// Queue the invitations of the last applied commit that added members;
    /// each once, whatever the retries.
    fn send_invitations(&mut self, id: &str, now: u64) -> Result<(), CoreError> {
        let (state, _) = self.require_group(id)?;
        let Some((invitations, _)) = self.invitations_of(id)? else {
            return Ok(());
        };
        if invitations.invitees.is_empty() {
            return Ok(());
        }
        let (data, _) = self.data()?;
        let identity = self.identity_for(&data)?;
        let group = parse_id(id)?;
        let mailbox =
            unhex32(mailbox_at(&state, invitations.epoch).ok_or(CoreError::InvalidState)?)?;
        let tree = invitations.tree_ref()?;
        let batch_welcome = unhex(&invitations.welcome)?;
        for invitee in &invitations.invitees {
            let card = unhex(&invitee.card)?;
            let (verified, packet) = self.verified_card(&invitee.network_id, &card, now)?;
            let Packet::IntroCard {
                addresses,
                seal_key,
                package,
                ..
            } = packet
            else {
                return Err(CoreError::InvalidState);
            };
            // Only its own entry of the batch's Welcome.
            let welcome = agentic_crypto::welcome_for(&batch_welcome, &package)?;
            let member = Sha256::digest(verified.author()).into();
            let membership = self.member_cert(id, &state, member, invitations.epoch, now)?;
            let wire = self.sign(
                Packet::GroupWelcome {
                    group,
                    card: verified.id(),
                    name: state.name.clone(),
                    owner: unhex32(&state.owner)?,
                    roster: unhex(&state.roster)?,
                    welcome,
                    mailbox,
                    epoch: invitations.epoch,
                    tree: tree.clone(),
                    membership,
                },
                now,
                None,
            )?;
            let mut hash = Sha256::new();
            hash.update(b"group-invite");
            hash.update(group);
            hash.update(invitee.network_id.as_bytes());
            hash.update(invitations.epoch.to_be_bytes());
            let request_hash: [u8; 32] = hash.finalize().into();
            let operation = format!("group-invite:{}", hex::encode(request_hash));
            if self.store.operation_message(&operation)?.is_some() {
                continue;
            }
            let message_id = hex::encode(Sha256::digest(&wire));
            let sent = self.intro_sent_state(
                &message_id,
                verified.id(),
                seal_key,
                &invitee.network_id,
                addresses,
            )?;
            self.store.commit_outgoing_with_retry_states_and_records(
                agentic_store::OutgoingCommit {
                    operation_id: operation,
                    request_hash,
                    message: record(
                        message_id,
                        id,
                        &identity.network_id,
                        now,
                        true,
                        Event::Invite,
                    )?,
                    destination: invitee.network_id.clone(),
                    wire,
                    states: vec![sent],
                },
                vec![],
                vec![],
            )?;
        }
        // All queued: nothing more to keep.
        self.drop_invitations(id)
    }

    /// Change the group's members, admins or bans: one commit, held until
    /// the notary names an epoch's commit. The same operation id is the same
    /// commit.
    pub fn change_group(
        &mut self,
        id: &str,
        change: GroupChange,
        operation_id: &str,
        now: u64,
    ) -> Result<GroupCommitMade, CoreError> {
        let key = operation_key("group-change", operation_id)?;
        let (mut state, revision) = self.require_group(id)?;
        if let Some(made) = state.operations.get(&key) {
            return Ok(made.clone());
        }
        if !state.active {
            return Err(CoreError::UnknownConversation);
        }
        if state.pending.is_some() {
            return Err(CoreError::GroupBusy);
        }
        let group = parse_id(id)?;
        let own_root = self.store.identity()?.public_key;
        let own = network_id(&own_root);
        let owner = unhex32(&state.owner)?;
        let roster = self.roster_of(&state, id)?;
        let is_owner = own_root == owner;
        let admin_ids: Vec<String> = roster.admins.iter().map(network_id).collect();
        if !is_owner && !admin_ids.contains(&own) {
            return Err(CoreError::Unauthorized);
        }
        let owner_id = network_id(&owner);
        // An admin bans whom it may remove.
        if !is_owner
            && (change.admins.is_some()
                || change.access.is_some()
                || change
                    .remove
                    .iter()
                    .chain(&change.ban)
                    .any(|r| *r == owner_id || admin_ids.contains(r)))
        {
            return Err(CoreError::Unauthorized);
        }
        // A channel's team is its admins: the owner alone changes it, by
        // adding and removing, never by naming admins.
        if state.channel && change.admins.is_some() {
            return Err(CoreError::InvalidInput);
        }
        if state.channel
            && !is_owner
            && !(change.add.is_empty() && change.remove.is_empty() && change.replace.is_empty())
        {
            return Err(CoreError::Unauthorized);
        }
        // A closed channel keeps no history; a channel stays open or closed
        // as it was made.
        let closed_channel = state.channel && roster.access != Access::Public;
        if change.retention.is_some_and(|kept| {
            !state.channel || closed_channel || !Retention::CHOICES.contains(&kept)
        }) || (state.channel
            && change
                .access
                .is_some_and(|to| (to == Access::Public) != (roster.access == Access::Public)))
            || (!closed_channel && (!change.unsubscribe.is_empty() || change.reseed))
        {
            return Err(CoreError::InvalidInput);
        }
        // Opening or closing as it already is changes nothing.
        let access_after = change.access.filter(|access| *access != roster.access);
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let members: Vec<String> = crypto
            .members(group)?
            .into_iter()
            .filter_map(|m| String::from_utf8(m.identity).ok())
            .collect();
        let before = bans_of(&crypto, group)?;
        let mut after = before.clone();
        for lifted in &change.unban {
            let digest = identity_digest(lifted)?;
            let ban = before.get(&digest).ok_or(CoreError::InvalidInput)?;
            if ban.by_owner && !is_owner {
                return Err(CoreError::Unauthorized);
            }
            if after.get(&digest).is_none() {
                return Err(CoreError::InvalidInput);
            }
            after.entries.retain(|b| b.id != digest);
        }
        for banned in &change.ban {
            let digest = identity_digest(banned)?;
            if *banned == owner_id || after.get(&digest).is_some() || before.get(&digest).is_some()
            {
                return Err(CoreError::InvalidInput);
            }
            after.entries.push(Ban {
                id: digest,
                by_owner: is_owner,
            });
        }
        if after.entries.len() > MAX_BANS {
            return Err(CoreError::InvalidInput);
        }
        after.entries.sort_by_key(|b| b.id);
        if let Some(kept) = change.retention {
            after.retention = kept;
        }
        // A closed channel's subscribers removed or banned leave its key
        // tree; its new team members get branches; a full log, or a
        // decision, reseeds it.
        let mut leaves: Vec<u32> = vec![];
        if let Some(tree) = after.tree.as_mut() {
            let subscribers = self.channel_subscribers(id)?;
            for gone in change.unsubscribe.iter().chain(&change.ban) {
                match subscribers.get(gone) {
                    Some(given) => {
                        for leaf in given {
                            if !tree.removed.contains(leaf) && !leaves.contains(leaf) {
                                leaves.push(*leaf);
                            }
                        }
                    }
                    None if change.unsubscribe.contains(gone) => {
                        return Err(CoreError::InvalidInput);
                    }
                    None => {}
                }
            }
            tree.removed.extend(&leaves);
            let first = tree
                .branches
                .iter()
                .map(|(_, b)| usize::from(*b) + 1)
                .max()
                .unwrap_or(0);
            for (next, invitee) in (first..).zip(&change.add) {
                if next >= MAX_BRANCHES {
                    return Err(CoreError::InvalidInput);
                }
                tree.branches
                    .push((identity_digest(&invitee.network_id)?, next as u8));
            }
            if change.reseed || tree.removed.len() > MAX_TREE_LOG {
                getrandom::fill(&mut tree.seed).map_err(|_| CoreError::InvalidState)?;
                tree.generation = tree.generation.wrapping_add(1);
                tree.removed.clear();
            }
        }
        let empty = change.add.is_empty()
            && change.replace.is_empty()
            && change.remove.is_empty()
            && change.admins.is_none()
            && change.ban.is_empty()
            && change.unban.is_empty()
            && access_after.is_none()
            && after.retention == before.retention
            && after.tree == before.tree;
        let mut removed = std::collections::BTreeSet::new();
        for r in &change.remove {
            if *r == owner_id || !members.contains(r) || !removed.insert(r.clone()) {
                return Err(CoreError::InvalidInput);
            }
        }
        // A banned member leaves with the same commit.
        removed.extend(change.ban.iter().filter(|b| members.contains(b)).cloned());
        // Only plain members, still in and not otherwise changed, take
        // their place again.
        let mut replaced = std::collections::BTreeSet::new();
        for back in &change.replace {
            let id = &back.network_id;
            if !members.contains(id)
                || *id == owner_id
                || (admin_ids.contains(id) && !state.channel)
                || removed.contains(id)
                || !replaced.insert(id.clone())
            {
                return Err(CoreError::InvalidInput);
            }
        }
        if empty
            || change.add.iter().any(|i| members.contains(&i.network_id))
            || members.len() + change.add.len() - removed.len() > MAX_MEMBERS
            || change.add.len() + change.replace.len() > MAX_ADDS
        {
            return Err(CoreError::InvalidInput);
        }
        for invitee in &change.add {
            if after.get(&identity_digest(&invitee.network_id)?).is_some() {
                return Err(CoreError::Banned);
            }
        }
        let members_after: Vec<[u8; 32]> = members
            .iter()
            .filter(|m| !removed.contains(*m))
            .chain(change.add.iter().map(|i| &i.network_id))
            .map(|m| identity_digest(m))
            .collect::<Result<_, _>>()?;
        check_bans(
            &before,
            &after,
            root_digest(&owner),
            root_digest(&own_root),
            &roster.admins.iter().map(root_digest).collect::<Vec<_>>(),
            &members_after,
        )?;
        let data = (after != before).then(|| after.encode());
        let newcomers: Vec<Invitee> = change.add.iter().chain(&change.replace).cloned().collect();
        let (packages, roots) = self.invitee_packages(&newcomers, &own, now)?;
        // The roster after: a new version when the owner changes admins or
        // removes one.
        let admins_after: Option<Vec<[u8; 32]>> = match &change.admins {
            Some(list) => {
                let mut chosen = vec![];
                for admin in list {
                    // A member whose root key this profile knows.
                    let root = state
                        .roots
                        .get(admin)
                        .filter(|_| members.contains(admin))
                        .map(|r| unhex32(r))
                        .transpose()?
                        .ok_or(CoreError::InvalidInput)?;
                    if *admin == owner_id || removed.contains(admin) || chosen.contains(&root) {
                        return Err(CoreError::InvalidInput);
                    }
                    chosen.push(root);
                }
                if chosen.len() > MAX_ADMINS {
                    return Err(CoreError::InvalidInput);
                }
                Some(chosen)
            }
            // Those the owner adds to a channel join its team.
            None if state.channel && !change.add.is_empty() => {
                let mut team: Vec<[u8; 32]> = roster
                    .admins
                    .iter()
                    .filter(|a| !removed.contains(&network_id(a)))
                    .copied()
                    .collect();
                for invitee in &change.add {
                    let root = roots
                        .get(&invitee.network_id)
                        .ok_or(CoreError::InvalidInput)?;
                    team.push(unhex32(root)?);
                }
                if team.len() > MAX_ADMINS {
                    return Err(CoreError::InvalidInput);
                }
                Some(team)
            }
            None if roster
                .admins
                .iter()
                .any(|a| removed.contains(&network_id(a))) =>
            {
                Some(
                    roster
                        .admins
                        .iter()
                        .filter(|a| !removed.contains(&network_id(a)))
                        .copied()
                        .collect(),
                )
            }
            None => None,
        };
        let roster_after = if admins_after.is_some() || access_after.is_some() {
            let mut admins = admins_after.unwrap_or_else(|| roster.admins.clone());
            admins.sort();
            self.sign_roster(
                &Roster {
                    group: roster.group,
                    version: roster.version + 1,
                    admins,
                    access: access_after.unwrap_or(roster.access),
                    kind: roster.kind,
                },
                now,
            )?
        } else {
            unhex(&state.roster)?
        };
        let removes: Vec<Vec<u8>> = removed
            .iter()
            .chain(&replaced)
            .map(|r| r.as_bytes().to_vec())
            .collect();
        // Whoever commits sends newcomers the tree apart.
        let apart = MlsClient::from_state(crypto.set_tree_apart(group)?.next_state);
        let made =
            apart.commit_changes(group, &packages, &removes, &roster_after, data.as_deref())?;
        let epoch = crypto.epoch(group)?;
        let commit = commit_hash(&made.value.commit);
        let claim_roster = state.roster.clone();
        let claim = self.sign_claim(
            &CommitClaim {
                owner,
                group_id: group,
                epoch,
                round: 0,
                commit,
                roster: unhex(&claim_roster)?,
            },
            now,
        )?;
        state.roots.extend(roots);
        let wire = self.sign_whole(
            Packet::GroupCommit {
                group,
                commit: made.value.commit.clone(),
                claim: claim.clone(),
            },
            now,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let (_, count) = agentic_protocol::parts::wire_reference(&wire);
        let commit_hex = hex::encode(commit);
        state.candidates.insert(
            commit_hex.clone(),
            Candidate {
                epoch,
                round: 0,
                claim: hex::encode(&claim),
                commit: commit_hex.clone(),
            },
        );
        state
            .wires
            .insert(commit_hex.clone(), hex::encode(&made.value.commit));
        state.pending = Some(Pending {
            epoch,
            commit: commit_hex.clone(),
            operation: key.clone(),
            message_id: message_id.clone(),
            _invitees: None,
            roster: hex::encode(&roster_after),
            claim_roster,
            issued_at: now,
            removed: removed.iter().cloned().collect(),
            tree_before: before
                .tree
                .as_ref()
                .filter(|_| !leaves.is_empty() || change.reseed || after.tree != before.tree)
                .map(StoredTree::of),
            leaves: leaves.clone(),
            // The seed of a commit that takes a member out reaches that
            // member: only a later one is new to it.
            hard: change.reseed
                && removed.is_empty()
                && is_owner
                && before.tree.is_some()
                && !self.hard_reseed_due(id)?.is_empty(),
        });
        // Kept with the pending commit; sent only if it wins.
        let invitations = made.value.welcome.as_ref().map(|welcome| Invitations {
            epoch: epoch + 1,
            welcome: hex::encode(welcome),
            invitees: newcomers
                .iter()
                .map(|i| StoredInvitee {
                    network_id: i.network_id.clone(),
                    card: hex::encode(&i.card),
                })
                .collect(),
            tree: String::new(),
            whole: String::new(),
            count: 0,
        });
        let made_result = GroupCommitMade {
            epoch,
            commit: commit_hex,
            message_id: message_id.clone(),
        };
        state.operations.insert(key.clone(), made_result.clone());
        while state.operations.len() > MAX_OPERATIONS {
            state.operations.pop_first();
        }
        let (mls, records) = crypto_change(Some(&crypto), &made.next_state, crypto_revision);
        let mut states = vec![
            mls,
            Self::group_change(id, &state, revision)?,
            Self::send_epoch(&message_id, epoch)?,
        ];
        // Too big for one envelope: it goes in parts.
        if wire.len() > agentic_protocol::MAX_DOCUMENT_BYTES {
            states.push(Self::whole_parts(&message_id, count)?);
        }
        if let Some(invitations) = &invitations {
            let revision = self.invitations_of(id)?.map_or(0, |(_, revision)| revision);
            states.push(Self::invitations_change(id, invitations, revision)?);
        }
        self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                // Asked again after it lost, an operation makes a new commit.
                operation_id: format!("{key}:{epoch}"),
                request_hash: Sha256::digest(&wire).into(),
                message: record(message_id.clone(), id, &own, now, true, Event::Commit)?,
                destination: id.into(),
                wire,
                states,
            },
            vec![],
            vec![records],
        )?;
        Ok(made_result)
    }

    fn send_epoch(message_id: &str, epoch: u64) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{SENDS}{message_id}"),
            expected_revision: 0,
            bytes: serde_json::to_vec(&epoch).map_err(invalid)?,
        })
    }

    /// The root key of a member, when this profile knows it: the owner's and
    /// the admins'. Other members are named by network id only.
    fn member_root(
        &self,
        state: &GroupState,
        id: &str,
        member: &str,
    ) -> Result<Option<[u8; 32]>, CoreError> {
        let group = parse_id(id)?;
        let (crypto, _) = self.crypto(Some(group))?;
        if !crypto
            .members(group)?
            .iter()
            .any(|m| m.identity == member.as_bytes())
        {
            return Ok(None);
        }
        let owner = unhex32(&state.owner)?;
        if network_id(&owner) == member {
            return Ok(Some(owner));
        }
        let roster = self.roster_of(state, id)?;
        if let Some(root) = roster.admins.iter().find(|a| network_id(a) == member) {
            return Ok(Some(*root));
        }
        // A plain member: its id digest stands for it.
        Ok(Some(
            hex::decode(member.strip_prefix("ain1").unwrap_or_default())
                .ok()
                .and_then(|d| d.try_into().ok())
                .ok_or(CoreError::InvalidInput)?,
        ))
    }

    /// Undecided claims of the current epoch, the committer's signed
    /// statements, for the notary.
    pub fn group_claims(&self, id: &str) -> Result<Vec<GroupClaim>, CoreError> {
        let (state, _) = self.require_group(id)?;
        state
            .candidates
            .values()
            .filter(|c| !state.decided.contains_key(&c.epoch))
            .map(|c| {
                Ok(GroupClaim {
                    epoch: c.epoch,
                    round: c.round,
                    commit: c.commit.clone(),
                    claim: unhex(&c.claim)?,
                    message_id: state
                        .pending
                        .as_ref()
                        .filter(|p| p.commit == c.commit)
                        .map(|p| p.message_id.clone()),
                })
            })
            .collect()
    }

    /// The group an operation made, if it did.
    pub fn created_group(&self, operation_id: &str) -> Result<Option<GroupInfo>, CoreError> {
        let key = operation_key("group-create", operation_id)?;
        let (data, _) = self.data()?;
        data.imported.get(&key).map(|id| self.group(id)).transpose()
    }

    /// The commit an operation made in a group, if it made one that did not
    /// lose.
    pub fn group_operation(
        &self,
        id: &str,
        operation_id: &str,
    ) -> Result<Option<GroupCommitMade>, CoreError> {
        let key = operation_key("group-change", operation_id)?;
        let (state, _) = self.require_group(id)?;
        Ok(state.operations.get(&key).cloned())
    }

    /// Sign this profile's pending claim again for `round`: the rounds
    /// before had no winner.
    pub fn renew_group_claim(&mut self, id: &str, round: u32) -> Result<GroupClaim, CoreError> {
        let (mut state, revision) = self.require_group(id)?;
        let pending = state.pending.clone().ok_or(CoreError::InvalidInput)?;
        let candidate = state
            .candidates
            .get(&pending.commit)
            .cloned()
            .ok_or(CoreError::InvalidState)?;
        if round <= candidate.round {
            return Err(CoreError::InvalidInput);
        }
        let claim = self.sign_claim(
            &CommitClaim {
                owner: unhex32(&state.owner)?,
                group_id: parse_id(id)?,
                epoch: pending.epoch,
                round,
                commit: unhex32(&pending.commit)?,
                roster: unhex(&pending.claim_roster)?,
            },
            pending.issued_at,
        )?;
        state.candidates.insert(
            pending.commit.clone(),
            Candidate {
                round,
                claim: hex::encode(&claim),
                ..candidate
            },
        );
        // Stored beside the commit, so its readers can put it on record.
        let group = parse_id(id)?;
        let commit_wire = unhex(
            state
                .wires
                .get(&pending.commit)
                .ok_or(CoreError::InvalidState)?,
        )?;
        let wire = self.sign(
            Packet::GroupCommit {
                group,
                commit: commit_wire,
                claim: claim.clone(),
            },
            pending.issued_at,
            None,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let own = self.store.identity()?.network_id;
        self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: format!("group-renew:{}:{round}", pending.commit),
                request_hash: Sha256::digest(&wire).into(),
                message: record(
                    message_id.clone(),
                    id,
                    &own,
                    pending.issued_at,
                    true,
                    Event::Commit,
                )?,
                destination: id.into(),
                wire,
                states: vec![
                    Self::group_change(id, &state, revision)?,
                    Self::send_epoch(&message_id, pending.epoch)?,
                ],
            },
            vec![],
            vec![],
        )?;
        Ok(GroupClaim {
            epoch: pending.epoch,
            round,
            commit: pending.commit.clone(),
            claim,
            message_id: Some(pending.message_id),
        })
    }

    /// The notary named `commit` for `epoch`.
    pub fn decide_group_commit(
        &mut self,
        id: &str,
        epoch: u64,
        commit: &str,
        now: u64,
    ) -> Result<CommitDecision, CoreError> {
        parse_id(commit)?;
        let (mut state, revision) = self.require_group(id)?;
        if let Some(decided) = state.decided.get(&epoch) {
            return if decided == commit {
                Ok(if state.candidates.contains_key(commit) {
                    CommitDecision::Waiting
                } else {
                    CommitDecision::Applied
                })
            } else {
                Err(CoreError::InvalidInput)
            };
        }
        let group = parse_id(id)?;
        let (crypto, _) = self.crypto(Some(group))?;
        let current = if state.active {
            crypto.epoch(group)?
        } else {
            return Err(CoreError::UnknownConversation);
        };
        if epoch < current {
            return Err(CoreError::InvalidInput);
        }
        state.decided.insert(epoch, commit.into());
        while state.decided.len() > 8 {
            state.decided.pop_first();
        }
        if epoch > current {
            self.store
                .commit_states(vec![Self::group_change(id, &state, revision)?])?;
            return Ok(CommitDecision::Waiting);
        }
        let own = state.pending.as_ref().map(|p| p.commit.clone());
        if own.as_deref() == Some(commit) {
            self.win_group_commit(id, state, revision, now)?;
            return Ok(CommitDecision::Applied);
        }
        let lost = own.is_some();
        if lost {
            self.lose_group_commit(id, state, revision)?;
        } else {
            self.store
                .commit_states(vec![Self::group_change(id, &state, revision)?])?;
        }
        let applied = self.apply_decided(id, now)?;
        Ok(if lost {
            CommitDecision::Lost
        } else if applied {
            CommitDecision::Applied
        } else {
            CommitDecision::Waiting
        })
    }

    fn win_group_commit(
        &mut self,
        id: &str,
        mut state: GroupState,
        revision: u64,
        now: u64,
    ) -> Result<(), CoreError> {
        let group = parse_id(id)?;
        let pending = state.pending.take().ok_or(CoreError::InvalidState)?;
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let activated = MlsClient::from_state(crypto.activate_pending_commit(group)?.next_state);
        let secret = activated.group_mailbox_secret(group, self.domain)?;
        let epoch = secret.epoch;
        self.enter_epoch(&mut state, secret, !pending.removed.is_empty());
        let wire = unhex(&pending.roster)?;
        let owner = unhex32(&state.owner)?;
        let roster = verify_roster(
            &wire,
            self.domain,
            now,
            &owner,
            &group_ref(&self.domain, &owner, &group),
        )?;
        let was_public = state.public;
        let demoted: Vec<String> = state
            .admins
            .iter()
            .filter(|admin| !roster.admins.iter().any(|a| hex::encode(a) == **admin))
            .filter_map(|admin| unhex32(admin).ok())
            .map(|admin| network_id(&admin))
            .collect();
        Self::set_roster(&mut state, &wire, &roster);
        Self::note_closing(&mut state, was_public, epoch, now);
        note_removed(&mut state, &pending.removed, pending.epoch);
        note_departed(&mut state, &pending.removed, pending.epoch);
        state.candidates.retain(|_, c| c.epoch > pending.epoch);
        let kept: Vec<String> = state.candidates.keys().cloned().collect();
        state.wires.retain(|hash, _| kept.contains(hash));
        let own = network_id(&self.store.identity()?.public_key);
        let mut states = vec![];
        let tree = match self.invitations_of(id)? {
            Some((mut invitations, stored)) if invitations.epoch == epoch => {
                let roster = unhex(&state.roster)?;
                let tree =
                    self.invitation_tree(id, &mut invitations, &activated, &roster, &own, now)?;
                states.push(Self::invitations_change(id, &invitations, stored)?);
                tree
            }
            _ => None,
        };
        state.view = Some(view_of(&activated, group)?);
        let (mls, records) = crypto_change(Some(&crypto), activated.snapshot(), crypto_revision);
        states.extend([mls, Self::group_change(id, &state, revision)?]);
        // A closed channel's new keys: what to publish is kept with the
        // commit.
        if let Some(before) = &pending.tree_before {
            states.push(self.key_plan_change(id, before, &pending.leaves, pending.hard)?);
        }
        // A team member out of a closed channel knows its seed: new keys
        // onto subscribers' own are due at the owner, until made.
        if own == network_id(&owner) && state.channel {
            let keyed = state.view.as_ref().is_some_and(|view| view.tree.is_some());
            if pending.hard {
                states.push(self.hard_reseed_change(id, &[])?);
            } else if keyed && !pending.removed.is_empty() {
                states.push(self.hard_reseed_change(id, &pending.removed)?);
            }
        }
        // Those the demoted admins let in get the owner's certificates.
        let mut reissue = vec![];
        if own == network_id(&owner) {
            let (mut issued, issued_revision) = self.issued(id)?;
            let before = issued.len();
            issued.retain(|member, admin| {
                if pending.removed.contains(member) {
                    return false;
                }
                if demoted.contains(admin) {
                    reissue.push(member.clone());
                    return false;
                }
                true
            });
            if issued.len() != before {
                states.push(Self::issued_change(id, &issued, issued_revision)?);
            }
        }
        self.commit_with_tree(states, records, tree)?;
        if state.public {
            self.retry_public_posts(id, now)?;
        }
        self.send_invitations(id, now)?;
        self.reissue_certs(id, &reissue, now)?;
        // The commit is in; its keys go out now or with the next look.
        let _ = self.channel_key_changes(id, now);
        self.announce_subscribers(id, now)
    }

    /// A commit of this profile waits for the notary in group `id`.
    pub(super) fn group_busy(&self, id: &str) -> Result<bool, CoreError> {
        Ok(self
            .group_state(id)?
            .is_none_or(|(state, _)| !state.active || state.pending.is_some()))
    }

    /// The owner's record of whom its admins let in in group `id`.
    fn issued(&self, id: &str) -> Result<(BTreeMap<String, String>, u64), CoreError> {
        match self.store.state(&format!("{ISSUED}{id}"))? {
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
            None => Ok((BTreeMap::new(), 0)),
        }
    }

    fn issued_change(
        id: &str,
        issued: &BTreeMap<String, String>,
        revision: u64,
    ) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{ISSUED}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(issued).map_err(invalid)?,
        })
    }

    /// The owner gives `members` certificates of its own, in notices among
    /// the group's members.
    fn reissue_certs(&mut self, id: &str, members: &[String], now: u64) -> Result<(), CoreError> {
        if members.is_empty() {
            return Ok(());
        }
        let (state, _) = self.require_group(id)?;
        let epoch = state.view.as_ref().map_or(0, |view| view.epoch);
        for chunk in members.chunks(CERTS_PER_NOTICE) {
            let mut e = minicbor::Encoder::new(Vec::new());
            e.array(2)
                .and_then(|e| e.str("member-certs"))
                .and_then(|e| e.array(chunk.len() as u64))
                .map_err(invalid)?;
            for member in chunk {
                let cert = self.member_cert(id, &state, identity_digest(member)?, epoch, now)?;
                e.bytes(&cert).map_err(invalid)?;
            }
            self.send_group_notice(id, &e.into_writer(), now)?;
        }
        Ok(())
    }

    /// This profile's certificate among those a notice of the owner of
    /// group `id` carries.
    fn own_cert_in(
        &self,
        id: &str,
        state: &GroupState,
        author: &[u8; 32],
        body: &[u8],
        now: u64,
    ) -> Result<Option<Vec<u8>>, CoreError> {
        let owner = unhex32(&state.owner)?;
        if *author != owner {
            return Ok(None);
        }
        let mut d = minicbor::Decoder::new(body);
        let certs = (|| {
            if d.array().ok()? != Some(2) || d.str().ok()? != "member-certs" {
                return None;
            }
            let count = d.array().ok()??;
            (0..count)
                .map(|_| d.bytes().ok().map(<[u8]>::to_vec))
                .collect::<Option<Vec<_>>>()
        })();
        let own: [u8; 32] = Sha256::digest(self.store.identity()?.public_key).into();
        let group = group_ref(&self.domain, &owner, &parse_id(id)?);
        Ok(certs.into_iter().flatten().find(|wire| {
            agentic_protocol::group::verify_member_cert(wire, self.domain, now, &group)
                .is_ok_and(|cert| cert.member == own && cert.issuer == owner)
        }))
    }

    fn lose_group_commit(
        &mut self,
        id: &str,
        mut state: GroupState,
        revision: u64,
    ) -> Result<(), CoreError> {
        let group = parse_id(id)?;
        let pending = state.pending.take().ok_or(CoreError::InvalidState)?;
        state.candidates.remove(&pending.commit);
        state.wires.remove(&pending.commit);
        state.operations.remove(&pending.operation);
        self.drop_invitations(id)?;
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let cleared = crypto.clear_pending_commit(group)?;
        let (mls, records) = crypto_change(Some(&crypto), &cleared.next_state, crypto_revision);
        self.store.commit_states_with_records(
            vec![mls, Self::group_change(id, &state, revision)?],
            vec![records],
        )?;
        // The lost commit is not sent any further.
        self.store.acknowledge_outbox(&pending.message_id)?;
        Ok(())
    }

    /// Move to the epoch of `secret`: to its fresh mailbox when the commit
    /// removed someone, who must lose the address; else the mailbox stays.
    fn enter_epoch(
        &self,
        state: &mut GroupState,
        secret: agentic_crypto::mailbox::EpochMailbox,
        removed: bool,
    ) {
        if !removed && !state.mailboxes.is_empty() {
            return;
        }
        state
            .mailboxes
            .insert(secret.epoch, hex::encode(*secret.secret.expose()));
        while state.mailboxes.len() > KEPT_MAILBOXES {
            state.mailboxes.pop_first();
        }
    }

    /// Apply the commit named for the current epoch, if it has been read.
    fn apply_decided(&mut self, id: &str, now: u64) -> Result<bool, CoreError> {
        let (mut state, revision) = self.require_group(id)?;
        if !state.active {
            return Ok(false);
        }
        let group = parse_id(id)?;
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let epoch = crypto.epoch(group)?;
        let Some(commit) = state.decided.get(&epoch).cloned() else {
            return Ok(false);
        };
        let (Some(candidate), Some(wire)) = (
            state.candidates.get(&commit).cloned(),
            state.wires.get(&commit).cloned(),
        ) else {
            return Ok(false);
        };
        let wire = unhex(&wire)?;
        let claim = verify_claim(&unhex(&candidate.claim)?, self.domain, now)?;
        let view = crypto.view_commit(group, &wire)?;
        let owner = unhex32(&state.owner)?;
        if view.committer != network_id(&claim.committer).as_bytes() {
            return Err(CoreError::Unauthorized);
        }
        let current = self.roster_of(&state, id)?;
        let next = verify_roster(&view.aad, self.domain, now, &owner, &current.group)?;
        let mut removed = vec![];
        for identity in &view.removed {
            let member = String::from_utf8(identity.clone()).map_err(invalid)?;
            removed.push(
                self.member_root(&state, id, &member)?
                    .ok_or(CoreError::InvalidInput)?,
            );
        }
        check_transition(&current, &next, owner, claim.committer, &removed)?;
        let removed: Vec<String> = view
            .removed
            .iter()
            .map(|identity| String::from_utf8(identity.clone()).map_err(invalid))
            .collect::<Result<_, _>>()?;
        let mut members_after = vec![];
        for member in crypto.members(group)? {
            let member = String::from_utf8(member.identity).map_err(invalid)?;
            if !removed.contains(&member) {
                members_after.push(identity_digest(&member)?);
            }
        }
        for identity in &view.added {
            members_after.push(identity_digest(
                std::str::from_utf8(identity).map_err(invalid)?,
            )?);
        }
        let after = view
            .data
            .as_deref()
            .map(Bans::decode)
            .transpose()?
            .unwrap_or_default();
        check_bans(
            &bans_of(&crypto, group)?,
            &after,
            root_digest(&owner),
            root_digest(&claim.committer),
            &current.admins.iter().map(root_digest).collect::<Vec<_>>(),
            &members_after,
        )?;
        // Nobody is in a channel but its owner and admins.
        if next.kind == GroupKind::Channel {
            let team: Vec<[u8; 32]> = next
                .admins
                .iter()
                .chain([&owner])
                .map(root_digest)
                .collect();
            if members_after.iter().any(|m| !team.contains(m)) {
                return Err(CoreError::Unauthorized);
            }
        }
        let tree_before = bans_of(&crypto, group)?.tree;
        let applied = crypto.apply_commit(group, &wire)?;
        let next_client = MlsClient::from_state(applied.next_state);
        let gone_leaves: Vec<u32> = match (&tree_before, &after.tree) {
            (Some(before), Some(now_tree)) if before.generation == now_tree.generation => {
                now_tree.removed[before.removed.len().min(now_tree.removed.len())..].to_vec()
            }
            _ => vec![],
        };
        let was_public = state.public;
        Self::set_roster(&mut state, &view.aad, &next);
        Self::note_closing(&mut state, was_public, epoch + 1, now);
        note_removed(&mut state, &removed, epoch);
        // A member whose leaf was replaced is still in: no removal.
        let departed: Vec<String> = removed
            .iter()
            .filter(|member| !view.added.contains(&member.as_bytes().to_vec()))
            .cloned()
            .collect();
        note_departed(&mut state, &departed, epoch);
        state
            .roots
            .insert(network_id(&claim.committer), hex::encode(claim.committer));
        state.candidates.retain(|_, c| c.epoch > epoch);
        let kept: Vec<String> = state.candidates.keys().cloned().collect();
        state.wires.retain(|hash, _| kept.contains(hash));
        if applied.value.self_removed {
            state.active = false;
        } else {
            let secret = next_client.group_mailbox_secret(group, self.domain)?;
            self.enter_epoch(&mut state, secret, !departed.is_empty());
        }
        if state.active {
            state.view = Some(view_of(&next_client, group)?);
        } else if let Some(view) = state.view.as_mut() {
            view.epoch = epoch + 1;
        }
        let (mls, records) = crypto_change(Some(&crypto), next_client.snapshot(), crypto_revision);
        let mut states = vec![mls, Self::group_change(id, &state, revision)?];
        // The owner notes whom its admins let in.
        if self.store.identity()?.public_key == owner
            && (!view.added.is_empty() || !departed.is_empty())
        {
            let (mut issued, issued_revision) = self.issued(id)?;
            for member in &departed {
                issued.remove(member);
            }
            if claim.committer != owner {
                for added in &view.added {
                    let added = String::from_utf8(added.clone()).map_err(invalid)?;
                    issued.insert(added, network_id(&claim.committer));
                }
            }
            states.push(Self::issued_change(id, &issued, issued_revision)?);
        }
        self.store
            .commit_states_with_records(states, vec![records])?;
        if state.active && state.public {
            self.retry_public_posts(id, now)?;
        }
        self.forget_subscribers(id, &gone_leaves)?;
        if state.active {
            self.announce_subscribers(id, now)?;
        }
        // A decision may already wait for the next epoch.
        if state.active {
            self.apply_decided(id, now)?;
        }
        Ok(true)
    }

    /// The mailboxes of a group to read: the current epoch's, then the
    /// previous one's.
    pub fn group_mailboxes(&self, id: &str, at: u64) -> Result<Vec<[u8; 32]>, CoreError> {
        let (state, _) = self.require_group(id)?;
        if !state.active {
            return Err(CoreError::UnknownConversation);
        }
        state
            .mailboxes
            .values()
            .rev()
            .take(2)
            .map(|secret| {
                Ok(MailboxSecret::from_bytes(unhex32(secret)?)
                    .swarm_mailbox(&self.domain, period(at)))
            })
            .collect()
    }

    /// The current epoch's mailbox secret, or the one of `epoch`.
    pub(super) fn group_secret(
        &self,
        id: &str,
        epoch: Option<u64>,
    ) -> Result<MailboxSecret, CoreError> {
        let (state, _) = self.require_group(id)?;
        if !state.active && epoch.is_none() {
            return Err(CoreError::UnknownConversation);
        }
        let secret = match epoch {
            Some(epoch) => mailbox_at(&state, epoch),
            None => state.mailboxes.values().next_back(),
        }
        .ok_or(CoreError::InvalidInput)?;
        Ok(MailboxSecret::from_bytes(unhex32(secret)?))
    }

    /// The epoch a group message or commit was written in.
    pub(super) fn group_send_epoch(&self, message_id: &str) -> Result<Option<u64>, CoreError> {
        self.store
            .state(&format!("{SENDS}{message_id}"))?
            .map(|state| serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState))
            .transpose()
    }

    /// A group envelope read from one of the group's mailboxes.
    /// A group mailbox envelope opened with one of the kept epoch secrets.
    fn open_group_envelope(
        &self,
        id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<VerifiedDocument, CoreError> {
        let wire = self.open_group_wire(id, period, envelope)?;
        Ok(VerifiedDocument::decode(&wire, self.domain, now)?)
    }

    /// The signed wire in a group mailbox envelope, opened with one of the
    /// kept mailbox secrets.
    fn open_group_wire(
        &self,
        id: &str,
        period: u64,
        envelope: &[u8],
    ) -> Result<Vec<u8>, CoreError> {
        let (state, _) = self.require_group(id)?;
        for secret in state.mailboxes.values().rev() {
            if let Ok(wire) = MailboxSecret::from_bytes(unhex32(secret)?).open_swarm_envelope(
                &self.domain,
                period,
                envelope,
            ) {
                return Ok(wire);
            }
        }
        Err(CoreError::Unauthorized)
    }

    /// Where a group message falls in its sender's MLS order.
    pub(super) fn group_envelope_order(
        &self,
        id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<EnvelopeOrder, CoreError> {
        let verified = self.open_group_envelope(id, period, envelope, now)?;
        let group = parse_id(id)?;
        let Packet::Application { group: g, message } =
            Packet::decode(verified.body(), verified.kind())?
        else {
            return Err(CoreError::InvalidInput);
        };
        if g != group {
            return Err(CoreError::InvalidInput);
        }
        let (crypto, _) = self.crypto(Some(group))?;
        let aad = application_aad_for(self.domain, group)?;
        let metadata = crypto.inspect_application_message(group, &message, &aad)?;
        Ok(EnvelopeOrder {
            epoch: metadata.epoch,
            sender: metadata.sender,
            generation: metadata.generation,
        })
    }

    pub(super) fn receive_group_envelope(
        &mut self,
        id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<SwarmReceived, CoreError> {
        let wire = self.open_group_wire(id, period, envelope)?;
        let verified = VerifiedDocument::decode(&wire, self.domain, now)?;
        let message_id = hex::encode(verified.id());
        // A part of a document too big for one envelope: taken once whole.
        if verified.kind() == DocumentKind::Part {
            if let Some(whole) = self.group_part(id, &wire, now)? {
                self.receive_group_document(id, &whole, now)?;
            }
            return Ok(SwarmReceived { message_id });
        }
        self.receive_group_document(id, &verified, now)?;
        Ok(SwarmReceived { message_id })
    }

    fn receive_group_document(
        &mut self,
        id: &str,
        verified: &VerifiedDocument,
        now: u64,
    ) -> Result<(), CoreError> {
        root_epoch(verified)?;
        match Packet::decode(verified.body(), verified.kind())? {
            Packet::Application { group, message } if hex::encode(group) == id => {
                self.receive_group_message(id, verified, &message, now)?;
            }
            Packet::GroupCommit {
                group,
                commit,
                claim,
            } if hex::encode(group) == id => {
                self.receive_group_commit(id, verified, &commit, &claim, now)?;
            }
            _ => return Err(CoreError::InvalidInput),
        }
        Ok(())
    }

    fn receive_group_message(
        &mut self,
        id: &str,
        verified: &VerifiedDocument,
        message: &[u8],
        now: u64,
    ) -> Result<ReceiveOutcome, CoreError> {
        let message_id = hex::encode(verified.id());
        if self.store.message(&message_id)?.is_some() {
            return Ok(ReceiveOutcome { reply: None });
        }
        // Its own message sealed again: this profile never opens what it
        // sealed, and keeps it under its first id.
        if verified.author() == &self.store.identity()?.public_key {
            return Ok(ReceiveOutcome { reply: None });
        }
        // Written before this profile joined, in the mailbox it shares:
        // passed over unread.
        let (state, _) = self.require_group(id)?;
        if agentic_crypto::message_epoch(message).is_ok_and(|epoch| epoch < state.joined) {
            return Ok(ReceiveOutcome { reply: None });
        }
        let group = parse_id(id)?;
        let peer = network_id(verified.author());
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        // A sender's messages are taken in its order: one ahead of its
        // predecessor is a gap, held by the reader until it arrives.
        let received =
            crypto.decrypt_contiguous(group, message, &application_aad_for(self.domain, group)?)?;
        if received.value.sender != peer.as_bytes() {
            return Err(CoreError::Unauthorized);
        }
        // Nothing more of a removed member, whatever epoch it seals for.
        let (mut state, revision) = self.require_group(id)?;
        if state
            .removed
            .get(&peer)
            .is_some_and(|last| received.value.epoch <= *last)
        {
            return Err(CoreError::Unauthorized);
        }
        let (mls, records) = crypto_change(Some(&crypto), &received.next_state, crypto_revision);
        let mut states = vec![mls];
        let mut changed = false;
        // A notice among the members: taken, never talk.
        let event = if received.value.plaintext.first() == Some(&NOTICE) {
            let body = &received.value.plaintext[1..];
            match self.own_cert_in(id, &state, verified.author(), body, now)? {
                Some(cert) => {
                    state.membership = Some(hex::encode(cert));
                    changed = true;
                }
                None => match self.channel_notice(id, verified.author(), body)? {
                    Some(changes) => states.extend(changes),
                    None => states.extend(self.group_notice(id, verified.author(), body)?),
                },
            }
            Event::Notice
        } else {
            let text = String::from_utf8(received.value.plaintext).map_err(invalid)?;
            valid_text(&text)?;
            Event::Text { text }
        };
        if !state.roots.contains_key(&peer) {
            state
                .roots
                .insert(peer.clone(), hex::encode(verified.author()));
            changed = true;
        }
        if changed {
            states.push(Self::group_change(id, &state, revision)?);
        }
        self.store.commit_incoming_with_records(
            agentic_store::IncomingCommit {
                message: record(message_id, id, &peer, verified.issued_at(), false, event)?,
                states,
            },
            vec![records],
        )?;
        Ok(ReceiveOutcome { reply: None })
    }

    fn receive_group_commit(
        &mut self,
        id: &str,
        verified: &VerifiedDocument,
        commit: &[u8],
        claim_wire: &[u8],
        now: u64,
    ) -> Result<(), CoreError> {
        let (mut state, revision) = self.require_group(id)?;
        if !state.active {
            return Ok(());
        }
        let claim = verify_claim(claim_wire, self.domain, now)?;
        if &claim.committer != verified.author()
            || hex::encode(claim.claim.group_id) != id
            || hex::encode(claim.claim.owner) != state.owner
            || claim.claim.commit != commit_hash(commit)
        {
            return Err(CoreError::Unauthorized);
        }
        let (crypto, _) = self.crypto(Some(parse_id(id)?))?;
        let current = crypto.epoch(parse_id(id)?)?;
        if claim.claim.epoch < current {
            return Ok(());
        }
        let hash = hex::encode(claim.claim.commit);
        let known_round = state.candidates.get(&hash).map(|c| c.round);
        if known_round.is_none_or(|round| claim.claim.round > round) {
            state.candidates.insert(
                hash.clone(),
                Candidate {
                    epoch: claim.claim.epoch,
                    round: claim.claim.round,
                    claim: hex::encode(claim_wire),
                    commit: hash.clone(),
                },
            );
            state.wires.insert(hash, hex::encode(commit));
            self.store
                .commit_states(vec![Self::group_change(id, &state, revision)?])?;
        }
        self.apply_decided(id, now)?;
        Ok(())
    }

    /// Join a group from its Welcome: the roster must be its owner's, and the
    /// inviter and this profile its members.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn join_group(
        &mut self,
        inviter_root: [u8; 32],
        message_id: String,
        issued_at: u64,
        group: [u8; 32],
        name: String,
        invite: &GroupInvite,
        welcome: &[u8],
        tree: &[u8],
        now: u64,
        states: Vec<StateChange>,
    ) -> Result<Conversation, CoreError> {
        let id = hex::encode(group);
        let reference = group_ref(&self.domain, &invite.owner, &group);
        let roster = verify_roster(&invite.roster, self.domain, now, &invite.owner, &reference)?;
        let (crypto, crypto_revision) = self.crypto(Some(group))?;
        let joined_state = if crypto.knows_group(group)? {
            // Only a member back after a long absence, who asked for its
            // place, joins a group it still holds.
            if crypto.is_active(group)? && !self.applied_to(&id)? {
                return Err(CoreError::InvalidInput);
            }
            let forgotten = MlsClient::from_state(crypto.forget_group(group)?.next_state);
            forgotten.join_with_tree(group, welcome, tree)?.next_state
        } else {
            crypto.join_with_tree(group, welcome, tree)?.next_state
        };
        let joined = MlsClient::from_state(joined_state);
        let members: Vec<Vec<u8>> = joined
            .members(group)?
            .into_iter()
            .map(|m| m.identity)
            .collect();
        let own = self.store.identity()?.network_id;
        let inviter = network_id(&inviter_root);
        let owner = network_id(&invite.owner);
        for needed in [&own, &inviter, &owner] {
            if !members.contains(&needed.as_bytes().to_vec()) {
                return Err(CoreError::Unauthorized);
            }
        }
        // The group's mailbox, which the join did not change.
        let epoch = joined.epoch(group)?;
        let previous = self.group_state(&id)?;
        let roots = BTreeMap::from([
            (owner.clone(), hex::encode(invite.owner)),
            (inviter.clone(), hex::encode(inviter_root)),
        ]);
        let mut state = GroupState {
            version: 1,
            name,
            owner: hex::encode(invite.owner),
            roster: String::new(),
            roster_version: 0,
            admins: vec![],
            roots,
            active: true,
            read_through: previous.as_ref().map_or(0, |(s, _)| s.read_through),
            mailboxes: BTreeMap::from([(epoch, hex::encode(invite.mailbox))]),
            decided: BTreeMap::new(),
            candidates: BTreeMap::new(),
            wires: BTreeMap::new(),
            pending: None,
            _invitations: None,
            operations: BTreeMap::new(),
            public: false,
            by_request: false,
            closed: None,
            removed: BTreeMap::new(),
            joined: epoch,
            joined_at: now,
            behind: false,
            view: Some(view_of(&joined, group)?),
            membership: (!invite.membership.is_empty()).then(|| hex::encode(&invite.membership)),
            departed: previous
                .as_ref()
                .map(|(s, _)| s.departed.clone())
                .unwrap_or_default(),
            channel: false,
        };
        Self::set_roster(&mut state, &invite.roster, &roster);
        let (mls, records) = crypto_change(Some(&crypto), joined.snapshot(), crypto_revision);
        let mut all = vec![
            mls,
            Self::group_change(&id, &state, previous.map_or(0, |(_, r)| r))?,
        ];
        all.extend(states);
        self.store.commit_incoming_with_records(
            agentic_store::IncomingCommit {
                message: record(message_id, &id, &inviter, issued_at, false, Event::Welcome)?,
                states: all,
            },
            vec![records],
        )?;
        // A knock at its door is answered.
        self.forget_knock(&id)?;
        self.group_conversation(&id)
    }

    /// Groups this profile is a plain member of and fell behind in: the
    /// first period after the last one it read through, or joined in, is no
    /// longer kept. An owner or admin who fell behind is not replaced so.
    pub fn stale_groups(&self, now: u64) -> Result<Vec<StaleGroup>, CoreError> {
        let own = self.store.identity()?.public_key;
        let mut stale = vec![];
        for id in self.group_ids()? {
            let Some((state, _)) = self.group_state(&id)? else {
                continue;
            };
            if !state.active {
                continue;
            }
            let owner = unhex32(&state.owner)?;
            let roster = self.roster_of(&state, &id)?;
            if own == owner || roster.admins.contains(&own) {
                continue;
            }
            let joined = (state.joined_at > 0).then(|| period(state.joined_at));
            let Some(last) = self
                .swarm_read_through(&id)?
                .into_iter()
                .chain(joined)
                .max()
            else {
                continue;
            };
            if state.behind || !agentic_mailbox_swarm::address::live(last + 1, now) {
                let info = self.info_of(&id, &state)?;
                stale.push(StaleGroup {
                    group_id: id,
                    group_ref: info.group_ref,
                    owner: info.owner,
                    access: info.access,
                });
            }
        }
        Ok(stale)
    }

    /// A group's reading moves from `read` to `to`: past a day the mailboxes
    /// no longer keep (counting from its join), the group is behind.
    pub(super) fn note_group_reading(
        &mut self,
        id: &str,
        read: Option<u64>,
        to: u64,
        now: u64,
    ) -> Result<(), CoreError> {
        let Some((mut state, revision)) = self.group_state(id)? else {
            return Ok(());
        };
        let joined = (state.joined_at > 0).then(|| period(state.joined_at));
        let Some(base) = read.into_iter().chain(joined).max() else {
            return Ok(());
        };
        if state.behind || to <= base + 1 || agentic_mailbox_swarm::address::live(base + 1, now) {
            return Ok(());
        }
        state.behind = true;
        self.store
            .commit_states(vec![Self::group_change(id, &state, revision)?])?;
        Ok(())
    }

    /// Send a notice among a group's members: sealed like its talk, never
    /// shown as talk (a refusal at its door). Skipped while a commit of this
    /// profile waits.
    pub(super) fn send_group_notice(
        &mut self,
        id: &str,
        body: &[u8],
        now: u64,
    ) -> Result<(), CoreError> {
        let (state, _) = self.require_group(id)?;
        if !state.active || state.pending.is_some() {
            return Ok(());
        }
        let group = parse_id(id)?;
        let identity = self.store.identity()?.network_id;
        let (crypto, revision) = self.crypto(Some(group))?;
        let epoch = crypto.epoch(group)?;
        let plaintext = [&[NOTICE][..], body].concat();
        let prepared =
            crypto.encrypt(group, &plaintext, &application_aad_for(self.domain, group)?)?;
        let wire = self.sign(
            Packet::Application {
                group,
                message: prepared.value.wire,
            },
            now,
            None,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, revision);
        let kept = StateChange {
            namespace: format!("{NOTICE_BODIES}{message_id}"),
            expected_revision: 0,
            bytes: plaintext,
        };
        self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: format!("group-notice:{message_id}"),
                request_hash: Sha256::digest(&wire).into(),
                message: record(message_id.clone(), id, &identity, now, true, Event::Notice)?,
                destination: id.into(),
                wire,
                states: vec![mls, Self::send_epoch(&message_id, epoch)?, kept],
            },
            vec![],
            vec![records],
        )?;
        Ok(())
    }

    /// Forget what an acknowledged notice said.
    pub(super) fn forget_notice_body(&mut self, message_id: &str) -> Result<(), CoreError> {
        let namespace = format!("{NOTICE_BODIES}{message_id}");
        if let Some(kept) = self.store.state(&namespace)? {
            self.store
                .commit_state_maintenance(vec![], vec![(namespace, kept.revision)])?;
        }
        Ok(())
    }

    /// Talk and notices of this profile's groups still waiting in its
    /// outbox, sealed again in the group's current epoch under the same id
    /// once the members could no longer open them: three epochs or more
    /// after they were sealed, or once the members no longer read the
    /// mailbox of their epoch (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
    /// part 1). Commits, posts in the clear, what a quorum of holders stored
    /// and groups with a commit of this profile waiting are left alone, and
    /// a message that cannot be sealed again is left for the next time.
    /// The ids sealed again, in the outbox's order.
    pub fn reseal_stale_group_sends(&mut self) -> Result<Vec<String>, CoreError> {
        let mut waiting = vec![];
        for (message_id, conversation_id) in self.swarm_queue()? {
            if let Ok(Some(send)) = self.waiting_group_send(&message_id, &conversation_id) {
                waiting.push(send);
            }
        }
        if waiting.is_empty() {
            return Ok(vec![]);
        }
        // Each group's epoch, its records loaded alone.
        let mut epochs = BTreeMap::new();
        for send in &waiting {
            if epochs.contains_key(&send.group) {
                continue;
            }
            if let Ok(epoch) = self
                .crypto(Some(send.group))
                .and_then(|(crypto, _)| Ok(crypto.epoch(send.group)?))
            {
                epochs.insert(send.group, epoch);
            }
        }
        let mut resealed = vec![];
        for send in waiting {
            let Some(&current) = epochs.get(&send.group) else {
                continue;
            };
            if current.saturating_sub(send.epoch) < RESEAL_EPOCHS && !send.mailbox_left {
                continue;
            }
            if self.reseal_group_send(&send).is_ok() {
                resealed.push(send.message_id);
            }
        }
        Ok(resealed)
    }

    /// A message of this profile waiting to be stored in a group's mailbox
    /// that could be sealed again: talk or a notice, sealed with MLS.
    fn waiting_group_send(
        &self,
        message_id: &str,
        conversation_id: &str,
    ) -> Result<Option<WaitingSend>, CoreError> {
        let Some((state, _)) = self.group_state(conversation_id)? else {
            return Ok(None);
        };
        if !state.active
            || state.pending.is_some()
            || self.is_public_send(message_id)?
            || self.parts_of(message_id)?.is_some()
            || self
                .swarm_pin(message_id)?
                .is_some_and(|(_, _, stored)| stored)
        {
            return Ok(None);
        }
        let Some(epoch) = self.group_send_epoch(message_id)? else {
            return Ok(None);
        };
        let Some(item) = self.store.pending_outbox_item(message_id)? else {
            return Ok(None);
        };
        let record = &item.message.record;
        let plaintext = match serde_json::from_slice(&record.content).map_err(invalid)? {
            Event::Text { text } => text.into_bytes(),
            Event::Notice => match self.store.state(&format!("{NOTICE_BODIES}{message_id}"))? {
                Some(kept) => kept.bytes.clone(),
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        // Members read the mailboxes of the newest two: one older is left.
        let began = state.mailboxes.range(..=epoch).next_back().map(|(k, _)| *k);
        let mailbox_left = began.is_none_or(|began| {
            !state
                .mailboxes
                .keys()
                .rev()
                .take(2)
                .any(|read| *read == began)
        });
        Ok(Some(WaitingSend {
            message_id: message_id.into(),
            group: parse_id(conversation_id)?,
            epoch,
            mailbox_left,
            plaintext,
            written_at: record.created_at,
        }))
    }

    /// Seal `send` again in its group's current epoch, signed as when it was
    /// written, and let it take a new pin in the swarm.
    fn reseal_group_send(&mut self, send: &WaitingSend) -> Result<(), CoreError> {
        let (crypto, revision) = self.crypto(Some(send.group))?;
        let epoch = crypto.epoch(send.group)?;
        let prepared = crypto.encrypt(
            send.group,
            &send.plaintext,
            &application_aad_for(self.domain, send.group)?,
        )?;
        let wire = self.sign(
            Packet::Application {
                group: send.group,
                message: prepared.value.wire,
            },
            send.written_at,
            None,
        )?;
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, revision);
        let sent_in = format!("{SENDS}{}", send.message_id);
        let written = self.store.state(&sent_in)?.ok_or(CoreError::InvalidState)?;
        let states = vec![
            mls,
            StateChange {
                namespace: sent_in,
                expected_revision: written.revision,
                bytes: serde_json::to_vec(&epoch).map_err(invalid)?,
            },
        ];
        let removals = self
            .swarm_pin(&send.message_id)?
            .map(|(namespace, revision, _)| (namespace, revision))
            .into_iter()
            .collect();
        self.store
            .replace_outbox_wire(&send.message_id, wire, states, vec![records], removals)?;
        Ok(())
    }

    /// Whether `peer` invited this profile before into an active group.
    pub(super) fn member_of(&self, id: &str) -> Result<bool, CoreError> {
        Ok(self.group_state(id)?.is_some_and(|(state, _)| state.active))
    }

    /// Send text into a group: through its mailbox only.
    pub(super) fn send_group_text(
        &mut self,
        id: &str,
        text: &str,
        operation_id: &str,
        request_hash: [u8; 32],
        now: u64,
        authorization: broker::AuthorizationChanges,
    ) -> Result<StoredMessage, CoreError> {
        let (state, _) = self.require_group(id)?;
        if !state.active {
            return Err(CoreError::UnknownConversation);
        }
        if state.pending.is_some() {
            return Err(CoreError::GroupBusy);
        }
        let group = parse_id(id)?;
        let identity = self.store.identity()?.network_id;
        let (crypto, revision) = self.crypto(Some(group))?;
        let epoch = crypto.epoch(group)?;
        // A channel's posts go to its readers: in the clear, or sealed
        // under a closed channel's key.
        if state.public || state.channel {
            return self.send_public_post(
                id,
                &state,
                epoch,
                text,
                operation_id,
                request_hash,
                now,
                authorization,
            );
        }
        let prepared = crypto
            .encrypt(
                group,
                text.as_bytes(),
                &application_aad_for(self.domain, group)?,
            )
            .map_err(|error| match error {
                agentic_crypto::CryptoError::PendingCommit => CoreError::GroupBusy,
                error => error.into(),
            })?;
        let wire = self.sign(
            Packet::Application {
                group,
                message: prepared.value.wire,
            },
            now,
            None,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let message = record(
            message_id.clone(),
            id,
            &identity,
            now,
            true,
            Event::Text { text: text.into() },
        )?;
        let (mls, records) = crypto_change(Some(&crypto), &prepared.next_state, revision);
        let mut states = vec![mls, Self::send_epoch(&message_id, epoch)?];
        states.extend(authorization.new_operation);
        Ok(self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: operation_id.into(),
                request_hash,
                message,
                destination: id.into(),
                wire,
                states,
            },
            authorization.retry,
            vec![records],
        )?)
    }

    /// A post of an open group: a document signed by this profile's root
    /// key, sent in the clear to the group's public mailbox.
    #[allow(clippy::too_many_arguments)]
    fn send_public_post(
        &mut self,
        id: &str,
        state: &GroupState,
        epoch: u64,
        text: &str,
        operation_id: &str,
        request_hash: [u8; 32],
        now: u64,
        authorization: broker::AuthorizationChanges,
    ) -> Result<StoredMessage, CoreError> {
        let identity = self.store.identity()?;
        let owner = unhex32(&state.owner)?;
        // The owner and admins vouch for themselves; a member carries the
        // certificate it was let in with.
        let own = identity.public_key;
        let membership = if own == owner || state.admins.contains(&hex::encode(own)) {
            self.member_cert(id, state, Sha256::digest(own).into(), epoch, now)?
        } else {
            state
                .membership
                .as_deref()
                .map(unhex)
                .transpose()?
                .unwrap_or_default()
        };
        let identity = identity.network_id;
        let post = PublicPost {
            group: group_ref(&self.domain, &owner, &parse_id(id)?),
            epoch,
            operation: Sha256::digest(operation_id.as_bytes()).into(),
            text: text.into(),
            membership,
        };
        let wire = self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::PublicPost,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: post.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire();
        let message_id = hex::encode(Sha256::digest(&wire));
        let message = record(
            message_id.clone(),
            id,
            &identity,
            now,
            true,
            Event::Text { text: text.into() },
        )?;
        let mut states = vec![
            Self::send_epoch(&message_id, epoch)?,
            StateChange {
                namespace: format!("{PUBLIC_SENDS}{message_id}"),
                expected_revision: 0,
                bytes: b"true".to_vec(),
            },
        ];
        // An open channel's team packs what it writes.
        if state.channel && state.public {
            states.extend(self.channel_post_change(id, &wire, now, now)?);
        }
        states.extend(authorization.new_operation);
        Ok(self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: operation_id.into(),
                request_hash,
                message,
                destination: id.into(),
                wire,
                states,
            },
            authorization.retry,
            vec![],
        )?)
    }

    /// Whether a message went out as a public post.
    pub(super) fn is_public_send(&self, message_id: &str) -> Result<bool, CoreError> {
        Ok(self
            .store
            .state(&format!("{PUBLIC_SENDS}{message_id}"))?
            .is_some())
    }

    /// A member's view of an open group for reading its public mailbox.
    pub(super) fn public_view(&self, id: &str) -> Result<PublicView, CoreError> {
        let (state, _) = self.require_group(id)?;
        let group = parse_id(id)?;
        let owner = unhex32(&state.owner)?;
        let (epoch, members, bans) = match (&state.view, state.active) {
            (Some(view), true) => (
                view.epoch,
                view.members.clone(),
                view.bans.iter().map(|(ban, _)| ban.clone()).collect(),
            ),
            (None, true) => {
                let (crypto, _) = self.crypto(Some(group))?;
                let view = view_of(&crypto, group)?;
                (
                    view.epoch,
                    view.members,
                    view.bans.into_iter().map(|(ban, _)| ban).collect(),
                )
            }
            _ => (0, vec![], vec![]),
        };
        let reference = group_ref(&self.domain, &owner, &group);
        let tree = match (&state.view, state.active && state.channel) {
            (Some(view), true) => view.tree.as_ref().map(StoredTree::tree).transpose()?,
            _ => None,
        };
        Ok(PublicView {
            group: reference,
            owner,
            public: state.active && (state.public || tree.is_some()),
            epoch,
            members,
            roster: self.roster_of(&state, id)?,
            removed: removed_of(&state, &bans),
            joined_at: state.joined_at,
            channel: state.channel,
            team: {
                let own = self.store.identity()?.public_key;
                own == owner || state.admins.contains(&hex::encode(own))
            },
            retention: state.view.as_ref().map_or(Retention::DEFAULT, |view| {
                Retention::from_code(view.retention)
            }),
            tree,
            name: state.name.clone(),
            roster_wire: unhex(&state.roster)?,
        })
    }

    /// What the roster an owner or admin publishes for its open group says
    /// now: the roster in force and who may no longer write; in the epoch it
    /// closed, the closing roster. `None` for a plain member, or a group not
    /// open nor just closed.
    fn public_roster_body(&self, id: &str) -> Result<Option<PublicRoster>, CoreError> {
        let (state, _) = self.require_group(id)?;
        if !state.active {
            return Ok(None);
        }
        let info = self.info_of(id, &state)?;
        if info.role == "member" {
            return Ok(None);
        }
        let closing = !state.public && state.closed.is_some_and(|(epoch, _)| epoch == info.epoch);
        let sealed = state.channel && state.view.as_ref().is_some_and(|v| v.tree.is_some());
        if !state.public && !closing && !sealed {
            return Ok(None);
        }
        let view = self.public_view(id)?;
        let mut removed: Vec<([u8; 32], u64)> = view
            .removed
            .iter()
            .filter_map(|(member, epoch)| Some((parse_id(member).ok()?, *epoch)))
            .collect();
        removed.sort();
        let history = match &state.view {
            Some(group_view) => history_of(&state, group_view),
            None => Retention::Days(1),
        };
        Ok(Some(PublicRoster {
            group: view.group,
            epoch: view.epoch,
            roster: unhex(&state.roster)?,
            removed,
            retention: history,
        }))
    }

    /// What the published roster of open group `id` would say, whatever
    /// its epoch: the node publishes again when it changes (a removal, a
    /// ban, a new roster, a closing), else once a day. `None` when this
    /// profile publishes none.
    pub fn public_roster_key(&self, id: &str) -> Result<Option<String>, CoreError> {
        Ok(self.public_roster_body(id)?.map(|mut body| {
            body.epoch = 0;
            hex::encode(Sha256::digest(body.encode()))
        }))
    }

    /// The roster an owner or admin publishes for its open group now; in
    /// the epoch it closed, on that day and the next, the closing roster.
    pub(super) fn public_roster_document(
        &self,
        id: &str,
        now: u64,
    ) -> Result<Option<PublicRosterWire>, CoreError> {
        let Some(body) = self.public_roster_body(id)? else {
            return Ok(None);
        };
        let (state, _) = self.require_group(id)?;
        if !state.public
            && !state.channel
            && state.closed.is_some_and(|(_, day)| period(now) > day + 1)
        {
            return Ok(None);
        }
        let wire = self
            .store
            .sign_document(DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::PublicRoster,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body: body.encode(),
                extensions: BTreeMap::new(),
            })?
            .to_wire();
        Ok(Some((body.group, wire)))
    }
}

/// How far back its readers read: a channel's retention, a day for a
/// group.
fn history_of(state: &GroupState, view: &View) -> Retention {
    if state.channel {
        Retention::from_code(view.retention)
    } else {
        Retention::Days(1)
    }
}

/// Who may no longer write in the clear: the latest removals, and every
/// ban (`bans`, digests in hex) for good.
fn removed_of(state: &GroupState, bans: &[String]) -> BTreeMap<String, u64> {
    let mut removed = state.departed.clone();
    for ban in bans {
        removed.insert(ban.clone(), BANNED);
    }
    removed
}

/// What a member reads an open group's public mailbox with.
pub(super) struct PublicView {
    pub group: [u8; 32],
    pub owner: [u8; 32],
    pub public: bool,
    pub epoch: u64,
    pub members: Vec<String>,
    /// The owner-signed roster in force.
    pub roster: Roster,
    /// Network id digests in hex that may no longer write under a
    /// certificate of that epoch or before (`BANNED`: under none).
    pub removed: BTreeMap<String, u64>,
    /// When this profile joined: it reads the day before.
    pub joined_at: u64,
    pub channel: bool,
    /// This profile is the owner or an admin.
    pub team: bool,
    /// A channel's retention.
    pub retention: Retention,
    /// A closed channel's key tree, which its team keeps.
    pub tree: Option<KeyTree>,
    pub name: String,
    /// The owner-signed roster in force, as signed.
    pub roster_wire: Vec<u8>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const NOW: u64 = 1_788_570_000;

    /// A notice as big as a group document holds (a notice of member
    /// certificates comes near) is kept apart from its record, sealed again
    /// once stale and forgotten once stored.
    #[test]
    fn a_notice_as_big_as_its_document_holds_is_sealed_again_from_what_was_kept() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ProfileStore::open(dir.path().join("profile.db"), &[7; 32]).unwrap();
        let mut core = AppCore::new(store, [9; 32]).unwrap();
        core.create_profile("Alice").unwrap();
        let g = core.create_group("Solo", &[], "g", NOW).unwrap().id;
        let before = core.swarm_queue().unwrap();
        core.send_group_notice(&g, &[0x5a; 40_000], NOW).unwrap();
        let [(notice, _)] = core
            .swarm_queue()
            .unwrap()
            .into_iter()
            .filter(|item| !before.contains(item))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let kept = format!("{NOTICE_BODIES}{notice}");
        assert_eq!(
            core.store.state(&kept).unwrap().unwrap().bytes.len(),
            40_001
        );
        let first = core
            .store
            .pending_outbox_item(&notice)
            .unwrap()
            .unwrap()
            .wire;
        let outsider = format!("ain1{}", "ab".repeat(32));
        for turn in 0..3 {
            let e = core.group(&g).unwrap().epoch;
            let change = if turn % 2 == 0 {
                GroupChange {
                    ban: vec![outsider.clone()],
                    ..GroupChange::default()
                }
            } else {
                GroupChange {
                    unban: vec![outsider.clone()],
                    ..GroupChange::default()
                }
            };
            let commit = core
                .change_group(&g, change, &format!("c-{turn}"), NOW)
                .unwrap()
                .commit;
            core.decide_group_commit(&g, e, &commit, NOW).unwrap();
        }
        assert_eq!(
            core.reseal_stale_group_sends().unwrap(),
            vec![notice.clone()]
        );
        let again = core
            .store
            .pending_outbox_item(&notice)
            .unwrap()
            .unwrap()
            .wire;
        assert_ne!(again, first);
        assert!(core.store.state(&kept).unwrap().is_some());
        core.forget_notice_body(&notice).unwrap();
        assert!(core.store.state(&kept).unwrap().is_none());
    }
}
