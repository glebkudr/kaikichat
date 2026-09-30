//! Group rosters and commit claims (spec/groups-v1.md): what a notary checks
//! before it records a claim on a group epoch, and what a reader checks of a
//! commit's change to the roster.
use crate::{DocumentKind, VerifiedDocument, WireError};
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};

/// Admins a roster names at most.
pub const MAX_ADMINS: usize = 49;

fn digest(tag: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(tag.as_bytes());
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    hash.finalize().into()
}

/// `G`: the group's reference, binding its owner.
pub fn group_ref(domain: &[u8; 32], owner: &[u8; 32], group_id: &[u8; 32]) -> [u8; 32] {
    digest("AIN_GROUP_V1", &[domain, owner, group_id])
}

/// Where the claims on a group epoch's commit meet, per round.
pub fn claim_key(group: &[u8; 32], epoch: u64, round: u32) -> [u8; 32] {
    digest(
        "AIN_GROUP_COMMIT_V1",
        &[group, &epoch.to_be_bytes(), &round.to_be_bytes()],
    )
}

fn id(d: &mut Decoder<'_>) -> Result<[u8; 32], WireError> {
    d.bytes()
        .map_err(|_| WireError::Malformed)?
        .try_into()
        .map_err(|_| WireError::Malformed)
}

/// What a group is: talk among its members, or a channel its owner and
/// admins write for its followers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    Group,
    /// Its members are its owner and admins, the only ones who write.
    Channel,
}

/// The admins of a group and who reads it, as its owner signed them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roster {
    pub group: [u8; 32],
    pub version: u64,
    /// Root keys, in ascending order, each once.
    pub admins: Vec<[u8; 32]>,
    pub access: Access,
    /// Fixed when the group is made.
    pub kind: GroupKind,
}

impl Roster {
    /// `roster-v2` for a group, `roster-v3` with the kind for a channel.
    pub fn encode(&self) -> Vec<u8> {
        let mut admins = self.admins.clone();
        admins.sort();
        admins.dedup();
        let channel = self.kind == GroupKind::Channel;
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(if channel { 6 } else { 5 });
        let _ = e.str(if channel { "roster-v3" } else { "roster-v2" });
        let _ = e.bytes(&self.group);
        let _ = e.u64(self.version);
        let _ = e.array(admins.len() as u64);
        for admin in &admins {
            let _ = e.bytes(admin);
        }
        let _ = e.u8(self.access.code());
        if channel {
            let _ = e.u8(1);
        }
        e.into_writer()
    }

    /// Whether the group takes applications through a door: a group open
    /// to anyone or by request; a channel by request only, a public one
    /// being followed without one.
    pub fn has_door(&self) -> bool {
        match self.kind {
            GroupKind::Group => self.access.has_door(),
            GroupKind::Channel => self.access == Access::Request,
        }
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        let channel = match (
            d.array().map_err(|_| WireError::Malformed)?,
            d.str().map_err(|_| WireError::Malformed)?,
        ) {
            (Some(5), "roster-v2") => false,
            (Some(6), "roster-v3") => true,
            _ => return Err(WireError::Malformed),
        };
        let group = id(&mut d)?;
        let version = d.u64().map_err(|_| WireError::Malformed)?;
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        if count > MAX_ADMINS as u64 {
            return Err(WireError::TooLarge);
        }
        let mut admins = Vec::new();
        for _ in 0..count {
            admins.push(id(&mut d)?);
        }
        let access = Access::from_code(d.u8().map_err(|_| WireError::Malformed)?)?;
        let kind = if channel && d.u8().map_err(|_| WireError::Malformed)? == 1 {
            GroupKind::Channel
        } else if channel {
            return Err(WireError::NonCanonical);
        } else {
            GroupKind::Group
        };
        let roster = Self {
            group,
            version,
            admins,
            access,
            kind,
        };
        if d.position() != bytes.len() || roster.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(roster)
    }
}

/// A committer's claim that its commit is the one of a group epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitClaim {
    pub owner: [u8; 32],
    pub group_id: [u8; 32],
    pub epoch: u64,
    pub round: u32,
    /// The commit's hash.
    pub commit: [u8; 32],
    /// The owner-signed roster in force before the commit.
    pub roster: Vec<u8>,
}

impl CommitClaim {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(7);
        let _ = e.str("commit-v1");
        let _ = e.bytes(&self.owner);
        let _ = e.bytes(&self.group_id);
        let _ = e.u64(self.epoch);
        let _ = e.u32(self.round);
        let _ = e.bytes(&self.commit);
        let _ = e.bytes(&self.roster);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(7)
            || d.str().map_err(|_| WireError::Malformed)? != "commit-v1"
        {
            return Err(WireError::Malformed);
        }
        let claim = Self {
            owner: id(&mut d)?,
            group_id: id(&mut d)?,
            epoch: d.u64().map_err(|_| WireError::Malformed)?,
            round: d.u32().map_err(|_| WireError::Malformed)?,
            commit: id(&mut d)?,
            roster: d.bytes().map_err(|_| WireError::Malformed)?.to_vec(),
        };
        if d.position() != bytes.len() || claim.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(claim)
    }
}

/// A roster document signed by `owner` for group `group`.
pub fn verify_roster(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    owner: &[u8; 32],
    group: &[u8; 32],
) -> Result<Roster, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::GroupRoster || document.author() != owner {
        return Err(WireError::GroupRule);
    }
    let roster = Roster::decode(document.body())?;
    if &roster.group != group {
        return Err(WireError::GroupRule);
    }
    Ok(roster)
}

/// A verified claim: who made it, what it says, under which roster, and the
/// notary key it is recorded under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedClaim {
    pub committer: [u8; 32],
    pub claim: CommitClaim,
    pub roster: Roster,
    pub key: [u8; 32],
}

/// What a notary checks: the claim's roster is signed by the group's owner,
/// and the committer is that owner or one of its admins.
pub fn verify_claim(wire: &[u8], domain: [u8; 32], now: u64) -> Result<VerifiedClaim, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::GroupCommit {
        return Err(WireError::GroupRule);
    }
    let claim = CommitClaim::decode(document.body())?;
    let group = group_ref(&domain, &claim.owner, &claim.group_id);
    let roster = verify_roster(&claim.roster, domain, now, &claim.owner, &group)?;
    let committer = *document.author();
    if committer != claim.owner && !roster.admins.contains(&committer) {
        return Err(WireError::GroupRule);
    }
    Ok(VerifiedClaim {
        committer,
        key: claim_key(&group, claim.epoch, claim.round),
        claim,
        roster,
    })
}

/// What a reader checks of a commit: the committer's authority under the
/// current roster, and the roster after it.
pub fn check_transition(
    current: &Roster,
    next: &Roster,
    owner: [u8; 32],
    committer: [u8; 32],
    removed: &[[u8; 32]],
) -> Result<(), WireError> {
    let admin = |id: &[u8; 32]| current.admins.contains(id);
    let unchanged = next.version == current.version
        && next.admins == current.admins
        && next.access == current.access;
    let rules = next.group == current.group
        && next.kind == current.kind
        && (committer == owner || admin(&committer))
        && !removed.contains(&owner)
        && (unchanged || (committer == owner && next.version == current.version + 1))
        && (committer == owner || !removed.iter().any(admin))
        && !removed.iter().any(|id| next.admins.contains(id));
    rules.then_some(()).ok_or(WireError::GroupRule)
}

/// Ids a group's bans name at most.
pub const MAX_BANS: usize = 256;

/// An id banned from a group: its network id's digest (the `ain1` hex), and
/// whether the owner banned it, so that only the owner lifts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ban {
    pub id: [u8; 32],
    pub by_owner: bool,
}

/// How long a channel keeps its history
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retention {
    Days(u32),
    Forever,
}

impl Retention {
    /// As long as a mailbox keeps a post: no archive.
    pub const DEFAULT: Self = Self::Days(30);
    /// What a channel may keep its history for.
    pub const CHOICES: [Self; 5] = [
        Self::Days(30),
        Self::Days(90),
        Self::Days(180),
        Self::Days(365),
        Self::Forever,
    ];

    /// Days, 0 for ever.
    pub fn code(self) -> u32 {
        match self {
            Self::Days(days) => days,
            Self::Forever => 0,
        }
    }

    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Forever,
            days => Self::Days(days),
        }
    }

    /// Days, `None` for ever.
    pub fn days(self) -> Option<u32> {
        match self {
            Self::Days(days) => Some(days),
            Self::Forever => None,
        }
    }
}

/// Removals a closed channel's key tree records before it is reseeded.
pub const MAX_TREE_LOG: usize = 1000;
/// Branches of a key tree: one per member of a closed channel's team.
pub const MAX_BRANCHES: usize = 256;

/// A closed channel's key tree as its team keeps it
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 10c): the seed its
/// node keys derive from, the leaves removed since, and each member's
/// branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyTree {
    pub generation: u32,
    pub seed: [u8; 32],
    /// Leaves removed, in order: a node's version is how many lie under it.
    pub removed: Vec<u32>,
    /// Network id digests of the team and their branches.
    pub branches: Vec<([u8; 32], u8)>,
}

/// A group's bans, how long it keeps its history and, for a closed
/// channel, its key tree, as its members agree on them in the MLS group
/// context (spec/groups-v1.md): one entry per id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bans {
    pub entries: Vec<Ban>,
    /// A channel's; a group keeps the default.
    pub retention: Retention,
    pub tree: Option<KeyTree>,
}

impl Default for Bans {
    fn default() -> Self {
        Self {
            entries: vec![],
            retention: Retention::DEFAULT,
            tree: None,
        }
    }
}

impl Bans {
    /// `["bans-v1", [[id, by owner], …]]`, ascending by id; with a
    /// retention other than the default, `["bans-v2", […], days (0 for
    /// ever)]`; with a key tree, `["bans-v3", […], days, [generation, seed,
    /// [removed leaf…], [[digest, branch]…]]]`.
    pub fn encode(&self) -> Vec<u8> {
        let mut entries = self.entries.clone();
        entries.sort_by_key(|ban| ban.id);
        let version = if self.tree.is_some() {
            3
        } else if self.retention != Retention::DEFAULT {
            2
        } else {
            1
        };
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(version + 1);
        let _ = e.str(["bans-v1", "bans-v2", "bans-v3"][version as usize - 1]);
        let _ = e.array(entries.len() as u64);
        for ban in &entries {
            let _ = e.array(2);
            let _ = e.bytes(&ban.id);
            let _ = e.bool(ban.by_owner);
        }
        if version >= 2 {
            let _ = e.u32(self.retention.code());
        }
        if let Some(tree) = &self.tree {
            let _ = e.array(4);
            let _ = e.u32(tree.generation);
            let _ = e.bytes(&tree.seed);
            let _ = e.array(tree.removed.len() as u64);
            for leaf in &tree.removed {
                let _ = e.u32(*leaf);
            }
            let _ = e.array(tree.branches.len() as u64);
            for (member, branch) in &tree.branches {
                let _ = e.array(2);
                let _ = e.bytes(member);
                let _ = e.u8(*branch);
            }
        }
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        let version = match (
            d.array().map_err(|_| WireError::Malformed)?,
            d.str().map_err(|_| WireError::Malformed)?,
        ) {
            (Some(2), "bans-v1") => 1,
            (Some(3), "bans-v2") => 2,
            (Some(4), "bans-v3") => 3,
            _ => return Err(WireError::Malformed),
        };
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        if count > MAX_BANS as u64 {
            return Err(WireError::TooLarge);
        }
        let mut entries = Vec::new();
        for _ in 0..count {
            if d.array().map_err(|_| WireError::Malformed)? != Some(2) {
                return Err(WireError::Malformed);
            }
            entries.push(Ban {
                id: id(&mut d)?,
                by_owner: d.bool().map_err(|_| WireError::Malformed)?,
            });
        }
        let retention = if version >= 2 {
            Retention::from_code(d.u32().map_err(|_| WireError::Malformed)?)
        } else {
            Retention::DEFAULT
        };
        let tree = if version == 3 {
            if d.array().map_err(|_| WireError::Malformed)? != Some(4) {
                return Err(WireError::Malformed);
            }
            let generation = d.u32().map_err(|_| WireError::Malformed)?;
            let seed = id(&mut d)?;
            let count = d
                .array()
                .map_err(|_| WireError::Malformed)?
                .ok_or(WireError::Malformed)?;
            if count > MAX_TREE_LOG as u64 {
                return Err(WireError::TooLarge);
            }
            let mut removed = Vec::new();
            for _ in 0..count {
                removed.push(d.u32().map_err(|_| WireError::Malformed)?);
            }
            let count = d
                .array()
                .map_err(|_| WireError::Malformed)?
                .ok_or(WireError::Malformed)?;
            if count > MAX_BRANCHES as u64 {
                return Err(WireError::TooLarge);
            }
            let mut branches = Vec::new();
            for _ in 0..count {
                if d.array().map_err(|_| WireError::Malformed)? != Some(2) {
                    return Err(WireError::Malformed);
                }
                branches.push((id(&mut d)?, d.u8().map_err(|_| WireError::Malformed)?));
            }
            Some(KeyTree {
                generation,
                seed,
                removed,
                branches,
            })
        } else {
            None
        };
        let bans = Self {
            entries,
            retention,
            tree,
        };
        if d.position() != bytes.len()
            || bans.encode() != bytes
            || bans.entries.windows(2).any(|w| w[0].id == w[1].id)
        {
            return Err(WireError::NonCanonical);
        }
        Ok(bans)
    }

    pub fn get(&self, id: &[u8; 32]) -> Option<&Ban> {
        self.entries.iter().find(|ban| &ban.id == id)
    }
}

/// A key tree's change: made with the channel, it neither comes nor goes
/// after; within a generation, the same seed, removals and branches only
/// added, each once; a reseed starts the next generation with no removals
/// and the same branches first; branches each once.
fn tree_follows(before: Option<&KeyTree>, after: Option<&KeyTree>) -> bool {
    let (before, after) = match (before, after) {
        (None, None) => return true,
        (Some(before), Some(after)) => (before, after),
        _ => return false,
    };
    let mut numbers: Vec<u8> = after.branches.iter().map(|(_, b)| *b).collect();
    numbers.sort_unstable();
    numbers.dedup();
    let mut leaves = after.removed.clone();
    leaves.sort_unstable();
    leaves.dedup();
    if numbers.len() != after.branches.len() || leaves.len() != after.removed.len() {
        return false;
    }
    if after.generation == before.generation {
        after.seed == before.seed
            && after.removed.starts_with(&before.removed)
            && after.branches.starts_with(&before.branches)
    } else {
        before.generation.checked_add(1) == Some(after.generation)
            && after.removed.is_empty()
            && after.branches.starts_with(&before.branches)
    }
}

/// What a reader checks of a commit's change to the bans. Ids are network
/// ids' digests: `admins` of the roster before the commit, `members` of the
/// group after it. The owner's bans are marked the owner's and only the
/// owner lifts or re-marks them; an admin bans and unbans others than the
/// owner and admins; no banned id is a member.
pub fn check_bans(
    before: &Bans,
    after: &Bans,
    owner: [u8; 32],
    committer: [u8; 32],
    admins: &[[u8; 32]],
    members: &[[u8; 32]],
) -> Result<(), WireError> {
    let by_owner = committer == owner;
    let placed: Vec<&Ban> = after
        .entries
        .iter()
        .filter(|ban| !before.entries.contains(ban))
        .collect();
    let lifted: Vec<&Ban> = before
        .entries
        .iter()
        .filter(|ban| !after.entries.contains(ban))
        .collect();
    let unchanged = placed.is_empty()
        && lifted.is_empty()
        && before.retention == after.retention
        && before.tree == after.tree;
    let rules = (unchanged || by_owner || admins.contains(&committer))
        && placed.iter().all(|ban| {
            ban.id != owner && ban.by_owner == by_owner && (by_owner || !admins.contains(&ban.id))
        })
        && lifted.iter().all(|ban| by_owner || !ban.by_owner)
        && !after.entries.iter().any(|ban| members.contains(&ban.id))
        && tree_follows(before.tree.as_ref(), after.tree.as_ref());
    rules.then_some(()).ok_or(WireError::GroupRule)
}

/// Who reads a group's messages and how one joins it
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 5): its members only,
/// by invitation; anyone reads and anyone joins through its door; its
/// members only, and one joins through its door once let in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Private,
    Public,
    Request,
}

impl Access {
    fn code(self) -> u8 {
        match self {
            Self::Private => 0,
            Self::Public => 1,
            Self::Request => 2,
        }
    }
    fn from_code(code: u8) -> Result<Self, WireError> {
        match code {
            0 => Ok(Self::Private),
            1 => Ok(Self::Public),
            2 => Ok(Self::Request),
            _ => Err(WireError::Malformed),
        }
    }
    /// Whether the group takes applications through its door.
    pub fn has_door(self) -> bool {
        self != Self::Private
    }
}

/// Characters a group name in a door card takes at most.
const MAX_DOOR_NAME: usize = 256;

/// A group's door (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 5):
/// signed by its owner or an admin, with the owner-signed roster behind it,
/// in the group's door mailbox for anyone who knows `G`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorCard {
    pub group_id: [u8; 32],
    /// The owner's root key.
    pub owner: [u8; 32],
    /// The owner-signed roster in force.
    pub roster: Vec<u8>,
    pub name: String,
    /// The X25519 key applications are sealed to.
    pub door_key: [u8; 32],
}

impl DoorCard {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(6);
        let _ = e.str("door-v1");
        let _ = e.bytes(&self.group_id);
        let _ = e.bytes(&self.owner);
        let _ = e.bytes(&self.roster);
        let _ = e.str(&self.name);
        let _ = e.bytes(&self.door_key);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(6)
            || d.str().map_err(|_| WireError::Malformed)? != "door-v1"
        {
            return Err(WireError::Malformed);
        }
        let group_id = id(&mut d)?;
        let owner = id(&mut d)?;
        let roster = d.bytes().map_err(|_| WireError::Malformed)?.to_vec();
        let name = d.str().map_err(|_| WireError::Malformed)?.to_owned();
        if name.len() > MAX_DOOR_NAME {
            return Err(WireError::TooLarge);
        }
        let door_key = id(&mut d)?;
        let card = Self {
            group_id,
            owner,
            roster,
            name,
            door_key,
        };
        if d.position() != bytes.len() || card.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(card)
    }
}

/// A door card as an applicant takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedDoor {
    pub card: DoorCard,
    pub access: Access,
    /// The owner or admin that signed it.
    pub signer: [u8; 32],
}

/// The door card of group `G`: its roster is signed by the owner it names,
/// for this group, the group has a door, and the signer is the owner or one
/// of its admins.
pub fn verify_door_card(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<VerifiedDoor, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::GroupDoor {
        return Err(WireError::GroupRule);
    }
    let card = DoorCard::decode(document.body())?;
    if group_ref(&domain, &card.owner, &card.group_id) != *group {
        return Err(WireError::GroupRule);
    }
    let roster = verify_roster(&card.roster, domain, now, &card.owner, group)?;
    let signer = *document.author();
    if !roster.has_door() || (signer != card.owner && !roster.admins.contains(&signer)) {
        return Err(WireError::GroupRule);
    }
    Ok(VerifiedDoor {
        card,
        access: roster.access,
        signer,
    })
}

/// Removals a public roster names at most: the latest removals and every
/// ban.
pub const MAX_PUBLIC_REMOVED: usize = 1024 + MAX_BANS;
/// A removal that no certificate outlives: a ban.
pub const BANNED: u64 = u64::MAX;

/// A member's certificate: whoever let it in (the owner, or an admin of the
/// owner-signed roster inside) says it is a member of `G` since `epoch`.
/// A post carries its author's, so readers need no list of the members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberCert {
    /// `G`.
    pub group: [u8; 32],
    /// The member's network id digest.
    pub member: [u8; 32],
    /// The epoch it joined at: a removal after it ends the certificate.
    pub epoch: u64,
    /// The owner-signed roster in force; empty when the owner signs.
    pub roster: Vec<u8>,
}

impl MemberCert {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(5);
        let _ = e.str("member-v1");
        let _ = e.bytes(&self.group);
        let _ = e.bytes(&self.member);
        let _ = e.u64(self.epoch);
        let _ = e.bytes(&self.roster);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(5)
            || d.str().map_err(|_| WireError::Malformed)? != "member-v1"
        {
            return Err(WireError::Malformed);
        }
        let cert = Self {
            group: id(&mut d)?,
            member: id(&mut d)?,
            epoch: d.u64().map_err(|_| WireError::Malformed)?,
            roster: d.bytes().map_err(|_| WireError::Malformed)?.to_vec(),
        };
        if d.position() != bytes.len() || cert.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(cert)
    }
}

/// A certificate as a reader takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedCert {
    pub member: [u8; 32],
    pub epoch: u64,
    /// Who signed it.
    pub issuer: [u8; 32],
    /// The owner, by the roster inside, or the issuer when there is none:
    /// the reader checks it is the owner it knows.
    pub owner: [u8; 32],
    /// The roster inside, when an admin signed.
    pub roster: Option<Roster>,
}

/// A certificate for `group`: an owner's, or an admin's under the roster
/// inside, which its own author signed for this group.
pub fn verify_member_cert(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<VerifiedCert, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::GroupMember {
        return Err(WireError::GroupRule);
    }
    let cert = MemberCert::decode(document.body())?;
    let issuer = *document.author();
    if cert.group != *group {
        return Err(WireError::GroupRule);
    }
    let (owner, roster) = if cert.roster.is_empty() {
        (issuer, None)
    } else {
        let signed = VerifiedDocument::decode(&cert.roster, domain, now)?;
        if signed.kind() != DocumentKind::GroupRoster {
            return Err(WireError::GroupRule);
        }
        let roster = Roster::decode(signed.body())?;
        let owner = *signed.author();
        if roster.group != *group || (issuer != owner && !roster.admins.contains(&issuer)) {
            return Err(WireError::GroupRule);
        }
        (owner, Some(roster))
    };
    Ok(VerifiedCert {
        member: cert.member,
        epoch: cert.epoch,
        issuer,
        owner,
        roster,
    })
}

/// An open group's post: written by a member, signed by its root key, in
/// the clear in the group's public mailbox, with its author's certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicPost {
    /// `G`.
    pub group: [u8; 32],
    /// The epoch its author wrote it in.
    pub epoch: u64,
    /// The sending operation's SHA-256: two equal posts stay two.
    pub operation: [u8; 32],
    pub text: String,
    /// The author's `MemberCert`, signed.
    pub membership: Vec<u8>,
}

impl PublicPost {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(6);
        let _ = e.str("post-v2");
        let _ = e.bytes(&self.group);
        let _ = e.u64(self.epoch);
        let _ = e.bytes(&self.operation);
        let _ = e.str(&self.text);
        let _ = e.bytes(&self.membership);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(6)
            || d.str().map_err(|_| WireError::Malformed)? != "post-v2"
        {
            return Err(WireError::Malformed);
        }
        let post = Self {
            group: id(&mut d)?,
            epoch: d.u64().map_err(|_| WireError::Malformed)?,
            operation: id(&mut d)?,
            text: d.str().map_err(|_| WireError::Malformed)?.to_owned(),
            membership: d.bytes().map_err(|_| WireError::Malformed)?.to_vec(),
        };
        if d.position() != bytes.len() || post.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(post)
    }
}

/// What readers of an open group check its posts against, signed by its
/// owner or an admin: the owner-signed roster in force and who may no
/// longer write — the latest removals and every ban.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicRoster {
    pub group: [u8; 32],
    pub epoch: u64,
    /// The owner-signed roster in force at the epoch.
    pub roster: Vec<u8>,
    /// Network id digests with the last epoch each was a member in: no
    /// certificate of that epoch or before holds. A ban is `BANNED`.
    pub removed: Vec<([u8; 32], u64)>,
    /// How far back followers read: a day for a group, the channel's
    /// retention for a channel.
    pub retention: Retention,
}

impl PublicRoster {
    pub fn encode(&self) -> Vec<u8> {
        let mut removed = self.removed.clone();
        removed.sort();
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(6);
        let _ = e.str("public-roster-v2");
        let _ = e.bytes(&self.group);
        let _ = e.u64(self.epoch);
        let _ = e.bytes(&self.roster);
        let _ = e.array(removed.len() as u64);
        for (member, epoch) in &removed {
            let _ = e.array(2);
            let _ = e.bytes(member);
            let _ = e.u64(*epoch);
        }
        let _ = e.u32(self.retention.code());
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(6)
            || d.str().map_err(|_| WireError::Malformed)? != "public-roster-v2"
        {
            return Err(WireError::Malformed);
        }
        let group = id(&mut d)?;
        let epoch = d.u64().map_err(|_| WireError::Malformed)?;
        let roster = d.bytes().map_err(|_| WireError::Malformed)?.to_vec();
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        if count > MAX_PUBLIC_REMOVED as u64 {
            return Err(WireError::TooLarge);
        }
        let mut removed = Vec::new();
        for _ in 0..count {
            if d.array().map_err(|_| WireError::Malformed)? != Some(2) {
                return Err(WireError::Malformed);
            }
            removed.push((id(&mut d)?, d.u64().map_err(|_| WireError::Malformed)?));
        }
        let retention = Retention::from_code(d.u32().map_err(|_| WireError::Malformed)?);
        let public = Self {
            group,
            epoch,
            roster,
            removed,
            retention,
        };
        if d.position() != bytes.len()
            || public.encode() != bytes
            || public.removed.windows(2).any(|w| w[0].0 == w[1].0)
        {
            return Err(WireError::NonCanonical);
        }
        Ok(public)
    }
}

/// A public roster as a reader takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPublicRoster {
    /// The root key that signed the roster behind it: the reader checks it
    /// is the owner it follows.
    pub owner: [u8; 32],
    pub committer: [u8; 32],
    pub epoch: u64,
    pub access: Access,
    /// The roster behind it.
    pub roster: Roster,
    pub removed: Vec<([u8; 32], u64)>,
    pub retention: Retention,
}

/// A public roster of `group`: the roster inside is signed by its own
/// author (the owner, which the caller checks) for this group, and the
/// committer is that owner or one of its admins.
pub fn verify_public_roster(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<VerifiedPublicRoster, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::PublicRoster {
        return Err(WireError::GroupRule);
    }
    let public = PublicRoster::decode(document.body())?;
    let signed = VerifiedDocument::decode(&public.roster, domain, now)?;
    if public.group != *group || signed.kind() != DocumentKind::GroupRoster {
        return Err(WireError::GroupRule);
    }
    let roster = Roster::decode(signed.body())?;
    let owner = *signed.author();
    let committer = *document.author();
    if roster.group != *group || (committer != owner && !roster.admins.contains(&committer)) {
        return Err(WireError::GroupRule);
    }
    Ok(VerifiedPublicRoster {
        owner,
        committer,
        epoch: public.epoch,
        access: roster.access,
        roster,
        removed: public.removed,
        retention: public.retention,
    })
}

/// A post as a reader takes it. Whether its certificate still holds —
/// its owner the one the reader knows, its issuer an admin of the newest
/// roster it knows, its author not removed since — is the reader's check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPost {
    pub author: [u8; 32],
    /// The owner the certificate stands on.
    pub owner: [u8; 32],
    /// Who signed the certificate.
    pub issuer: [u8; 32],
    /// The certificate's epoch.
    pub since: u64,
    /// The roster inside the certificate, when an admin signed it.
    pub roster: Option<Roster>,
    pub epoch: u64,
    pub text: String,
    pub issued_at: u64,
}

pub fn verify_public_post(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<VerifiedPost, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::PublicPost {
        return Err(WireError::GroupRule);
    }
    let post = PublicPost::decode(document.body())?;
    if post.group != *group {
        return Err(WireError::GroupRule);
    }
    let author = *document.author();
    let cert = verify_member_cert(&post.membership, domain, now, group)?;
    if cert.member != <[u8; 32]>::from(Sha256::digest(author)) {
        return Err(WireError::GroupRule);
    }
    Ok(VerifiedPost {
        author,
        owner: cert.owner,
        issuer: cert.issuer,
        since: cert.epoch,
        roster: cert.roster,
        epoch: post.epoch,
        text: post.text,
        issued_at: document.issued_at(),
    })
}

/// Posts an archive part holds at most.
pub const MAX_ARCHIVE_POSTS: usize = 1024;

/// A part of a channel's archive (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
/// part 10b): its posts as they are, each signed by its author with its
/// certificate; nobody signs the part, and readers check every post.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivePart {
    /// `C`.
    pub group: [u8; 32],
    /// Post documents, oldest first.
    pub posts: Vec<Vec<u8>>,
}

impl ArchivePart {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(3);
        let _ = e.str("archive-v1");
        let _ = e.bytes(&self.group);
        let _ = e.array(self.posts.len() as u64);
        for post in &self.posts {
            let _ = e.bytes(post);
        }
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(3)
            || d.str().map_err(|_| WireError::Malformed)? != "archive-v1"
        {
            return Err(WireError::Malformed);
        }
        let group = id(&mut d)?;
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        if count > MAX_ARCHIVE_POSTS as u64 {
            return Err(WireError::TooLarge);
        }
        let mut posts = Vec::new();
        for _ in 0..count {
            posts.push(d.bytes().map_err(|_| WireError::Malformed)?.to_vec());
        }
        let part = Self { group, posts };
        if d.position() != bytes.len() || part.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(part)
    }
}

/// New keys of a closed channel's key tree, signed by its owner or an
/// admin of the roster inside (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
/// part 10c). A removal's: the new keys of the removed leaf's path, each
/// sealed under the key beside it and under the new key below it, in order
/// from the leaf up. A reseed's: the next generation's key of each node
/// sealed under its key in this one. A reseed onto subscribers' own keys
/// (`personal`, the ephemeral key it seals with; the owner's alone): each
/// leaf's new key sealed to its subscriber's key, each node's under the new
/// keys of its children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyUpdate {
    /// `C`.
    pub group: [u8; 32],
    /// The generation the sealed keys are under.
    pub generation: u32,
    /// The root's version the update starts from.
    pub version: u32,
    /// A removal's leaf, or none for a reseed.
    pub removed: Option<u32>,
    /// A removal's sealed keys in order, or a reseed's by node.
    pub entries: Vec<(u64, Vec<u8>)>,
    pub roster: Vec<u8>,
    /// A reseed onto subscribers' own keys: the X25519 key it seals with.
    pub personal: Option<[u8; 32]>,
}

impl KeyUpdate {
    /// `["keys-v1", C, generation, version, removed or null, entries,
    /// roster]`; a reseed onto subscribers' keys is `keys-v2` with the
    /// ephemeral key last.
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(if self.personal.is_some() { 8 } else { 7 });
        let _ = e.str(if self.personal.is_some() {
            "keys-v2"
        } else {
            "keys-v1"
        });
        let _ = e.bytes(&self.group);
        let _ = e.u32(self.generation);
        let _ = e.u32(self.version);
        match self.removed {
            Some(leaf) => {
                let _ = e.u32(leaf);
            }
            None => {
                let _ = e.null();
            }
        }
        let _ = e.array(self.entries.len() as u64);
        for (node, sealed) in &self.entries {
            // A removal's entries are in order: no node is named.
            if self.removed.is_some() {
                let _ = e.bytes(sealed);
            } else {
                let _ = e.array(2);
                let _ = e.u64(*node);
                let _ = e.bytes(sealed);
            }
        }
        let _ = e.bytes(&self.roster);
        if let Some(key) = &self.personal {
            let _ = e.bytes(key);
        }
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        let personal = match (
            d.array().map_err(|_| WireError::Malformed)?,
            d.str().map_err(|_| WireError::Malformed)?,
        ) {
            (Some(7), "keys-v1") => false,
            (Some(8), "keys-v2") => true,
            _ => return Err(WireError::Malformed),
        };
        let group = id(&mut d)?;
        let generation = d.u32().map_err(|_| WireError::Malformed)?;
        let version = d.u32().map_err(|_| WireError::Malformed)?;
        let removed =
            if d.datatype().map_err(|_| WireError::Malformed)? == minicbor::data::Type::Null {
                d.skip().map_err(|_| WireError::Malformed)?;
                None
            } else {
                Some(d.u32().map_err(|_| WireError::Malformed)?)
            };
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        if count > 4096 {
            return Err(WireError::TooLarge);
        }
        let mut entries = Vec::new();
        for _ in 0..count {
            if removed.is_some() {
                entries.push((0, d.bytes().map_err(|_| WireError::Malformed)?.to_vec()));
            } else {
                if d.array().map_err(|_| WireError::Malformed)? != Some(2) {
                    return Err(WireError::Malformed);
                }
                let node = d.u64().map_err(|_| WireError::Malformed)?;
                entries.push((node, d.bytes().map_err(|_| WireError::Malformed)?.to_vec()));
            }
        }
        let roster = d.bytes().map_err(|_| WireError::Malformed)?.to_vec();
        let personal = if personal {
            // Onto subscribers' keys comes a reseed, never a removal.
            if removed.is_some() {
                return Err(WireError::Malformed);
            }
            Some(id(&mut d)?)
        } else {
            None
        };
        let update = Self {
            group,
            generation,
            version,
            removed,
            entries,
            roster,
            personal,
        };
        if d.position() != bytes.len() || update.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(update)
    }
}

/// A closed channel's subscriber's own X25519 key for leaf `leaf` of
/// channel `group` (`C`), signed by the subscriber: the owner seals new
/// keys to it when an admin leaves the team.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriberKey {
    pub group: [u8; 32],
    pub leaf: u32,
    pub key: [u8; 32],
}

impl SubscriberKey {
    /// `["subscriber-v1", C, leaf, key]`.
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(4);
        let _ = e.str("subscriber-v1");
        let _ = e.bytes(&self.group);
        let _ = e.u32(self.leaf);
        let _ = e.bytes(&self.key);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(4)
            || d.str().map_err(|_| WireError::Malformed)? != "subscriber-v1"
        {
            return Err(WireError::Malformed);
        }
        let key = Self {
            group: id(&mut d)?,
            leaf: d.u32().map_err(|_| WireError::Malformed)?,
            key: id(&mut d)?,
        };
        if d.position() != bytes.len() || key.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(key)
    }
}

/// A subscriber's key of channel `group`, and the root key that signed it.
pub fn verify_subscriber_key(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<(SubscriberKey, [u8; 32]), WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::ChannelSubscriber {
        return Err(WireError::GroupRule);
    }
    let key = SubscriberKey::decode(document.body())?;
    if key.group != *group {
        return Err(WireError::GroupRule);
    }
    Ok((key, *document.author()))
}

/// A key update as a subscriber takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedKeyUpdate {
    pub update: KeyUpdate,
    pub signer: [u8; 32],
    /// The owner, by the roster inside.
    pub owner: [u8; 32],
    pub roster: Roster,
}

/// A key update of `group` signed by the owner of the roster inside or one
/// of its admins.
pub fn verify_key_update(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    group: &[u8; 32],
) -> Result<VerifiedKeyUpdate, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::ChannelKeys {
        return Err(WireError::GroupRule);
    }
    let update = KeyUpdate::decode(document.body())?;
    let signed = VerifiedDocument::decode(&update.roster, domain, now)?;
    if update.group != *group || signed.kind() != DocumentKind::GroupRoster {
        return Err(WireError::GroupRule);
    }
    let roster = Roster::decode(signed.body())?;
    let owner = *signed.author();
    let signer = *document.author();
    if roster.group != *group
        || (signer != owner && !roster.admins.contains(&signer))
        // A reseed onto subscribers' keys is the owner's alone.
        || (update.personal.is_some() && signer != owner)
    {
        return Err(WireError::GroupRule);
    }
    Ok(VerifiedKeyUpdate {
        update,
        signer,
        owner,
        roster,
    })
}
